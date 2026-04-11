//! Phase 3 headless command handler — implements just enough of the
//! Swift `TerminalController` v1 + v2 dispatch for the `tests_v2/`
//! handshake subset to work against the Rust binary.
//!
//! The handler is deliberately stateless beyond a few in-memory maps:
//! metadata (`report_meta`, `report_meta_block`, `sidebar_state`,
//! `set_agent_pid`) is stored in `parking_lot::Mutex<BTreeMap<..>>`
//! so that round-tripping via the socket (set → get) works. Anything
//! that requires real workspace/tab state — `workspace.*`,
//! `surface.*`, `pane.*`, `browser.*` — returns a structured
//! `method_not_available` error so the Python client can distinguish a
//! deliberately-unimplemented method from a crash.
//!
//! This is the Phase 3 deliverable described in `PLAN.md`:
//!
//! > tests_v2/ Python suite passes against the Rust binary for the
//! > baseline handshake (`system.ping`, `system.capabilities`,
//! > `system.identify`, `auth.login`, plus the metadata mutation
//! > commands). Full workspace/surface parity lands in Phase 4.

use std::collections::BTreeMap;
use std::sync::Arc;

use parking_lot::Mutex;
use serde_json::{json, Value};

use cmux_core::socket::dispatch::{
    AuthOutcome, CommandHandler, JsonRequest, JsonResponse, RequestContext,
};
use cmux_core::VERSION;

/// Per-surface metadata record — mirrors the Swift in-memory stores
/// backing `report_meta`, `report_pwd`, `report_git_branch`, etc.
#[derive(Debug, Default, Clone)]
struct SurfaceMeta {
    /// Arbitrary key/value pairs from `report_meta`.
    meta: BTreeMap<String, String>,
    /// Multi-line blocks from `report_meta_block`.
    meta_blocks: BTreeMap<String, String>,
    pwd: Option<String>,
    git_branch: Option<String>,
    pr: Option<String>,
    tty: Option<String>,
    ports: Option<String>,
    shell_state: Option<String>,
    agent_pid: Option<u32>,
}

#[derive(Debug, Default)]
struct SharedState {
    /// Keyed by the surface ref / target string the client supplied,
    /// or `""` for the implicit "current" surface.
    surfaces: BTreeMap<String, SurfaceMeta>,
    sidebar_state: Option<Value>,
    capabilities_cache: Option<Vec<String>>,
}

/// Phase 3 headless command handler. Shareable by `Arc` — `listen`
/// requires `Send + Sync + 'static`.
pub struct HeadlessHandler {
    inner: Arc<Mutex<SharedState>>,
    socket_path: String,
}

impl HeadlessHandler {
    pub fn new(socket_path: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SharedState::default())),
            socket_path: socket_path.into(),
        }
    }

    fn capabilities(&self) -> Vec<String> {
        let mut state = self.inner.lock();
        if let Some(cached) = &state.capabilities_cache {
            return cached.clone();
        }
        let methods: Vec<String> = BASELINE_V2_METHODS.iter().map(|s| s.to_string()).collect();
        state.capabilities_cache = Some(methods.clone());
        methods
    }

    fn identify(&self, _params: &Value) -> Value {
        json!({
            "socket_path": self.socket_path,
            "version": VERSION,
            "focused": Value::Null,
            "caller": Value::Null,
            "implementation": "cmux-rs (Phase 3 headless)",
        })
    }

    fn with_surface<F, R>(&self, target: &str, f: F) -> R
    where
        F: FnOnce(&mut SurfaceMeta) -> R,
    {
        let mut state = self.inner.lock();
        let entry = state
            .surfaces
            .entry(target.to_string())
            .or_insert_with(SurfaceMeta::default);
        f(entry)
    }

    fn snapshot_surface(&self, target: &str) -> Option<SurfaceMeta> {
        self.inner.lock().surfaces.get(target).cloned()
    }

    /// Parse the Swift v1 shape `<surface_ref> key value...` where the
    /// surface ref is optional. The default surface ref is the empty
    /// string (representing "current surface" in this phase).
    fn parse_targeted_kv(args: &str) -> (String, String, String) {
        // Swift uses: first token = surface ref (or key), rest = value.
        // We keep things simple: if there are at least 3 tokens, treat
        // the first as target, second as key, rest as value. Otherwise
        // first is key, rest is value, target is "".
        let mut it = args.splitn(3, char::is_whitespace);
        let a = it.next().unwrap_or("").trim().to_string();
        let b = it.next().unwrap_or("").trim().to_string();
        let c = it.next().unwrap_or("").to_string();
        if !c.is_empty() && !b.is_empty() {
            (a, b, c)
        } else if !b.is_empty() {
            (String::new(), a, b)
        } else {
            (String::new(), a, String::new())
        }
    }

    fn parse_single_value(args: &str) -> (String, String) {
        // "<target> <value>" or just "<value>"
        let mut it = args.splitn(2, char::is_whitespace);
        let a = it.next().unwrap_or("").trim().to_string();
        let b = it.next().unwrap_or("").trim().to_string();
        if b.is_empty() {
            (String::new(), a)
        } else {
            (a, b)
        }
    }
}

