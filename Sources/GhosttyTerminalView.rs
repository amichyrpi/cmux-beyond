//! Rust port of [GhosttyTerminalView.swift](GhosttyTerminalView.swift).
//!
//! Phase 6 — this is the biggest departure from the Swift original. The
//! Swift file is ~12k LOC of AppKit + Metal + Ghostty C FFI. The Rust port
//! replaces all of it with `alacritty_terminal` + `portable-pty` and pushes
//! cell updates to the frontend via Tauri events. The frontend renders with
//! `xterm.js`.

// TODO(rewrite): port from GhosttyTerminalView.swift
#[allow(dead_code)]
pub(crate) fn __link() {}
