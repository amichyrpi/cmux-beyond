//! Rust port of [ContentView.swift](ContentView.swift).
//!
//! Phase 5 — thin Rust glue. The actual UI lives in React under
//! `cmux-rs/ui/`. This module only exposes Tauri command handlers that the
//! frontend calls to mutate workspace state.

use tauri::{Emitter, Window};

use crate::state::{state_event_name, AppSnapshot};

// Keep the file anchored in the mirror layout.
#[allow(dead_code)]
pub(crate) fn __link() {}

/// Emit the full app snapshot to the main Tauri window.
#[allow(dead_code)]
pub(crate) fn emit_app_state(window: &Window, snapshot: &AppSnapshot) -> tauri::Result<()> {
    window.emit(state_event_name(), snapshot.clone())
}
