//! Live workspace + tab-manager model. The Swift version stores a lot of
//! AppKit-bound state (`NSWindow` hooks, Combine publishers, git/PR
//! probe caches). Here we keep only the **authoritative data** that the
//! Tauri frontend has to render or persist. Terminal PTY state, port
//! scanner output and friends hang off panels in later phases.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::WorkspaceSnapshot;
use crate::bonsplit::{PaneId, SplitOrientation, SplitViewController, TabId, TabItem};

/// Unique identifier for a panel hosted inside a workspace. Independent
/// from [`TabId`] so that a bonsplit tab can be re-bound to another
/// panel (e.g. when remoting into a session).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PanelId(pub Uuid);

impl PanelId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
    pub fn uuid(self) -> Uuid {
        self.0
    }
}

impl Default for PanelId {
    fn default() -> Self {
        Self::new()
    }
}

/// Discriminant for the content inside a panel. Matches Swift's
/// `PanelType` enum. Each variant lands as a real type in Phase 6+.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PanelKind {
    Terminal,
    Browser,
    Markdown,
}

impl PanelKind {
    pub fn tab_kind(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Browser => "browser",
            Self::Markdown => "markdown",
        }
    }

    pub fn default_title(self) -> &'static str {
        match self {
            Self::Terminal => "Terminal",
            Self::Browser => "Browser",
            Self::Markdown => "Markdown",
        }
    }
}

/// Metadata shared by every panel kind. Extra per-kind state is tracked
/// under [`Panel::data`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelMetadata {
    pub title: Option<String>,
    pub custom_title: Option<String>,
    pub directory: Option<String>,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default)]
    pub is_manually_unread: bool,
    #[serde(default)]
    pub git_branch: Option<GitBranchState>,
    #[serde(default)]
    pub pull_request: Option<PullRequestState>,
    #[serde(default)]
    pub listening_ports: Vec<u16>,
    #[serde(default)]
    pub tty_name: Option<String>,
}

impl Default for PanelMetadata {
    fn default() -> Self {
        Self {
            title: None,
            custom_title: None,
            directory: None,
            is_pinned: false,
            is_manually_unread: false,
            git_branch: None,
            pull_request: None,
            listening_ports: Vec::new(),
            tty_name: None,
        }
    }
}

/// Per-kind panel data. Variants carry only serde-friendly fields so a
/// panel can round-trip through [`super::session`] without losing state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PanelData {
    Terminal {
        #[serde(default)]
        working_directory: Option<String>,
        #[serde(default)]
        scrollback: Option<String>,
    },
    Browser {
        #[serde(default)]
        url: Option<String>,
        #[serde(default)]
        profile_id: Option<Uuid>,
        #[serde(default)]
        should_render_webview: bool,
        #[serde(default = "default_page_zoom")]
        page_zoom: f64,
        #[serde(default)]
        developer_tools_visible: bool,
        #[serde(default)]
        back_history: Vec<String>,
        #[serde(default)]
        forward_history: Vec<String>,
    },
    Markdown {
        file_path: String,
    },
}

fn default_page_zoom() -> f64 {
    1.0
}

/// Panel = metadata + per-kind data. Mirrors the Swift `Panel` protocol
/// (`TerminalPanel`, `BrowserPanel`, `MarkdownPanel`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Panel {
    pub id: PanelId,
    pub kind: PanelKind,
    #[serde(default)]
    pub metadata: PanelMetadata,
    pub data: PanelData,
}

impl Panel {
    pub fn new_terminal<S: Into<Option<String>>>(directory: S) -> Self {
        let directory = directory.into();
        Self {
            id: PanelId::new(),
            kind: PanelKind::Terminal,
            metadata: PanelMetadata {
                directory: directory.clone(),
                ..Default::default()
            },
            data: PanelData::Terminal {
                working_directory: directory,
                scrollback: None,
            },
        }
    }

    pub fn new_browser<S: Into<Option<String>>>(url: S) -> Self {
        Self {
            id: PanelId::new(),
            kind: PanelKind::Browser,
            metadata: PanelMetadata::default(),
            data: PanelData::Browser {
                url: url.into(),
                profile_id: None,
                should_render_webview: true,
                page_zoom: default_page_zoom(),
                developer_tools_visible: false,
                back_history: Vec::new(),
                forward_history: Vec::new(),
            },
        }
    }

    pub fn new_markdown<S: Into<String>>(file_path: S) -> Self {
        Self {
            id: PanelId::new(),
            kind: PanelKind::Markdown,
            metadata: PanelMetadata::default(),
            data: PanelData::Markdown {
                file_path: file_path.into(),
            },
        }
    }
}

