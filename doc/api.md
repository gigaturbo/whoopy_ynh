# API and access

The addresses Whoopy answers on, who may reach each, and the model names.

## Who reaches what

| Path | Who | Guarded by |
|---|---|---|
| `/` | the group chosen at install | the YunoHost portal |
| `/ui/` | the same group | the portal, then LiteLLM's admin login |
| `/docs` | the same group | the portal (LiteLLM's Swagger) |
| `/v1`, `/chat/completions`, `/completions`, `/embeddings`, `/models` | anyone on the internet | a LiteLLM token |
| `/mcp` | the [LAN ranges](config.md#lan-ranges) only | a LiteLLM token |
| `/v1/mcp`, `/mcp-rest` | YunoHost admins, from the LAN ranges only | the portal, then a token |
| `/v1/tool`, `/v1/traces`, `/v1/decisions` | YunoHost admins | the portal, then a token |

Every MCP route, LiteLLM's admin page for MCP (`/ui/mcp/`) included, answers
403 outside the LAN ranges.

Tokens are made in LiteLLM's admin page (`/ui/`, Virtual Keys). The admin
login is user `admin` with the password set at install or in the
[config panel](config.md).

## The status page

The chip at the top right says whether the Ollama machine is awake, waking,
asleep, or out of reach (offline). `/` has three tabs and two links out:

- **Status:** whether the Ollama machine is awake, its last wake, and a Wake
  button. On a wide screen the MCP servers sit beside it.
- **MCP Servers:** the [MCP servers](mcp.md#adding-an-mcp-server), listed,
  added and removed once signed in to LiteLLM's admin page.
- **Config:** the [wake settings](config.md#the-status-page).
- **LiteLLM Admin** opens LiteLLM's admin page (`/ui/`) in a new tab, and
  **LiteLLM API** its API reference (`/docs`).

## Models

| Name | What |
|---|---|
| `ollama/<name>` | every chat model Ollama has |
| `ollama-embed/<name>` | the same machine's embedding models |
| anything else | the models added in LiteLLM's admin page (`/ui/`, Models) |

While the Ollama machine sleeps, `/v1/models` lists none of its models. A
listing is cached for 5 minutes, so they appear up to 5 minutes after a wake.

## Calls

OpenAI-style chat, completions, embeddings and models under `/v1`, and
Anthropic-style messages at `/v1/messages`.

```bash
curl https://<domain>/v1/chat/completions \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"model": "ollama/<name>", "messages": [{"role": "user", "content": "Hello"}]}'
```

A request for a sleeping machine waits while it wakes, up to the timeout set
on the [status page](config.md#the-status-page). Past it, the answer is a 503.
Requests arriving together share one wake.
