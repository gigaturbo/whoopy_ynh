# MCP servers

The tool servers Whoopy publishes. A client adds one as an MCP server over
Streamable HTTP, at `https://<domain>/mcp/<name>`, with a Whoopy token as
Bearer. A call without a token gets a 401, never the portal.

**Two checks, by two parts.** A call reaches a server only if both pass:

- **Where from: Whoopy.** A server answers the panel's
  [LAN ranges](config.md#lan-ranges) only, unless its "From the internet"
  box is ticked on the status page (MCP Servers tab). nginx asks the wake
  daemon at each outside call; anything else gets a 403.
- **Which key: LiteLLM.** A server with "Every key" ticked takes any Whoopy
  token. Without it, only the keys granted that server: in LiteLLM's admin
  page, Virtual Keys, a key's MCP servers.

The LLM API under `/v1` is not affected by either. It does not reach the
MCP servers either: a model request whose `tools` name one (`{"type":
"mcp", "server_url": "litellm_proxy/mcp/<name>"}`) is refused with 403. A
client calls `/mcp/<name>` itself.

**From the internet.** Away from home, either connect through the VPN, or
tick the server's "From the internet" box. Then:

- **Use exactly `/mcp/<name>`.** Other forms of the address
  (`/<name>/mcp`, `/mcp/web,docs`, `/mcp/<name>/...`) answer 403 from
  outside, and so do the bare `/mcp` and the routes that add or change a
  server.
- **Make a key per outside client**, granted only the servers it needs.
  Never use the master key away from home: it reaches every server.
- **A server's write tools are open too.** A key granted `docs` from outside
  can write files.
- **Keep server names apart from MCP access groups.** LiteLLM reads a name
  with no server as an access group's, and would open that group's servers.
- **If the wake service is down**, outside calls fail (500); the LAN ranges
  still get in.

**Tool names.** Each tool is named `<server>-<tool>`: LiteLLM puts the
server's name and a `-` in front of the name the app gives it. The tables
below use that full name. Each app's own documentation uses the bare name
(`read_file` there is `docs-read_file` here).

## The AI server (`/mcp/whoopy`)

Published by the package. Who may use it (every key, granted keys, or
nobody) is the config panel's
[package MCP server](config.md#access) setting; it cannot be removed from
the status page. From the internet: its box on the status page, as any other.

| Tool | Does |
|---|---|
| `whoopy-wake` | wakes the machine and waits for Ollama to answer |
| `whoopy-status` | awake or asleep, models in memory, last wake |
| `whoopy-models` | the models Ollama has |
| `whoopy-settings` | the wake settings and the servers open from the internet |

Only `whoopy-wake` wakes the machine. None changes a setting: that is the
status page's and the command line's.

## Adding an MCP server

Any other MCP server, such as an app on this machine that publishes its own
on loopback, is added on Whoopy's status page (its MCP Servers tab), or in LiteLLM's
admin page (`/ui/`, MCP Servers) for the finer settings, never in the
package. The status page acts with your admin page session: sign in to `/ui/`
first. Both work from the LAN ranges only.

- **Address:** its Streamable HTTP URL, e.g. `http://127.0.0.1:<port>/mcp`.
  A port on this machine must first be added to the config panel's
  loopback ports, or LiteLLM cannot reach it. The same goes for a model
  served on this machine.
- **Authentication:** Bearer token, the one that app gives. LiteLLM stores it
  encrypted in its database.
- **Who:** every key, or only the keys it is granted to (per key in the
  admin page, the key's MCP servers). Changed later with its "Every key" box.
- **A new token** (the app rotated it) goes in with the server's Token
  button. The server stays the same, so the keys granted it keep their grant;
  removing and re-adding it would lose them.

It is then served at `/mcp/<name>`, its tools named `<server>-<tool>`. A
server that runs on the AI server does not wake it: only `/mcp/whoopy`
does.

### From the command line

On the server, as root. The command reads LiteLLM's master key from
Whoopy's root-only env file; a token is read from stdin, never from the
command line, which every local user can see.

```bash
sudo /var/www/whoopy/bin/whoopy mcp list
read -rs t && printf '%s' "$t" | sudo /var/www/whoopy/bin/whoopy mcp add docs http://127.0.0.1:8770/mcp --token-stdin
sudo /var/www/whoopy/bin/whoopy mcp update docs --granted
read -rs t && printf '%s' "$t" | sudo /var/www/whoopy/bin/whoopy mcp update docs --token-stdin
unset t
```

`read -rs` takes the token without showing it or writing it to a file; the
command refuses a token typed straight at it.

`add` uses the status page's settings: Streamable HTTP, a Bearer token if
one is given, every key unless `--granted`. `update` changes only what it is
given (`--url`, `--every-key` or `--granted`, `--token-stdin`), so the keys
granted the server keep their grant. A loopback address gets a reminder of
the loopback ports. The package's own server cannot be changed there.

### With the API

The same can be done with the API, as an admin, from the LAN ranges:

```bash
curl https://<domain>/v1/mcp/server -H "Authorization: Bearer <master key>" \
  -H "Content-Type: application/json" -d @server.json
```

where `server.json` holds `server_name`, `url`, `"transport": "http"`,
`"auth_type": "bearer_token"`, `"credentials": {"auth_value": "<token>"}` and
`allow_all_keys`. A `server_id` given there is kept: keys granted a server
keep their grant when it is re-created with the same id. A new token needs no
re-creation: `PUT` on the same route, with only `server_id`,
`"auth_type": "bearer_token"` and `credentials`.

Database servers travel in Whoopy's backup with the rest of the database.

## All of them (`/mcp`)

LiteLLM's own endpoint gathers every server a key may use, each tool
prefixed with its server's name.