/// Sidebar "git branch" summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitBranchState {
    pub branch: String,
    #[serde(default)]
    pub is_dirty: bool,
}

/// Sidebar "pull request" summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PullRequestState {
    pub number: u32,
    pub url: String,
    /// `"open" | "merged" | "closed"` — kept as string to match Swift.
    pub status: String,
    pub branch: String,
}

/// Reason the UI should flash a workspace tab for attention (notifications,
/// command completion, etc). Matches Swift
/// `WorkspaceAttentionFlashReason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceAttentionFlash {
    CommandComplete,
    Notification,
    AgentStatus,
}

/// Remote connection state for the workspace. Simplified from the
/// Swift enum for Phase 4 — only the top-level lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceRemoteConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// A single workspace ("tab" in the top-level tab manager). Hosts a
/// split tree + its panels.
#[derive(Debug, Clone)]
pub struct Workspace {
    pub id: Uuid,
    pub title: String,
    pub custom_title: Option<String>,
    pub custom_description: Option<String>,
    pub custom_color: Option<String>,
    pub is_pinned: bool,
    pub current_directory: String,
    pub preferred_browser_profile_id: Option<Uuid>,
    pub port_ordinal: u32,
    pub bonsplit: SplitViewController,
    /// Panels keyed by their [`PanelId`] — cannot use `HashMap` because
    /// the Tauri frontend expects deterministic iteration for diffing.
    pub panels: BTreeMap<PanelId, Panel>,
    /// Tab → panel linkage. Each bonsplit [`TabId`] in the workspace
    /// points to exactly one panel.
    pub tab_to_panel: BTreeMap<TabId, PanelId>,
    pub git_branch: Option<GitBranchState>,
    pub pull_request: Option<PullRequestState>,
    pub remote_state: WorkspaceRemoteConnectionState,
}

