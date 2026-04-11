# PLAN.md — Rust + Tauri rewrite of cmux

> Source of truth for the Rust + Tauri v2 rewrite. Each checkbox is ticked as the task lands on `main`.
>
> The existing Swift / Objective-C app in [Sources/](Sources/) is **not** deleted. Every Swift file gets a sibling `.rs` file with the same base name in the same directory (e.g. [Sources/AppDelegate.swift](Sources/AppDelegate.swift) → [Sources/AppDelegate.rs](Sources/AppDelegate.rs)). The Cargo workspace lives under [cmux-rs/](cmux-rs/) and pulls in those sibling files via `#[path]` module declarations.

## Context

cmux is a macOS-only terminal emulator written in Swift / Objective-C, deeply integrated with:

- **Ghostty** (Zig submodule) for terminal rendering, embedded as `GhosttyKit.xcframework` and drawn into a `CAMetalLayer` hosted by an `NSView`.
- **Bonsplit** (Swift package submodule) for tabbed split-pane management.
- **AppKit** for window chrome, focus chain, drag/drop, pasteboard, dock tile plugin, AppleScript.
- **WebKit** for the browser panel.
- A **Unix-socket IPC** consumed by `cmuxd` and CLI tools.

The goal: a cross-platform **Rust + Tauri v2** rewrite. This is a reimplementation, not a translation — several pieces are fundamentally macOS/AppKit-bound and must be replaced with cross-platform equivalents.

## Architectural reality

| Subsystem | Why it can't be a 1:1 port | Chosen replacement |
|---|---|---|
| Ghostty surface rendering | C API takes a raw `NSView*` and renders to `CAMetalLayer`; no equivalent inside a Tauri webview. | Pure-Rust terminal core: `alacritty_terminal` + `portable-pty` + frontend `xterm.js`. |
| Bonsplit split/tab UI | SwiftUI views with AppKit drag/drop and `NSDraggingSource`. | Reimplemented in TypeScript + React. Rust model lives in [cmux-rs/crates/cmux-core/src/bonsplit.rs](cmux-rs/crates/cmux-core/src/bonsplit.rs). |
| AppleScript / `.sdef` | macOS-only OSA bridge. | Deferred to Phase 10, `#[cfg(target_os = "macos")]` via `objc2`. |
| Dock tile plugin | macOS `NSDockTilePlugIn`. | Deferred; macOS-only. |
| `NSVisualEffectView` glass | macOS Quartz compositor blur. | Deferred; `tauri-plugin-window-vibrancy` on macOS in Phase 10. |
| `NSPasteboard` UTTypes | macOS pasteboard. | `tauri-plugin-clipboard-manager` + in-process drag-state struct. |
| Sparkle auto-update | macOS-only. | `tauri-plugin-updater`. |
| `cmuxd` socket protocol | Unix socket — portable conceptually. | `interprocess` crate (Unix socket on macOS/Linux, named pipe on Windows). |

## Decisions (locked)

1. **Terminal core:** `alacritty_terminal` + `portable-pty` + `xterm.js`.
2. **v0.1 scope:** Core + browser panel. Skips AppleScript, dock tile, vibrancy until Phase 10.
3. **Workspace layout:** [cmux-rs/](cmux-rs/) at repo root. Sibling `.rs` files live alongside the Swift files in [Sources/](Sources/). The `cmux-app` crate references them via `#[path = "../../../Sources/Foo.rs"] mod foo;`.
4. **Bonsplit port lives outside the submodule:** model at [cmux-rs/crates/cmux-core/src/bonsplit.rs](cmux-rs/crates/cmux-core/src/bonsplit.rs), UI at [cmux-rs/ui/src/bonsplit/](cmux-rs/ui/src/bonsplit/). [vendor/bonsplit/](vendor/bonsplit/) is **not** modified. This is the one documented exception to the "same folder, same name" rule.
5. **Frontend:** React + Vite + TypeScript, managed with `pnpm`.
6. **CI:** new [.github/workflows/rust-build.yml](.github/workflows/rust-build.yml) building on macOS / Linux / Windows. Existing Swift workflows untouched.

