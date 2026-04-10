//! Wires the sibling `.rs` stubs under `Sources/` into the `cmux-app` crate
//! via `#[path]` module declarations.
//!
//! Each submodule below mirrors a Swift file one-for-one. The path is
//! relative to this file (`cmux-rs/crates/cmux-app/src/sources.rs`), so the
//! prefix is `../../../Sources/...`.
//!
//! Keep this file sorted alphabetically per subdirectory so that `cargo fmt`
//! output matches the layout of `Sources/`.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

// ----- Sources/ (root) --------------------------------------------------

#[path = "../../../../Sources/AppDelegate.rs"]
mod app_delegate;
#[path = "../../../../Sources/AppIconDockTilePlugin.rs"]
mod app_icon_dock_tile_plugin;
#[path = "../../../../Sources/AppleScriptSupport.rs"]
mod apple_script_support;
#[path = "../../../../Sources/Backport.rs"]
mod backport;
#[path = "../../../../Sources/BrowserWindowPortal.rs"]
mod browser_window_portal;
#[path = "../../../../Sources/CmuxConfig.rs"]
mod cmux_config;
#[path = "../../../../Sources/CmuxConfigExecutor.rs"]
mod cmux_config_executor;
#[path = "../../../../Sources/CmuxDirectoryTrust.rs"]
mod cmux_directory_trust;
#[path = "../../../../Sources/ContentView.rs"]
mod content_view;
#[path = "../../../../Sources/GhosttyConfig.rs"]
mod ghostty_config;
#[path = "../../../../Sources/GhosttyTerminalView.rs"]
mod ghostty_terminal_view;
#[path = "../../../../Sources/KeyboardLayout.rs"]
mod keyboard_layout;
#[path = "../../../../Sources/KeyboardShortcutSettings.rs"]
mod keyboard_shortcut_settings;
#[path = "../../../../Sources/KeyboardShortcutSettingsFileStore.rs"]
mod keyboard_shortcut_settings_file_store;
#[path = "../../../../Sources/NotificationsPage.rs"]
mod notifications_page;
#[path = "../../../../Sources/PortScanner.rs"]
mod port_scanner;
#[path = "../../../../Sources/PostHogAnalytics.rs"]
mod posthog_analytics;
#[path = "../../../../Sources/RemoteRelayZshBootstrap.rs"]
mod remote_relay_zsh_bootstrap;
#[path = "../../../../Sources/SentryHelper.rs"]
mod sentry_helper;
#[path = "../../../../Sources/SessionPersistence.rs"]
mod session_persistence;
#[path = "../../../../Sources/SidebarSelectionState.rs"]
mod sidebar_selection_state;
#[path = "../../../../Sources/SocketControlSettings.rs"]
mod socket_control_settings;
#[path = "../../../../Sources/TabManager.rs"]
mod tab_manager;
#[path = "../../../../Sources/TerminalController.rs"]
mod terminal_controller;
#[path = "../../../../Sources/TerminalImageTransfer.rs"]
mod terminal_image_transfer;
#[path = "../../../../Sources/TerminalNotificationStore.rs"]
mod terminal_notification_store;
#[path = "../../../../Sources/TerminalSSHSessionDetector.rs"]
mod terminal_ssh_session_detector;
#[path = "../../../../Sources/TerminalView.rs"]
mod terminal_view;
#[path = "../../../../Sources/TerminalWindowPortal.rs"]
mod terminal_window_portal;
#[path = "../../../../Sources/UITestRecorder.rs"]
mod ui_test_recorder;
#[path = "../../../../Sources/WindowAccessor.rs"]
mod window_accessor;
#[path = "../../../../Sources/WindowDecorationsController.rs"]
mod window_decorations_controller;
#[path = "../../../../Sources/WindowDragHandleView.rs"]
mod window_drag_handle_view;
#[path = "../../../../Sources/WindowToolbarController.rs"]
mod window_toolbar_controller;
#[path = "../../../../Sources/Workspace.rs"]
mod workspace;
#[path = "../../../../Sources/WorkspaceContentView.rs"]
mod workspace_content_view;
// `cmuxApp.swift` — renamed to avoid clashing with the `cmux-app` crate name.
#[path = "../../../../Sources/cmuxApp.rs"]
mod cmux_app_entry;

// ----- Sources/Panels/ --------------------------------------------------

