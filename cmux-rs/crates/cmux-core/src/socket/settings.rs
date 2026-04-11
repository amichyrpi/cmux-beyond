//! Port of [Sources/SocketControlSettings.swift].
//!
//! Owns:
//! - [`SocketControlMode`]: which processes may connect and how they
//!   are authenticated (`off`, `cmuxOnly`, `automation`, `password`,
//!   `allowAll`).
//! - Socket path resolution — the same layered rules the Swift app
//!   uses so the `CMUX_SOCKET` / `CMUX_SOCKET_PATH` env vars and the
//!   tagged-debug bundle-ID trick keep working across binaries.
//!
//! Keychain access is intentionally **not** ported: the legacy macOS
//! Keychain fallback from Swift is replaced by the plain file store
//! at `~/Library/Application Support/cmux/socket-control-password`
//! (macOS) or the XDG equivalent on Linux / `%APPDATA%` on Windows.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Auth / access mode for the local control socket. Values match the
/// Swift `SocketControlMode` rawValue strings for on-disk compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SocketControlMode {
    /// Listener is not started at all.
    Off,
    /// Only processes whose ancestry chains back to cmux may connect.
    CmuxOnly,
    /// Any local process belonging to the current user may connect.
    Automation,
    /// Any local process with the correct `AUTH <password>` handshake
    /// may connect.
    Password,
    /// No authentication at all — socket file permissions are the only
    /// gate. Dangerous on shared hosts.
    AllowAll,
}

impl SocketControlMode {
    pub const UI_CASES: &'static [SocketControlMode] = &[
        Self::Off,
        Self::CmuxOnly,
        Self::Automation,
        Self::Password,
        Self::AllowAll,
    ];

    /// Unix permission bits applied to the socket file for this mode.
    pub fn socket_file_permissions(self) -> u32 {
        match self {
            Self::AllowAll => 0o666,
            _ => 0o600,
        }
    }

    pub fn requires_password_auth(self) -> bool {
        matches!(self, Self::Password)
    }

    /// Default mode for new installs — matches Swift `defaultMode`.
    pub fn default_mode() -> Self {
        Self::CmuxOnly
    }

    /// Parse a user-facing mode string. Accepts the canonical name
    /// (`cmuxOnly`), snake-case (`cmux_only`), kebab-case
    /// (`cmux-only`), and legacy aliases `notifications`/`full`.
    pub fn parse(raw: &str) -> Option<Self> {
        let normalised: String = raw
            .trim()
            .to_ascii_lowercase()
            .chars()
            .filter(|c| *c != '_' && *c != '-')
            .collect();
        match normalised.as_str() {
            "off" => Some(Self::Off),
            "cmuxonly" => Some(Self::CmuxOnly),
            "automation" => Some(Self::Automation),
            "password" => Some(Self::Password),
            "allowall" | "openaccess" | "fullopenaccess" => Some(Self::AllowAll),
            "notifications" => Some(Self::Automation),
            "full" => Some(Self::AllowAll),
            _ => None,
        }
    }

    /// Load the persisted mode from a user-supplied string, falling
    /// back to [`default_mode`] on unknown values.
    pub fn migrate(raw: &str) -> Self {
        Self::parse(raw).unwrap_or_else(Self::default_mode)
    }
}

pub const SOCKET_PASSWORD_ENV: &str = "CMUX_SOCKET_PASSWORD";
pub const SOCKET_ENABLE_ENV: &str = "CMUX_SOCKET_ENABLE";
pub const SOCKET_MODE_ENV: &str = "CMUX_SOCKET_MODE";
pub const SOCKET_PATH_ENV: &str = "CMUX_SOCKET_PATH";
pub const SOCKET_PATH_ALT_ENV: &str = "CMUX_SOCKET";
pub const ALLOW_SOCKET_PATH_OVERRIDE_ENV: &str = "CMUX_ALLOW_SOCKET_OVERRIDE";
pub const LAUNCH_TAG_ENV: &str = "CMUX_TAG";

