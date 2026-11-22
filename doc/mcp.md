# MCP servers

The tool servers Whoopy publishes. A client adds one as an MCP server over
Streamable HTTP, at `https://<domain>/mcp/<name>`, with a Whoopy token as
Bearer. A call without a token gets a 401, never the portal.

## The Ollama machine (`/mcp/whoopy`)

Always on, for any token.

| Tool | Does |
|---|---|
| `whoopy-wake` | wakes the machine and waits for Ollama to answer |
| `whoopy-status` | awake or asleep, models in memory, last wake |
| `whoopy-models` | the models Ollama has |

Only `whoopy-wake` wakes the machine.

## Web search (`/mcp/web`)

Needs the cheruby-search-mcp app, its token in the panel and the switch on
(see [Configuration](config.md#web-search)). Any token may use it.

| Tool | Does |
|---|---|
| `search` | a web search through that app's private SearXNG |
| `fetch` | one public page, as text |

Never wakes the Ollama machine.

## Documents (`/mcp/docs`)

Needs the cheruby-docs-mcp app, one of its tokens in the panel and the switch
on (see [Configuration](config.md#documents)). Serves the folders that app
opens to the token: list, find by name and read, never write.

Only keys granted `docs` reach it. Grant it per key in LiteLLM's admin page
(`/ui/`, the key's MCP servers).

## All of them (`/mcp`)

LiteLLM's own endpoint gathers every server a key may use, each tool
prefixed with its server's name.
