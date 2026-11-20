//! The daemon: the wake socket for the LiteLLM hook and the CLI, and the MCP
//! server on loopback.
//!
//! Socket routes, all JSON:
//!   POST /ensure  a request is about to reach Ollama; wakes only if enabled
//!   POST /wake    a wake asked for by hand; runs even when disabled
//!   GET  /status  awake or not, waking, the last wake
//! /ensure and /wake answer 200 once Ollama answers, 503 when the wake fails.

use std::{os::unix::fs::PermissionsExt, path::Path, sync::Arc};

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use serde_json::json;
use tokio::net::{TcpListener, UnixListener};

use crate::{
    mcp::Tools,
    ollama::Ollama,
    wake::{Machine, Outcome, Waker},
};

type Shared = Arc<Waker<Ollama>>;

/// Refused below this: a token is the only thing guarding the MCP server.
const MIN_TOKEN_LEN: usize = 32;

pub struct Mcp {
    pub port: u16,
    pub token: String,
}

pub async fn run(waker: Shared, socket: &Path, mcp: Option<Mcp>) -> Result<(), String> {
    if mcp.as_ref().is_some_and(|m| m.token.len() < MIN_TOKEN_LEN) {
        return Err(format!(
            "WHOOPY_MCP_TOKEN must hold at least {MIN_TOKEN_LEN} characters"
        ));
    }

    // A socket left by a killed run would make bind fail.
    let _ = std::fs::remove_file(socket);
    let unix = UnixListener::bind(socket).map_err(|e| format!("{}: {e}", socket.display()))?;
    // The hook runs as the same user; root (the panel) passes anyway.
    std::fs::set_permissions(socket, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("{}: {e}", socket.display()))?;
    let local = Router::new()
        .route("/ensure", post(ensure))
        .route("/wake", post(wake))
        .route("/status", get(status))
        .with_state(Arc::clone(&waker));
    let local = axum::serve(unix, local).with_graceful_shutdown(shutdown());

    let Some(Mcp { port, token }) = mcp else {
        eprintln!("whoopy: wake socket at {}, no MCP server", socket.display());
        return local.await.map_err(|e| e.to_string());
    };

    let tools = Tools { waker };
    // Stateless and JSON: a restart leaves no session for a client to trip on.
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_sse_keep_alive(None);
    let service = StreamableHttpService::new(
        move || Ok(tools.clone()),
        Arc::new(LocalSessionManager::default()),
        config,
    );
    let token: Arc<str> = token.into();
    let app = Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(token, require_token));
    let tcp = TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|e| format!("port {port}: {e}"))?;
    eprintln!(
        "whoopy: wake socket at {}, MCP at http://127.0.0.1:{port}/mcp",
        socket.display()
    );
    let remote = axum::serve(tcp, app).with_graceful_shutdown(shutdown());

    let (a, b) = tokio::join!(local, remote);
    a.and(b).map_err(|e| e.to_string())
}

/// Waits for SIGTERM or Ctrl-C. In-flight wakes are not waited for long:
/// whoopy-wake.service stops the process 5 s later (TimeoutStopSec).
async fn shutdown() {
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("SIGTERM handler");
    tokio::select! {
        _ = term.recv() => {}
        _ = tokio::signal::ctrl_c() => {}
    }
}

/// Every MCP request, whatever its path, carries the token. 401 says no more.
async fn require_token(State(token): State<Arc<str>>, req: Request, next: Next) -> Response {
    let given = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if !same(given.as_bytes(), token.as_bytes()) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    next.run(req).await
}

/// Constant time for equal lengths; the length itself is not secret.
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn answer(out: Outcome) -> Response {
    if !out.ok {
        eprintln!("whoopy: wake failed: {}", out.reason);
    }
    let code = if out.ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (code, Json(out)).into_response()
}

async fn ensure(State(w): State<Shared>) -> Response {
    answer(w.ensure_awake(false).await)
}

async fn wake(State(w): State<Shared>) -> Response {
    answer(w.ensure_awake(true).await)
}

async fn status(State(w): State<Shared>) -> Response {
    let up = w.machine().up().await;
    Json(json!({
        "enabled": w.enabled(),
        "up": up,
        "waking": w.waking(),
        "last": w.last(),
    }))
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::same;

    #[test]
    fn token_compare() {
        assert!(same(b"abc", b"abc"));
        assert!(!same(b"abc", b"abd"));
        assert!(!same(b"abc", b"abcd"));
        assert!(!same(b"", b"a"));
    }
}
