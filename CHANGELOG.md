# Changelog

## 0.4.1~ynh4 (2026-11-26)

- [x] **FIX** LiteLLM reaches only the services on this machine it needs; the panel lists the ports of the MCP servers and models added here

## 0.4.1~ynh2 (2026-11-26)

- [x] **FEAT** a request to any model on the AI server wakes it, not only Ollama's: `speaches/*` through froaky transcribes from asleep

## 0.4.1~ynh1 (2026-11-24)

- [x] **FEAT** the page, the CLI and the MCP tools call the sleeping machine the AI server, since it runs more than Ollama

## 0.4.0~ynh2 (2026-11-24)

- [x] **FEAT** the status page takes the shared look of the family's pages: dark, the mascot in the header, the machine's state in a chip at the top right
- [x] **FEAT** on a wide screen the status page shows the MCP servers beside the status; LiteLLM's links open a new tab and say so

## 0.4.0~ynh1 (2026-11-23)

- [x] **FEAT** the status page opens each MCP server to the internet with a box, applied at once; the panel's list goes, and its servers stay open
- [x] **FEAT** the status page switches each MCP server between every key and granted keys
- [x] **FEAT** the panel chooses who may use `/mcp/whoopy`: every key, granted keys or nobody

## 0.3.0~ynh5 (2026-11-23)

- [x] **FEAT** the panel can open chosen MCP servers to the internet, at `/mcp/<name>` with a key; the others stay on the local networks

## 0.3.0~ynh4 (2026-11-23)

- [x] **FEAT** the status page has a tab each for MCP servers and the wake settings (Config); LiteLLM's are named LiteLLM Admin and LiteLLM API

## 0.3.0~ynh3 (2026-11-23)

- [x] **FIX** a restore or reinstall after removing the app no longer fails on the data folder the remove left behind

## 0.3.0~ynh2 (2026-11-23)

- [x] **FIX** the status page's MCP section works after signing in to the admin page, instead of saying the session expired

## 0.3.0~ynh1 (2026-11-22)

- [x] **FEAT** the wake settings and how long a model stays loaded move from the config panel to the status page, and apply without a restart
- [x] **PKG** the config panel keeps only the Ollama address, access and the master key; its Wake section goes, the page has one
- [x] **FIX** the wake daemon's `url` dependency moves to 2.5.9 (RUSTSEC-2026-0592)

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