/// The `/tmp/cmux-...sock` ownership check result used by the Swift
/// `inspectStableDefaultSocketPathEntry` helper. Windows builds never
/// inspect `/tmp` so this enum is still returned but only used on
/// Unix-like platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StableDefaultSocketPathEntry {
    Missing,
    Socket { owner_uid: u32 },
    Other { owner_uid: u32 },
    Inaccessible { errno_code: i32 },
}

/// Honour [`SOCKET_ENABLE_ENV`] exactly like Swift. Returns `Some(true)`
/// to force-enable, `Some(false)` to force-disable, `None` if the env
/// var is unset / unparseable.
pub fn env_override_enabled(env: &HashMap<String, String>) -> Option<bool> {
    let raw = env.get(SOCKET_ENABLE_ENV)?;
    if raw.is_empty() {
        return None;
    }
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// Honour [`SOCKET_MODE_ENV`] — a raw mode name override.
pub fn env_override_mode(env: &HashMap<String, String>) -> Option<SocketControlMode> {
    let raw = env.get(SOCKET_MODE_ENV)?;
    if raw.is_empty() {
        return None;
    }
    SocketControlMode::parse(raw)
}

/// Combine the user preference, the `CMUX_SOCKET_ENABLE` kill switch,
/// and the `CMUX_SOCKET_MODE` explicit override into the effective
/// runtime mode. Matches Swift `SocketControlSettings.effectiveMode`.
pub fn effective_mode(
    user_mode: SocketControlMode,
    env: &HashMap<String, String>,
) -> SocketControlMode {
    if let Some(enabled) = env_override_enabled(env) {
        if !enabled {
            return SocketControlMode::Off;
        }
        if let Some(mode) = env_override_mode(env) {
            return mode;
        }
        return if user_mode == SocketControlMode::Off {
            SocketControlMode::CmuxOnly
        } else {
            user_mode
        };
    }
    if let Some(mode) = env_override_mode(env) {
        return mode;
    }
    user_mode
}

/// The stable default path for non-debug builds —
/// `~/Library/Application Support/cmux/cmux.sock` on macOS, the XDG
/// equivalent on Linux, `%APPDATA%\cmux\cmux.sock` on Windows. Falls
/// back to `/tmp/cmux.sock` if no data directory is available.
pub fn stable_default_socket_path() -> PathBuf {
    stable_socket_directory()
        .map(|d| d.join("cmux.sock"))
        .unwrap_or_else(|| PathBuf::from("/tmp/cmux.sock"))
}

/// Directory backing the stable socket path — also used to store the
/// `last-socket-path` marker.
pub fn stable_socket_directory() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("cmux"))
}

/// The per-user scoped fallback used when the stable default is owned
/// by another UID. Matches Swift `userScopedStableSocketPath`.
pub fn user_scoped_stable_socket_path(current_uid: u32) -> PathBuf {
    stable_socket_directory()
        .map(|d| d.join(format!("cmux-{current_uid}.sock")))
        .unwrap_or_else(|| PathBuf::from(format!("/tmp/cmux-{current_uid}.sock")))
}

/// Inspect the stable default path and return its ownership category.
#[cfg(unix)]
pub fn inspect_stable_default_socket_path_entry(path: &Path) -> StableDefaultSocketPathEntry {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            let file_type = meta.file_type();
            if file_type.is_socket() {
                StableDefaultSocketPathEntry::Socket { owner_uid: meta.uid() }
            } else {
                StableDefaultSocketPathEntry::Other { owner_uid: meta.uid() }
            }
        }
        Err(err) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                StableDefaultSocketPathEntry::Missing
            } else {
                let code = err.raw_os_error().unwrap_or(0);
                StableDefaultSocketPathEntry::Inaccessible { errno_code: code }
            }
        }
    }
}

#[cfg(not(unix))]
pub fn inspect_stable_default_socket_path_entry(path: &Path) -> StableDefaultSocketPathEntry {
    // On Windows there is no concept of "the stable default path is a
    // Unix socket owned by another uid" — the listener uses a named
    // pipe instead, so we report missing/other and let the listener
    // fall back to the named-pipe path.
    if path.exists() {
        StableDefaultSocketPathEntry::Other { owner_uid: 0 }
    } else {
        StableDefaultSocketPathEntry::Missing
    }
}

