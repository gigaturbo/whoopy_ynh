//! whoopy: wakes the Ollama machine for the Whoopy gateway.
//!
//! `whoopy serve` is the daemon (whoopy-wake.service). `wake` and `status`
//! ask it over its socket, so every wake, from the panel or a request, is the
//! same shared wake. Settings come from the WHOOPY_* variables in wake.env.

mod mcp;
mod ollama;
mod serve;
mod wake;

use std::{
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    process::ExitCode,
    time::Duration,
};

use clap::{Parser, Subcommand};
use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper_util::rt::TokioIo;
use serde_json::Value;

#[derive(Parser)]
#[command(version, about = "Wakes the Ollama machine for the Whoopy gateway")]
struct Cli {
    /// The daemon's socket.
    #[arg(
        long,
        env = "WHOOPY_SOCKET",
        default_value = "/run/whoopy/wake.sock",
        global = true
    )]
    socket: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the daemon.
    Serve(ServeArgs),
    /// Wake the Ollama machine and wait until it answers.
    Wake,
    /// Tell whether the Ollama machine is awake, and how the last wake went.
    Status,
}

#[derive(clap::Args)]
struct ServeArgs {
    /// Ollama, scheme://host:port with no path.
    #[arg(long, env = "WHOOPY_OLLAMA_URL")]
    ollama_url: String,
    /// "1" wakes the machine for requests; a manual wake works either way.
    #[arg(long, env = "WHOOPY_WOL_ENABLED", default_value = "0")]
    wol_enabled: String,
    #[arg(long, env = "WHOOPY_WOL_MAC", default_value = "")]
    wol_mac: String,
    /// Where the wake packet goes, on UDP port 9.
    #[arg(long, env = "WHOOPY_WOL_BROADCAST", default_value = "255.255.255.255")]
    wol_broadcast: IpAddr,
    /// How long Ollama has to answer before a wake fails.
    #[arg(long, env = "WHOOPY_WAKE_TIMEOUT_S", default_value_t = 90)]
    wake_timeout_s: u64,
    /// The MCP server's loopback port; none without it. Its token is read
    /// from WHOOPY_MCP_TOKEN only, never from the command line.
    #[arg(long, env = "WHOOPY_MCP_PORT")]
    mcp_port: Option<u16>,
    /// The status page's loopback port; none without it.
    #[arg(long, env = "WHOOPY_WEB_PORT", requires = "web_page")]
    web_port: Option<u16>,
    /// The status page's HTML, read at each request.
    #[arg(long, env = "WHOOPY_WEB_PAGE")]
    web_page: Option<PathBuf>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Serve(args) => serve(args, &cli.socket).await,
        Command::Wake => wake(&cli.socket).await,
        Command::Status => status(&cli.socket).await,
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("whoopy: {err}");
            ExitCode::FAILURE
        }
    }
}

async fn serve(args: ServeArgs, socket: &Path) -> Result<(), String> {
    let mcp = match args.mcp_port {
        None => None,
        Some(port) => {
            let token = std::env::var("WHOOPY_MCP_TOKEN")
                .map_err(|_| "WHOOPY_MCP_PORT is set but WHOOPY_MCP_TOKEN is not".to_string())?;
            Some(serve::Mcp { port, token })
        }
    };
    let ollama = ollama::Ollama::new(
        &args.ollama_url,
        &args.wol_mac,
        SocketAddr::new(args.wol_broadcast, 9),
    );
    let waker = wake::Waker::new(
        ollama,
        args.wol_enabled == "1",
        Duration::from_secs(args.wake_timeout_s),
    );
    let web = args
        .web_port
        .zip(args.web_page)
        .map(|(port, page)| serve::Web { port, page });
    serve::run(waker, socket, mcp, web).await
}

async fn wake(socket: &Path) -> Result<(), String> {
    let (code, body) = ask(socket, "POST", "/wake").await?;
    if code == 200 {
        println!(
            "Ollama answers, {} s after the wake request.",
            body["seconds"]
        );
        return Ok(());
    }
    Err(format!(
        "the wake failed: {}",
        body["reason"].as_str().unwrap_or("no reason given")
    ))
}

async fn status(socket: &Path) -> Result<(), String> {
    let (_, body) = ask(socket, "GET", "/status").await?;
    let state = match (body["up"].as_bool(), body["waking"].as_bool()) {
        (Some(true), _) => "Ollama is awake.",
        (_, Some(true)) => "Ollama is waking up.",
        _ => "Ollama is asleep or unreachable.",
    };
    println!("{state}");
    if body["enabled"] != Value::Bool(true) {
        println!("Requests do not wake it: Wake-on-LAN is off.");
    }
    let last = &body["last"];
    match last["ok"].as_bool() {
        Some(true) => println!("Last wake: answered after {} s.", last["seconds"]),
        Some(false) => println!(
            "Last wake failed: {}.",
            last["reason"].as_str().unwrap_or("")
        ),
        None => println!("No wake since the service started."),
    }
    Ok(())
}

/// One HTTP/1.1 request over the daemon's socket; the reply parsed as JSON.
async fn ask(socket: &Path, method: &str, path: &str) -> Result<(u16, Value), String> {
    let unreachable =
        |e: &dyn std::fmt::Display| format!("the daemon at {}: {e}", socket.display());
    let stream = tokio::net::UnixStream::connect(socket)
        .await
        .map_err(|e| unreachable(&e))?;
    let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
        .await
        .map_err(|e| unreachable(&e))?;
    tokio::spawn(conn);
    let req = hyper::Request::builder()
        .method(method)
        .uri(path)
        .header("host", "whoopy")
        .body(Empty::<Bytes>::new())
        .map_err(|e| e.to_string())?;
    let resp = sender
        .send_request(req)
        .await
        .map_err(|e| unreachable(&e))?;
    let code = resp.status().as_u16();
    let bytes = resp
        .into_body()
        .collect()
        .await
        .map_err(|e| unreachable(&e))?
        .to_bytes();
    let body = serde_json::from_slice(&bytes).map_err(|e| format!("unreadable reply: {e}"))?;
    Ok((code, body))
}
