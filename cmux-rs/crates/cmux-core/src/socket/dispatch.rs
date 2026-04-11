//! Command dispatch — the bridge between the wire protocol and the
//! application state owned by `cmux-app`.
//!
//! The listener in [`super::listener`] parses raw lines off the wire
//! and invokes one of the two methods on [`CommandHandler`]:
//!
//! - [`CommandHandler::handle_v1`] for the legacy space-delimited
//!   protocol (`report_meta key value ...`).
//! - [`CommandHandler::handle_v2`] for the JSON-RPC-style protocol
//!   (`{"id":1,"method":"system.ping","params":{}}`).
//!
//! The handler owns all state. The listener deliberately has no
//! knowledge of workspaces, tabs, or panes — so the exact same socket
//! plumbing can back the future real implementation and the Phase 3
//! stub that only supports metadata mutation and the handshake.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The wire payload for a single v2 request line. Matches the Swift
/// `ControlSocketRequestV2` shape so the existing Python client in
/// `tests_v2/cmux.py` interoperates unchanged.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRequest {
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// The wire payload for a single v2 response line.
#[derive(Debug, Clone, Serialize)]
pub struct JsonResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonResponseError>,
}

impl JsonResponse {
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            id,
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    pub fn failure(
        id: Option<serde_json::Value>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id,
            ok: false,
            result: None,
            error: Some(JsonResponseError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonResponseError {
    pub code: String,
    pub message: String,
}

/// A handler error surfaced back to the listener. The listener will
/// log this and drop the connection.
#[derive(Debug)]
pub enum ProtocolError {
    Io(std::io::Error),
    InvalidUtf8,
    InvalidJson(serde_json::Error),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::InvalidUtf8 => write!(f, "invalid utf-8 on control socket"),
            Self::InvalidJson(e) => write!(f, "invalid v2 json request: {e}"),
        }
    }
}

impl std::error::Error for ProtocolError {}

impl From<std::io::Error> for ProtocolError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Context passed with every request so handlers can implement
/// mode-specific auth checks and peer ancestry verification.
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    pub authenticated: bool,
    pub peer_pid: Option<u32>,
}

/// The application-supplied command handler. `cmux-app` implements
/// this and passes it to [`super::listen`].
pub trait CommandHandler: Send + Sync + 'static {
    /// Handle a v1 command. `cmd` is the lowercased verb;
    /// `args` is the raw remainder of the line (including embedded
    /// whitespace, matching Swift's `split(separator:" ", maxSplits:1)`).
    /// Returns the full response body to write back to the client.
    fn handle_v1(&self, ctx: &RequestContext, cmd: &str, args: &str) -> String;

    /// Handle a v2 JSON request. Return a [`JsonResponse`]; the
    /// listener will serialise it to a single line.
    fn handle_v2(&self, ctx: &RequestContext, request: JsonRequest) -> JsonResponse;

    /// Whether the handler considers a given line a successful AUTH.
    /// The default implementation returns `true` — handlers in
    /// `Password` mode should override this.
    fn authenticate(&self, _line: &str) -> AuthOutcome {
        AuthOutcome::AlreadyAuthenticated
    }
}

/// Result of an authentication attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthOutcome {
    /// No auth required in this mode.
    AlreadyAuthenticated,
    /// The caller provided valid credentials.
    Ok,
    /// Not an auth line — let the normal command path process it
    /// (subject to `authenticated == true`).
    NotAnAuthLine,
    /// Auth failed. The listener will write an `ERROR: ...` line and
    /// close the connection.
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_response_success_shape() {
        let r = JsonResponse::success(Some(serde_json::json!(1)), serde_json::json!({"a":1}));
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains(r#""ok":true"#));
        assert!(s.contains(r#""id":1"#));
        assert!(s.contains(r#""a":1"#));
        assert!(!s.contains("error"));
    }

    #[test]
    fn json_response_failure_shape() {
        let r = JsonResponse::failure(Some(serde_json::json!(2)), "method_not_found", "nope");
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains(r#""ok":false"#));
        assert!(s.contains(r#""code":"method_not_found""#));
        assert!(s.contains(r#""message":"nope""#));
    }

    #[test]
    fn json_request_allows_missing_params() {
        let raw = r#"{"id":1,"method":"system.ping"}"#;
        let req: JsonRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.method, "system.ping");
        assert!(req.params.is_null());
    }
}