/// Pick the stable default path, falling back to the user-scoped path
/// if the primary is owned by another UID. Matches the Swift
/// `resolvedStableDefaultSocketPath`.
pub fn resolved_stable_default_socket_path(
    current_uid: u32,
    probe: impl Fn(&Path) -> StableDefaultSocketPathEntry,
) -> PathBuf {
    let primary = stable_default_socket_path();
    match probe(&primary) {
        StableDefaultSocketPathEntry::Missing => primary,
        StableDefaultSocketPathEntry::Socket { owner_uid } if owner_uid == current_uid => primary,
        _ => user_scoped_stable_socket_path(current_uid),
    }
}

/// Is the given bundle-id string a debug bundle (`com.cmuxterm.app.debug`
/// or a tagged variant `com.cmuxterm.app.debug.<tag>`).
pub fn is_debug_like_bundle_identifier(bundle_id: Option<&str>) -> bool {
    let Some(bundle_id) = bundle_id else {
        return false;
    };
    bundle_id == "com.cmuxterm.app.debug" || bundle_id.starts_with("com.cmuxterm.app.debug.")
}

pub fn is_staging_bundle_identifier(bundle_id: Option<&str>) -> bool {
    let Some(bundle_id) = bundle_id else {
        return false;
    };
    bundle_id == "com.cmuxterm.app.staging" || bundle_id.starts_with("com.cmuxterm.app.staging.")
}

