//! whoopy: wakes the AI server for the Whoopy gateway.
//!
//! `whoopy serve` is the daemon (whoopy-wake.service). `wake`, `status` and
//! `settings` ask it over its socket, so every wake, from the panel or a
//! request, is the same shared wake. Its addresses come from the WHOOPY_*
//! variables in wake.env; the wake settings from the file the status page and
//! `whoopy settings` write. `mcp` talks to LiteLLM instead (litellm.rs).

mod litellm;
mod mcp;
mod ollama;
mod serve;
mod settings;
mod wake;

use std::{
    net::Ipv4Addr,
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::{Parser, Subcommand};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper_util::rt::TokioIo;
use serde_json::Value;

#[derive(Parser)]
#[command(version, about = "Wakes the AI server for the Whoopy gateway")]
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
    /// Wake the AI server and wait until it answers.
    Wake,
    /// Tell whether the AI server is awake, and how the last wake went.
    Status,
    /// Show the wake settings, or change the ones given; applied at once.
    Settings(SettingsArgs),
    /// List, add or update LiteLLM's MCP servers. Run as root.
    Mcp(McpArgs),
}

#[derive(clap::Args)]
struct SettingsArgs {
    /// Requests wake the AI server.
    #[arg(long, conflicts_with = "off")]
    on: bool,
    /// Requests no longer wake it; a manual wake still does.
    #[arg(long)]
    off: bool,
    /// MAC address of its network card, aa:bb:cc:dd:ee:ff.
    #[arg(long)]
    mac: Option<String>,
    /// Where the wake packet goes.
    #[arg(long)]
    broadcast: Option<Ipv4Addr>,
    /// Seconds to wait for the AI server after a wake, 10 to 540.
    #[arg(long)]
    timeout: Option<u64>,
    /// How long a model stays loaded after a request, e.g. 5m; "" for Ollama's own.
    #[arg(long)]
    keep_alive: Option<String>,
}

#[derive(clap::Args)]
struct McpArgs {
    /// Whoopy's env file, holding LiteLLM's master key and address.
    #[arg(
        long,
        env = "WHOOPY_ENV_FILE",
        default_value = "/var/www/whoopy/whoopy.env"
    )]
    env_file: PathBuf,
    #[command(subcommand)]
    command: McpCommand,
}

#[derive(Subcommand)]
enum McpCommand {
    /// List the servers, where each answers and which keys may use it.
    List,
    /// Add a server over HTTP, served at /mcp/<name>.
    Add {
        /// Lower-case letters, digits and _, up to 32.
        name: String,
        /// Its address, e.g. http://127.0.0.1:8770/mcp.
        url: String,
        /// Only the keys granted it may use it; every key otherwise.
        #[arg(long)]
        granted: bool,
        /// Read its Bearer token from stdin.
        #[arg(long)]
        token_stdin: bool,
    },
    /// Change a server's address, access or token; its grants stay.
    Update {
        /// The name it answers at, /mcp/<name>.
        name: String,
        #[arg(long)]
        url: Option<String>,
        /// Every key may use it.
        #[arg(long, conflicts_with = "granted")]
        every_key: bool,
        /// Only the keys granted it may use it.
        #[arg(long)]
        granted: bool,
        /// Read a new Bearer token from stdin.
        #[arg(long)]
        token_stdin: bool,
    },
}

#[derive(clap::Args)]
struct ServeArgs {
    /// Ollama, scheme://host:port with no path.
    #[arg(long, env = "WHOOPY_OLLAMA_URL")]
    ollama_url: String,
    /// The wake settings' JSON file; the defaults while it does not exist.
    #[arg(long, env = "WHOOPY_SETTINGS")]
    settings: PathBuf,
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
        Command::Settings(args) => settings(&cli.socket, args).await,
        Command::Mcp(args) => mcp(args).await,
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
    // A file the page would refuse stops the start: the wake must not
    // quietly fall back to off.
    let settings = settings::Settings::load(&args.settings)?;
    let waker = wake::Waker::new(ollama::Ollama::new(&args.ollama_url), settings);
    let web = args
        .web_port
        .zip(args.web_page)
        .map(|(port, page)| serve::Web { port, page });
    serve::run(waker, socket, args.settings, mcp, web).await
}

