//! Tauri v2 entry point for the Rust rewrite of cmux.
//!
//! See `PLAN.md` at the repo root. This binary is the `cmux-dev` replacement
//! that will eventually run alongside the existing Swift `cmux` build.

mod sources;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Force every sibling .rs stub to be compiled / linked so the mirror
    // layout stays valid as files are ported incrementally.
    sources::link_all();

    tracing::info!(
        cmux_core_version = cmux_core::VERSION,
        "cmux-app booting (Phase 1 skeleton — empty window)"
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .run(tauri::generate_context!())
        .expect("error while running cmux-app");
}
