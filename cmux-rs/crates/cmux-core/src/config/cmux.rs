//! Port of [Sources/CmuxConfig.swift].
//!
//! The Swift type [`CmuxConfigFile`](../../../../../Sources/CmuxConfig.swift)
//! describes the JSON schema at `~/.config/cmux/cmux.json` (global) and
//! `cmux.json` files anywhere up the CWD ancestry (local). Commands listed
//! in the local config override those in the global config by name.
//!
//! The on-disk format is stable; we match the Swift parser byte-for-byte
//! so existing `cmux.json` files keep working against either binary.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// Root of a parsed `cmux.json` file.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CmuxConfigFile {
    #[serde(default)]
    pub commands: Vec<CmuxCommandDefinition>,
}

/// Restart behaviour for a cmux command. Mirrors `CmuxRestartBehavior`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CmuxRestartBehavior {
    Recreate,
    Ignore,
    Confirm,
}

/// One command entry from `cmux.json`.
///
/// A command must define *exactly one* of `workspace` or `command` — if
/// it sets both or neither the config is rejected with [`ConfigLoadError`].
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CmuxCommandDefinition {
    pub name: String,
    pub description: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub restart: Option<CmuxRestartBehavior>,
    pub workspace: Option<CmuxWorkspaceDefinition>,
    pub command: Option<String>,
    pub confirm: Option<bool>,
}

impl CmuxCommandDefinition {
    /// Stable id used as a dictionary key — matches the Swift `id`
    /// computed property exactly (percent-encoded name).
    pub fn id(&self) -> String {
        let mut out = String::from("cmux.config.command.");
        for byte in self.name.bytes() {
            if byte.is_ascii_alphanumeric() {
                out.push(byte as char);
            } else {
                out.push_str(&format!("%{byte:02X}"));
            }
        }
        out
    }
}

impl<'de> Deserialize<'de> for CmuxCommandDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            name: String,
            description: Option<String>,
            keywords: Option<Vec<String>>,
            restart: Option<CmuxRestartBehavior>,
            workspace: Option<CmuxWorkspaceDefinition>,
            command: Option<String>,
            confirm: Option<bool>,
        }

        let raw = Raw::deserialize(deserializer)?;
        if raw.name.trim().is_empty() {
            return Err(serde::de::Error::custom("Command name must not be blank"));
        }
        if let Some(cmd) = raw.command.as_ref() {
            if cmd.trim().is_empty() {
                return Err(serde::de::Error::custom(format!(
                    "Command '{}' must not define a blank 'command'",
                    raw.name
                )));
            }
        }
        if raw.workspace.is_some() && raw.command.is_some() {
            return Err(serde::de::Error::custom(format!(
                "Command '{}' must not define both 'workspace' and 'command'",
                raw.name
            )));
        }
        if raw.workspace.is_none() && raw.command.is_none() {
            return Err(serde::de::Error::custom(format!(
                "Command '{}' must define either 'workspace' or 'command'",
                raw.name
            )));
        }
        Ok(Self {
            name: raw.name,
            description: raw.description,
            keywords: raw.keywords,
            restart: raw.restart,
            workspace: raw.workspace,
            command: raw.command,
            confirm: raw.confirm,
        })
    }
}

/// Workspace template — mirrors `CmuxWorkspaceDefinition`. Note that
/// colour normalisation (6-digit `#RRGGBB`) is enforced here, same as
/// the Swift implementation.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct CmuxWorkspaceDefinition {
    pub name: Option<String>,
    pub cwd: Option<String>,
    pub color: Option<String>,
    pub layout: Option<CmuxLayoutNode>,
}

impl<'de> Deserialize<'de> for CmuxWorkspaceDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            name: Option<String>,
            cwd: Option<String>,
            color: Option<String>,
            layout: Option<CmuxLayoutNode>,
        }
        let raw = Raw::deserialize(deserializer)?;
        let color = match raw.color.as_deref() {
            Some(c) => Some(normalize_hex_color(c).ok_or_else(|| {
                serde::de::Error::custom(format!(
                    "Invalid color \"{c}\". Expected 6-digit hex format: #RRGGBB"
                ))
            })?),
            None => None,
        };
        Ok(Self {
            name: raw.name,
            cwd: raw.cwd,
            color,
            layout: raw.layout,
        })
    }
}

