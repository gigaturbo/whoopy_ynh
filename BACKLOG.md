# BACKLOG

A first install on the server, proven end to end: a token reaches a model on
the sleeping desktop through its domain, and nothing is downloaded at
runtime.

## Categories

| Letter | Covers |
|---|---|
| `W` | Waking the Ollama machine |
| `P` | The packaging, the lifecycle scripts and the config panel |
| `M` | The MCP tool servers Whoopy publishes |

## Bugs

### B-P-5 · a restore after a remove fails on the old data folder

`todo` `small` `filed 2026-11-23`

`app remove` keeps `/home/yunohost.app/whoopy`, its files owned by the old
user id. The restore re-creates `whoopy` under a new id (982 then 989 on
2026-11-23) and owns only the folder itself, so LiteLLM cannot overwrite its
copied migrations, crash-loops, and the restore rolls back, leaving Whoopy
uninstalled. The same holds for a reinstall.
**Fix:** `whoopy_configure` hands the folder's contents to the app user
(0.3.0~ynh3, upgraded 2026-11-23: every file there the user's). Closes with a T-P-3 run on the release,
the old folder left in place.

## Features

### F-M-1 · a Tandoor recipe tool server behind `/mcp/tandoor`

`todo` `large` `filed 2026-11-06`

Agents on local models (Hermes Agent, Open WebUI) cannot search, read or add
Tandoor recipes. LiteLLM's built-in OpenAPI-to-MCP route cannot do it: it sends
JSON bodies only, so no photo, and leaves body `$ref`s unresolved.

**Do:** its own YunoHost app, `cheruby-recipes-mcp_ynh` in its own repository,
not in Whoopy (decided 2026-11-07: one app per MCP server). A loopback tool
server in Python, with its own root-600 env holding the Tandoor token and a
shared secret. Outbound HTTPS to the internet (decided 2026-11-09): the
photo download of `pack` needs it already. A URL resolving to loopback, the
LAN or link-local is refused, rechecked after each redirect, as
`ai-websearch-mcp` does. Seven tools, named and described as the
`tandoor-recipe` skill's `references/mcp-tools.json`, which the recipe-tuner
model is trained on:

- **On Tandoor:** `search_recipes`, `get_recipe`, `list_keywords`, and
  `add_recipe` taking the skill's compact format, packing it as the skill's
  `pack` does, then setting the photo with a multipart
  `PUT /api/recipe/{id}/image/`.
- **On the web:** `find_recipes`, `read_page`, `pick_recipes`, the skill's
  `search`, `read` and `pick` moves.

The skill's `scripts/move.py` is the code for all of them but the Tandoor
calls: ported and kept in step, or imported from a pinned copy, so its output
stays what the model was trained on.
Calls to Tandoor on loopback carry its domain as `Host`. Whoopy's package does not
know it: it is added as a database MCP server `tandoor` (LiteLLM's admin page
or Whoopy's page, F-M-13), over `http`, Bearer auth, every key. Install, upgrade and
anything written to the live Tandoor need the author's go-ahead.

### F-M-2 · update a recipe, the shopping list and the meal plan

`todo` `medium` `filed 2026-11-06` `needs: F-M-1`

The first Tandoor tool server only searches, reads and adds recipes.

### F-P-5 · move off LiteLLM 1.106.6

`todo` `medium` `filed 2026-11-20`

Checked 2026-11-20: no advisory on LiteLLM 1.106.6 itself, but three open on
its dependencies (oauthlib, opentelemetry-api, dnspython; R35 on the server).
Two ways, to be chosen:

- **Stay on 1.106:** re-lock 1.106.6 with the three fixed releases (it accepts
  them), then 1.106.12 or later from 2026-11-27. That brings the `/key/info`
  fix (behind the portal here) and lifts the filelock cap, which clears the 13
  accepted filelock ids.
- **Move to 1.108.x (1.108.5+):** fixes `ollama_chat/` streams giving every
  parallel tool call index 0, so clients ran only the first call. Never
  backported to 1.106. A feature reason, so it bends R35; needs a retest of the
  wake hook and MCP. 1.109 adds more MCP fixes, a chat page and
  `LITELLM_OFFLINE_MODE`, with five action items. 1.110 is not worth it yet.

## Tests

### T-P-3 · backup, remove, restore

`todo` `playtest` `reopened 2026-11-22` `needs: B-P-5`

**Do:** after asking, back up, remove and restore Whoopy, the data folder
left in place by the remove.
**Pass:** both services up; the settings, master key and MCP token identical
(fingerprints, never printed); `/v1/models` and `/mcp/whoopy` answer; the
database servers `web` and `docs` (same ids) and the model `embeddinggemma`
are back and answer, their stored tokens readable.
**Last:** 2026-11-23 at `6a47334`, 0.3.0~ynh2: the first restore failed and
rolled back (B-P-5). With the old data folder moved aside to
`whoopy.tp3-aside-20261123`, the second passed in 80 s: the 22 settings, both
env files and the wake settings identical; `web` and `docs` with their ids,
`/mcp/web` 2 tools, `/mcp/docs` 7, `/mcp/whoopy` 3, `embeddinggemma` 768
dimensions; `/v1/models` 200.

### T-M-1 · the compact format packs to Tandoor's JSON

`todo` `spec` `filed 2026-11-06` `needs: F-M-1`

**Do:** convert the tiramisu from the `tandoor-recipe` skill's `SKILL.md`
offline. **Pass:** the missing amount, the single unnumbered action, the
"Variantes" step and water with `ignore_shopping` come out as the skill's
`pack` makes them.

### T-M-2 · the tool server against the live Tandoor

`todo` `playtest` `filed 2026-11-06` `needs: F-M-1`

**Do:** after asking, run the server by hand with a token the author gives;
search, read, then add one test recipe with a photo URL and delete it.
**Pass:** the recipe and its photo show on its Tandoor page.

### T-M-3 · `/mcp/tandoor` through Whoopy after an upgrade

`todo` `playtest` `filed 2026-11-06` `needs: F-M-1`

**Do:** after asking to upgrade, call `/mcp/tandoor` without a token, with a
Whoopy token, and the tool server directly without the shared secret.
**Pass:** 401 without a token and no portal redirect; with one, four tools
listed and a search works; the bare call refused; no wake packet in
`journalctl -u whoopy`.

### T-M-4 · local-model clients add a recipe from a URL

`todo` `playtest` `filed 2026-11-06` `needs: T-M-3`

**Do:** with `ollama/gemma4-12b-ud-64k`, have Hermes Agent and Open WebUI
0.11.3 (External Tools, Streamable HTTP) each add one recipe from a URL.
**Pass:** both succeed; record how each passes the Whoopy token and how close
the result is to the house rules.

## Closed

- `T-M-11` done `playtest` 2026-11-23 at `09e09fd` · through the portal as an admin (run by the author): signed out of `/ui/` a sign-in message and no form, signed in the servers listed, a test server added and removed, then 403 at `/mcp/test`; on loopback with a real sign-in, `pwtest` listed its 3 tools through the domain with a key, then 403 once removed
- `T-P-5` done `playtest` 2026-11-23 at `09e09fd` · through the portal as an admin (run by the author): state, Wake, Admin and API docs tabs, the wake settings loaded, a bad value refused and a good one kept across a reload; on loopback, `settings.json` back to identical and no service restart
- `B-M-1` closed 2026-11-23 · the status page's MCP section sends the `key` claim of LiteLLM's sign-in cookie, a JWT LiteLLM refuses as a Bearer (0.3.0~ynh2, upgraded 2026-11-23): with a real `/ui/` sign-in it lists, adds and removes servers, where 0.3.0~ynh1 said "session has expired"
- `F-P-9` done 2026-11-22 · the wake settings and `keep_alive` moved from the panel to the status page, kept by the daemon in `/var/lib/whoopy/settings.json` and applied live (0.3.0~ynh1, released as `whoopy-0.3.0`, upgraded 2026-11-22): the upgrade carried the panel's values (on, MAC, 120 s) and dropped them; the panel holds five fields and the master key button; `tests/wake-playtest.sh` passed T-P-4 (page wake in 10 s), T-W-1 (chat 200 in 27 s, one resume, `keep_alive` 3m from the page: the model expired 180 s later) and T-W-2 (503 in 21 s, then 0 s) with settings changed through the page and no restart; `/mcp/whoopy`, `web`, `docs` list 3, 2, 7 tools after it
- `T-M-10` done `playtest` 2026-11-23 at `6a47334` · 0.3.0~ynh2, `lan_ranges` the defaults plus the IPv6 prefix: from the server through the domain over its LAN IPv4, LAN IPv6 and its VPN addresses (10.10.10.0, fd42::, as nginx logged them; no real VPN client), `/mcp/web` and `/mcp/whoopy` list their tools and `/v1/models` 200; from the off-site backup server with the master key, `/mcp/web`, `/v1/mcp/server`, `/ui/mcp/` 403 and `/v1/models` 200
- `F-M-12` done 2026-11-22 · the package names no side app: web search, documents and `embeddinggemma` left the panel, the settings and `config.yaml` (0.2.0~ynh5, upgraded 2026-11-22); re-created in LiteLLM's database, `docs` with its old id: `/mcp/web` 401 without a token and searched with an ordinary key, `/mcp/docs` refused an ordinary key and listed its seven tools to a granted one, `embeddinggemma` 768 dimensions in 0.2 s with the desktop asleep, `/mcp/whoopy` lists its three tools; no token in YunoHost's logs. T-P-3 reopened
- `F-M-13` done 2026-11-22 · the status page lists, adds and removes MCP servers with the admin page's sign-in cookie (0.2.0~ynh5); checked offline in a browser against the live listing, T-M-11 open for the portal run
- `F-M-11` done 2026-11-22 · the gateway docs name every tool `<server>-<tool>`, as LiteLLM lists them (separator `-`); `/mcp/docs` lists its seven tools, write ones included, and links each side app's own `doc/mcp.md`
- `F-P-8` done 2026-11-22 · the master key button is `new_master_key`, like the side apps' `new_token`; also run as `yunohost app action run whoopy main.master_key.new_master_key`
- `F-M-9` done 2026-11-22 · the MCP routes (`/mcp`, `/mcp-rest`, `/v1/mcp`, `/<server>/mcp`, `/toolset/*/mcp`, `/public/mcp_hub`, `/api/v1/inspect/mcp`, and `/ui/mcp/`) answer only the panel's `lan_ranges`, 403 elsewhere; checked on a throwaway nginx against the live LiteLLM (0.2.0~ynh4, not yet installed: T-M-10)
- `B-P-4` closed 2026-11-21 · the token defaults and generated secrets run untraced, and so does the rotate button (0.2.0~ynh3, upgraded 2026-11-21): the new MCP token and the new master key have 0 lines in `yunohost-cli.log`; old master key 401, new 200, `/mcp/whoopy` lists its tools. 93 traced lines purged from `yunohost-cli.log`, `.1` and `.2.gz`. Salt key kept (nothing stored encrypted), search token not rotated
- `F-P-1` done 2026-11-21 · the hook empties LiteLLM's static Ollama list, so a failed `/api/tags` lists nothing instead of `ollama/llama2` (0.2.0~ynh2); checked against the installed LiteLLM with a dead address, not yet on the server. A listing is cached 5 min, so models appear up to 5 min after a wake
- `F-P-7` done 2026-11-21 · Whoopy's status page at `/`, served by the wake daemon: state, last wake, a Wake button (needs `X-Whoopy: 1`), tabs to `/ui/` and `/docs`, where `DOCS_URL` moved Swagger (0.2.0~ynh1, released as `whoopy-0.2.0`, upgraded 2026-11-20); T-P-5 passed
- `T-W-1` done `playtest` 2026-11-20 at `d19ad98` · desktop asleep, a chat request through the domain answered 200 in 16 s; froaky's events showed one resume; it suspended again 123 s later (idle lowered to 2 min, model unloaded). The daemon logs failed wakes only, so the evidence is froaky's events, not `journalctl -u whoopy-wake`
- `T-W-2` done `playtest` 2026-11-20 at `d19ad98` · made-up MAC and `wake_timeout_s` 20: a 503 after 21 s, "Ollama did not answer within 20 s"; a second request at once, a 503 in 0 s, "a wake failed less than 15 s ago"; MAC and 120 s restored
- `T-P-4` done `playtest` 2026-11-20 at `d19ad98` · status line asleep, Wake now "Ollama answers, 10 s after the wake request" in 13 s, then awake with "Last wake: answered after 10 s"; the daemon-down reading passed at `d9779d4`
- `F-P-6` done 2026-11-20 · the wake, the panel's Wake button and an MCP server (`wake`, `status`, `models` at `/mcp/whoopy`) in one Rust daemon, `whoopy-wake.service`, the hook its client (0.1.0~ynh1, released as `whoopy-0.1.0`); T-W-1, T-W-2, T-P-3, T-P-4, T-M-8 passed; `tests/wake-playtest.sh` reruns the wake tests
- `T-M-8` done `playtest` 2026-11-20 at `d9779d4` · `/mcp/whoopy`: 401 without a token and no portal redirect; with the master key `whoopy-wake`, `whoopy-status`, `whoopy-models` listed and each answered in under 0.2 s; the daemon's loopback port 401 without its token
- `F-P-4` done 2026-11-19 · the redrawn icon set from ynh-app-icons 0ca6205: `conf/logo.png`, the six favicons in `conf/icons/` and `doc/logo.svg` replaced, `logo_update/` deleted (~ynh13)
- `F-P-3` done 2026-11-19 · the desktop's embedding models as `ollama-embed/<name>`, LiteLLM's `ollama` provider at Ollama's address (~ynh12, upgraded 2026-11-19): `ollama/<name>` answered 400 to an embedding; through the new route 768 dimensions in 4 s, and cheruby-docs-mcp indexed 333 files in 3 min 47 s with it
- `F-M-8` done 2026-11-10 · the side apps are reached by an address and a password-type token pasted in the panel, never by reading their settings (~ynh9); documents are one `docs` server at `/mcp/docs` whose id stays that of `docs_<user>`, so grants carried over; Rikka Hub reached it with its ordinary key once `_<user>` was dropped from its URL; `/mcp/web` and `embeddinggemma` answer
- `B-P-3` closed 2026-11-10 · a webadmin Save no longer sets the admin password to `None`: each button in a section of its own (~ynh11, upgraded 2026-11-10; login and a password change from the panel both work)
- `B-P-2` closed 2026-11-10 · `whoopy.env` checksum dropped before each render since ~ynh7; the user's docs token rotated, the 2026-11-09 upgrade log and appconfbackup copy deleted; the old token gets 401 at the docs app, the new one passes through `/mcp/docs_<user>` (master key)
- `B-P-1` closed 2026-11-10 · the `/ollama` pass-through is dropped, with its permission URL and its wake path: only the master key could use it, the grant being Enterprise-only (~ynh8, upgraded 2026-11-10; `/ollama` gone from the portal's URL list)
- `F-M-7` done 2026-11-10 · `/mcp/web` served by the cheruby-search-mcp app on this server, its address and token read from its settings (~ynh7); a search through the domain answered and woke nothing
- `F-P-2` done 2026-11-09 · `embeddinggemma`, the cheruby-docs-mcp app's EmbeddingGemma 300M on loopback, offered to every key when the documents are on (~ynh6); 768 dimensions through Whoopy, the Ollama machine stayed asleep
- `F-M-6` done 2026-11-09 · each user of `cheruby-docs-mcp` has a `docs_<user>` MCP server, read from that app's settings and switched on in the panel; only keys granted it by `object_permission.mcp_servers` reach it, which needs no Enterprise licence in 1.106.6; tested through the domain with a granted, an ordinary and no key
- `F-M-3` done 2026-11-07 · models search the web and read pages through `/mcp/web`, served by the `ai-websearch-mcp` package on the desktop; a tool call wakes it, a listing does not
- `T-M-7` done `playtest` 2026-11-07 at `6b4f206` · through `/mcp/web`: 401 without a token, no portal redirect; Open WebUI with an ordinary key fetched pages; desktop asleep: listing empty without waking, a call woke it and answered in 11 s; after a restart while asleep, "Tool search not found" without waking, then fine once listed awake
- `T-M-6` done `playtest` 2026-11-07 at ai-websearch-mcp `12c535d` · installed by `paru -Ui` (searxng-git built from the AUR); both services up; from the Pi, real results, a public page read under the block, LAN and loopback refused, 401 without the token
- `T-M-5` done `spec` 2026-11-07 at ai-websearch-mcp `d89aa79` · `python tests/check_server.py` on the desktop: all 8 checks pass
- `T-P-2` done 2026-11-04 at `0c91f6a` · no download in the journal at start-up; a unit with the same outbound rule could not reach pypi.org but reached loopback
- `T-P-1` done 2026-11-04 at `0c91f6a` · first start applied 213 migrations in about 10 s; `/v1/models` 401 without a token, 200 with; `/key/list` and `/v1/mcp/server` sent to the portal even with a token
