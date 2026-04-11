//! UI-facing snapshots and helpers for the workspace model.
//!
//! These types are what the Tauri shell emits to the React frontend.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::bonsplit::ExternalTreeNode;

use super::model::{TabManager, Workspace};

/// UI snapshot of a workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub id: Uuid,
    pub title: String,
    #[serde(default)]
    pub custom_title: Option<String>,
    #[serde(default)]
    pub custom_description: Option<String>,
    #[serde(default)]
    pub custom_color: Option<String>,
    pub is_pinned: bool,
    pub current_directory: String,
    #[serde(default)]
    pub preferred_browser_profile_id: Option<Uuid>,
    pub port_ordinal: u32,
    #[serde(default)]
    pub focused_pane_id: Option<crate::bonsplit::PaneId>,
    #[serde(default)]
    pub zoomed_pane_id: Option<crate::bonsplit::PaneId>,
    pub tree: ExternalTreeNode,
}

/// UI snapshot of the tab manager.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabManagerSnapshot {
    #[serde(default)]
    pub selected_workspace_id: Option<Uuid>,
    pub workspaces: Vec<WorkspaceSnapshot>,
}

impl Workspace {
    /// Helper for the Tauri shell and tests. Produces the same payload
    /// as [`Self::snapshot`], but keeps the type rooted in this module
    /// for ergonomic imports from `workspace`.
    pub fn ui_snapshot(&self) -> WorkspaceSnapshot {
        self.snapshot()
    }
}

impl TabManager {
    /// Index of the selected workspace, if any.
    pub fn selected_workspace_index(&self) -> Option<usize> {
        let id = self.selected_workspace_id?;
        self.workspaces.iter().position(|w| w.id == id)
    }

    /// Snapshot the entire tab manager for the Tauri frontend.
    pub fn snapshot(&self) -> TabManagerSnapshot {
        TabManagerSnapshot {
            selected_workspace_id: self.selected_workspace_id,
            workspaces: self.workspaces.iter().map(Workspace::snapshot).collect(),
        }
    }

    /// Insert a workspace at a specific index and select it.
    pub fn insert_workspace(&mut self, index: usize, workspace: Workspace) -> Uuid {
        let id = workspace.id;
        let idx = index.min(self.workspaces.len());
        self.workspaces.insert(idx, workspace);
        self.selected_workspace_id = Some(id);
        id
    }

    /// Reorder a workspace by id.
    pub fn reorder_workspace(&mut self, id: Uuid, target_index: usize) -> bool {
        let Some(from) = self.workspaces.iter().position(|w| w.id == id) else {
            return false;
        };
        self.move_workspace(from, target_index);
        true
    }
}

/// Small convenience so callers can turn a single workspace into the
/// same JSON payload shape the frontend expects.
impl From<&Workspace> for WorkspaceSnapshot {
    fn from(value: &Workspace) -> Self {
        value.snapshot()
    }
}

/// Convenience helper for callers that want a normalized tree snapshot
/// without going through the controller directly.
pub fn workspace_tree_snapshot(workspace: &Workspace) -> ExternalTreeNode {
    workspace.bonsplit.tree_snapshot()
}
