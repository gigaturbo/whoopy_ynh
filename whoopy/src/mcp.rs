//! The MCP tools. None takes an argument and none changes a setting: a
//! steered model can wake the desktop, which a chat request does anyway, and
//! read model names and the wake settings. Changing them is the page's and
//! the CLI's.

use std::sync::Arc;

use rmcp::{
    ErrorData, ServerHandler,
    model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router,
};

use crate::{ollama::Ollama, settings::Settings, wake::Waker};

#[derive(Clone)]
pub struct Tools {
    pub waker: Arc<Waker<Ollama>>,
}

fn text(s: String) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![ContentBlock::text(s)]))
}

#[tool_router]
impl Tools {
    #[tool(
        description = "Wake the AI server (the machine that runs the local models) \
        and wait until it answers, up to the configured timeout. Chat requests to a local \
        model already wake it; use this to wake it ahead of time."
    )]
    async fn wake(&self) -> Result<CallToolResult, ErrorData> {
        let out = self.waker.ensure_awake(true).await;
        if out.ok {
            return text(format!(
                "The AI server is awake (waited {} s).",
                out.seconds
            ));
        }
        Ok(CallToolResult::error(vec![ContentBlock::text(format!(
            "The AI server did not wake: {}.",
            out.reason
        ))]))
    }

    #[tool(
        description = "Tell whether the AI server is awake, which models it has in \
        memory, and how the last wake went. Never wakes it."
    )]
    async fn status(&self) -> Result<CallToolResult, ErrorData> {
        let ollama = self.waker.machine();
        let mut lines = vec![match ollama.loaded().await {
            Some(models) if models.is_empty() => "Awake, no model in memory.".to_string(),
            Some(models) => format!("Awake. In memory: {}.", models.join(", ")),
            None if self.waker.waking() => "Waking up.".to_string(),
            None => "Asleep or unreachable.".to_string(),
        }];
        if let Some(last) = self.waker.last() {
            lines.push(match last.outcome.ok {
                true => format!("Last wake: answered after {} s.", last.outcome.seconds),
                false => format!("Last wake failed: {}.", last.outcome.reason),
            });
        }
        text(lines.join("\n"))
    }

    #[tool(
        description = "List the models installed on the AI server. Never wakes it: \
        while it sleeps, the list is unavailable."
    )]
    async fn models(&self) -> Result<CallToolResult, ErrorData> {
        match self.waker.machine().installed().await {
            Some(models) => text(models.join("\n")),
            None => text("The AI server is asleep or unreachable; use wake first.".into()),
        }
    }

    #[tool(
        description = "Show the wake settings: whether requests wake the AI server, \
        its MAC and broadcast address, the wake timeout, how long a model stays loaded, \
        and the MCP servers open from the internet. Read-only."
    )]
    async fn settings(&self) -> Result<CallToolResult, ErrorData> {
        text(describe(&self.waker.settings()))
    }
}

/// The settings in words, one per line; shared with the CLI.
pub fn describe(s: &Settings) -> String {
    let or_none = |v: &str, none: &str| match v {
        "" => none.to_string(),
        v => v.to_string(),
    };
    [
        format!(
            "Requests wake the AI server: {}",
            if s.wol_enabled { "yes" } else { "no" }
        ),
        format!("MAC address: {}", or_none(&s.wol_mac, "none")),
        format!("Broadcast address: {}", s.wol_broadcast),
        format!("Wake timeout: {} s", s.wake_timeout_s),
        format!(
            "Model kept loaded: {}",
            or_none(&s.keep_alive, "Ollama's own setting")
        ),
        format!(
            "Open from the internet: {}",
            or_none(&s.mcp_public.join(", "), "none")
        ),
    ]
    .join("\n")
}

#[tool_handler]
impl ServerHandler for Tools {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("whoopy", env!("CARGO_PKG_VERSION")))
    }
}
