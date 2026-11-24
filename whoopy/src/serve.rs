//! The daemon: the wake socket for the LiteLLM hook and the CLI, the MCP
//! server on loopback, and the status page on loopback.
//!
//! Socket routes, all JSON:
//!   POST /ensure  a request is about to reach Ollama; wakes only if enabled
//!   POST /wake    a wake asked for by hand; runs even when disabled
//!   GET  /status  awake or not, waking, the last wake
//!   GET  /settings  the wake settings; the hook reads keep_alive there
//! /ensure and /wake answer 200 once Ollama answers, 503 when the wake fails.
//!
//! Page routes, behind the portal's main permission through nginx:
//!   GET  /                 the page (WHOOPY_WEB_PAGE)
//!   GET  /whoopy/status    as the socket's /status
//!   POST /whoopy/wake      as the socket's /wake; needs the header X-Whoopy: 1
//!   GET  /whoopy/settings  as the socket's /settings
//!   POST /whoopy/settings  checks, saves and applies them, the open MCP
//!                          servers excepted; needs X-Whoopy: 1
//!   POST /whoopy/mcp-public  opens or closes one MCP server to the internet,
//!                          `{"name", "public"}`; needs X-Whoopy: 1
//!   GET  /whoopy/mcp-public/check  nginx's auth subrequest: 204 when the
//!                          server in the header X-Whoopy-Mcp is open, else 403