fn slugify_tag(raw: &str) -> String {
    raw.to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Compute the tagged-debug socket path — `/tmp/cmux-debug-<slug>.sock`
/// — from the bundle id and/or `CMUX_TAG` env var. Returns `None`
/// when the current build is not a tagged debug build.
pub fn tagged_debug_socket_path(
    bundle_id: Option<&str>,
    env: &HashMap<String, String>,
) -> Option<PathBuf> {
    let bundle_id = bundle_id.map(str::trim).unwrap_or("");
    if bundle_id.starts_with("com.cmuxterm.app.debug.") {
        let suffix = &bundle_id["com.cmuxterm.app.debug.".len()..];
        let slug = suffix.replace('.', "-");
        let slug = slug.trim_matches('-');
        if !slug.is_empty() {
            return Some(PathBuf::from(format!("/tmp/cmux-debug-{slug}.sock")));
        }
    }

    let tag = env.get(LAUNCH_TAG_ENV).map(|s| s.trim()).unwrap_or("");
    if tag.is_empty() || bundle_id != "com.cmuxterm.app.debug" {
        return None;
    }
    let slug = slugify_tag(tag);
    if slug.is_empty() {
        return None;
    }
    Some(PathBuf::from(format!("/tmp/cmux-debug-{slug}.sock")))
}

/// Return the socket path to listen on. Matches the layered
/// rules in `SocketControlSettings.socketPath`:
///
/// 1. A tagged debug build always uses `/tmp/cmux-debug-<slug>.sock`.
/// 2. Otherwise `CMUX_SOCKET_PATH` / `CMUX_SOCKET` is honoured if the
///    bundle is debug-like *or* `CMUX_ALLOW_SOCKET_OVERRIDE` is truthy.
/// 3. Otherwise fall back to [`default_socket_path`].
pub fn socket_path(
    env: &HashMap<String, String>,
    bundle_id: Option<&str>,
    is_debug_build: bool,
    current_uid: u32,
    probe: impl Fn(&Path) -> StableDefaultSocketPathEntry,
) -> PathBuf {
    let fallback = default_socket_path(bundle_id, is_debug_build, current_uid, &probe);

    if let Some(tagged) = tagged_debug_socket_path(bundle_id, env) {
        if is_truthy(env.get(ALLOW_SOCKET_PATH_OVERRIDE_ENV).map(String::as_str)) {
            if let Some(override_path) = env
                .get(SOCKET_PATH_ENV)
                .or_else(|| env.get(SOCKET_PATH_ALT_ENV))
            {
                if !override_path.is_empty() {
                    return PathBuf::from(override_path);
                }
            }
        }
        return tagged;
    }

    let override_path = env
        .get(SOCKET_PATH_ENV)
        .or_else(|| env.get(SOCKET_PATH_ALT_ENV))
        .cloned()
        .unwrap_or_default();
    if override_path.is_empty() {
        return fallback;
    }

    if should_honour_override(env, bundle_id, is_debug_build) {
        return PathBuf::from(override_path);
    }

    fallback
}

fn should_honour_override(
    env: &HashMap<String, String>,
    bundle_id: Option<&str>,
    is_debug_build: bool,
) -> bool {
    if is_truthy(env.get(ALLOW_SOCKET_PATH_OVERRIDE_ENV).map(String::as_str)) {
        return true;
    }
    if is_debug_like_bundle_identifier(bundle_id) || is_staging_bundle_identifier(bundle_id) {
        return true;
    }
    is_debug_build
}

fn is_truthy(raw: Option<&str>) -> bool {
    let Some(raw) = raw else {
        return false;
    };
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Compute the effective socket path without any env override.
pub fn default_socket_path(
    bundle_id: Option<&str>,
    is_debug_build: bool,
    current_uid: u32,
    probe: &impl Fn(&Path) -> StableDefaultSocketPathEntry,
) -> PathBuf {
    if let Some(tagged) = tagged_debug_socket_path(bundle_id, &HashMap::new()) {
        return tagged;
    }
    if bundle_id == Some("com.cmuxterm.app.nightly") {
        return PathBuf::from("/tmp/cmux-nightly.sock");
    }
    if is_debug_like_bundle_identifier(bundle_id) || is_debug_build {
        return PathBuf::from("/tmp/cmux-debug.sock");
    }
    if is_staging_bundle_identifier(bundle_id) {
        return PathBuf::from("/tmp/cmux-staging.sock");
    }
    resolved_stable_default_socket_path(current_uid, probe)
}

/// Write `path` to `last-socket-path` markers so clients that lost the
/// CLI-provided override can still rendezvous. Mirrors the Swift
/// `recordLastSocketPath` helper.
pub fn record_last_socket_path(path: &Path) {
    let Some(dir) = stable_socket_directory() else {
        return;
    };
    let _ = fs::create_dir_all(&dir);
    let primary = dir.join("last-socket-path");
    let mut payload = path.display().to_string();
    payload.push('\n');
    let _ = fs::write(&primary, &payload);
    // Legacy /tmp marker for older clients.
    let _ = fs::write("/tmp/cmux-last-socket-path", &payload);
}

/// Capture the current process environment into a map suitable for
/// passing to the Settings helpers. Convenience wrapper around
/// `std::env::vars`.
pub fn process_env() -> HashMap<String, String> {
    env::vars().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn parses_canonical_mode_names() {
        assert_eq!(SocketControlMode::parse("off"), Some(SocketControlMode::Off));
        assert_eq!(
            SocketControlMode::parse("cmuxOnly"),
            Some(SocketControlMode::CmuxOnly)
        );
        assert_eq!(
            SocketControlMode::parse("cmux_only"),
            Some(SocketControlMode::CmuxOnly)
        );
        assert_eq!(
            SocketControlMode::parse("cmux-only"),
            Some(SocketControlMode::CmuxOnly)
        );
        assert_eq!(
            SocketControlMode::parse("allowAll"),
            Some(SocketControlMode::AllowAll)
        );
        assert_eq!(
            SocketControlMode::parse("notifications"),
            Some(SocketControlMode::Automation)
        );
        assert_eq!(
            SocketControlMode::parse("full"),
            Some(SocketControlMode::AllowAll)
        );
        assert_eq!(SocketControlMode::parse("whatever"), None);
    }

    #[test]
    fn effective_mode_kill_switch_disables_listener() {
        let env = env(&[("CMUX_SOCKET_ENABLE", "0")]);
        assert_eq!(
            effective_mode(SocketControlMode::CmuxOnly, &env),
            SocketControlMode::Off
        );
    }

    #[test]
    fn effective_mode_kill_switch_with_off_user_defaults_to_cmux_only() {
        let env = env(&[("CMUX_SOCKET_ENABLE", "1")]);
        assert_eq!(
            effective_mode(SocketControlMode::Off, &env),
            SocketControlMode::CmuxOnly
        );
    }

    #[test]
    fn effective_mode_env_override_mode_wins() {
        let env = env(&[("CMUX_SOCKET_MODE", "password")]);
        assert_eq!(
            effective_mode(SocketControlMode::CmuxOnly, &env),
            SocketControlMode::Password
        );
    }

    #[test]
    fn tagged_debug_socket_path_uses_slugified_tag() {
        let env = env(&[("CMUX_TAG", "My Tag!")]);
        let path =
            tagged_debug_socket_path(Some("com.cmuxterm.app.debug"), &env).unwrap();
        assert!(path.display().to_string().ends_with("cmux-debug-my-tag.sock"));
    }

    #[test]
    fn tagged_debug_socket_path_honours_bundle_suffix() {
        let env = env(&[]);
        let path = tagged_debug_socket_path(
            Some("com.cmuxterm.app.debug.feature.branch"),
            &env,
        )
        .unwrap();
        assert!(path
            .display()
            .to_string()
            .ends_with("cmux-debug-feature-branch.sock"));
    }

    #[test]
    fn tagged_debug_socket_path_returns_none_for_release_bundle() {
        let env = env(&[("CMUX_TAG", "anything")]);
        assert!(tagged_debug_socket_path(Some("com.cmuxterm.app"), &env).is_none());
    }

    #[test]
    fn socket_path_prefers_tagged_debug_over_override() {
        let env = env(&[("CMUX_SOCKET_PATH", "/tmp/user-override.sock")]);
        let p = socket_path(
            &env,
            Some("com.cmuxterm.app.debug.mytag"),
            true,
            1000,
            |_| StableDefaultSocketPathEntry::Missing,
        );
        assert_eq!(p, PathBuf::from("/tmp/cmux-debug-mytag.sock"));
    }

    #[test]
    fn socket_path_respects_override_when_allowed() {
        let env = env(&[
            ("CMUX_SOCKET_PATH", "/tmp/custom.sock"),
            ("CMUX_ALLOW_SOCKET_OVERRIDE", "1"),
        ]);
        let p = socket_path(&env, Some("com.cmuxterm.app"), false, 1000, |_| {
            StableDefaultSocketPathEntry::Missing
        });
        assert_eq!(p, PathBuf::from("/tmp/custom.sock"));
    }

    #[test]
    fn socket_path_ignores_override_when_disallowed() {
        let env = env(&[("CMUX_SOCKET_PATH", "/tmp/custom.sock")]);
        let p = socket_path(&env, Some("com.cmuxterm.app"), false, 1000, |_| {
            StableDefaultSocketPathEntry::Missing
        });
        assert_ne!(p, PathBuf::from("/tmp/custom.sock"));
    }

    #[test]
    fn default_socket_path_debug_like_bundles() {
        let p = default_socket_path(Some("com.cmuxterm.app.debug"), true, 1000, &|_| {
            StableDefaultSocketPathEntry::Missing
        });
        assert_eq!(p, PathBuf::from("/tmp/cmux-debug.sock"));

        let p = default_socket_path(Some("com.cmuxterm.app.nightly"), false, 1000, &|_| {
            StableDefaultSocketPathEntry::Missing
        });
        assert_eq!(p, PathBuf::from("/tmp/cmux-nightly.sock"));

        let p = default_socket_path(Some("com.cmuxterm.app.staging"), false, 1000, &|_| {
            StableDefaultSocketPathEntry::Missing
        });
        assert_eq!(p, PathBuf::from("/tmp/cmux-staging.sock"));
    }

    #[test]
    fn resolved_stable_default_picks_user_scoped_when_owned_by_other() {
        let other_uid = 9999;
        let p = resolved_stable_default_socket_path(1000, |_| {
            StableDefaultSocketPathEntry::Socket { owner_uid: other_uid }
        });
        assert!(p.display().to_string().contains("cmux-1000.sock"));
    }

    #[test]
    fn is_truthy_accepts_common_values() {
        assert!(is_truthy(Some("1")));
        assert!(is_truthy(Some("YES")));
        assert!(!is_truthy(Some("0")));
        assert!(!is_truthy(Some("")));
        assert!(!is_truthy(None));
    }
}
