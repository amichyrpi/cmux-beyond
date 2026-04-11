//! Unix socket / Windows named pipe IPC listener — ported from the command
//! dispatch in [Sources/TerminalController.swift] and
//! [Sources/SocketControlSettings.swift].
//!
//! Phase 3 of `PLAN.md`.
//!
//! Architecture
//! ============
//!
//! The Swift implementation is a hand-rolled Unix-socket accept loop
//! living on the main actor of the running app. Each connection speaks
//! one of two protocols on a single line-delimited stream:
//!
//! 1. **v1 (legacy) text protocol** — `command arg1 arg2 ...\n`, returns
//!    `OK: ...` / `ERROR: ...` / free-form text.
//! 2. **v2 JSON protocol** — `{"id":N,"method":"system.ping","params":{}}`,
//!    returns `{"id":N,"ok":true,"result":{...}}` or
//!    `{"id":N,"ok":false,"error":{"code":"...","message":"..."}}`.
//!
//! This module splits responsibilities into three pieces:
//!
//! - [`settings`] — auth modes, socket-path resolution, password store,
//!   ported from [Sources/SocketControlSettings.swift].
//! - [`listener`] — tokio-backed line-oriented accept loop that is
//!   cross-platform (Unix socket on macOS/Linux, named pipe on Windows)
//!   via the `interprocess` crate.
//! - [`dispatch`] — a [`CommandHandler`] trait the binary implements to
//!   supply the behaviour for every v1 + v2 command listed in Phase 3
//!   of `PLAN.md`. The listener does not know about workspaces or tabs;
//!   it just demultiplexes requests and serialises responses.
//!
//! Only the listener + handler wiring lives here. The actual handler
//! is plugged in by `cmux-app` so the core crate stays UI-agnostic.

pub mod dispatch;
pub mod listener;
pub mod settings;

pub use dispatch::{
    CommandHandler, JsonRequest, JsonResponse, JsonResponseError, ProtocolError,
};
pub use listener::{listen, ListenerConfig};
pub use settings::{
    default_socket_path, effective_mode, env_override_enabled, env_override_mode,
    resolved_stable_default_socket_path, stable_default_socket_path, StableDefaultSocketPathEntry,
    SocketControlMode,
};
