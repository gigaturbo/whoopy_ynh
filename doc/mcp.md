# MCP servers

The tool servers Whoopy publishes. A client adds one as an MCP server over
Streamable HTTP, at `https://<domain>/mcp/<name>`, with a Whoopy token as
Bearer. A call without a token gets a 401, never the portal.

**Local networks only.** The MCP routes answer only the networks listed in
the panel's [LAN ranges](config.md#lan-ranges); from anywhere else they answer
403. Away from home, connect through the VPN first. The LLM API under `/v1`
is not affected.

**Tool names.** Each tool is named `<server>-<tool>`: LiteLLM puts the
server's name and a `-` in front of the name the app gives it. The tables
below use that full name. Each app's own documentation uses the bare name
(`read_file` there is `docs-read_file` here).

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
(see [Configuration](config.md#web-search)). Any token may use it. Arguments
are in that app's
[MCP tools](https://github.com/gigaturbo/cheruby-search-mcp_ynh/blob/main/doc/mcp.md).

| Tool | Does |
|---|---|
| `web-search` | a web search through that app's private SearXNG |
| `web-fetch` | one public page, as text |

Never wakes the Ollama machine.

## Documents (`/mcp/docs`)

Needs the cheruby-docs-mcp app, its token in the panel and the switch on (see
[Configuration](config.md#documents)). Serves the folders that app opens to
Whoopy's token. Arguments are in that app's
[MCP tools](https://github.com/gigaturbo/cheruby-docs-mcp_ynh/blob/main/doc/mcp.md).

| Tool | Does |
|---|---|
| `docs-list_folder` | a folder's subfolders and files |
| `docs-find_files` | files whose path holds every word |
| `docs-read_file` | a text, PDF, office file or image |
| `docs-search_documents` | the passages that best answer a question |
| `docs-write_file` | creates a text file, in a folder marked writable |
| `docs-create_folder` | creates a folder, in a folder marked writable |
| `docs-run_index` | asks for an index run, of some files or all |

Only keys granted `docs` reach it. Grant it per key in LiteLLM's admin page
(`/ui/`, the key's MCP servers).

## Other apps

Apps that publish their own MCP server on this machine, such as flambyno and
oshawyt, are not set up by the package. Add each in LiteLLM's admin page
(`/ui/`, MCP Servers) with its loopback address and the token its
`<app> token` command shows, then grant it to chosen keys. Its tools are then
named `<server>-<tool>` like the ones above.

## All of them (`/mcp`)

LiteLLM's own endpoint gathers every server a key may use, each tool
prefixed with its server's name.
