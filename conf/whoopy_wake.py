"""Have the AI server woken before LiteLLM sends it a request.

Loaded by LiteLLM from `litellm_settings.callbacks` ("whoopy_wake.handler"),
from the directory of config.yaml. The waking itself is done by the wake
daemon (whoopy-wake.service): this hook asks it over its socket, and the
daemon answers once Ollama does, at once when waking is off, or with 503 when
the wake fails. The daemon holds the shared wake, the wake settings (changed
on the status page) and the timers.

From the service environment:

    WHOOPY_OLLAMA_URL       scheme://host:port of Ollama, no path
    WHOOPY_SOCKET           the daemon's socket (default /run/whoopy/wake.sock)

Two entry points, because LiteLLM has two request paths:
- model calls go through the router, which hands `async_pre_call_check` the
  deployment it picked. Only a deployment whose api_base is Ollama wakes.
- a tool call reaches `async_pre_call_hook` as call_type "call_mcp_tool",
  and wakes only for a server in WAKE_MCP_SERVERS.

A tool listing reaches no hook (LiteLLM 1.106.6), so it never wakes: while
the machine sleeps, its servers are missing from the list. And LiteLLM refuses
a tool name it has never listed before any hook runs, so after a restart
while the machine sleeps, a call fails until a listing succeeds.

It sets the page's keep_alive on requests to the ollama/ and ollama-embed/
routes that carry none, read from the daemon at most every KEEP_ALIVE_TTL.

It also empties LiteLLM's static Ollama model list: while the machine sleeps,
/api/tags fails and /v1/models would list that instead ("ollama/llama2").
"""

import logging
import os
import time

import httpx
import litellm
from fastapi import HTTPException
from litellm.exceptions import ServiceUnavailableError
from litellm.integrations.custom_logger import CustomLogger

OLLAMA_URL = os.environ.get("WHOOPY_OLLAMA_URL", "").rstrip("/")
SOCKET = os.environ.get("WHOOPY_SOCKET", "/run/whoopy/wake.sock")
# Past the daemon's longest timeout (MAX_TIMEOUT_S in its settings.rs), so its
# own 503 and reason arrive first.
WAIT_S = 540 + 10
# The model routes config.yaml points at Ollama.
OLLAMA_ROUTES = ("ollama/", "ollama-embed/")
KEEP_ALIVE_TTL = 10.0

# The `mcp_servers` names in config.yaml that run on the AI server; none
# today. A server on this host, whoopy or any other, must never be listed.
WAKE_MCP_SERVERS: set[str] = set()

log = logging.getLogger("whoopy_wake")

# The fallback LiteLLM lists when Ollama's /api/tags fails (LiteLLM 1.106.6,
# OllamaModelInfo.get_models): a model this machine may not have.
litellm.models_by_provider["ollama"] = []


class WakeOnLan(CustomLogger):
    def __init__(self):
        super().__init__()
        self._client = httpx.AsyncClient(
            transport=httpx.AsyncHTTPTransport(uds=SOCKET),
            base_url="http://whoopy",
            timeout=WAIT_S,
        )
        self._keep_alive = ""
        self._keep_alive_until = 0.0

    async def _keep_alive_setting(self):
        """The page's keep_alive, "" when unset or the daemon does not answer."""
        if time.monotonic() < self._keep_alive_until:
            return self._keep_alive
        try:
            resp = await self._client.get("/settings", timeout=2)
            self._keep_alive = resp.json().get("keep_alive") or ""
        except (httpx.HTTPError, ValueError, AttributeError):
            self._keep_alive = ""
        self._keep_alive_until = time.monotonic() + KEEP_ALIVE_TTL
        return self._keep_alive

    async def _not_awake(self):
        """None once Ollama answers, or when nothing needs waking; else why not."""
        try:
            resp = await self._client.post("/ensure")
        except httpx.TimeoutException:
            return f"The AI server did not answer within {int(WAIT_S)} s."
        except httpx.TransportError as err:
            # Daemon down or restarting: let the request through. If Ollama
            # is awake it answers; if not, the client gets the connection error.
            log.error("whoopy-wake did not answer (%s); request sent unwoken", type(err).__name__)
            return None
        if resp.status_code == 200:
            return None
        try:
            reason = resp.json().get("reason") or "no reason given"
        except ValueError:
            reason = f"HTTP {resp.status_code}"
        return f"The AI server did not wake: {reason}."

    async def async_pre_call_check(self, deployment, parent_otel_span):
        params = deployment.get("litellm_params") or {}
        api_base = (params.get("api_base") or "").rstrip("/")
        if not OLLAMA_URL or api_base != OLLAMA_URL:
            return None
        reason = await self._not_awake()
        if reason is None:
            return None
        # The router turns this into an OpenAI-style 503 for the client.
        raise ServiceUnavailableError(
            message=reason, llm_provider="ollama", model=params.get("model") or ""
        )

    async def async_pre_call_hook(self, user_api_key_dict, cache, data, call_type):
        if call_type != "call_mcp_tool":
            model = data.get("model")
            if isinstance(model, str) and model.startswith(OLLAMA_ROUTES) and "keep_alive" not in data:
                keep_alive = await self._keep_alive_setting()
                if keep_alive:
                    data["keep_alive"] = keep_alive
            return data
        # LiteLLM puts the server's alias or config name in mcp_server_name.
        if data.get("mcp_server_name") not in WAKE_MCP_SERVERS:
            return data
        reason = await self._not_awake()
        if reason is None:
            return data
        raise HTTPException(status_code=503, detail=reason)


handler = WakeOnLan()
