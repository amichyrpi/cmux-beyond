//! Tauri v2 entry point for the Rust rewrite of cmux.
//!
//! See `PLAN.md` at the repo root. This binary is the `cmux-dev` replacement
//! that will eventually run alongside the existing Swift `cmux` build.

mod commands;
mod browser;
mod notifications;
mod ports;
mod terminal;
mod sources;
mod state;

use tauri::Manager;

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
        "cmux-app booting (Phase 8)"
    );

    tauri::Builder::default()
        .manage(state::app_state())
        .manage(browser::browser_state_handle())
        .manage(terminal::terminal_state_handle())
        .manage(notifications::notification_state_handle())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let state = app.state::<state::AppState>();
            state.emit(&app.handle())?;
            let browser_state = app.state::<browser::BrowserState>();
            browser_state.emit(&app.handle())?;
            let terminal_state = app.state::<terminal::TerminalState>();
            terminal_state.emit(&app.handle())?;
            let notification_state = app.state::<notifications::NotificationState>();
            notification_state.emit(&app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::workspace_state,
            commands::workspace_create,
            commands::workspace_select,
            commands::workspace_close,
            commands::workspace_reorder,
            commands::workspace_add_tab,
            commands::workspace_split_pane,
            commands::workspace_close_pane,
            commands::workspace_close_tab,
            commands::workspace_move_tab,
            commands::workspace_select_pane,
            commands::workspace_select_tab,
            commands::workspace_set_divider_position,
            commands::workspace_select_next_tab,
            commands::workspace_select_previous_tab,
            browser::browser_state,
            browser::browser_ensure,
            browser::browser_snapshot_by_label,
            browser::browser_open,
            browser::browser_navigate,
            browser::browser_reload,
            browser::browser_back,
            browser::browser_forward,
            browser::browser_set_title,
            browser::browser_close,
            terminal::terminal_state,
            terminal::terminal_ensure,
            terminal::terminal_snapshot,
            terminal::terminal_input,
            terminal::terminal_resize,
            terminal::terminal_search,
            notifications::notifications_state,
            notifications::notifications_push,
            notifications::notifications_mark_read,
            notifications::notifications_clear,
            ports::ports_scan,
        ])
        .run(tauri::generate_context!())
        .expect("error while running cmux-app");
}