impl CommandHandler for HeadlessHandler {
    fn handle_v1(&self, _ctx: &RequestContext, cmd: &str, args: &str) -> String {
        match cmd {
            "ping" => "PONG".into(),
            "auth" => "OK: Authentication not required".into(),

            // Metadata — per-surface key/value store.
            "report_meta" => {
                let (target, key, value) = Self::parse_targeted_kv(args);
                if key.is_empty() {
                    return "ERROR: report_meta requires a key".into();
                }
                self.with_surface(&target, |s| {
                    s.meta.insert(key, value);
                });
                "OK".into()
            }
            "clear_meta" => {
                let (target, key) = Self::parse_single_value(args);
                self.with_surface(&target, |s| {
                    if key.is_empty() {
                        s.meta.clear();
                    } else {
                        s.meta.remove(&key);
                    }
                });
                "OK".into()
            }
            "list_meta" => {
                let (target, _) = Self::parse_single_value(args);
                let snapshot = self.snapshot_surface(&target).unwrap_or_default();
                let body: String = snapshot
                    .meta
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                if body.is_empty() {
                    "OK".into()
                } else {
                    format!("OK\n{body}")
                }
            }

            "report_meta_block" => {
                let (target, key, value) = Self::parse_targeted_kv(args);
                if key.is_empty() {
                    return "ERROR: report_meta_block requires a key".into();
                }
                self.with_surface(&target, |s| {
                    s.meta_blocks.insert(key, value);
                });
                "OK".into()
            }
            "clear_meta_block" => {
                let (target, key) = Self::parse_single_value(args);
                self.with_surface(&target, |s| {
                    if key.is_empty() {
                        s.meta_blocks.clear();
                    } else {
                        s.meta_blocks.remove(&key);
                    }
                });
                "OK".into()
            }
            "list_meta_blocks" => {
                let (target, _) = Self::parse_single_value(args);
                let snapshot = self.snapshot_surface(&target).unwrap_or_default();
                let body: String = snapshot
                    .meta_blocks
                    .iter()
                    .map(|(k, _)| k.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                if body.is_empty() {
                    "OK".into()
                } else {
                    format!("OK\n{body}")
                }
            }

            "report_pwd" => {
                let (target, pwd) = Self::parse_single_value(args);
                self.with_surface(&target, |s| s.pwd = Some(pwd));
                "OK".into()
            }
            "report_git_branch" => {
                let (target, branch) = Self::parse_single_value(args);
                self.with_surface(&target, |s| s.git_branch = Some(branch));
                "OK".into()
            }
            "clear_git_branch" => {
                let (target, _) = Self::parse_single_value(args);
                self.with_surface(&target, |s| s.git_branch = None);
                "OK".into()
            }
            "report_pr" | "report_review" => {
                let (target, payload) = Self::parse_single_value(args);
                self.with_surface(&target, |s| s.pr = Some(payload));
                "OK".into()
            }
            "clear_pr" => {
                let (target, _) = Self::parse_single_value(args);
                self.with_surface(&target, |s| s.pr = None);
                "OK".into()
            }
            "report_ports" => {
                let (target, payload) = Self::parse_single_value(args);
                self.with_surface(&target, |s| s.ports = Some(payload));
                "OK".into()
            }
            "clear_ports" => {
                let (target, _) = Self::parse_single_value(args);
                self.with_surface(&target, |s| s.ports = None);
                "OK".into()
            }
            "report_tty" => {
                let (target, payload) = Self::parse_single_value(args);
                self.with_surface(&target, |s| s.tty = Some(payload));
                "OK".into()
            }
            "ports_kick" => "OK".into(),
            "report_shell_state" => {
                let (target, payload) = Self::parse_single_value(args);
                self.with_surface(&target, |s| s.shell_state = Some(payload));
                "OK".into()
            }
            "set_agent_pid" => {
                let (target, pid_str) = Self::parse_single_value(args);
                let pid: Option<u32> = pid_str.parse().ok();
                self.with_surface(&target, |s| s.agent_pid = pid);
                "OK".into()
            }
            "clear_agent_pid" => {
                let (target, _) = Self::parse_single_value(args);
                self.with_surface(&target, |s| s.agent_pid = None);
                "OK".into()
            }

            "sidebar_state" => {
                let parsed: Option<Value> = if args.trim().is_empty() {
                    None
                } else {
                    serde_json::from_str::<Value>(args.trim()).ok()
                };
                self.inner.lock().sidebar_state = parsed;
                "OK".into()
            }
            "reset_sidebar" => {
                self.inner.lock().sidebar_state = None;
                "OK".into()
            }
            "read_screen" => {
                // Phase 3 has no terminal core. Returning an empty
                // screen keeps the command usable as a smoke test.
                "OK\n".into()
            }

            // Focus-intent commands are no-ops in the headless build.
            "window.focus" | "surface.focus" | "surface.report_tty" | "surface.ports_kick" => {
                "OK".into()
            }

            _ => format!("ERROR: Unknown command: {cmd}"),
        }
    }

    fn handle_v2(&self, _ctx: &RequestContext, request: JsonRequest) -> JsonResponse {
        let id = request.id.clone();
        match request.method.as_str() {
            "system.ping" => JsonResponse::success(id, json!({"pong": true})),
            "system.capabilities" => JsonResponse::success(
                id,
                json!({
                    "methods": self.capabilities(),
                    "version": VERSION,
                    "implementation": "cmux-rs (Phase 3 headless)",
                }),
            ),
            "system.identify" => JsonResponse::success(id, self.identify(&request.params)),
            "system.tree" => JsonResponse::success(id, json!({"windows": []})),
            "auth.login" => JsonResponse::success(
                id,
                json!({"authenticated": true, "required": false}),
            ),
            _ => JsonResponse::failure(
                id,
                "method_not_available",
                format!(
                    "method {:?} is not implemented in the Phase 3 headless build",
                    request.method
                ),
            ),
        }
    }

    fn authenticate(&self, _line: &str) -> AuthOutcome {
        // Phase 3 only runs in modes that don't require password auth;
        // the listener calls this when `SocketControlMode::Password`
        // is active. Until we port the password store, always refuse.
        AuthOutcome::Failed
    }
}

/// Subset of the Swift v2 capability list the headless build
/// currently answers affirmatively. Anything outside this list falls
/// through to `method_not_available`.
const BASELINE_V2_METHODS: &[&str] = &[
    "system.ping",
    "system.capabilities",
    "system.identify",
    "system.tree",
    "auth.login",
];

#[cfg(test)]
mod tests {
    use super::*;
    use cmux_core::socket::listener::dispatch_line;

