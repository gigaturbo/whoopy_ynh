# Configuration

Settings that change after install live in two places, split by who may
change them.

- **The status page** (`/`, its Config tab): how the wake behaves and how long
  a model stays loaded. Anyone in the group allowed to open the page may
  change them, and a change applies to the next request.
- **The config panel** (webadmin: Applications, Whoopy, Configuration): what
  guards Whoopy, root-only.

## The status page

| Setting | Default | Effect |
|---|---|---|
| Wake the AI server | off | wake it when a request arrives and it does not answer |
| MAC address | empty | its card with Wake-on-LAN enabled; needed to switch the wake on |
| Broadcast address | `255.255.255.255` | where the wake packet goes; the subnet's own broadcast if the server has several networks |
| Seconds to wait | `90` | 10 to 540; past it the request fails with a 503 |
| How long a model stays loaded | empty | after a request through Whoopy, e.g. `5m`. Empty: Ollama's own setting. A request that sets `keep_alive` itself keeps its own. |

The page's Wake button sends the wake packet and waits for the AI server, with
these settings. They are kept in `/var/lib/whoopy/settings.json`, which
Whoopy's backup carries, with the MCP servers opened to the internet on the
MCP Servers tab (see [MCP servers](mcp.md)).

## The config panel

Set a value from the command line, a key at a time:

```bash
sudo yunohost app config set whoopy main.access.lan_ranges -v "<ranges>"
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

## Access

| Key | Default | Effect |
|---|---|---|
| `main.access.outbound` | `lan` | `lan`: LiteLLM reaches local networks only. `any`: anywhere, needed for cloud providers. |
| `main.access.lan_ranges` | the private networks | who may reach the [MCP routes](mcp.md); anyone else gets a 403 |
| `main.access.whoopy_mcp` | `all` | who may use the package's MCP server, `/mcp/whoopy`: `all` every key, `granted` the keys granted it in LiteLLM's admin page, `off` not published |
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
stop working; tokens made in the admin page keep working. From the command
line:

```bash
sudo yunohost app action run whoopy main.master_key.new_master_key
```
