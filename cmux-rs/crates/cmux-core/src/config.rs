//! Configuration parsing — ported from [Sources/CmuxConfig.swift],
//! [Sources/GhosttyConfig.swift], [Sources/KeyboardShortcutSettings.swift],
//! and [Sources/KeyboardShortcutSettingsFileStore.swift].
//!
//! Phase 3 of `PLAN.md`.
//!
//! The three Swift files collectively total ~4k LOC and are deeply
//! intertwined with AppKit / Carbon / SwiftUI (NSColor, NSEvent, Carbon
//! hotkey registration, `@AppStorage`, `NSVisualEffectView`). This Rust
//! port deliberately keeps only the *data model* and *on-disk format*
//! layers — the pieces that are platform-independent and needed by the
//! headless socket binary. Carbon-hotkey / AppKit bindings land in Phase
//! 5 alongside the Tauri window wiring, where they are replaced by
//! frontend-side event handling.

pub mod cmux;
pub mod ghostty;
pub mod shortcuts;

pub use cmux::{
    CmuxCommandDefinition, CmuxConfigFile, CmuxLayoutNode, CmuxPaneDefinition,
    CmuxRestartBehavior, CmuxSplitDefinition, CmuxSplitDirection, CmuxSurfaceDefinition,
    CmuxSurfaceType, CmuxWorkspaceDefinition, ConfigLoadError,
};
pub use ghostty::{ColorSchemePreference, GhosttyConfig, GhosttyPaletteColor};
pub use shortcuts::{
    KeyboardShortcutAction, KeyboardShortcutSettings, ShortcutLoadError, StoredShortcut,
};