    fn handler() -> HeadlessHandler {
        HeadlessHandler::new("/tmp/cmux-rs-test.sock")
    }

    #[test]
    fn v1_ping() {
        let h = handler();
        let ctx = RequestContext::default();
        assert_eq!(dispatch_line(&h, &ctx, "ping"), "PONG");
    }

    #[test]
    fn v1_report_meta_roundtrip() {
        let h = handler();
        let ctx = RequestContext::default();
        assert_eq!(dispatch_line(&h, &ctx, "report_meta status ready"), "OK");
        let out = dispatch_line(&h, &ctx, "list_meta");
        assert!(out.contains("status=ready"), "got: {out}");
    }

    #[test]
    fn v2_ping_returns_pong() {
        let h = handler();
        let ctx = RequestContext::default();
        let out = dispatch_line(&h, &ctx, r#"{"id":1,"method":"system.ping"}"#);
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["ok"], json!(true));
        assert_eq!(v["result"]["pong"], json!(true));
    }

    #[test]
    fn v2_capabilities_lists_baseline_methods() {
        let h = handler();
        let ctx = RequestContext::default();
        let out = dispatch_line(&h, &ctx, r#"{"id":2,"method":"system.capabilities"}"#);
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["ok"], json!(true));
        let methods = v["result"]["methods"].as_array().unwrap();
        assert!(methods.iter().any(|m| m == "system.ping"));
        assert!(methods.iter().any(|m| m == "system.identify"));
    }

    #[test]
    fn v2_identify_reports_socket_path() {
        let h = HeadlessHandler::new("/tmp/foo.sock");
        let ctx = RequestContext::default();
        let out = dispatch_line(&h, &ctx, r#"{"id":3,"method":"system.identify"}"#);
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["ok"], json!(true));
        assert_eq!(v["result"]["socket_path"], "/tmp/foo.sock");
    }

    #[test]
    fn v2_unknown_method_is_method_not_available() {
        let h = handler();
        let ctx = RequestContext::default();
        let out = dispatch_line(&h, &ctx, r#"{"id":4,"method":"workspace.list"}"#);
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["ok"], json!(false));
        assert_eq!(v["error"]["code"], "method_not_available");
    }

    #[test]
    fn v1_sidebar_state_accepts_json_blob() {
        let h = handler();
        let ctx = RequestContext::default();
        let out = dispatch_line(&h, &ctx, r#"sidebar_state {"pinned":["a","b"]}"#);
        assert_eq!(out, "OK");
    }
}