async fn wake(socket: &Path) -> Result<(), String> {
    let (code, body) = ask(socket, "POST", "/wake", None).await?;
    if code == 200 {
        println!(
            "The AI server answers, {} s after the wake request.",
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
    let (code, body) = ask(socket, "GET", "/status", None).await?;
    if code != 200 {
        return Err(format!("the daemon answered {code}"));
    }
    let state = match (body["up"].as_bool(), body["waking"].as_bool()) {
        (Some(true), _) => "The AI server is awake.",
        (_, Some(true)) => "The AI server is waking up.",
        _ => "The AI server is asleep or unreachable.",
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

/// No flag shows the settings; flags change those fields only, checked by
/// the daemon as the page's are. The open MCP servers are never touched.
async fn settings(socket: &Path, args: SettingsArgs) -> Result<(), String> {
    let (_, body) = ask(socket, "GET", "/settings", None).await?;
    let mut s: settings::Settings =
        serde_json::from_value(body).map_err(|e| format!("unreadable settings: {e}"))?;
    let before = s.clone();
    if args.on || args.off {
        s.wol_enabled = args.on;
    }
    s.wol_mac = args.mac.unwrap_or(s.wol_mac);
    s.wol_broadcast = args.broadcast.unwrap_or(s.wol_broadcast);
    s.wake_timeout_s = args.timeout.unwrap_or(s.wake_timeout_s);
    s.keep_alive = args.keep_alive.unwrap_or(s.keep_alive);
    if s != before {
        let body = serde_json::to_value(&s).map_err(|e| e.to_string())?;
        let (code, body) = ask(socket, "POST", "/settings", Some(body)).await?;
        if code != 200 {
            return Err(format!(
                "the daemon refused the settings ({code}): {}",
                body.as_str()
                    .filter(|s| !s.is_empty())
                    .unwrap_or("no reason given")
            ));
        }
        s = serde_json::from_value(body).map_err(|e| format!("unreadable settings: {e}"))?;
    }
    println!("{}", mcp::describe(&s));
    Ok(())
}

async fn mcp(args: McpArgs) -> Result<(), String> {
    let admin = litellm::Admin::from_env_file(&args.env_file)?;
    match args.command {
        McpCommand::List => admin.list().await,
        McpCommand::Add {
            name,
            url,
            granted,
            token_stdin,
        } => admin.add(&name, &url, !granted, token_stdin).await,
        McpCommand::Update {
            name,
            url,
            every_key,
            granted,
            token_stdin,
        } => {
            let change = litellm::Change {
                url,
                every_key: (every_key || granted).then_some(every_key),
                token: token_stdin,
            };
            admin.update(&name, change).await
        }
    }
}

/// One HTTP/1.1 request over the daemon's socket, with an optional JSON body.
/// A JSON reply is parsed; a plain-text one (a refusal's reason) comes back
/// as a JSON string.
async fn ask(
    socket: &Path,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> Result<(u16, Value), String> {
    let unreachable =
        |e: &dyn std::fmt::Display| format!("the daemon at {}: {e}", socket.display());
    let stream = tokio::net::UnixStream::connect(socket)
        .await
        .map_err(|e| unreachable(&e))?;
    let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
        .await
        .map_err(|e| unreachable(&e))?;
    tokio::spawn(conn);
    let bytes = match &body {
        Some(b) => Bytes::from(b.to_string()),
        None => Bytes::new(),
    };
    let req = hyper::Request::builder()
        .method(method)
        .uri(path)
        .header("host", "whoopy")
        .header("content-type", "application/json")
        .body(Full::new(bytes))
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
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    Ok((code, body))
}
