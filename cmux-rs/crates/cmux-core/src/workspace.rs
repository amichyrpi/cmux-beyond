//! Workspace + tab manager data models. Ported from:
//!
//! - [Sources/Workspace.swift](../../../../../Sources/Workspace.swift)
//!   (6k+ lines of AppKit-bound business logic — only the published
//!   data layer is ported here; terminal/pty/sidebar surface
//!   behaviour lives in Phase 6+).
//! - [Sources/WorkspaceContentView.swift](../../../../../Sources/WorkspaceContentView.swift)
//!   (SwiftUI container view — the Rust port owns only the layout
//!   state; the React frontend in
//!   [cmux-rs/ui/src/bonsplit/](../../../../ui/src/bonsplit/)
//!   renders it).
//! - [Sources/TabManager.swift](../../../../../Sources/TabManager.swift)
//!   (top-level "list of workspaces + selected index" state).
//! - [Sources/SessionPersistence.swift](../../../../../Sources/SessionPersistence.swift)
//!   (Codable snapshot schema — see [`session`] submodule).
//!
//! Phase 4 of `PLAN.md`.

pub mod model;
pub mod session;
pub mod view;

pub use model::{
    GitBranchState, Panel, PanelId, PanelKind, PanelMetadata, PullRequestState,
    TabManager, Workspace, WorkspaceAttentionFlash, WorkspaceRemoteConnectionState,
};
pub use view::{TabManagerSnapshot, WorkspaceSnapshot};
pub use session::{
    AppSessionSnapshot, SessionBrowserPanelSnapshot, SessionGitBranchSnapshot,
    SessionLogEntrySnapshot, SessionMarkdownPanelSnapshot, SessionPaneLayoutSnapshot,
    SessionPanelSnapshot, SessionPanelType, SessionPersistencePolicy,
    SessionPersistenceStore, SessionProgressSnapshot, SessionRectSnapshot,
    SessionSidebarSelection, SessionSidebarSnapshot, SessionSnapshotSchema,
    SessionSplitLayoutSnapshot, SessionSplitOrientation, SessionStatusEntrySnapshot,
    SessionTabManagerSnapshot, SessionTerminalPanelSnapshot, SessionWindowSnapshot,
    SessionWorkspaceLayoutSnapshot, SessionWorkspaceSnapshot,
};

/// Kept for the Phase 2 sibling-stub scheme — see
/// [cmux-rs/crates/cmux-app/src/sources.rs](../../../cmux-app/src/sources.rs).
pub fn __link() {}
