//! Port of the data layer of [Sources/KeyboardShortcutSettings.swift]
//! and [Sources/KeyboardShortcutSettingsFileStore.swift].
//!
//! This module keeps only the platform-independent bits:
//!
//! - the `Action` enum (one variant per user-facing shortcut),
//! - the default shortcut table (Cmd+, for openSettings, etc.),
//! - serde-compatible [`StoredShortcut`] that round-trips through JSON,
//! - a merge helper that layers user overrides on top of defaults.
//!
//! Carbon hotkey registration, NSEvent matching, and menu bar wiring
//! are **not** ported here — they re-appear in Phase 5 alongside the
//! Tauri window bootstrap.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One user-configurable action. Mirrors the Swift `Action` enum so the
/// JSON settings file layout is stable across the old Swift build and
/// the new Rust build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyboardShortcutAction {
    // App / window
    OpenSettings,
    ReloadConfiguration,
    ShowHideAllWindows,
    NewWindow,
    CloseWindow,
    ToggleFullScreen,
    Quit,

    // Titlebar / primary UI
    ToggleSidebar,
    NewTab,
    OpenFolder,
    GoToWorkspace,
    CommandPalette,
    SendFeedback,
    ShowNotifications,
    JumpToUnread,
    TriggerFlash,

    // Navigation
    NextSurface,
    PrevSurface,
    SelectSurfaceByNumber,
    NextSidebarTab,
    PrevSidebarTab,
    SelectWorkspaceByNumber,
    RenameTab,
    RenameWorkspace,
    EditWorkspaceDescription,
    CloseTab,
    CloseOtherTabsInPane,
    CloseWorkspace,
    ReopenClosedBrowserPanel,
    NewSurface,
    ToggleTerminalCopyMode,

    // Panes / splits
    FocusLeft,
    FocusRight,
    FocusUp,
    FocusDown,
    SplitRight,
    SplitDown,
    ToggleSplitZoom,
    SplitBrowserRight,
    SplitBrowserDown,

    // Panels
    OpenBrowser,
    FocusBrowserAddressBar,
    BrowserBack,
    BrowserForward,
    BrowserReload,
    BrowserZoomIn,
    BrowserZoomOut,
    BrowserZoomReset,
    Find,
    FindNext,
    FindPrevious,
    HideFind,
    UseSelectionForFind,
    ToggleBrowserDeveloperTools,
    ShowBrowserJavaScriptConsole,
    ToggleReactGrab,
}

