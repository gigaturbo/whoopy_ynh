# API and access

The addresses Whoopy answers on, who may reach each, and the model names.

## Who reaches what

| Path | Who | Guarded by |
|---|---|---|
| `/` | the group chosen at install | the YunoHost portal |
| `/ui/` | the same group | the portal, then LiteLLM's admin login |
| `/docs` | the same group | the portal (LiteLLM's Swagger) |
| `/v1`, `/chat/completions`, `/completions`, `/embeddings`, `/models`, `/mcp` | anyone on the internet | a LiteLLM token |
| `/v1/mcp`, `/mcp-rest`, `/v1/tool`, `/v1/traces`, `/v1/decisions` | YunoHost admins | the portal, then a token |

Tokens are made in LiteLLM's admin page (`/ui/`, Virtual Keys). The admin
login is user `admin` with the password set at install or in the
[config panel](config.md).

## The status page

`/` shows whether the Ollama machine is awake, its last wake, and a Wake
button. Its tabs open LiteLLM's admin page (`/ui/`) and the API reference
(`/docs`).

## Models

| Name | What |
|---|---|
| `ollama/<name>` | every chat model Ollama has |
| `ollama-embed/<name>` | the same machine's embedding models |
| `embeddinggemma` | the cheruby-docs-mcp app's embedding model, when [documents](mcp.md#documents-mcpdocs) are on |

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
in the [config panel](config.md#wake-on-lan). Past it, the answer is a 503.
Requests arriving together share one wake.
