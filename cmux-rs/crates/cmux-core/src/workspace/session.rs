//! Session snapshot schema. Wire-compatible with
//! [Sources/SessionPersistence.swift](../../../../../../Sources/SessionPersistence.swift)
//! so a workspace saved by the Swift build opens in the Rust build
//! (and vice-versa). The `#[serde(rename = "...")]` camelCase names
//! match Swift's default `JSONEncoder` output exactly.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Schema version. Bump when the fields change in a non-additive way.
pub struct SessionSnapshotSchema;

impl SessionSnapshotSchema {
    /// Current schema version (matches Swift `SessionSnapshotSchema.currentVersion`).
    pub const CURRENT_VERSION: u32 = 1;
}

/// Policy knobs (sidebar width, max workspaces per window, etc.).
pub struct SessionPersistencePolicy;

impl SessionPersistencePolicy {
    pub const DEFAULT_SIDEBAR_WIDTH: f64 = 200.0;
    pub const MINIMUM_SIDEBAR_WIDTH: f64 = 180.0;
    pub const MAXIMUM_SIDEBAR_WIDTH: f64 = 600.0;
    pub const MINIMUM_WINDOW_WIDTH: f64 = 300.0;
    pub const MINIMUM_WINDOW_HEIGHT: f64 = 200.0;
    pub const AUTOSAVE_INTERVAL_SECONDS: f64 = 8.0;
    pub const MAX_WINDOWS_PER_SNAPSHOT: usize = 12;
    pub const MAX_WORKSPACES_PER_WINDOW: usize = 128;
    pub const MAX_PANELS_PER_WORKSPACE: usize = 512;
    pub const MAX_SCROLLBACK_LINES_PER_TERMINAL: usize = 4_000;
    pub const MAX_SCROLLBACK_CHARACTERS_PER_TERMINAL: usize = 400_000;

    /// Clamp a sidebar width to the legal range, falling back to the
    /// default if the input is `None` or non-finite.
    pub fn sanitized_sidebar_width(candidate: Option<f64>) -> f64 {
        match candidate {
            Some(v) if v.is_finite() => v
                .max(Self::MINIMUM_SIDEBAR_WIDTH)
                .min(Self::MAXIMUM_SIDEBAR_WIDTH),
            _ => Self::DEFAULT_SIDEBAR_WIDTH,
        }
    }

    /// Tail-truncate scrollback to the character cap. The Swift
    /// version does ANSI-safe slicing; here we simply slice on a char
    /// boundary since the Rust terminal layer lands in Phase 6 and
    /// will re-run ANSI sanitization before replay.
    pub fn truncated_scrollback(text: Option<&str>) -> Option<String> {
        let text = text?;
        if text.is_empty() {
            return None;
        }
        if text.chars().count() <= Self::MAX_SCROLLBACK_CHARACTERS_PER_TERMINAL {
            return Some(text.to_string());
        }
        let skip = text
            .chars()
            .count()
            .saturating_sub(Self::MAX_SCROLLBACK_CHARACTERS_PER_TERMINAL);
        Some(text.chars().skip(skip).collect())
    }
}

/// Serializable rectangle. Matches Swift `SessionRectSnapshot`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SessionRectSnapshot {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Serializable display info. Matches Swift `SessionDisplaySnapshot`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SessionDisplaySnapshot {
    #[serde(rename = "displayID", default)]
    pub display_id: Option<u32>,
    #[serde(default)]
    pub frame: Option<SessionRectSnapshot>,
    #[serde(rename = "visibleFrame", default)]
    pub visible_frame: Option<SessionRectSnapshot>,
}

/// Sidebar content selector. Matches Swift `SessionSidebarSelection`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionSidebarSelection {
    Tabs,
    Notifications,
}

/// Sidebar persisted state. Matches Swift `SessionSidebarSnapshot`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionSidebarSnapshot {
    #[serde(rename = "isVisible")]
    pub is_visible: bool,
    pub selection: SessionSidebarSelection,
    #[serde(default)]
    pub width: Option<f64>,
}

/// Single status key/value row. Matches Swift `SessionStatusEntrySnapshot`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionStatusEntrySnapshot {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    pub timestamp: f64,
}

/// Sidebar log row. Matches Swift `SessionLogEntrySnapshot`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionLogEntrySnapshot {
    pub message: String,
    pub level: String,
    #[serde(default)]
    pub source: Option<String>,
    pub timestamp: f64,
}

