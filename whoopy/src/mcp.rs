//! The MCP tools. None takes an argument: a steered model can wake the
//! desktop, which a chat request does anyway, and read model names.

use std::sync::Arc;

use rmcp::{
    ErrorData, ServerHandler,
    model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router,
};

use crate::{ollama::Ollama, wake::Waker};

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
        description = "Wake the Ollama machine (the desktop that runs the local models) \
        and wait until it answers, up to the configured timeout. Chat requests to a local \
        model already wake it; use this to wake it ahead of time."
    )]
    async fn wake(&self) -> Result<CallToolResult, ErrorData> {
        let out = self.waker.ensure_awake(true).await;
        if out.ok {
            return text(format!(
                "The Ollama machine is awake (waited {} s).",
                out.seconds
            ));
        }
        Ok(CallToolResult::error(vec![ContentBlock::text(format!(
            "The Ollama machine did not wake: {}.",
            out.reason
        ))]))
    }

    #[tool(
        description = "Tell whether the Ollama machine is awake, which models it has in \
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
        description = "List the models installed on the Ollama machine. Never wakes it: \
        while it sleeps, the list is unavailable."
    )]
    async fn models(&self) -> Result<CallToolResult, ErrorData> {
        match self.waker.machine().installed().await {
            Some(models) => text(models.join("\n")),
            None => text("The Ollama machine is asleep or unreachable; use wake first.".into()),
        }
    }
}

#[tool_handler]
impl ServerHandler for Tools {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("whoopy", env!("CARGO_PKG_VERSION")))
    }
}