#[path = "../../../../Sources/Panels/BrowserPanel.rs"]
mod browser_panel;
#[path = "../../../../Sources/Panels/BrowserPanelView.rs"]
mod browser_panel_view;
#[path = "../../../../Sources/Panels/BrowserPopupWindowController.rs"]
mod browser_popup_window_controller;
#[path = "../../../../Sources/Panels/CmuxWebView.rs"]
mod cmux_web_view;
#[path = "../../../../Sources/Panels/MarkdownPanel.rs"]
mod markdown_panel;
#[path = "../../../../Sources/Panels/MarkdownPanelView.rs"]
mod markdown_panel_view;
#[path = "../../../../Sources/Panels/Panel.rs"]
mod panel;
#[path = "../../../../Sources/Panels/PanelContentView.rs"]
mod panel_content_view;
#[path = "../../../../Sources/Panels/ReactGrab.rs"]
mod react_grab;
#[path = "../../../../Sources/Panels/TerminalPanel.rs"]
mod terminal_panel;
#[path = "../../../../Sources/Panels/TerminalPanelView.rs"]
mod terminal_panel_view;

// ----- Sources/Find/ ----------------------------------------------------

#[path = "../../../../Sources/Find/BrowserFindJavaScript.rs"]
mod browser_find_javascript;
#[path = "../../../../Sources/Find/BrowserSearchOverlay.rs"]
mod browser_search_overlay;
#[path = "../../../../Sources/Find/SurfaceSearchOverlay.rs"]
mod surface_search_overlay;

// ----- Sources/Update/ --------------------------------------------------

#[path = "../../../../Sources/Update/UpdateBadge.rs"]
mod update_badge;
#[path = "../../../../Sources/Update/UpdateController.rs"]
mod update_controller;
#[path = "../../../../Sources/Update/UpdateDelegate.rs"]
mod update_delegate;
#[path = "../../../../Sources/Update/UpdateDriver.rs"]
mod update_driver;
#[path = "../../../../Sources/Update/UpdateLogStore.rs"]
mod update_log_store;
#[path = "../../../../Sources/Update/UpdatePill.rs"]
mod update_pill;
#[path = "../../../../Sources/Update/UpdatePopoverView.rs"]
mod update_popover_view;
#[path = "../../../../Sources/Update/UpdateTestSupport.rs"]
mod update_test_support;
#[path = "../../../../Sources/Update/UpdateTestURLProtocol.rs"]
mod update_test_url_protocol;
#[path = "../../../../Sources/Update/UpdateTiming.rs"]
mod update_timing;
#[path = "../../../../Sources/Update/UpdateTitlebarAccessory.rs"]
mod update_titlebar_accessory;
#[path = "../../../../Sources/Update/UpdateViewModel.rs"]
mod update_view_model;

/// Force every sibling stub to be referenced so Cargo compiles and checks
/// them all, even though none are currently called from business logic.
///
/// Remove entries as modules gain real callers during Phase 3 onward.
pub(crate) fn link_all() {
    // Sources/ (root)
    app_delegate::__link();
    app_icon_dock_tile_plugin::__link();
    apple_script_support::__link();
    backport::__link();
    browser_window_portal::__link();
    cmux_config::__link();
    cmux_config_executor::__link();
    cmux_directory_trust::__link();
    content_view::__link();
    ghostty_config::__link();
    ghostty_terminal_view::__link();
    keyboard_layout::__link();
    keyboard_shortcut_settings::__link();
    keyboard_shortcut_settings_file_store::__link();
    notifications_page::__link();
    port_scanner::__link();
    posthog_analytics::__link();
    remote_relay_zsh_bootstrap::__link();
    sentry_helper::__link();
    session_persistence::__link();
    sidebar_selection_state::__link();
    socket_control_settings::__link();
    tab_manager::__link();
    terminal_controller::__link();
    terminal_image_transfer::__link();
    terminal_notification_store::__link();
    terminal_ssh_session_detector::__link();
    terminal_view::__link();
    terminal_window_portal::__link();
    ui_test_recorder::__link();
    window_accessor::__link();
    window_decorations_controller::__link();
    window_drag_handle_view::__link();
    window_toolbar_controller::__link();
    workspace::__link();
    workspace_content_view::__link();
    cmux_app_entry::__link();

    // Sources/Panels/
    browser_panel::__link();
    browser_panel_view::__link();
    browser_popup_window_controller::__link();
    cmux_web_view::__link();
    markdown_panel::__link();
    markdown_panel_view::__link();
    panel::__link();
    panel_content_view::__link();
    react_grab::__link();
    terminal_panel::__link();
    terminal_panel_view::__link();

    // Sources/Find/
    browser_find_javascript::__link();
    browser_search_overlay::__link();
    surface_search_overlay::__link();

    // Sources/Update/
    update_badge::__link();
    update_controller::__link();
    update_delegate::__link();
    update_driver::__link();
    update_log_store::__link();
    update_pill::__link();
    update_popover_view::__link();
    update_test_support::__link();
    update_test_url_protocol::__link();
    update_timing::__link();
    update_titlebar_accessory::__link();
    update_view_model::__link();
}