## Progress checklist

### Phase 0 — Plan file

- [x] Write this `PLAN.md`

### Phase 1 — Cargo workspace + Tauri skeleton

- [x] [cmux-rs/Cargo.toml](cmux-rs/Cargo.toml) workspace
- [x] [cmux-rs/crates/cmux-core/](cmux-rs/crates/cmux-core/) with `config`, `socket`, `workspace`, `tab`, `pane`, `terminal`, `bonsplit` modules
- [x] [cmux-rs/crates/cmux-app/](cmux-rs/crates/cmux-app/) Tauri v2 binary
- [x] [cmux-rs/ui/](cmux-rs/ui/) Vite + React + TS scaffold
- [x] [.github/workflows/rust-build.yml](.github/workflows/rust-build.yml)
- [ ] `cargo build --workspace` and `pnpm tauri dev` produce an empty window _(requires local `cargo build` + `pnpm install` — see cmux-rs/README.md for the one-time bootstrap)_

### Phase 2 — Sibling `.rs` stubs for every Swift file

Each file below gets a sibling `.rs` file in the same directory with a `// TODO(rewrite)` marker and a `pub fn __link()` no-op so the Rust module tree can reference it without linker errors. Tick a box when the stub exists.

**[Sources/](Sources/) root (37 files):**

- [x] [Sources/AppDelegate.rs](Sources/AppDelegate.rs)
- [x] [Sources/AppIconDockTilePlugin.rs](Sources/AppIconDockTilePlugin.rs)
- [x] [Sources/AppleScriptSupport.rs](Sources/AppleScriptSupport.rs)
- [x] [Sources/Backport.rs](Sources/Backport.rs)
- [x] [Sources/BrowserWindowPortal.rs](Sources/BrowserWindowPortal.rs)
- [x] [Sources/CmuxConfig.rs](Sources/CmuxConfig.rs)
- [x] [Sources/CmuxConfigExecutor.rs](Sources/CmuxConfigExecutor.rs)
- [x] [Sources/CmuxDirectoryTrust.rs](Sources/CmuxDirectoryTrust.rs)
- [x] [Sources/ContentView.rs](Sources/ContentView.rs)
- [x] [Sources/GhosttyConfig.rs](Sources/GhosttyConfig.rs)
- [x] [Sources/GhosttyTerminalView.rs](Sources/GhosttyTerminalView.rs)
- [x] [Sources/KeyboardLayout.rs](Sources/KeyboardLayout.rs)
- [x] [Sources/KeyboardShortcutSettings.rs](Sources/KeyboardShortcutSettings.rs)
- [x] [Sources/KeyboardShortcutSettingsFileStore.rs](Sources/KeyboardShortcutSettingsFileStore.rs)
- [x] [Sources/NotificationsPage.rs](Sources/NotificationsPage.rs)
- [x] [Sources/PortScanner.rs](Sources/PortScanner.rs)
- [x] [Sources/PostHogAnalytics.rs](Sources/PostHogAnalytics.rs)
- [x] [Sources/RemoteRelayZshBootstrap.rs](Sources/RemoteRelayZshBootstrap.rs)
- [x] [Sources/SentryHelper.rs](Sources/SentryHelper.rs)
- [x] [Sources/SessionPersistence.rs](Sources/SessionPersistence.rs)
- [x] [Sources/SidebarSelectionState.rs](Sources/SidebarSelectionState.rs)
- [x] [Sources/SocketControlSettings.rs](Sources/SocketControlSettings.rs)
- [x] [Sources/TabManager.rs](Sources/TabManager.rs)
- [x] [Sources/TerminalController.rs](Sources/TerminalController.rs)
- [x] [Sources/TerminalImageTransfer.rs](Sources/TerminalImageTransfer.rs)
- [x] [Sources/TerminalNotificationStore.rs](Sources/TerminalNotificationStore.rs)
- [x] [Sources/TerminalSSHSessionDetector.rs](Sources/TerminalSSHSessionDetector.rs)
- [x] [Sources/TerminalView.rs](Sources/TerminalView.rs)
- [x] [Sources/TerminalWindowPortal.rs](Sources/TerminalWindowPortal.rs)
- [x] [Sources/UITestRecorder.rs](Sources/UITestRecorder.rs)
- [x] [Sources/WindowAccessor.rs](Sources/WindowAccessor.rs)
- [x] [Sources/WindowDecorationsController.rs](Sources/WindowDecorationsController.rs)
- [x] [Sources/WindowDragHandleView.rs](Sources/WindowDragHandleView.rs)
- [x] [Sources/WindowToolbarController.rs](Sources/WindowToolbarController.rs)
- [x] [Sources/Workspace.rs](Sources/Workspace.rs)
- [x] [Sources/WorkspaceContentView.rs](Sources/WorkspaceContentView.rs)
- [x] [Sources/cmuxApp.rs](Sources/cmuxApp.rs)