impl Workspace {
    /// New empty workspace with a blank split root and no panels.
    pub fn new<T: Into<String>, D: Into<String>>(title: T, current_directory: D) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            custom_title: None,
            custom_description: None,
            custom_color: None,
            is_pinned: false,
            current_directory: current_directory.into(),
            preferred_browser_profile_id: None,
            port_ordinal: 0,
            bonsplit: SplitViewController::new_empty(),
            panels: BTreeMap::new(),
            tab_to_panel: BTreeMap::new(),
            git_branch: None,
            pull_request: None,
            remote_state: WorkspaceRemoteConnectionState::Disconnected,
        }
    }

    /// Convenience constructor for the Tauri shell: create a workspace
    /// with a starter tab so the split UI has something visible before
    /// the user creates more panes.
    pub fn with_starting_tab<T: Into<String>, D: Into<String>, U: Into<String>>(
        title: T,
        current_directory: D,
        tab_title: U,
    ) -> Self {
        let mut workspace = Self::new(title, current_directory);
        let pane_id = workspace
            .bonsplit
            .focused_pane_id()
            .expect("new workspace always has a focused pane");
        let _ = workspace.add_tab_to_pane(pane_id, tab_title);
        workspace
    }

    /// Focused pane id, if any.
    pub fn focused_pane_id(&self) -> Option<PaneId> {
        self.bonsplit.focused_pane_id()
    }

    /// Snapshot the workspace into a UI-friendly recursive tree. This
    /// is what the Tauri frontend consumes.
    pub fn snapshot(&self) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            id: self.id,
            title: self.title.clone(),
            custom_title: self.custom_title.clone(),
            custom_description: self.custom_description.clone(),
            custom_color: self.custom_color.clone(),
            is_pinned: self.is_pinned,
            current_directory: self.current_directory.clone(),
            preferred_browser_profile_id: self.preferred_browser_profile_id,
            port_ordinal: self.port_ordinal,
            focused_pane_id: self.bonsplit.focused_pane_id(),
            zoomed_pane_id: self.bonsplit.zoomed_pane_id(),
            tree: self.bonsplit.tree_snapshot(),
        }
    }

    /// Attach a panel and bind it to a fresh tab in the focused pane.
    /// Returns the new tab + panel ids.
    pub fn add_panel_in_focused_pane(&mut self, panel: Panel) -> Option<(TabId, PanelId)> {
        let pane_id = self.bonsplit.focused_pane_id()?;
        self.add_panel_in_pane(pane_id, panel)
    }

    /// Attach a panel to an explicit pane.
    pub fn add_panel_in_pane(
        &mut self,
        pane_id: PaneId,
        panel: Panel,
    ) -> Option<(TabId, PanelId)> {
        self.bonsplit.focus_pane(pane_id);
        let panel_id = panel.id;
        let title = panel
            .metadata
            .custom_title
            .clone()
            .or_else(|| panel.metadata.title.clone())
            .unwrap_or_else(|| panel.kind.default_title().to_string());
        let mut tab = TabItem::new(title);
        tab.kind = Some(panel.kind.tab_kind().to_string());
        let tab_id = tab.id;
        self.bonsplit.add_tab(tab, Some(pane_id), None);
        self.panels.insert(panel_id, panel);
        self.tab_to_panel.insert(tab_id, panel_id);
        Some((tab_id, panel_id))
    }

    /// Add a UI tab to the focused pane. This is the Phase 5 path used
    /// by the Tauri shell before terminal panels exist.
    pub fn add_tab_to_focused_pane<S: Into<String>>(
        &mut self,
        title: S,
    ) -> Option<(TabId, PaneId)> {
        let pane_id = self.bonsplit.focused_pane_id()?;
        self.add_tab_to_pane(pane_id, title)
    }

    /// Add a UI tab to an explicit pane.
    pub fn add_tab_to_pane<S: Into<String>>(
        &mut self,
        pane_id: PaneId,
        title: S,
    ) -> Option<(TabId, PaneId)> {
        self.add_tab_to_pane_with_kind(pane_id, title, None)
    }

    /// Add a UI tab to an explicit pane with a logical kind.
    pub fn add_tab_to_pane_with_kind<S: Into<String>>(
        &mut self,
        pane_id: PaneId,
        title: S,
        kind: Option<String>,
    ) -> Option<(TabId, PaneId)> {
        if self.bonsplit.root().find_pane(pane_id).is_none() {
            return None;
        }
        self.bonsplit.focus_pane(pane_id);
        let mut tab = TabItem::new(title);
        tab.kind = kind;
        let tab_id = tab.id;
        self.bonsplit.add_tab(tab, Some(pane_id), None);
        Some((tab_id, pane_id))
    }

    /// Add a tab to the focused pane with a logical kind.
    pub fn add_tab_to_focused_pane_with_kind<S: Into<String>>(
        &mut self,
        title: S,
        kind: Option<String>,
    ) -> Option<(TabId, PaneId)> {
        let pane_id = self.bonsplit.focused_pane_id()?;
        self.add_tab_to_pane_with_kind(pane_id, title, kind)
    }

    /// Split a pane and optionally place a fresh tab in the new side.
    pub fn split_pane(
        &mut self,
        pane_id: PaneId,
        orientation: SplitOrientation,
        tab_title: Option<String>,
        insert_first: bool,
    ) -> Option<PaneId> {
        self.split_pane_with_kind(pane_id, orientation, tab_title, None, insert_first)
    }

    /// Split a pane and optionally place a typed tab in the new side.
    pub fn split_pane_with_kind(
        &mut self,
        pane_id: PaneId,
        orientation: SplitOrientation,
        tab_title: Option<String>,
        tab_kind: Option<String>,
        insert_first: bool,
    ) -> Option<PaneId> {
        let new_tab = tab_title.map(TabItem::new);
        let new_tab = new_tab.map(|mut tab| {
            tab.kind = tab_kind;
            tab
        });
        match (new_tab, insert_first) {
            (Some(tab), true) => self
                .bonsplit
                .split_pane_with_tab(pane_id, orientation, tab, true),
            (Some(tab), false) => self
                .bonsplit
                .split_pane_with_tab(pane_id, orientation, tab, false),
            (None, _) => self.bonsplit.split_pane(pane_id, orientation, None),
        }
    }

    /// Close a pane.
    pub fn close_pane(&mut self, pane_id: PaneId) -> bool {
        self.bonsplit.close_pane(pane_id)
    }

    /// Close a tab from an explicit pane.
    pub fn close_tab_in_pane(&mut self, tab_id: TabId, pane_id: PaneId) -> bool {
        self.close_tab(tab_id, pane_id)
    }

    /// Move or reorder a tab. A same-pane move is treated as a reorder.
    pub fn move_tab(
        &mut self,
        tab_id: TabId,
        source_pane: PaneId,
        target_pane: PaneId,
        index: Option<usize>,
    ) -> bool {
        self.bonsplit.move_tab(tab_id, source_pane, target_pane, index)
    }

    /// Reorder a tab within its pane.
    pub fn reorder_tab_in_pane(
        &mut self,
        pane_id: PaneId,
        from_index: usize,
        to_index: usize,
    ) -> bool {
        let Some(pane) = self.bonsplit.root().find_pane(pane_id) else {
            return false;
        };
        let Some(tab_id) = pane.tabs.get(from_index).map(|tab| tab.id) else {
            return false;
        };
        self.bonsplit
            .move_tab(tab_id, pane_id, pane_id, Some(to_index))
    }

    /// Update a split divider.
    pub fn set_divider_position(&mut self, split_id: uuid::Uuid, position: f64) -> bool {
        self.bonsplit.set_divider_position(split_id, position)
    }

    /// Close a tab + its bound panel.
    pub fn close_tab(&mut self, tab_id: TabId, pane_id: PaneId) -> bool {
        if !self.bonsplit.close_tab(tab_id, pane_id) {
            return false;
        }
        if let Some(panel_id) = self.tab_to_panel.remove(&tab_id) {
            self.panels.remove(&panel_id);
        }
        true
    }

    /// Currently focused panel (if any).
    pub fn focused_panel(&self) -> Option<&Panel> {
        let pane_id = self.bonsplit.focused_pane_id()?;
        let pane = self.bonsplit.root().find_pane(pane_id)?;
        let tab_id = pane.selected_tab_id?;
        let panel_id = self.tab_to_panel.get(&tab_id)?;
        self.panels.get(panel_id)
    }

    /// Mutable focused panel.
    pub fn focused_panel_mut(&mut self) -> Option<&mut Panel> {
        let pane_id = self.bonsplit.focused_pane_id()?;
        let tab_id = self
            .bonsplit
            .root()
            .find_pane(pane_id)
            .and_then(|p| p.selected_tab_id)?;
        let panel_id = *self.tab_to_panel.get(&tab_id)?;
        self.panels.get_mut(&panel_id)
    }
}