/// Sidebar progress bar state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionProgressSnapshot {
    pub value: f64,
    #[serde(default)]
    pub label: Option<String>,
}

/// Git branch metadata persisted alongside a panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionGitBranchSnapshot {
    pub branch: String,
    #[serde(rename = "isDirty", default)]
    pub is_dirty: bool,
}

/// Terminal-specific panel payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SessionTerminalPanelSnapshot {
    #[serde(rename = "workingDirectory", default)]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub scrollback: Option<String>,
}

/// Browser-specific panel payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionBrowserPanelSnapshot {
    #[serde(rename = "urlString", default)]
    pub url_string: Option<String>,
    #[serde(rename = "profileID", default)]
    pub profile_id: Option<Uuid>,
    #[serde(rename = "shouldRenderWebView")]
    pub should_render_web_view: bool,
    #[serde(rename = "pageZoom")]
    pub page_zoom: f64,
    #[serde(rename = "developerToolsVisible")]
    pub developer_tools_visible: bool,
    #[serde(rename = "backHistoryURLStrings", default)]
    pub back_history_url_strings: Option<Vec<String>>,
    #[serde(rename = "forwardHistoryURLStrings", default)]
    pub forward_history_url_strings: Option<Vec<String>>,
}

/// Markdown-specific panel payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionMarkdownPanelSnapshot {
    #[serde(rename = "filePath")]
    pub file_path: String,
}

/// Discriminant for the on-disk panel kind. Matches Swift `PanelType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionPanelType {
    Terminal,
    Browser,
    Markdown,
}

/// Single panel snapshot. Mirrors Swift `SessionPanelSnapshot`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionPanelSnapshot {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub panel_type: SessionPanelType,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(rename = "customTitle", default)]
    pub custom_title: Option<String>,
    #[serde(default)]
    pub directory: Option<String>,
    #[serde(rename = "isPinned")]
    pub is_pinned: bool,
    #[serde(rename = "isManuallyUnread")]
    pub is_manually_unread: bool,
    #[serde(rename = "gitBranch", default)]
    pub git_branch: Option<SessionGitBranchSnapshot>,
    #[serde(rename = "listeningPorts", default)]
    pub listening_ports: Vec<i32>,
    #[serde(rename = "ttyName", default)]
    pub tty_name: Option<String>,
    #[serde(default)]
    pub terminal: Option<SessionTerminalPanelSnapshot>,
    #[serde(default)]
    pub browser: Option<SessionBrowserPanelSnapshot>,
    #[serde(default)]
    pub markdown: Option<SessionMarkdownPanelSnapshot>,
}

/// Split orientation used by the persisted layout tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionSplitOrientation {
    Horizontal,
    Vertical,
}

/// Leaf pane in the persisted layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionPaneLayoutSnapshot {
    #[serde(rename = "panelIds")]
    pub panel_ids: Vec<Uuid>,
    #[serde(rename = "selectedPanelId", default)]
    pub selected_panel_id: Option<Uuid>,
}

/// Split node in the persisted layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionSplitLayoutSnapshot {
    pub orientation: SessionSplitOrientation,
    #[serde(rename = "dividerPosition")]
    pub divider_position: f64,
    pub first: Box<SessionWorkspaceLayoutSnapshot>,
    pub second: Box<SessionWorkspaceLayoutSnapshot>,
}

/// Recursive layout node. The Swift version uses a manual `Codable`
/// adapter with a `"type": "pane" | "split"` tag; we match that shape
/// via `#[serde(tag = "type", rename_all = "lowercase")]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SessionWorkspaceLayoutSnapshot {
    Pane {
        pane: SessionPaneLayoutSnapshot,
    },
    Split {
        split: SessionSplitLayoutSnapshot,
    },
}

/// One workspace in the on-disk session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionWorkspaceSnapshot {
    #[serde(rename = "processTitle")]
    pub process_title: String,
    #[serde(rename = "customTitle", default)]
    pub custom_title: Option<String>,
    #[serde(rename = "customDescription", default)]
    pub custom_description: Option<String>,
    #[serde(rename = "customColor", default)]
    pub custom_color: Option<String>,
    #[serde(rename = "isPinned")]
    pub is_pinned: bool,
    #[serde(rename = "currentDirectory")]
    pub current_directory: String,
    #[serde(rename = "focusedPanelId", default)]
    pub focused_panel_id: Option<Uuid>,
    pub layout: SessionWorkspaceLayoutSnapshot,
    pub panels: Vec<SessionPanelSnapshot>,
    #[serde(rename = "statusEntries", default)]
    pub status_entries: Vec<SessionStatusEntrySnapshot>,
    #[serde(rename = "logEntries", default)]
    pub log_entries: Vec<SessionLogEntrySnapshot>,
    #[serde(default)]
    pub progress: Option<SessionProgressSnapshot>,
    #[serde(rename = "gitBranch", default)]
    pub git_branch: Option<SessionGitBranchSnapshot>,
}

