//! `cmux-headless` — Phase 3 headless binary.
//!
//! This is the executable that `tests_v2/` (and the Python client in
//! `tests_v2/cmux.py`) connects to when the `CMUX_SOCKET` env var is
//! pointed at it. It runs the cross-platform listener in
//! `cmux_core::socket` against the [`HeadlessHandler`] defined in the
//! sibling `handler` module — no Tauri window, no frontend.
//!
//! Usage:
//!
//! ```bash
//! CMUX_SOCKET_PATH=/tmp/cmux-rs.sock cargo run -p cmux-headless
//! CMUX_SOCKET=/tmp/cmux-rs.sock python tests_v2/your_test.py
//! ```
//!
//! The socket path resolution honours the same rules as the Swift app
//! via [`cmux_core::socket::settings::socket_path`] — so
//! `CMUX_SOCKET_PATH`, `CMUX_SOCKET`, `CMUX_TAG` and friends all work.

mod handler;

use std::sync::Arc;

use cmux_core::socket::settings::{
    self, SocketControlMode, StableDefaultSocketPathEntry,
};
use cmux_core::socket::{listen, ListenerConfig};

use handler::HeadlessHandler;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let env = settings::process_env();
    let user_mode = SocketControlMode::default_mode();
    let mode = settings::effective_mode(user_mode, &env);
    if mode == SocketControlMode::Off {
        tracing::info!("CMUX_SOCKET_ENABLE=0 — listener disabled, exiting");
        return;
    }

    let path = settings::socket_path(
        &env,
        Some("com.cmuxterm.app.debug"),
        true,
        current_uid(),
        probe_socket,
    );

    settings::record_last_socket_path(&path);

    tracing::info!(
        socket_path = %path.display(),
        mode = ?mode,
        version = cmux_core::VERSION,
        "cmux-headless starting"
    );

    let handler = Arc::new(HeadlessHandler::new(path.display().to_string()));
    let config = ListenerConfig { path, mode };

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to start tokio runtime");

    if let Err(err) = runtime.block_on(listen(config, handler)) {
        tracing::error!(?err, "listener exited with error");
        std::process::exit(1);
    }
}

#[cfg(unix)]
fn current_uid() -> u32 {
    // SAFETY: getuid() is always safe on POSIX.
    unsafe { libc_getuid() }
}

#[cfg(not(unix))]
fn current_uid() -> u32 {
    0
}

#[cfg(unix)]
extern "C" {
    #[link_name = "getuid"]
    fn libc_getuid() -> u32;
}

fn probe_socket(path: &std::path::Path) -> StableDefaultSocketPathEntry {
    settings::inspect_stable_default_socket_path_entry(path)
}
