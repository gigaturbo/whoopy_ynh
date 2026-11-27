<p align="center"><img src="doc/logo.svg" width="160" alt="Whoopy"></p>

# Whoopy, an AI gateway for YunoHost

One authenticated endpoint for the models on an AI server that sleeps.
Whoopy wraps the [LiteLLM](https://github.com/BerriAI/litellm) proxy and wakes
the machine by Wake-on-LAN when a request arrives.

## Features

- **An OpenAI-style API** at `/v1` (chat, completions, embeddings, models),
  plus Anthropic-style `/v1/messages`. Ollama's chat models appear as
  `ollama/<name>`, its embedding models as `ollama-embed/<name>`.
- **Wake-on-LAN**: a request for a sleeping machine waits while it wakes.
  Requests arriving together share one wake.
- **A status page** at `/`: is the machine awake, its last wake, a Wake
  button, and tabs to LiteLLM's admin page and the API reference.
- **The AI server as MCP tools** at `/mcp/whoopy`: wake it ahead of
  time, see its state, list its models.
- **Any other MCP server** on the network, added from the status page and
  served at `/mcp/<name>` behind the same tokens.
- **MCP for the local network only**: the tool routes answer 403 from the
  internet, while the LLM API stays open to any token.
- **Confined by default**: no internet, root-owned code, a pinned and
  hash-checked LiteLLM.

## Install

Needs YunoHost 12.1 or later on arm64, network access during install,
upgrade and restore, and an AI server on the same network with
Wake-on-LAN enabled on its card.

```bash
sudo yunohost app install https://github.com/gigaturbo/whoopy_ynh
```

Then set the machine's MAC address and switch the wake on, on the
[status page](doc/config.md#the-status-page) at `/`.

## Usage

Make a token in LiteLLM's admin page (`/ui/`), then:

```bash
curl https://<domain>/v1/chat/completions \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"model": "ollama/<name>", "messages": [{"role": "user", "content": "Hello"}]}'
```

## Documentation

- [API and access](doc/api.md): the paths, who reaches each, the model names
- [Configuration](doc/config.md): the wake settings on the status page, the rest in the config panel
- [MCP servers](doc/mcp.md): the tools at `/mcp/whoopy`, and adding others from the page or the command line
- [Security](doc/security.md): how the app is confined

## Latest release

**0.3.0~ynh1** (2026-11-22). See the [changelog](CHANGELOG.md) for what changed.

## License

MIT, see [LICENSE](LICENSE).