impl KeyboardShortcutAction {
    /// Every known action, in the order the Swift `Action.allCases`
    /// iterator yields them. Stable so that tests and UI lists stay
    /// consistent across languages.
    pub const ALL: &'static [KeyboardShortcutAction] = &[
        Self::OpenSettings,
        Self::ReloadConfiguration,
        Self::ShowHideAllWindows,
        Self::NewWindow,
        Self::CloseWindow,
        Self::ToggleFullScreen,
        Self::Quit,
        Self::ToggleSidebar,
        Self::NewTab,
        Self::OpenFolder,
        Self::GoToWorkspace,
        Self::CommandPalette,
        Self::SendFeedback,
        Self::ShowNotifications,
        Self::JumpToUnread,
        Self::TriggerFlash,
        Self::NextSurface,
        Self::PrevSurface,
        Self::SelectSurfaceByNumber,
        Self::NextSidebarTab,
        Self::PrevSidebarTab,
        Self::SelectWorkspaceByNumber,
        Self::RenameTab,
        Self::RenameWorkspace,
        Self::EditWorkspaceDescription,
        Self::CloseTab,
        Self::CloseOtherTabsInPane,
        Self::CloseWorkspace,
        Self::ReopenClosedBrowserPanel,
        Self::NewSurface,
        Self::ToggleTerminalCopyMode,
        Self::FocusLeft,
        Self::FocusRight,
        Self::FocusUp,
        Self::FocusDown,
        Self::SplitRight,
        Self::SplitDown,
        Self::ToggleSplitZoom,
        Self::SplitBrowserRight,
        Self::SplitBrowserDown,
        Self::OpenBrowser,
        Self::FocusBrowserAddressBar,
        Self::BrowserBack,
        Self::BrowserForward,
        Self::BrowserReload,
        Self::BrowserZoomIn,
        Self::BrowserZoomOut,
        Self::BrowserZoomReset,
        Self::Find,
        Self::FindNext,
        Self::FindPrevious,
        Self::HideFind,
        Self::UseSelectionForFind,
        Self::ToggleBrowserDeveloperTools,
        Self::ShowBrowserJavaScriptConsole,
        Self::ToggleReactGrab,
    ];

    /// camelCase name used as the dictionary key in the on-disk
    /// settings JSON. Stable and matches the Swift `rawValue`.
    pub fn camel_case(self) -> &'static str {
        use KeyboardShortcutAction::*;
        match self {
            OpenSettings => "openSettings",
            ReloadConfiguration => "reloadConfiguration",
            ShowHideAllWindows => "showHideAllWindows",
            NewWindow => "newWindow",
            CloseWindow => "closeWindow",
            ToggleFullScreen => "toggleFullScreen",
            Quit => "quit",
            ToggleSidebar => "toggleSidebar",
            NewTab => "newTab",
            OpenFolder => "openFolder",
            GoToWorkspace => "goToWorkspace",
            CommandPalette => "commandPalette",
            SendFeedback => "sendFeedback",
            ShowNotifications => "showNotifications",
            JumpToUnread => "jumpToUnread",
            TriggerFlash => "triggerFlash",
            NextSurface => "nextSurface",
            PrevSurface => "prevSurface",
            SelectSurfaceByNumber => "selectSurfaceByNumber",
            NextSidebarTab => "nextSidebarTab",
            PrevSidebarTab => "prevSidebarTab",
            SelectWorkspaceByNumber => "selectWorkspaceByNumber",
            RenameTab => "renameTab",
            RenameWorkspace => "renameWorkspace",
            EditWorkspaceDescription => "editWorkspaceDescription",
            CloseTab => "closeTab",
            CloseOtherTabsInPane => "closeOtherTabsInPane",
            CloseWorkspace => "closeWorkspace",
            ReopenClosedBrowserPanel => "reopenClosedBrowserPanel",
            NewSurface => "newSurface",
            ToggleTerminalCopyMode => "toggleTerminalCopyMode",
            FocusLeft => "focusLeft",
            FocusRight => "focusRight",
            FocusUp => "focusUp",
            FocusDown => "focusDown",
            SplitRight => "splitRight",
            SplitDown => "splitDown",
            ToggleSplitZoom => "toggleSplitZoom",
            SplitBrowserRight => "splitBrowserRight",
            SplitBrowserDown => "splitBrowserDown",
            OpenBrowser => "openBrowser",
            FocusBrowserAddressBar => "focusBrowserAddressBar",
            BrowserBack => "browserBack",
            BrowserForward => "browserForward",
            BrowserReload => "browserReload",
            BrowserZoomIn => "browserZoomIn",
            BrowserZoomOut => "browserZoomOut",
            BrowserZoomReset => "browserZoomReset",
            Find => "find",
            FindNext => "findNext",
            FindPrevious => "findPrevious",
            HideFind => "hideFind",
            UseSelectionForFind => "useSelectionForFind",
            ToggleBrowserDeveloperTools => "toggleBrowserDeveloperTools",
            ShowBrowserJavaScriptConsole => "showBrowserJavaScriptConsole",
            ToggleReactGrab => "toggleReactGrab",
        }
    }

    /// Parse a camelCase key back into an action.
    pub fn from_camel_case(raw: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|a| a.camel_case() == raw)
    }

    /// Default shortcut for this action. The table matches
    /// `Action.defaultShortcut` in the Swift source exactly so that
    /// existing user overrides keep mapping to the same actions.
    pub fn default_shortcut(self) -> StoredShortcut {
        use KeyboardShortcutAction::*;
        let make = |key: &str, command, shift, option, control| StoredShortcut {
            key: key.into(),
            command,
            shift,
            option,
            control,
            key_code: None,
            chord_key: None,
            chord_command: false,
            chord_shift: false,
            chord_option: false,
            chord_control: false,
            chord_key_code: None,
        };
        match self {
            OpenSettings => make(",", true, false, false, false),
            ReloadConfiguration => make(",", true, true, false, false),
            ShowHideAllWindows => make(".", true, false, true, true),
            NewWindow => make("n", true, true, false, false),
            CloseWindow => make("w", true, false, false, true),
            ToggleFullScreen => make("f", true, false, false, true),
            Quit => make("q", true, false, false, false),
            ToggleSidebar => make("b", true, false, false, false),
            NewTab => make("n", true, false, false, false),
            OpenFolder => make("o", true, false, false, false),
            GoToWorkspace => make("p", true, false, false, false),
            CommandPalette => make("p", true, true, false, false),
            SendFeedback => make("f", true, false, true, false),
            ShowNotifications => make("i", true, false, false, false),
            JumpToUnread => make("u", true, true, false, false),
            TriggerFlash => make("h", true, true, false, false),
            NextSidebarTab => make("]", true, false, false, true),
            PrevSidebarTab => make("[", true, false, false, true),
            RenameTab => make("r", true, false, false, false),
            RenameWorkspace => make("r", true, true, false, false),
            EditWorkspaceDescription => make("e", true, true, false, false),
            CloseTab => make("w", true, false, false, false),
            CloseOtherTabsInPane => make("t", true, false, true, false),
            CloseWorkspace => make("w", true, true, false, false),
            ReopenClosedBrowserPanel => make("t", true, true, false, false),
            FocusLeft => make("\u{2190}", true, false, true, false),
            FocusRight => make("\u{2192}", true, false, true, false),
            FocusUp => make("\u{2191}", true, false, true, false),
            FocusDown => make("\u{2193}", true, false, true, false),
            SplitRight => make("d", true, false, false, false),
            SplitDown => make("d", true, true, false, false),
            ToggleSplitZoom => make("\r", true, true, false, false),
            SplitBrowserRight => make("d", true, false, true, false),
            SplitBrowserDown => make("d", true, true, true, false),
            NextSurface => make("]", true, true, false, false),
            PrevSurface => make("[", true, true, false, false),
            SelectSurfaceByNumber => make("1", false, false, false, true),
            NewSurface => make("t", true, false, false, false),
            ToggleTerminalCopyMode => make("m", true, true, false, false),
            SelectWorkspaceByNumber => make("1", true, false, false, false),
            OpenBrowser => make("l", true, true, false, false),
            FocusBrowserAddressBar => make("l", true, false, false, false),
            BrowserBack => make("[", true, false, false, false),
            BrowserForward => make("]", true, false, false, false),
            BrowserReload => make("r", true, false, false, false),
            BrowserZoomIn => make("=", true, false, false, false),
            BrowserZoomOut => make("-", true, false, false, false),
            BrowserZoomReset => make("0", true, false, false, false),
            Find => make("f", true, false, false, false),
            FindNext => make("g", true, false, false, false),
            FindPrevious => make("g", true, false, true, false),
            HideFind => make("f", true, true, false, false),
            UseSelectionForFind => make("e", true, false, false, false),
            ToggleBrowserDeveloperTools => make("i", true, false, true, false),
            ShowBrowserJavaScriptConsole => make("c", true, false, true, false),
            ToggleReactGrab => make("g", true, true, false, false),
        }
    }
}