/// Top-level tab manager snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionTabManagerSnapshot {
    #[serde(rename = "selectedWorkspaceIndex", default)]
    pub selected_workspace_index: Option<usize>,
    pub workspaces: Vec<SessionWorkspaceSnapshot>,
}

/// Per-window snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionWindowSnapshot {
    #[serde(default)]
    pub frame: Option<SessionRectSnapshot>,
    #[serde(default)]
    pub display: Option<SessionDisplaySnapshot>,
    #[serde(rename = "tabManager")]
    pub tab_manager: SessionTabManagerSnapshot,
    pub sidebar: SessionSidebarSnapshot,
}

/// Root snapshot. Matches Swift `AppSessionSnapshot`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppSessionSnapshot {
    pub version: u32,
    #[serde(rename = "createdAt")]
    pub created_at: f64,
    pub windows: Vec<SessionWindowSnapshot>,
}

/// Errors raised by the session store. IO is the only fallible bit
/// worth surfacing; schema mismatches are treated as "no snapshot".
#[derive(Debug, thiserror::Error)]
pub enum SessionStoreError {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("json error at {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

/// File-backed session store, matching the behaviour of Swift
/// `SessionPersistenceStore`.
pub struct SessionPersistenceStore;

impl SessionPersistenceStore {
    /// Load a snapshot from `path`. Returns `Ok(None)` if the file is
    /// missing, empty, or carries a non-matching schema version —
    /// mirroring Swift's "return nil on bad data" behaviour. IO and
    /// JSON errors surface as `Err` so callers can log them.
    pub fn load(path: &Path) -> Result<Option<AppSessionSnapshot>, SessionStoreError> {
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(path).map_err(|source| SessionStoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if text.trim().is_empty() {
            return Ok(None);
        }
        let snapshot: AppSessionSnapshot =
            serde_json::from_str(&text).map_err(|source| SessionStoreError::Json {
                path: path.to_path_buf(),
                source,
            })?;
        if snapshot.version != SessionSnapshotSchema::CURRENT_VERSION {
            return Ok(None);
        }
        if snapshot.windows.is_empty() {
            return Ok(None);
        }
        Ok(Some(snapshot))
    }

    /// Save a snapshot atomically. Creates the parent directory if
    /// needed; matches Swift's "write .atomic, skip when unchanged"
    /// logic by comparing against the existing file contents first.
    pub fn save(path: &Path, snapshot: &AppSessionSnapshot) -> Result<(), SessionStoreError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| SessionStoreError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let data = serde_json::to_vec(snapshot).map_err(|source| SessionStoreError::Json {
            path: path.to_path_buf(),
            source,
        })?;
        if let Ok(existing) = fs::read(path) {
            if existing == data {
                return Ok(());
            }
        }
        // Atomic replace: write to a sibling temp file then rename.
        let mut tmp = path.to_path_buf();
        let file_name = path
            .file_name()
            .map(|n| format!("{}.tmp", n.to_string_lossy()))
            .unwrap_or_else(|| "session.json.tmp".to_string());
        tmp.set_file_name(file_name);
        fs::write(&tmp, &data).map_err(|source| SessionStoreError::Io {
            path: tmp.clone(),
            source,
        })?;
        fs::rename(&tmp, path).map_err(|source| SessionStoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(())
    }

    /// Remove an existing snapshot. Missing file is treated as success.
    pub fn remove_snapshot(path: &Path) -> Result<(), SessionStoreError> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(SessionStoreError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    /// Default location under the user's application support dir.
    /// Matches Swift `SessionPersistenceStore.defaultSnapshotFileURL`
    /// including the `session-<bundle>.json` file name. Returns `None`
    /// when no application-support dir is available.
    pub fn default_snapshot_path(bundle_identifier: Option<&str>) -> Option<PathBuf> {
        let base = dirs::data_dir()?;
        let bundle = bundle_identifier
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or("com.cmuxterm.app");
        let sanitized: String = bundle
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        Some(
            base.join("cmux")
                .join(format!("session-{sanitized}.json")),
        )
    }
}
