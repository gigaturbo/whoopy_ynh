# MCP servers

The tool servers Whoopy publishes. A client adds one as an MCP server over
Streamable HTTP, at `https://<domain>/mcp/<name>`, with a Whoopy token as
Bearer. A call without a token gets a 401, never the portal.

**Local networks only, unless opened.** The MCP routes answer only the
networks listed in the panel's [LAN ranges](config.md#lan-ranges); from
anywhere else they answer 403. Away from home, connect through the VPN first,
or open a server to the internet with the panel's
[public MCP servers](config.md#public-mcp-servers). The LLM API under `/v1`
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

## Adding an MCP server

Any other MCP server, such as an app on this machine that publishes its own
on loopback, is added on Whoopy's status page (its MCP Servers tab), or in LiteLLM's
admin page (`/ui/`, MCP Servers) for the finer settings, never in the
package. The status page acts with your admin page session: sign in to `/ui/`
first. Both work from the LAN ranges only.

- **Address:** its Streamable HTTP URL, e.g. `http://127.0.0.1:<port>/mcp`.
- **Authentication:** Bearer token, the one that app gives. LiteLLM stores it
  encrypted in its database.
- **Who:** every key, or only the keys it is granted to (per key in the
  admin page, the key's MCP servers).

It is then served at `/mcp/<name>`, its tools named `<server>-<tool>`. A
server that runs on the Ollama machine does not wake it: only `/mcp/whoopy`
does.

The same can be done with the API, as an admin, from the LAN ranges:

```bash
curl https://<domain>/v1/mcp/server -H "Authorization: Bearer <master key>" \
  -H "Content-Type: application/json" -d @server.json
```

where `server.json` holds `server_name`, `url`, `"transport": "http"`,
`"auth_type": "bearer_token"`, `"credentials": {"auth_value": "<token>"}` and
`allow_all_keys`. A `server_id` given there is kept: keys granted a server
keep their grant when it is re-created with the same id.

Database servers travel in Whoopy's backup with the rest of the database.

## All of them (`/mcp`)

LiteLLM's own endpoint gathers every server a key may use, each tool
prefixed with its server's name.