/// Serde-compatible mirror of Swift's `StoredShortcut`. Field order and
/// names match the JSON layout in `settings.json` so existing configs
/// round-trip unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredShortcut {
    pub key: String,
    pub command: bool,
    pub shift: bool,
    pub option: bool,
    pub control: bool,
    #[serde(rename = "keyCode", default, skip_serializing_if = "Option::is_none")]
    pub key_code: Option<u16>,
    #[serde(rename = "chordKey", default, skip_serializing_if = "Option::is_none")]
    pub chord_key: Option<String>,
    #[serde(rename = "chordCommand", default)]
    pub chord_command: bool,
    #[serde(rename = "chordShift", default)]
    pub chord_shift: bool,
    #[serde(rename = "chordOption", default)]
    pub chord_option: bool,
    #[serde(rename = "chordControl", default)]
    pub chord_control: bool,
    #[serde(rename = "chordKeyCode", default, skip_serializing_if = "Option::is_none")]
    pub chord_key_code: Option<u16>,
}

impl StoredShortcut {
    pub fn has_chord(&self) -> bool {
        self.chord_key.is_some()
    }
}

/// The resolved keyboard shortcut state — a merge of the defaults with
/// any user-supplied overrides.
#[derive(Debug, Clone, Default)]
pub struct KeyboardShortcutSettings {
    overrides: BTreeMap<KeyboardShortcutAction, StoredShortcut>,
}

impl KeyboardShortcutSettings {
    /// Empty override set — every lookup falls back to the default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load the `shortcuts` block from a `settings.json` file. Unknown
    /// keys are silently ignored (to match Swift), but a malformed
    /// value type is reported.
    pub fn from_settings_json(value: &serde_json::Value) -> Result<Self, ShortcutLoadError> {
        let Some(shortcuts) = value.get("shortcuts") else {
            return Ok(Self::new());
        };
        let obj = shortcuts
            .as_object()
            .ok_or(ShortcutLoadError::ShortcutsNotObject)?;
        let mut overrides = BTreeMap::new();
        for (key, entry) in obj.iter() {
            let Some(action) = KeyboardShortcutAction::from_camel_case(key) else {
                continue;
            };
            let stored: StoredShortcut = serde_json::from_value(entry.clone())
                .map_err(|source| ShortcutLoadError::Invalid {
                    action: key.clone(),
                    source,
                })?;
            overrides.insert(action, stored);
        }
        Ok(Self { overrides })
    }

    /// Lookup a shortcut, falling back to the default.
    pub fn shortcut(&self, action: KeyboardShortcutAction) -> StoredShortcut {
        self.overrides
            .get(&action)
            .cloned()
            .unwrap_or_else(|| action.default_shortcut())
    }