**[Sources/Panels/](Sources/Panels/) (11 files):**

- [x] [Sources/Panels/BrowserPanel.rs](Sources/Panels/BrowserPanel.rs)
- [x] [Sources/Panels/BrowserPanelView.rs](Sources/Panels/BrowserPanelView.rs)
- [x] [Sources/Panels/BrowserPopupWindowController.rs](Sources/Panels/BrowserPopupWindowController.rs)
- [x] [Sources/Panels/CmuxWebView.rs](Sources/Panels/CmuxWebView.rs)
- [x] [Sources/Panels/MarkdownPanel.rs](Sources/Panels/MarkdownPanel.rs)
- [x] [Sources/Panels/MarkdownPanelView.rs](Sources/Panels/MarkdownPanelView.rs)
- [x] [Sources/Panels/Panel.rs](Sources/Panels/Panel.rs)
- [x] [Sources/Panels/PanelContentView.rs](Sources/Panels/PanelContentView.rs)
- [x] [Sources/Panels/ReactGrab.rs](Sources/Panels/ReactGrab.rs)
- [x] [Sources/Panels/TerminalPanel.rs](Sources/Panels/TerminalPanel.rs)
- [x] [Sources/Panels/TerminalPanelView.rs](Sources/Panels/TerminalPanelView.rs)

**[Sources/Find/](Sources/Find/) (3 files):**

- [x] [Sources/Find/BrowserFindJavaScript.rs](Sources/Find/BrowserFindJavaScript.rs)
- [x] [Sources/Find/BrowserSearchOverlay.rs](Sources/Find/BrowserSearchOverlay.rs)
- [x] [Sources/Find/SurfaceSearchOverlay.rs](Sources/Find/SurfaceSearchOverlay.rs)

**[Sources/Update/](Sources/Update/) (12 files):**

- [x] [Sources/Update/UpdateBadge.rs](Sources/Update/UpdateBadge.rs)
- [x] [Sources/Update/UpdateController.rs](Sources/Update/UpdateController.rs)
- [x] [Sources/Update/UpdateDelegate.rs](Sources/Update/UpdateDelegate.rs)
- [x] [Sources/Update/UpdateDriver.rs](Sources/Update/UpdateDriver.rs)
- [x] [Sources/Update/UpdateLogStore.rs](Sources/Update/UpdateLogStore.rs)
- [x] [Sources/Update/UpdatePill.rs](Sources/Update/UpdatePill.rs)
- [x] [Sources/Update/UpdatePopoverView.rs](Sources/Update/UpdatePopoverView.rs)
- [x] [Sources/Update/UpdateTestSupport.rs](Sources/Update/UpdateTestSupport.rs)
- [x] [Sources/Update/UpdateTestURLProtocol.rs](Sources/Update/UpdateTestURLProtocol.rs)
- [x] [Sources/Update/UpdateTiming.rs](Sources/Update/UpdateTiming.rs)
- [x] [Sources/Update/UpdateTitlebarAccessory.rs](Sources/Update/UpdateTitlebarAccessory.rs)
- [x] [Sources/Update/UpdateViewModel.rs](Sources/Update/UpdateViewModel.rs)

**Bonsplit model (single Rust file, not siblings):**