/// Layout tree node. Ported from `indirect enum CmuxLayoutNode`.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(untagged)]
pub enum CmuxLayoutNode {
    Pane {
        pane: CmuxPaneDefinition,
    },
    Split {
        #[serde(flatten)]
        split: CmuxSplitDefinition,
    },
}

impl<'de> Deserialize<'de> for CmuxLayoutNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let obj = value
            .as_object()
            .ok_or_else(|| serde::de::Error::custom("CmuxLayoutNode must be an object"))?;
        let has_pane = obj.contains_key("pane");
        let has_direction = obj.contains_key("direction");
        if has_pane && has_direction {
            return Err(serde::de::Error::custom(
                "CmuxLayoutNode must not contain both 'pane' and 'direction' keys",
            ));
        }
        if has_pane {
            let pane: CmuxPaneDefinition = serde_json::from_value(obj["pane"].clone())
                .map_err(serde::de::Error::custom)?;
            Ok(CmuxLayoutNode::Pane { pane })
        } else if has_direction {
            let split: CmuxSplitDefinition =
                serde_json::from_value(value).map_err(serde::de::Error::custom)?;
            Ok(CmuxLayoutNode::Split { split })
        } else {
            Err(serde::de::Error::custom(
                "CmuxLayoutNode must contain either a 'pane' key or a 'direction' key",
            ))
        }
    }
}

/// Split definition — exactly two children.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CmuxSplitDefinition {
    pub direction: CmuxSplitDirection,
    pub split: Option<f64>,
    pub children: Vec<CmuxLayoutNode>,
}

impl<'de> Deserialize<'de> for CmuxSplitDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            direction: CmuxSplitDirection,
            split: Option<f64>,
            children: Vec<CmuxLayoutNode>,
        }
        let raw = Raw::deserialize(deserializer)?;
        if raw.children.len() != 2 {
            return Err(serde::de::Error::custom(format!(
                "Split node requires exactly 2 children, got {}",
                raw.children.len()
            )));
        }
        Ok(Self {
            direction: raw.direction,
            split: raw.split,
            children: raw.children,
        })
    }
}

