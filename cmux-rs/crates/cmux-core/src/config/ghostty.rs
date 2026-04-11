//! Port of the data-only parts of [Sources/GhosttyConfig.swift].
//!
//! The Swift [`GhosttyConfig`](../../../../../Sources/GhosttyConfig.swift)
//! is half data (font family, palette, scrollback) and half AppKit
//! glue (`NSColor` loading, `NSAppearance` observation, theme lookup
//! via Ghostty's C API). This module keeps only the data layer: parsing
//! `key = value` files at `~/.config/ghostty/config` and the cmux
//! overrides at `~/.config/cmux/ghostty.config`.
//!
//! Colour values are stored as opaque `String`s here; mapping to a
//! runtime colour type is deferred to Phase 6 (frontend terminal
//! rendering), since the frontend will parse them directly.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::cmux::ConfigLoadError;

/// Which theme variant to load — matches `ColorSchemePreference`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorSchemePreference {
    Light,
    Dark,
}

/// Single palette colour entry (index 0..15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhosttyPaletteColor {
    pub index: u8,
    pub hex: String,
}

/// Data-only view of a parsed Ghostty config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GhosttyConfig {
    pub font_family: String,
    pub font_size: f32,
    pub surface_tab_bar_font_size: f32,
    pub theme: Option<String>,
    pub working_directory: Option<String>,
    pub scrollback_limit: u64,
    pub unfocused_split_opacity: f64,
    pub background: Option<String>,
    pub background_opacity: f64,
    pub foreground: Option<String>,
    pub cursor_color: Option<String>,
    pub cursor_text_color: Option<String>,
    pub selection_background: Option<String>,
    pub selection_foreground: Option<String>,
    pub palette: BTreeMap<u8, String>,
    pub sidebar_background: Option<String>,
    pub sidebar_background_light: Option<String>,
    pub sidebar_background_dark: Option<String>,
    pub sidebar_tint_opacity: Option<f64>,
    pub raw_entries: BTreeMap<String, String>,
}

impl Default for GhosttyConfig {
    fn default() -> Self {
        // Defaults mirror the Swift initial values in
        // [Sources/GhosttyConfig.swift] (see the struct field defaults).
        Self {
            font_family: "Menlo".into(),
            font_size: 12.0,
            surface_tab_bar_font_size: 11.0,
            theme: None,
            working_directory: None,
            scrollback_limit: 10_000,
            unfocused_split_opacity: 0.7,
            background: Some("#272822".into()),
            background_opacity: 1.0,
            foreground: Some("#fdfff1".into()),
            cursor_color: Some("#c0c1b5".into()),
            cursor_text_color: Some("#8d8e82".into()),
            selection_background: Some("#57584f".into()),
            selection_foreground: Some("#fdfff1".into()),
            palette: BTreeMap::new(),
            sidebar_background: None,
            sidebar_background_light: None,
            sidebar_background_dark: None,
            sidebar_tint_opacity: None,
            raw_entries: BTreeMap::new(),
        }
    }
}

impl GhosttyConfig {
    /// Parse a Ghostty-style `key = value` config from a string. Unknown
    /// keys are preserved in `raw_entries` so the Phase 6 terminal
    /// rendering layer can consume them without a second parse pass.
    pub fn parse_str(input: &str) -> Self {
        let mut cfg = GhosttyConfig::default();
        for raw_line in input.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim().to_string();
            cfg.raw_entries.insert(key.clone(), value.clone());
            apply_entry(&mut cfg, &key, &value);
        }
        cfg
    }

    /// Load from one or more config files. Later files override earlier
    /// ones. Missing files are silently skipped.
    pub fn load_from_files(paths: &[&Path]) -> Result<Self, ConfigLoadError> {
        let mut merged = String::new();
        for path in paths {
            if !path.exists() {
                continue;
            }
            let text = fs::read_to_string(path).map_err(|source| ConfigLoadError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            merged.push_str(&text);
            merged.push('\n');
        }
        Ok(Self::parse_str(&merged))
    }

    /// Standard search paths for cmux: first Ghostty's own config,
    /// then the cmux override file.
    pub fn default_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".config/ghostty/config"));
            paths.push(home.join(".config/cmux/ghostty.config"));
        }
        paths
    }
}