    /// Whether this action has a user override.
    pub fn is_overridden(&self, action: KeyboardShortcutAction) -> bool {
        self.overrides.contains_key(&action)
    }

    /// Set a user override in-memory. Use [`to_settings_json_fragment`]
    /// to persist the full set back out to `settings.json`.
    pub fn set(&mut self, action: KeyboardShortcutAction, shortcut: StoredShortcut) {
        self.overrides.insert(action, shortcut);
    }

    /// Remove a user override.
    pub fn clear(&mut self, action: KeyboardShortcutAction) {
        self.overrides.remove(&action);
    }

    /// Serialise the overrides as a JSON object keyed by the camelCase
    /// action name — the same shape consumed by `from_settings_json`.
    pub fn to_settings_json_fragment(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        for (action, stored) in &self.overrides {
            map.insert(
                action.camel_case().to_string(),
                serde_json::to_value(stored).expect("StoredShortcut serialises"),
            );
        }
        serde_json::Value::Object(map)
    }
}

#[derive(Debug, Error)]
pub enum ShortcutLoadError {
    #[error("`shortcuts` must be an object")]
    ShortcutsNotObject,
    #[error("invalid shortcut entry for action '{action}': {source}")]
    Invalid {
        action: String,
        #[source]
        source: serde_json::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_actions_have_unique_camel_case() {
        let mut seen = std::collections::HashSet::new();
        for action in KeyboardShortcutAction::ALL {
            assert!(seen.insert(action.camel_case()), "duplicate: {:?}", action);
        }
        assert_eq!(seen.len(), KeyboardShortcutAction::ALL.len());
    }

    #[test]
    fn camel_case_round_trips() {
        for action in KeyboardShortcutAction::ALL {
            let name = action.camel_case();
            assert_eq!(KeyboardShortcutAction::from_camel_case(name), Some(*action));
        }
    }

    #[test]
    fn default_shortcut_matches_swift_open_settings() {
        let s = KeyboardShortcutAction::OpenSettings.default_shortcut();
        assert_eq!(s.key, ",");
        assert!(s.command);
        assert!(!s.shift && !s.option && !s.control);
    }

    #[test]
    fn default_shortcut_matches_swift_focus_left() {
        let s = KeyboardShortcutAction::FocusLeft.default_shortcut();
        assert_eq!(s.key, "\u{2190}"); // ←
        assert!(s.command && s.option);
        assert!(!s.shift && !s.control);
    }

    #[test]
    fn overrides_round_trip_through_json() {
        let mut settings = KeyboardShortcutSettings::new();
        settings.set(
            KeyboardShortcutAction::Quit,
            StoredShortcut {
                key: "q".into(),
                command: true,
                shift: true,
                option: false,
                control: false,
                key_code: None,
                chord_key: None,
                chord_command: false,
                chord_shift: false,
                chord_option: false,
                chord_control: false,
                chord_key_code: None,
            },
        );
        let fragment = settings.to_settings_json_fragment();
        let wrapped = serde_json::json!({ "shortcuts": fragment });
        let parsed = KeyboardShortcutSettings::from_settings_json(&wrapped).unwrap();
        assert!(parsed.is_overridden(KeyboardShortcutAction::Quit));
        let s = parsed.shortcut(KeyboardShortcutAction::Quit);
        assert!(s.shift);
    }

    #[test]
    fn unknown_action_keys_are_ignored() {
        let wrapped = serde_json::json!({
            "shortcuts": {
                "someUnknownAction": {
                    "key": "x", "command": true, "shift": false, "option": false, "control": false
                }
            }
        });
        let parsed = KeyboardShortcutSettings::from_settings_json(&wrapped).unwrap();
        assert_eq!(parsed.overrides.len(), 0);
    }

    #[test]
    fn malformed_shortcut_entry_errors() {
        let wrapped = serde_json::json!({
            "shortcuts": {
                "quit": "nope"
            }
        });
        let err = KeyboardShortcutSettings::from_settings_json(&wrapped).unwrap_err();
        matches!(err, ShortcutLoadError::Invalid { .. });
    }

    #[test]
    fn shortcuts_top_level_must_be_object() {
        let wrapped = serde_json::json!({ "shortcuts": [] });
        let err = KeyboardShortcutSettings::from_settings_json(&wrapped).unwrap_err();
        matches!(err, ShortcutLoadError::ShortcutsNotObject);
    }

    #[test]
    fn missing_shortcuts_block_gives_empty_overrides() {
        let wrapped = serde_json::json!({});
        let parsed = KeyboardShortcutSettings::from_settings_json(&wrapped).unwrap();
        assert_eq!(parsed.overrides.len(), 0);
    }
}