impl CmuxSplitDefinition {
    /// Clamped split position, matching the Swift getter (0.1 .. 0.9).
    pub fn clamped_split_position(&self) -> f64 {
        let value = self.split.unwrap_or(0.5);
        value.clamp(0.1, 0.9)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CmuxSplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CmuxPaneDefinition {
    pub surfaces: Vec<CmuxSurfaceDefinition>,
}

impl<'de> Deserialize<'de> for CmuxPaneDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            surfaces: Vec<CmuxSurfaceDefinition>,
        }
        let raw = Raw::deserialize(deserializer)?;
        if raw.surfaces.is_empty() {
            return Err(serde::de::Error::custom(
                "Pane node must contain at least one surface",
            ));
        }
        Ok(Self {
            surfaces: raw.surfaces,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CmuxSurfaceDefinition {
    #[serde(rename = "type")]
    pub surface_type: CmuxSurfaceType,
    pub name: Option<String>,
    pub command: Option<String>,
    pub cwd: Option<String>,
    pub env: Option<std::collections::BTreeMap<String, String>>,
    pub url: Option<String>,
    pub focus: Option<bool>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CmuxSurfaceType {
    Terminal,
    Browser,
}

#[derive(Debug, Error)]
pub enum ConfigLoadError {
    #[error("io error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("parse error in {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

/// Load a single `cmux.json` file. Returns `Ok(None)` when the file is
/// absent or empty, mirroring the lenient Swift behaviour.
pub fn load_config_file(path: &Path) -> Result<Option<CmuxConfigFile>, ConfigLoadError> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path).map_err(|source| ConfigLoadError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if bytes.is_empty() {
        return Ok(None);
    }
    let cfg: CmuxConfigFile =
        serde_json::from_slice(&bytes).map_err(|source| ConfigLoadError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(Some(cfg))
}

/// Resolve a config `cwd` value relative to a base directory. Handles
/// `~/`, absolute, and relative paths, matching `CmuxConfigStore.resolveCwd`.
pub fn resolve_cwd(cwd: Option<&str>, base_cwd: &Path) -> PathBuf {
    let Some(cwd) = cwd else {
        return base_cwd.to_path_buf();
    };
    if cwd.is_empty() || cwd == "." {
        return base_cwd.to_path_buf();
    }
    if cwd == "~" {
        return dirs::home_dir().unwrap_or_else(|| base_cwd.to_path_buf());
    }
    if let Some(rest) = cwd.strip_prefix("~/") {
        return dirs::home_dir()
            .map(|h| h.join(rest))
            .unwrap_or_else(|| base_cwd.join(rest));
    }
    if Path::new(cwd).is_absolute() {
        return PathBuf::from(cwd);
    }
    base_cwd.join(cwd)
}

/// Walk upward from `start` looking for `cmux.json`, returning the first
/// match. Matches the Swift `findCmuxConfig(startingFrom:)` helper.
pub fn find_local_config(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join("cmux.json");
        if candidate.exists() {
            return Some(candidate);
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent.to_path_buf(),
            _ => return None,
        }
    }
}

/// Path to the global `cmux.json` — `~/.config/cmux/cmux.json`.
pub fn global_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".config/cmux/cmux.json"))
}

/// Merge local + global commands with local precedence, matching
/// `CmuxConfigStore.loadAll`. Returns `(commands, source_paths)` where
/// `source_paths` maps each command id to the file it came from.
pub fn merge_configs(
    local: Option<(PathBuf, CmuxConfigFile)>,
    global: Option<(PathBuf, CmuxConfigFile)>,
) -> (
    Vec<CmuxCommandDefinition>,
    std::collections::BTreeMap<String, PathBuf>,
) {
    let mut commands = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut source_paths = std::collections::BTreeMap::new();

    if let Some((path, file)) = local {
        for cmd in file.commands {
            if !seen.contains(&cmd.name) {
                seen.insert(cmd.name.clone());
                source_paths.insert(cmd.id(), path.clone());
                commands.push(cmd);
            }
        }
    }

    if let Some((path, file)) = global {
        for cmd in file.commands {
            if !seen.contains(&cmd.name) {
                seen.insert(cmd.name.clone());
                source_paths.insert(cmd.id(), path.clone());
                commands.push(cmd);
            }
        }
    }

    (commands, source_paths)
}

/// Normalise `#RRGGBB` or `RRGGBB` to `#rrggbb`. Returns `None` for any
/// other shape. Matches `WorkspaceTabColorSettings.normalizedHex`.
pub fn normalize_hex_color(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let stripped = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if stripped.len() != 6 {
        return None;
    }
    if !stripped.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{}", stripped.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_command_with_command_field() {
        let json = r#"{
            "commands": [
                {"name": "hello", "command": "echo hi"}
            ]
        }"#;
        let file: CmuxConfigFile = serde_json::from_str(json).unwrap();
        assert_eq!(file.commands.len(), 1);
        assert_eq!(file.commands[0].command.as_deref(), Some("echo hi"));
    }

    #[test]
    fn rejects_command_with_both_workspace_and_command() {
        let json = r#"{
            "commands": [
                {"name": "x", "command": "echo", "workspace": {}}
            ]
        }"#;
        let err = serde_json::from_str::<CmuxConfigFile>(json).unwrap_err();
        assert!(err.to_string().contains("must not define both"));
    }

    #[test]
    fn rejects_command_with_neither_workspace_nor_command() {
        let json = r#"{"commands":[{"name":"x"}]}"#;
        let err = serde_json::from_str::<CmuxConfigFile>(json).unwrap_err();
        assert!(err.to_string().contains("must define either"));
    }

    #[test]
    fn rejects_blank_command_name() {
        let json = r#"{"commands":[{"name":"   ","command":"echo"}]}"#;
        let err = serde_json::from_str::<CmuxConfigFile>(json).unwrap_err();
        assert!(err.to_string().contains("must not be blank"));
    }

    #[test]
    fn split_node_requires_two_children() {
        let json = r#"{
            "commands": [{
                "name": "w",
                "workspace": {
                    "layout": {"direction": "horizontal", "children": []}
                }
            }]
        }"#;
        let err = serde_json::from_str::<CmuxConfigFile>(json).unwrap_err();
        assert!(err.to_string().contains("exactly 2 children"));
    }

    #[test]
    fn pane_node_parses() {
        let json = r#"{
            "commands": [{
                "name": "w",
                "workspace": {
                    "layout": {"pane": {"surfaces": [{"type": "terminal"}]}}
                }
            }]
        }"#;
        let file: CmuxConfigFile = serde_json::from_str(json).unwrap();
        let layout = file.commands[0]
            .workspace
            .as_ref()
            .and_then(|w| w.layout.as_ref())
            .unwrap();
        match layout {
            CmuxLayoutNode::Pane { pane } => assert_eq!(pane.surfaces.len(), 1),
            _ => panic!("expected pane"),
        }
    }

    #[test]
    fn split_node_clamps_split_position() {
        let json = r#"{
            "commands": [{
                "name": "w",
                "workspace": {
                    "layout": {
                        "direction": "vertical",
                        "split": 0.01,
                        "children": [
                            {"pane": {"surfaces": [{"type": "terminal"}]}},
                            {"pane": {"surfaces": [{"type": "terminal"}]}}
                        ]
                    }
                }
            }]
        }"#;
        let file: CmuxConfigFile = serde_json::from_str(json).unwrap();
        let layout = file.commands[0]
            .workspace
            .as_ref()
            .and_then(|w| w.layout.as_ref())
            .unwrap();
        let CmuxLayoutNode::Split { split } = layout else {
            panic!("expected split");
        };
        assert!((split.clamped_split_position() - 0.1).abs() < 1e-9);
    }

    #[test]
    fn normalize_hex_color_accepts_prefixed_and_unprefixed() {
        assert_eq!(normalize_hex_color("#ABCDEF"), Some("#abcdef".into()));
        assert_eq!(normalize_hex_color("abcdef"), Some("#abcdef".into()));
        assert_eq!(normalize_hex_color("xyz"), None);
        assert_eq!(normalize_hex_color("#12345"), None);
    }

    #[test]
    fn command_id_percent_encodes_name() {
        let cmd = CmuxCommandDefinition {
            name: "hello world!".into(),
            description: None,
            keywords: None,
            restart: None,
            workspace: None,
            command: Some("true".into()),
            confirm: None,
        };
        // Space is 0x20, `!` is 0x21.
        assert_eq!(cmd.id(), "cmux.config.command.hello%20world%21");
    }

    #[test]
    fn resolve_cwd_handles_tilde_absolute_relative() {
        let base = Path::new("/tmp/base");
        assert_eq!(resolve_cwd(None, base), PathBuf::from("/tmp/base"));
        assert_eq!(resolve_cwd(Some("."), base), PathBuf::from("/tmp/base"));
        assert_eq!(resolve_cwd(Some(""), base), PathBuf::from("/tmp/base"));
        assert_eq!(resolve_cwd(Some("/abs/path"), base), PathBuf::from("/abs/path"));
        assert_eq!(resolve_cwd(Some("sub/dir"), base), PathBuf::from("/tmp/base/sub/dir"));
    }

    #[test]
    fn merge_prefers_local_over_global() {
        let local = CmuxConfigFile {
            commands: vec![CmuxCommandDefinition {
                name: "a".into(),
                description: None,
                keywords: None,
                restart: None,
                workspace: None,
                command: Some("local".into()),
                confirm: None,
            }],
        };
        let global = CmuxConfigFile {
            commands: vec![
                CmuxCommandDefinition {
                    name: "a".into(),
                    description: None,
                    keywords: None,
                    restart: None,
                    workspace: None,
                    command: Some("global".into()),
                    confirm: None,
                },
                CmuxCommandDefinition {
                    name: "b".into(),
                    description: None,
                    keywords: None,
                    restart: None,
                    workspace: None,
                    command: Some("global-b".into()),
                    confirm: None,
                },
            ],
        };
        let (merged, _) = merge_configs(
            Some((PathBuf::from("/local.json"), local)),
            Some((PathBuf::from("/global.json"), global)),
        );
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].name, "a");
        assert_eq!(merged[0].command.as_deref(), Some("local"));
        assert_eq!(merged[1].name, "b");
    }
}