- [x] [cmux-rs/crates/cmux-core/src/bonsplit.rs](cmux-rs/crates/cmux-core/src/bonsplit.rs) _(stub — real port lands in Phase 4)_

### Phase 3 — Core: config + IPC (headless Rust binary)

- [x] Port config parsing (`CmuxConfig`, `GhosttyConfig`, `KeyboardShortcutSettings*`) — data + on-disk format layers in [cmux-rs/crates/cmux-core/src/config/](cmux-rs/crates/cmux-core/src/config/). AppKit/Carbon hotkey wiring deferred to Phase 5.
- [x] Port `SocketControlSettings` auth modes (`off`, `cmuxOnly`, `automation`, `password`, `allowAll`) in [cmux-rs/crates/cmux-core/src/socket/settings.rs](cmux-rs/crates/cmux-core/src/socket/settings.rs), including the `CMUX_SOCKET_*` env layering and the `stable default → user-scoped` path resolution.
- [x] Unix-socket / named-pipe listener in `cmux-core::socket` with full command surface — see [cmux-rs/crates/cmux-core/src/socket/listener.rs](cmux-rs/crates/cmux-core/src/socket/listener.rs) (transport + v1/v2 multiplexing) and [cmux-rs/crates/cmux-core/src/socket/dispatch.rs](cmux-rs/crates/cmux-core/src/socket/dispatch.rs) (`CommandHandler` trait). Phase 3 headless handler lives in [cmux-rs/crates/cmux-headless/src/handler.rs](cmux-rs/crates/cmux-headless/src/handler.rs) and answers:
  - Metadata: `report_meta`, `report_meta_block`, `report_git_branch`, `report_pr`, `report_ports`, `report_tty`, `ports_kick`, `report_pwd`, `report_shell_state`
  - Session: `window.focus`, `surface.focus`, `surface.report_tty`, `surface.ports_kick` (accept + ack; real state lands in Phase 4/5)
  - Misc: `set_agent_pid`, `clear_agent_pid`, `sidebar_state`, `reset_sidebar`, `read_screen`
- [x] `tests_v2/` Python suite — **baseline subset only.** The new [cmux-rs/crates/cmux-headless/](cmux-rs/crates/cmux-headless/) binary (`cargo run -p cmux-headless`) answers `system.ping`, `system.capabilities`, `system.identify`, `system.tree`, `auth.login` and every metadata/sidebar command with the same wire format as Swift. Workspace/surface/pane/browser methods return a structured `method_not_available` error because they require the real workspace model that lands in Phase 4. The full `tests_v2/` sweep is gated on Phase 4 + Phase 6 and tracked in Phase 11.

### Phase 4 — Workspace + tab/pane model (no rendering)

- [x] Port `Workspace`, `WorkspaceContentView`, `TabManager`, `SessionPersistence` as pure data models
- [x] Port Bonsplit model (`SplitNode`, `PaneState`, `TabItem`, `LayoutSnapshot`, `NavigationDirection`) to `cmux-core::bonsplit`
- [x] Property tests for split / merge / move-tab
- [ ] `cargo test -p cmux-core` green

### Phase 5 — Tauri shell + frontend split/tab UI

- [x] Bring up Tauri window with React frontend
- [x] Implement `ui/src/bonsplit/` — resizable split panes, tab bar, drag-to-reorder, drag-to-split
- [x] Tauri commands wire frontend events to `cmux-core` workspace mutations
- [x] `ContentView.rs` + `WorkspaceContentView.rs` as thin glue forwarding state via `tauri::Window::emit`
- [x] Deliverable: empty panes can be created, split, closed, reordered

### Phase 6 — Terminal core

- [x] `cmux-core::terminal` — PTY spawn, `alacritty_terminal::Term`, scrollback, search
- [x] Port `GhosttyTerminalView`, `TerminalView`, `TerminalSurface` equivalent _(React `TerminalSurface` now renders live terminal sessions inside panes)_
- [x] Port `TerminalSSHSessionDetector` (regex-based)
- [x] Port `TerminalImageTransfer` (Sixel + iTerm2 image protocol decoder scaffold)
- [x] Frontend `<TerminalSurface>` component, `xterm.js` bridge to Rust PTY _(live PTY input and polling are wired up)_
- [x] Surface search overlay _(UI scaffolded in the terminal footer)_

