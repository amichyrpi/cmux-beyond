//! Terminal core — ported from [Sources/GhosttyTerminalView.swift],
//! [Sources/TerminalController.swift], [Sources/TerminalView.swift].
//!
//! Backed by `alacritty_terminal` + `portable-pty` (Phase 6 of `PLAN.md`).

// TODO(rewrite): spawn PTY, run alacritty_terminal::Term, expose cell diffs
// to the frontend via Tauri events, handle OSC sequences for cmux sidebar
// metadata, handle iTerm2/Sixel image protocols.
pub fn __link() {}
