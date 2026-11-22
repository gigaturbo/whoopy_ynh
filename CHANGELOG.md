# Changelog

## 0.2.0~ynh5 (2026-11-22)

- [x] **PKG** web search and documents leave the package and its panel: they, and any other MCP server or model, are added in LiteLLM's admin page
- [x] **FEAT** the status page lists, adds and removes MCP servers, with the admin page's sign-in
- [x] **DOC** how to add an MCP server, from the status page, the admin page or the API

## 0.2.0~ynh4 (2026-11-22)

- [x] **FEAT** the MCP routes answer the local networks only, set in the panel's LAN ranges; 403 from the internet
- [x] **PKG** the master key button is now `new_master_key`, like every other rotation button
- [x] **DOC** the user manual moves from the README into `doc/`
- [x] **DOC** every gateway tool is named `<server>-<tool>`; the documents server's seven tools are listed

## 0.2.0~ynh3 (2026-11-21)

- [x] **FIX** generated tokens and the master key rotation no longer appear in YunoHost's logs

## 0.2.0~ynh2 (2026-11-21)

- [x] **FIX** a sleeping Ollama machine lists no models, instead of a made-up `ollama/llama2`

## 0.2.0~ynh1 (2026-11-20)

- [x] **FEAT** a status page at `/`: the machine's state, its last wake, a Wake button
- [x] **FEAT** tabs to LiteLLM's admin page (`/ui/`) and the API reference, now at `/docs`

## 0.1.0~ynh1 (2026-11-20)

- [x] **FEAT** the wake moves to its own service, written in Rust, shared by requests, the panel and MCP
- [x] **FEAT** the Ollama machine as MCP tools at `/mcp/whoopy`: wake, status, models
- [x] **PKG** the version is Whoopy's own; upgrading from `1.106.6~ynh14` needs `--force`

Earlier releases were numbered after LiteLLM (`1.106.6~ynh1` to `~ynh14`) and
have no changelog.