use std::{
    future::IntoFuture,
    io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{
    Extension, Json, Router,
    extract::{Request, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
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
    settings::Settings,
    wake::{Machine, Outcome, Waker},
};

type Shared = Arc<Waker<Ollama>>;

/// Refused below this: a token is the only thing guarding the MCP server.
const MIN_TOKEN_LEN: usize = 32;

pub struct Mcp {
    pub port: u16,
    pub token: String,
}

pub struct Web {
    pub port: u16,
    pub page: PathBuf,
    /// Where a change made on the page is saved.
    pub settings: PathBuf,
}

pub async fn run(
    waker: Shared,
    socket: &Path,
    mcp: Option<Mcp>,
    web: Option<Web>,
) -> Result<(), String> {
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
        .route("/settings", get(settings))
        .with_state(Arc::clone(&waker));
    let local = axum::serve(unix, local).with_graceful_shutdown(shutdown());

    // "whoopy: wake socket at" is what whoopy_start waits for in the log.
    let mut started = format!("whoopy: wake socket at {}", socket.display());

    let remote = match mcp {
        None => None,
        Some(Mcp { port, token }) => {
            let tools = Tools {
                waker: Arc::clone(&waker),
            };
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
            started += &format!(", MCP at http://127.0.0.1:{port}/mcp");
            Some(serve_tcp(port, app).await?)
        }
    };

    let page = match web {
        None => None,
        Some(Web {
            port,
            page,
            settings: file,
        }) => {
            let page: Arc<Path> = page.into();
            let file: Arc<Path> = file.into();
            let app = Router::new()
                .route("/", get(move || send_page(Arc::clone(&page))))
                .route("/whoopy/status", get(status))
                .route(
                    "/whoopy/wake",
                    post(wake).layer(middleware::from_fn(require_page_header)),
                )
                .route(
                    "/whoopy/settings",
                    get(settings)
                        .merge(post(save_settings).layer(middleware::from_fn(require_page_header))),
                )
                .route(
                    "/whoopy/mcp-public",
                    post(set_public).layer(middleware::from_fn(require_page_header)),
                )
                .route("/whoopy/mcp-public/check", get(check_public))
                .layer(Extension(file))
                .with_state(Arc::clone(&waker));
            started += &format!(", page at http://127.0.0.1:{port}/");
            Some(serve_tcp(port, app).await?)
        }
    };

    eprintln!("{started}");
    let (a, b, c) = tokio::join!(local, run_some(remote), run_some(page));
    a.and(b).and(c).map_err(|e| e.to_string())
}

/// Binds a loopback port; the server runs once awaited.
async fn serve_tcp(
    port: u16,
    app: Router,
) -> Result<impl IntoFuture<Output = io::Result<()>>, String> {
    let tcp = TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|e| format!("port {port}: {e}"))?;
    Ok(axum::serve(tcp, app).with_graceful_shutdown(shutdown()))
}

async fn run_some(server: Option<impl IntoFuture<Output = io::Result<()>>>) -> io::Result<()> {
    match server {
        Some(s) => s.await,
        None => Ok(()),
    }
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

/// A browser sends a custom header only from a script on the same origin:
/// a form on another site cannot wake the desktop with the admin's session.
fn from_page(headers: &HeaderMap) -> bool {
    headers.get("x-whoopy").is_some_and(|v| v == "1")
}

async fn require_page_header(req: Request, next: Next) -> Response {
    if !from_page(req.headers()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    next.run(req).await
}

/// Read at each request, so a package upgrade changes it without a restart.
async fn send_page(path: Arc<Path>) -> Response {
    match tokio::fs::read(&*path).await {
        Ok(html) => (
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache"),
                (header::X_FRAME_OPTIONS, "DENY"),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            html,
        )
            .into_response(),
        Err(e) => {
            eprintln!("whoopy: {}: {e}", path.display());
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
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

async fn settings(State(w): State<Shared>) -> Json<Settings> {
    Json(w.settings())
}

/// Saved before applied: what runs is always what a restart would load.
/// A refusal is 400 with the reason as plain text, for the page to show.
async fn save_settings(
    State(w): State<Shared>,
    Extension(file): Extension<Arc<Path>>,
    body: Result<Json<Settings>, JsonRejection>,
) -> Response {
    let Json(new) = match body {
        Ok(body) => body,
        Err(e) => return (StatusCode::BAD_REQUEST, e.body_text()).into_response(),
    };
    // The Config tab sends no list: its Save must not close every server.
    let new = Settings {
        mcp_public: w.settings().mcp_public,
        ..new
    };
    apply(&w, &file, new)
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Public {
    name: String,
    public: bool,
}

async fn set_public(
    State(w): State<Shared>,
    Extension(file): Extension<Arc<Path>>,
    body: Result<Json<Public>, JsonRejection>,
) -> Response {
    let Json(Public { name, public }) = match body {
        Ok(body) => body,
        Err(e) => return (StatusCode::BAD_REQUEST, e.body_text()).into_response(),
    };
    let mut new = w.settings();
    new.toggle_public(&name, public);
    apply(&w, &file, new)
}

/// Fails closed: no header, or a server not listed, is a 403.
async fn check_public(State(w): State<Shared>, headers: HeaderMap) -> StatusCode {
    let name = headers.get("x-whoopy-mcp").and_then(|v| v.to_str().ok());
    if name.is_some_and(|n| w.settings().mcp_public.iter().any(|p| p == n)) {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::FORBIDDEN
    }
}

/// Checked, saved, then applied; answers the settings now in force.
fn apply(w: &Shared, file: &Path, new: Settings) -> Response {
    if let Err(why) = new.check() {
        return (StatusCode::BAD_REQUEST, why).into_response();
    }
    if let Err(e) = new.save(file) {
        eprintln!("whoopy: {}: {e}", file.display());
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "The settings could not be saved.",
        )
            .into_response();
    }
    w.set_settings(new.clone());
    Json(new).into_response()
}

async fn status(State(w): State<Shared>) -> Response {
    let up = w.machine().up().await;
    Json(json!({
        "enabled": w.settings().wol_enabled,
        "up": up,
        "waking": w.waking(),
        "last": w.last(),
    }))
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::{from_page, same};
    use axum::http::HeaderMap;

    #[test]
    fn page_header() {
        let mut h = HeaderMap::new();
        assert!(!from_page(&h));
        h.insert("x-whoopy", "0".parse().unwrap());
        assert!(!from_page(&h));
        h.insert("x-whoopy", "1".parse().unwrap());
        assert!(from_page(&h));
    }

    #[test]
    fn token_compare() {
        assert!(same(b"abc", b"abc"));
        assert!(!same(b"abc", b"abd"));
        assert!(!same(b"abc", b"abcd"));
        assert!(!same(b"", b"a"));
    }
}
