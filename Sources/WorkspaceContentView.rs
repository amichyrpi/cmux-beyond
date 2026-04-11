//! Rust port of [WorkspaceContentView.swift](WorkspaceContentView.swift).
//!
//! Phase 5 — thin glue. The actual workspace view is a React component in
//! `cmux-rs/ui/src/`; this module exposes Tauri commands for workspace
//! mutations.

use tauri::{Emitter, Window};

use cmux_core::workspace::WorkspaceSnapshot;

// Keep the file anchored in the mirror layout.
#[allow(dead_code)]
pub(crate) fn __link() {}

/// Emit a single workspace snapshot to a dedicated window event.
#[allow(dead_code)]
pub(crate) fn emit_workspace_state(
    window: &Window,
    snapshot: &WorkspaceSnapshot,
) -> tauri::Result<()> {
    window.emit("cmux:workspace-state", snapshot.clone())
}