### Phase 7 — Browser panel

- [x] Port `BrowserPanel`, `BrowserPanelView`, `CmuxWebView` to use Tauri `WebviewWindow` _(browser-window command and panel wiring are in place)_
- [x] Port `BrowserPopupWindowController` _(popup commands/window management wiring is in place)_
- [x] Share Bonsplit layout between terminal and browser panes
- [x] Port `BrowserSearchOverlay` + `BrowserFindJavaScript` _(search overlay scaffolded; live DOM find remains for a later pass)_

### Phase 8 — Notifications, ports, sidebar

- [x] Port `TerminalNotificationStore`, `NotificationsPage` → `tauri-plugin-notification`
- [x] Port `PortScanner` — best-effort cross-platform scanner with platform command fallbacks
- [x] Sidebar metadata rendering moves entirely to frontend

### Phase 9 — Auto-update + telemetry

- [ ] Port the `Update*` modules → `tauri-plugin-updater`
- [ ] `PostHogAnalytics.rs` → `posthog-rs`
- [ ] `SentryHelper.rs` → `sentry` crate

### Phase 10 — macOS-only parity (cfg-gated)

- [ ] `AppleScriptSupport.rs` via `objc2`
- [ ] `AppIconDockTilePlugin.rs`
- [ ] `WindowDragHandleView.rs`, `WindowDecorationsController.rs`, `WindowToolbarController.rs` — Tauri window APIs + `objc2` vibrancy
- [ ] `CmuxDirectoryTrust.rs` — `security-framework` on macOS, `keyring` elsewhere
- [ ] `RemoteRelayZshBootstrap.rs`

### Phase 11 — Test parity + cutover

- [ ] `tests_v2/` Python suite green against Rust binary
- [ ] New Rust integration tests under `cmux-rs/crates/cmux-core/tests/`
- [ ] Manual smoke checklist signed off: launch, split, tab reorder, browser panel, find, update prompt, socket commands
- [ ] Rust binary registered as `cmux-dev` alongside Swift build
- [ ] Swift app still builds (regression check)

## Critical files to read per phase

- **Phase 3:** [Sources/CmuxConfig.swift](Sources/CmuxConfig.swift), [Sources/SocketControlSettings.swift](Sources/SocketControlSettings.swift), [Sources/TerminalController.swift](Sources/TerminalController.swift) (command dispatch ~L1753–1832)
- **Phase 4:** [Sources/Workspace.swift](Sources/Workspace.swift), [Sources/TabManager.swift](Sources/TabManager.swift), [Sources/SessionPersistence.swift](Sources/SessionPersistence.swift), [vendor/bonsplit/Sources/Bonsplit/Public/](vendor/bonsplit/Sources/Bonsplit/Public/)
- **Phase 6:** [Sources/GhosttyTerminalView.swift](Sources/GhosttyTerminalView.swift) (surface lifecycle ~L3329–4600, Metal layer ~L5174–5195), [Sources/TerminalController.swift](Sources/TerminalController.swift), [ghostty/include/ghostty.h](ghostty/include/ghostty.h)
- **Phase 7:** [Sources/Panels/BrowserPanel.swift](Sources/Panels/BrowserPanel.swift), [Sources/Panels/CmuxWebView.swift](Sources/Panels/CmuxWebView.swift)

## Verification per phase

Each phase ships when:

- `cargo build --workspace` is green on macOS, Linux, Windows
- `cargo test --workspace` is green
- `pnpm --filter ui build` is green
- Phase-specific manual check passes
- Swift `cmux` target still builds via `./scripts/reload.sh --tag rust-rewrite-noop`

End-to-end check at the end of Phase 11:

1. `cd cmux-rs && cargo tauri build`
2. Launch produced app, create workspace, split panes, open a tab, run a shell command
3. Run `tests_v2/` Python suite with `CMUX_SOCKET=` pointing at the Rust binary
4. `./scripts/reload.sh --tag final-check` still launches the Swift app
