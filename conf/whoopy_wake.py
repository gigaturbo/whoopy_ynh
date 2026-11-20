"""Have the Ollama machine woken before LiteLLM sends it a request.

Loaded by LiteLLM from `litellm_settings.callbacks` ("whoopy_wake.handler"),
from the directory of config.yaml. The waking itself is done by the wake
daemon (whoopy-wake.service): this hook asks it over its socket, and the
daemon answers once Ollama does, or with 503 when the wake fails. The daemon
holds the shared wake, the Wake-on-LAN settings and the timers.

Settings come from the service environment:

    WHOOPY_OLLAMA_URL       scheme://host:port of Ollama, no path
    WHOOPY_WOL_ENABLED      "1" to wake; anything else turns this hook off
    WHOOPY_WAKE_TIMEOUT_S   the daemon's own timeout; the hook waits a little longer
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
"""

import logging
import os

import httpx
from fastapi import HTTPException
from litellm.exceptions import ServiceUnavailableError
from litellm.integrations.custom_logger import CustomLogger

OLLAMA_URL = os.environ.get("WHOOPY_OLLAMA_URL", "").rstrip("/")
ENABLED = os.environ.get("WHOOPY_WOL_ENABLED") == "1"
SOCKET = os.environ.get("WHOOPY_SOCKET", "/run/whoopy/wake.sock")
# Past the daemon's timeout, so its own 503 and reason arrive first.
WAIT_S = float(os.environ.get("WHOOPY_WAKE_TIMEOUT_S", "90")) + 10

# The `mcp_servers` names in config.yaml that run on the Ollama machine; none
# today. A server on this host (whoopy, web, docs) must never be listed.
WAKE_MCP_SERVERS: set[str] = set()

log = logging.getLogger("whoopy_wake")


class WakeOnLan(CustomLogger):
    def __init__(self):
        super().__init__()
        self._client = httpx.AsyncClient(
            transport=httpx.AsyncHTTPTransport(uds=SOCKET),
            base_url="http://whoopy",
            timeout=WAIT_S,
        )

    async def _not_awake(self):
        """None once Ollama answers, or when nothing needs waking; else why not."""
        if not ENABLED:
            return None
        try:
            resp = await self._client.post("/ensure")
        except httpx.TimeoutException:
            return f"The Ollama machine did not answer within {int(WAIT_S)} s."
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
        return f"The Ollama machine did not wake: {reason}."

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
        # LiteLLM puts the server's alias or config name in mcp_server_name.
        if call_type != "call_mcp_tool" or data.get("mcp_server_name") not in WAKE_MCP_SERVERS:
            return data
        reason = await self._not_awake()
        if reason is None:
            return data
        raise HTTPException(status_code=503, detail=reason)


handler = WakeOnLan()
