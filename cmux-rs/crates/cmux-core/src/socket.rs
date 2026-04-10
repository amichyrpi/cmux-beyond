//! Unix socket / Windows named pipe IPC listener — ported from the command
//! dispatch in [Sources/TerminalController.swift] and
//! [Sources/SocketControlSettings.swift].
//!
//! Phase 3 of `PLAN.md`.

// TODO(rewrite): implement the full socket command surface:
//   report_meta, report_meta_block, report_git_branch, report_pr,
//   report_ports, report_tty, ports_kick, report_pwd, report_shell_state,
//   window.focus, surface.focus, surface.report_tty, surface.ports_kick,
//   set_agent_pid, clear_agent_pid, sidebar_state, reset_sidebar, read_screen.
pub fn __link() {}
