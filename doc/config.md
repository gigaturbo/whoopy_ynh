# Configuration

Every setting that can change after install, in the app's config panel
(webadmin: Applications, Whoopy, Configuration).

Set one from the command line, a key at a time:

```bash
sudo yunohost app config set whoopy main.wol.wol_mac -v aa:bb:cc:dd:ee:ff
```

## Install questions

| Question | Effect |
|---|---|
| Domain | where Whoopy answers, at its root |
| Who may open the status page | the group behind the portal for `/`, default `admins` |
| Ollama URL | scheme, host and port, e.g. `http://192.168.1.10:11434` |
| Admin password | LiteLLM's admin page, user `admin` |

## Ollama

| Key | Default | Effect |
|---|---|---|
| `main.ollama.ollama_url` | set at install | Ollama's address, without a path. With outbound set to local networks, a private address. |
| `main.ollama.keep_alive` | empty | how long a model stays loaded after a request, e.g. `5m`. Empty: Ollama's own setting. |

## Wake-on-LAN

| Key | Default | Effect |
|---|---|---|
| `main.wol.wol_enabled` | off | wake the machine when a request arrives and it does not answer |
| `main.wol.wol_mac` | empty | MAC address of its card with Wake-on-LAN enabled |
| `main.wol.wol_broadcast` | `255.255.255.255` | where the wake packet goes; the subnet's own broadcast if the server has several networks |
| `main.wol.wake_timeout_s` | `90` | seconds to wait for Ollama after a wake, 10 to 540; past it the request fails with a 503 |

## Wake now

Shows the wake service's state. **Wake it now** sends the wake packet and
waits for Ollama, with the saved MAC, broadcast and timeout.

## Web search

| Key | Default | Effect |
|---|---|---|
| `main.search.search_enabled` | off | offer [web search](mcp.md#web-search-mcpweb) at `/mcp/web` |
| `main.search.search_url` | `http://127.0.0.1:8772/mcp` | the cheruby-search-mcp app's MCP server |
| `main.search.search_token` | kept | a new token; empty keeps the current one |

The token comes from that app:

```bash
sudo yunohost app setting cheruby-search-mcp token
```

## Documents

| Key | Default | Effect |
|---|---|---|
| `main.docs.docs_enabled` | off | offer [documents](mcp.md#documents-mcpdocs) at `/mcp/docs` and `embeddinggemma` |
| `main.docs.docs_url` | `http://127.0.0.1:8770/mcp` | the cheruby-docs-mcp app's MCP server |
| `main.docs.docs_embed_url` | `http://127.0.0.1:8771/v1` | its embedding model; empty: no `embeddinggemma` |
| `main.docs.docs_token` | kept | a new token; empty keeps the current one |

The token decides which folders are served. It comes from that app:

```bash
sudo yunohost app setting cheruby-docs-mcp token_<user>
```

## Access

| Key | Default | Effect |
|---|---|---|
| `main.access.outbound` | `lan` | `lan`: LiteLLM reaches local networks only. `any`: anywhere, needed for cloud providers. |
| `main.access.lan_ranges` | the private networks | who may reach the [MCP routes](mcp.md); anyone else gets a 403 |
| `main.access.ui_password` | kept | a new admin password for LiteLLM's page; empty keeps it |
| `main.access.env_login` | on | allow the shared `admin` login. Turning it off is refused until an admin account of your own exists in LiteLLM's page. |

### LAN ranges

Addresses or networks separated by spaces. The default holds the private
networks only: `127.0.0.0/8 ::1 10.0.0.0/8 172.16.0.0/12 192.168.0.0/16
fc00::/7`. Anything else is refused, since the list is written into nginx.

- **Add your public IPv6 prefix.** Home devices on IPv6 use public addresses,
  which the default does not cover. The prefix your router delegates can
  change: update it here when it does.
- **The VPN** is covered by the default if its addresses are private.
- **Through the router's public IPv4,** a home device arrives from the
  router's own LAN address, so it counts as local. If the router ever hid
  the real source of traffic from the internet in the same way, the internet
  would count as local too. The Whoopy token is still required, which is why
  both checks stay.

```bash
sudo yunohost app config set whoopy main.access.lan_ranges -v "127.0.0.0/8 ::1 10.0.0.0/8 172.16.0.0/12 192.168.0.0/16 fc00::/7 <your-prefix>/60"
```

## Master key

**Replace the master key** makes a new one. Scripts using the old master key
stop working; tokens made in the admin page keep working.
