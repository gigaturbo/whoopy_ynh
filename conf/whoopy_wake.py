"""Have the AI server woken before LiteLLM sends it a request.

Loaded by LiteLLM from `litellm_settings.callbacks` ("whoopy_wake.handler"),
from the directory of config.yaml. The waking itself is done by the wake
daemon (whoopy-wake.service): this hook asks it over its socket, and the
daemon answers once Ollama does, at once when waking is off, or with 503 when
the wake fails. The daemon holds the shared wake, the wake settings (changed
on the status page) and the timers.

From the service environment:

    WHOOPY_OLLAMA_URL       scheme://host:port of Ollama, no path; its host is
                            the AI server
    WHOOPY_SOCKET           the daemon's socket (default /run/whoopy/wake.sock)

Two entry points, because LiteLLM has two request paths:
- model calls go through the router, which hands `async_pre_call_check` the
  deployment it picked. Only a deployment whose api_base is on the AI
  server's host wakes: Ollama, and any database model there, such as
  `speaches/*` through froaky.
- a tool call reaches `async_pre_call_hook` as call_type "call_mcp_tool",
  and wakes only for a server in WAKE_MCP_SERVERS.

A tool listing reaches no hook (LiteLLM 1.106.6), so it never wakes: while
the machine sleeps, its servers are missing from the list. And LiteLLM refuses
a tool name it has never listed before any hook runs, so after a restart
while the machine sleeps, a call fails until a listing succeeds.

It refuses with 403 a model request whose `tools` name an MCP server of this
gateway (`litellm_proxy/mcp/<name>`, or an http(s) URL ending in
`/mcp/<name>`), whatever the caller: LiteLLM would list and run that server's
tools itself, behind `/v1`, where nginx's LAN rule and open list never apply.
MCP is reached at `/mcp/<name>` only. The test is LiteLLM's own, the one its
three gateway routes use, so a new URL form stays covered.

It sets the page's keep_alive on requests to the ollama/ and ollama-embed/
routes that carry none, read from the daemon at most every KEEP_ALIVE_TTL.

It also empties LiteLLM's static Ollama model list: while the machine sleeps,
/api/tags fails and /v1/models would list that instead ("ollama/llama2").
And it sorts the listing of the two wildcards: an Ollama model with the
"embedding" capability is listed under ollama-embed/ only, any other under
ollama/ only. Calls are not filtered.
"""

import logging
import os
import time
from urllib.parse import urlsplit

import httpx
import litellm
from fastapi import HTTPException
from litellm.exceptions import ServiceUnavailableError
from litellm.integrations.custom_logger import CustomLogger
from litellm.proxy.auth import model_checks
from litellm.responses.mcp.litellm_proxy_mcp_handler import LiteLLM_Proxy_MCP_Handler

OLLAMA_URL = os.environ.get("WHOOPY_OLLAMA_URL", "").rstrip("/")
AI_HOST = urlsplit(OLLAMA_URL).hostname
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

# Whether a wildcard of config.yaml lists embedding models, by the provider it
# routes through: ollama_chat for ollama/*, ollama for ollama-embed/*.
LISTS_EMBEDDERS = {"ollama_chat": False, "ollama": True}
# The embedding models last seen per Ollama URL, for when /api/tags fails.
_embedders: dict[str, frozenset[str]] = {}
_get_provider_models = model_checks.get_provider_models


def _provider_models(provider, litellm_params=None):
    """LiteLLM's wildcard expansion, each Ollama model under one route only.

    Replaces model_checks.get_provider_models (LiteLLM 1.106.6), which
    get_known_models_from_wildcard looks up at each call. LiteLLM's own list
    holds names, without the capabilities: those come from /api/tags here."""
    models = _get_provider_models(provider, litellm_params)
    lists_embedders = LISTS_EMBEDDERS.get(provider)
    base = (getattr(litellm_params, "api_base", None) or "").rstrip("/")
    if not models or lists_embedders is None or not AI_HOST or urlsplit(base).hostname != AI_HOST:
        return models
    try:
        tags = httpx.get(f"{base}/api/tags", timeout=5).json()["models"]
        _embedders[base] = frozenset(
            f"ollama/{m['name']}" for m in tags if "embedding" in (m.get("capabilities") or ())
        )
    except (httpx.HTTPError, ValueError, KeyError, TypeError):
        pass
    embedders = _embedders.get(base)
    if embedders is None:
        return models
    return [m for m in models if (m in embedders) == lists_embedders]


model_checks.get_provider_models = _provider_models


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
        if not AI_HOST or urlsplit(params.get("api_base") or "").hostname != AI_HOST:
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
            tools = data.get("tools")
            if isinstance(tools, list) and LiteLLM_Proxy_MCP_Handler.should_use_litellm_mcp_gateway(tools):
                raise HTTPException(
                    status_code=403,
                    detail="MCP tools in a model request are refused here: call /mcp/<name> instead.",
                )
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
