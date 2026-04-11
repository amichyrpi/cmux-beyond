//! Cross-platform line-oriented listener for the cmux control socket.
//!
//! On Unix the transport is a Unix domain socket (via `interprocess`).
//! On Windows it becomes a named pipe. The wire protocol is the same
//! newline-delimited mix of v1 (`report_meta key value`) and v2
//! (`{"id":1,"method":"..."}`) frames that the Swift build speaks.
//!
//! Responsibilities:
//!
//! - accept a new client,
//! - read lines until EOF,
//! - split v1 vs v2 on the first non-whitespace character,
//! - call into the supplied [`CommandHandler`],
//! - write the response back (newline-terminated).
//!
//! Auth-mode enforcement (`SocketControlMode`) is the handler's
//! responsibility: the listener asks the handler whether each line is
//! an AUTH handshake and then delegates the rest. Peer-pid ancestry
//! checks for `cmuxOnly` mode live in the handler as well, since they
//! depend on process-tree state the core crate doesn't track.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{GenericFilePath, ListenerOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, info, warn};

use super::dispatch::{
    AuthOutcome, CommandHandler, JsonRequest, JsonResponse, ProtocolError, RequestContext,
};
use super::settings::SocketControlMode;

/// Configuration for a single listener instance.
#[derive(Debug, Clone)]
pub struct ListenerConfig {
    /// Path of the Unix socket / named pipe to bind to.
    pub path: PathBuf,
    /// The runtime-effective access mode — used by the handler to
    /// decide whether to challenge for a password or verify peer
    /// ancestry.
    pub mode: SocketControlMode,
}

/// Bind to `config.path` and run the accept loop until cancelled.
///
/// On Unix this removes any stale socket at the path before binding,
/// matching the Swift listener startup behaviour.
pub async fn listen<H: CommandHandler>(
    config: ListenerConfig,
    handler: Arc<H>,
) -> std::io::Result<()> {
    ensure_parent_directory(&config.path)?;
    remove_stale_socket(&config.path);

    let fs_name = config
        .path
        .as_path()
        .to_fs_name::<GenericFilePath>()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let listener = ListenerOptions::new().name(fs_name).create_tokio()?;

    info!(path = %config.path.display(), mode = ?config.mode, "cmux control socket listening");

    #[cfg(unix)]
    apply_unix_permissions(&config.path, config.mode);

    loop {
        match listener.accept().await {
            Ok(conn) => {
                let handler = Arc::clone(&handler);
                let mode = config.mode;
                tokio::spawn(async move {
                    if let Err(err) = handle_client(conn, handler, mode).await {
                        warn!(?err, "client disconnected");
                    }
                });
            }
            Err(err) => {
                warn!(?err, "accept error");
            }
        }
    }
}

async fn handle_client<H: CommandHandler>(
    conn: interprocess::local_socket::tokio::Stream,
    handler: Arc<H>,
    mode: SocketControlMode,
) -> Result<(), ProtocolError> {
    let (read_half, mut write_half) = conn.split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    let mut ctx = RequestContext {
        authenticated: !mode.requires_password_auth(),
        peer_pid: None,
    };

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).await?;
        if bytes == 0 {
            break;
        }
        let trimmed = line.trim_end_matches(['\n', '\r']).trim();
        if trimmed.is_empty() {
            continue;
        }

        if !ctx.authenticated {
            match handler.authenticate(trimmed) {
                AuthOutcome::Ok | AuthOutcome::AlreadyAuthenticated => {
                    ctx.authenticated = true;
                    write_half.write_all(b"OK: AUTH\n").await?;
                    continue;
                }
                AuthOutcome::Failed => {
                    write_half
                        .write_all(b"ERROR: Authentication failed\n")
                        .await?;
                    break;
                }
                AuthOutcome::NotAnAuthLine => {
                    write_half
                        .write_all(b"ERROR: Authentication required\n")
                        .await?;
                    continue;
                }
            }
        }

        let response = dispatch_line(&*handler, &ctx, trimmed);
        write_half.write_all(response.as_bytes()).await?;
        if !response.ends_with('\n') {
            write_half.write_all(b"\n").await?;
        }
    }

    Ok(())
}

/// Pure helper so the listener can be unit tested without any socket
/// I/O. Returns the full response body (without trailing newline —
/// callers append their own).
pub fn dispatch_line(handler: &dyn CommandHandler, ctx: &RequestContext, line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return "ERROR: Empty command".into();
    }

    if trimmed.starts_with('{') {
        return match serde_json::from_str::<JsonRequest>(trimmed) {
            Ok(request) => {
                let id = request.id.clone();
                let response = handler.handle_v2(ctx, request);
                serde_json::to_string(&response).unwrap_or_else(|e| {
                    let fallback = JsonResponse::failure(id, "serialize_error", e.to_string());
                    serde_json::to_string(&fallback).expect("serialize fallback")
                })
            }
            Err(e) => {
                debug!(?e, "invalid v2 json");
                let fallback = JsonResponse::failure(None, "invalid_request", e.to_string());
                serde_json::to_string(&fallback).expect("serialize fallback")
            }
        };
    }

    // v1 text protocol.
    let (cmd, args) = match trimmed.split_once(' ') {
        Some((cmd, args)) => (cmd, args),
        None => (trimmed, ""),
    };
    handler.handle_v1(ctx, &cmd.to_ascii_lowercase(), args)
}

fn ensure_parent_directory(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

fn remove_stale_socket(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(unix)]
fn apply_unix_permissions(path: &Path, mode: SocketControlMode) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = std::fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(mode.socket_file_permissions());
        let _ = std::fs::set_permissions(path, perms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::socket::dispatch::{CommandHandler, JsonRequest, JsonResponse, RequestContext};

    struct Echo;

    impl CommandHandler for Echo {
        fn handle_v1(&self, _ctx: &RequestContext, cmd: &str, args: &str) -> String {
            format!("OK: {cmd} [{args}]")
        }
        fn handle_v2(&self, _ctx: &RequestContext, request: JsonRequest) -> JsonResponse {
            JsonResponse::success(request.id, serde_json::json!({"method": request.method}))
        }
    }

    #[test]
    fn dispatch_line_routes_v1_command() {
        let ctx = RequestContext::default();
        let out = dispatch_line(&Echo, &ctx, "report_meta foo bar baz");
        assert_eq!(out, "OK: report_meta [foo bar baz]");
    }

    #[test]
    fn dispatch_line_routes_v2_json() {
        let ctx = RequestContext::default();
        let out = dispatch_line(&Echo, &ctx, r#"{"id":7,"method":"system.ping"}"#);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["ok"], serde_json::json!(true));
        assert_eq!(parsed["id"], serde_json::json!(7));
        assert_eq!(parsed["result"]["method"], "system.ping");
    }

    #[test]
    fn dispatch_line_reports_invalid_json() {
        let ctx = RequestContext::default();
        let out = dispatch_line(&Echo, &ctx, "{not json");
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["ok"], serde_json::json!(false));
        assert_eq!(parsed["error"]["code"], "invalid_request");
    }

    #[test]
    fn dispatch_line_handles_empty_line() {
        let ctx = RequestContext::default();
        assert_eq!(dispatch_line(&Echo, &ctx, "   "), "ERROR: Empty command");
    }

    #[test]
    fn dispatch_line_lowercases_v1_verb() {
        let ctx = RequestContext::default();
        let out = dispatch_line(&Echo, &ctx, "REPORT_META foo bar");
        assert_eq!(out, "OK: report_meta [foo bar]");
    }
}
