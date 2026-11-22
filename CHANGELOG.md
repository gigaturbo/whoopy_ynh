# Changelog

## Unreleased

- [x] **DOC** the user manual moves from the README into `doc/`

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