fn apply_entry(cfg: &mut GhosttyConfig, key: &str, value: &str) {
    match key {
        "font-family" => cfg.font_family = value.to_string(),
        "font-size" => {
            if let Ok(n) = value.parse() {
                cfg.font_size = n;
            }
        }
        "surface-tab-bar-font-size" => {
            if let Ok(n) = value.parse() {
                cfg.surface_tab_bar_font_size = n;
            }
        }
        "theme" => cfg.theme = Some(value.to_string()),
        "working-directory" => cfg.working_directory = Some(value.to_string()),
        "scrollback-limit" => {
            if let Ok(n) = value.parse() {
                cfg.scrollback_limit = n;
            }
        }
        "unfocused-split-opacity" => {
            if let Ok(n) = value.parse() {
                cfg.unfocused_split_opacity = n;
            }
        }
        "background" => cfg.background = Some(value.to_string()),
        "background-opacity" => {
            if let Ok(n) = value.parse() {
                cfg.background_opacity = n;
            }
        }
        "foreground" => cfg.foreground = Some(value.to_string()),
        "cursor-color" => cfg.cursor_color = Some(value.to_string()),
        "cursor-text" => cfg.cursor_text_color = Some(value.to_string()),
        "selection-background" => cfg.selection_background = Some(value.to_string()),
        "selection-foreground" => cfg.selection_foreground = Some(value.to_string()),
        "sidebar-background" => cfg.sidebar_background = Some(value.to_string()),
        "sidebar-background-light" => cfg.sidebar_background_light = Some(value.to_string()),
        "sidebar-background-dark" => cfg.sidebar_background_dark = Some(value.to_string()),
        "sidebar-tint-opacity" => {
            if let Ok(n) = value.parse() {
                cfg.sidebar_tint_opacity = Some(n);
            }
        }
        _ => {
            if let Some(rest) = key.strip_prefix("palette") {
                // Accept `palette0 = ...`, `palette-0 = ...`, `palette = 0=...`.
                let index_str = rest.trim_start_matches(['-', '=']);
                if let Ok(idx) = index_str.parse::<u8>() {
                    cfg.palette.insert(idx, value.to_string());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_swift_defaults() {
        let cfg = GhosttyConfig::default();
        assert_eq!(cfg.font_family, "Menlo");
        assert_eq!(cfg.font_size, 12.0);
        assert_eq!(cfg.scrollback_limit, 10_000);
        assert_eq!(cfg.background.as_deref(), Some("#272822"));
    }

    #[test]
    fn parse_key_value_pairs() {
        let input = r#"
            # comment
            font-family = JetBrains Mono
            font-size = 14
            theme = dark:tokyo-night

            background = #000000
            palette0 = #ff0000
            palette-15 = #ffffff
            unknown-key = whatever
        "#;
        let cfg = GhosttyConfig::parse_str(input);
        assert_eq!(cfg.font_family, "JetBrains Mono");
        assert_eq!(cfg.font_size, 14.0);
        assert_eq!(cfg.theme.as_deref(), Some("dark:tokyo-night"));
        assert_eq!(cfg.background.as_deref(), Some("#000000"));
        assert_eq!(cfg.palette.get(&0).map(|s| s.as_str()), Some("#ff0000"));
        assert_eq!(cfg.palette.get(&15).map(|s| s.as_str()), Some("#ffffff"));
        assert_eq!(cfg.raw_entries.get("unknown-key").map(|s| s.as_str()), Some("whatever"));
    }

    #[test]
    fn later_files_override_earlier_ones() {
        let tmp = tempdir_sibling();
        let a = tmp.join("a.config");
        let b = tmp.join("b.config");
        fs::write(&a, "font-size = 12\nfont-family = A").unwrap();
        fs::write(&b, "font-size = 20").unwrap();
        let cfg = GhosttyConfig::load_from_files(&[a.as_path(), b.as_path()]).unwrap();
        assert_eq!(cfg.font_size, 20.0);
        assert_eq!(cfg.font_family, "A");
        fs::remove_file(&a).ok();
        fs::remove_file(&b).ok();
    }

    fn tempdir_sibling() -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "cmux-core-ghostty-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&base).unwrap();
        base
    }
}
