//! Rust port of [TerminalController.swift](TerminalController.swift).
//!
//! Phase 3 (socket command dispatch) + Phase 6 (PTY lifecycle).
//!
//! The Swift original is the main nerve center for terminal state: socket
//! command dispatch, shell-integration OSC handling, metadata sidebar,
//! session-id tracking. In the Rust port this is split:
//!  - Socket command dispatch → `cmux-core::socket`
//!  - PTY lifecycle + alacritty_terminal::Term → `cmux-core::terminal`
//!  - Sidebar state → Tauri events to the React frontend

// TODO(rewrite): port from TerminalController.swift
#[allow(dead_code)]
pub(crate) fn __link() {}