/// Top-level tab manager — the ordered list of workspaces + a selected
/// index. Mirrors the published state of Swift `TabManager`.
#[derive(Debug, Clone, Default)]
pub struct TabManager {
    pub workspaces: Vec<Workspace>,
    pub selected_workspace_id: Option<Uuid>,
}

impl TabManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a workspace and select it.
    pub fn push_workspace(&mut self, workspace: Workspace) -> Uuid {
        let id = workspace.id;
        self.workspaces.push(workspace);
        self.selected_workspace_id = Some(id);
        id
    }

    /// Remove a workspace by id. Returns `true` if something was
    /// removed. Selection falls through to the next workspace.
    pub fn close_workspace(&mut self, id: Uuid) -> bool {
        let Some(index) = self.workspaces.iter().position(|w| w.id == id) else {
            return false;
        };
        self.workspaces.remove(index);
        if self.selected_workspace_id == Some(id) {
            self.selected_workspace_id = self
                .workspaces
                .get(index)
                .or_else(|| self.workspaces.get(index.saturating_sub(1)))
                .map(|w| w.id);
        }
        true
    }

    /// Select a workspace by id. Noop if the id isn't present.
    pub fn select_workspace(&mut self, id: Uuid) {
        if self.workspaces.iter().any(|w| w.id == id) {
            self.selected_workspace_id = Some(id);
        }
    }

    /// Borrow the currently selected workspace.
    pub fn selected_workspace(&self) -> Option<&Workspace> {
        let id = self.selected_workspace_id?;
        self.workspaces.iter().find(|w| w.id == id)
    }

    /// Mutably borrow the currently selected workspace.
    pub fn selected_workspace_mut(&mut self) -> Option<&mut Workspace> {
        let id = self.selected_workspace_id?;
        self.workspaces.iter_mut().find(|w| w.id == id)
    }

    /// Mutably borrow a workspace by id.
    pub fn workspace_mut(&mut self, id: Uuid) -> Option<&mut Workspace> {
        self.workspaces.iter_mut().find(|w| w.id == id)
    }

    /// Move a workspace tab from one index to another. Mirrors the
    /// drag-reorder semantics from Swift `TabManager.moveWorkspace`.
    pub fn move_workspace(&mut self, from: usize, to: usize) {
        if from >= self.workspaces.len() || to > self.workspaces.len() || from == to {
            return;
        }
        let ws = self.workspaces.remove(from);
        let destination = if to > from { to - 1 } else { to };
        let destination = destination.min(self.workspaces.len());
        self.workspaces.insert(destination, ws);
    }
}
