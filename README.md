<p align="center"><img src="doc/logo.svg" width="160" alt="Whoopy"></p>

# Whoopy, an AI gateway for YunoHost

One authenticated endpoint for the models on an Ollama machine that sleeps.
Whoopy wraps the [LiteLLM](https://github.com/BerriAI/litellm) proxy: clients
call an OpenAI-style API with a token, and a small companion service wakes
the machine by Wake-on-LAN when a request arrives.

## What you get

- **An OpenAI-style API** at `/v1` (chat, completions, embeddings, models),
  plus Anthropic-style `/v1/messages`. Every Ollama model appears as
  `ollama/<name>`, and as `ollama-embed/<name>` for embeddings.
- **Wake-on-LAN.** A request for a sleeping machine waits while it wakes, up
  to a timeout set in the config panel. Requests arriving together share one
  wake.
- **The Ollama machine as tools** at `/mcp/whoopy`: `wake` it ahead of time,
  see its `status` (awake, models in memory, last wake) and list its
  `models`, with any Whoopy token. Only `wake` wakes it.
- **MCP.** LiteLLM can gather MCP servers behind one `/mcp` endpoint.
- **Web search for the models** at `/mcp/web`, when the
  [cheruby-search-mcp](#web-search) app runs on the same server.
- **Your documents for the models** at `/mcp/docs`, when the
  [cheruby-docs-mcp](#documents) app runs on the same server.

## Access

- **The admin page** (`/`) sits behind the YunoHost portal, for the group
  chosen at install, then asks for LiteLLM's own admin password (user `admin`).
- **The API paths** are open to the internet and guarded by LiteLLM tokens.

## Settings

The config panel holds:

- **Ollama**: its URL and how long models stay loaded.
- **Wake-on-LAN**: its settings, the machine's current state and a "wake
  now" button.
- **Admin**: the admin password and a master key rotation.
- **Internet access**: off by default, LiteLLM reaches local networks only, so
  a compromised release cannot send anything out. Cloud providers need it on.

## Web search

- **Needs** the `cheruby-search-mcp` app (a private SearXNG), its token
  pasted in the panel's "Web search" section, and the switch on. The token:
  `sudo yunohost app setting cheruby-search-mcp token`.
- **Tools**: `search`, and `fetch` (one public page as text).
- **Clients** add `https://<domain>/mcp/web` as an MCP server (Streamable
  HTTP), with any Whoopy token.
- **Its address** defaults to that app's own on this server.
- **Never wakes** the Ollama machine.

## Documents

- **Needs** the `cheruby-docs-mcp` app, one of its tokens pasted in the
  panel's "Documents" section, and the switch on.
- **The token decides the folders**: `/mcp/docs` serves the ones that app
  opens to it. List, find by name and read, never write.
- **Its addresses** (tool server and embedding model) default to that app's
  own on this server.
- **Access is per key**: grant `docs` to the keys that may read them, in
  LiteLLM's admin page.

## Security choices

- **LiteLLM is pinned** (1.106.6), installed from a hash-checked lock file.
  Its releases were hijacked twice in 2026. The pin moves only for a fix.
- **Code is owned by root**, readable by the app. Secrets sit in a root-only
  file, read by systemd before it drops privileges.
- **The service is confined**: no `/home` beyond its own data, no `/mnt`,
  `/media` or `/srv`. Memory capped at 1.5 GB.
- **Nothing is fetched at runtime**: price list, blog feed and similar
  downloads are off. Prisma's engines are built at install.
- **The wake service is separate and smaller**: written in Rust
  ([`whoopy/`](whoopy/)), it reads only the wake settings and its own token,
  never LiteLLM's secrets, and cannot reach the internet. Its binary is
  downloaded at install from this repository's releases and checked against
  the sha256 in the manifest.

## Versions

The package version is Whoopy's own (`0.1.0~ynh1`: the wake service's
version, then the packaging revision). Up to `1.106.6~ynh14` it followed
LiteLLM's; the first upgrade across needs `--force`, since 0.1.0 sorts lower.

## Requirements

- **arm64**: the lock file holds aarch64 wheels.
- **Network** during install, upgrade and restore: the code is rebuilt, not
  backed up.
- **Wake-on-LAN**: the Ollama machine on the same network, wake enabled on
  its card.
