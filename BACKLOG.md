# BACKLOG

A first install on the server, proven end to end: a token reaches a model on
the sleeping desktop through its domain, and nothing is downloaded at
runtime.

## Categories

| Letter | Covers |
|---|---|
| `W` | Waking the AI server |
| `P` | The packaging, the lifecycle scripts and the config panel |
| `M` | The MCP tool servers Whoopy publishes |

## Bugs

None open.

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

### T-W-1 · a request wakes the desktop

`todo` `playtest` `reopened 2026-11-27`

**Do:** `tests/wake-playtest.sh <ai server> ollama/<chat model>`, froaky's
idle time lowered first. **Pass:** chat 200 from asleep, one resume, asleep
again after its idle time.
**Last:** 2026-11-26 at `42253e3`, 0.4.1~ynh2: chat 200 in 27 s, 1 resume,
asleep after 113 s; `speaches/*` transcription 200 in 16 s. Reopened by the
0.5.0 binary (`a4ca6df`), whose wake path did not change.

### T-W-2 · a failed wake

`todo` `playtest` `reopened 2026-11-27`

**Do:** a made-up MAC and a 20 s timeout, then two requests. **Pass:** a 503
after about 21 s, then one in 0 s with "a wake failed less than 15 s ago"; the
settings restored. `whoopy settings` can now set and restore them.
**Last:** 2026-11-26 at `42253e3`, 0.4.1~ynh2: 503 in 22 s, then 0 s; the
open servers kept. Reopened by the 0.5.0 binary.

### T-P-4 · the page's wake

`todo` `playtest` `reopened 2026-11-27`

**Do:** with the desktop asleep, `whoopy status`, `POST /whoopy/wake`, status
again. **Pass:** asleep, `ok` once Ollama answers, then awake with the last
wake. **Last:** 2026-11-26 at `42253e3`, 0.4.1~ynh2: `ok` in 11 s, asleep
after 123 s. Reopened by the 0.5.0 binary.

### T-P-5 · the status page through the portal

`todo` `playtest` `reopened 2026-11-24`

**Do:** through the portal as an admin: the page, a wake, a refused and a
good Save on the Config tab, the LiteLLM links.
**Pass:** the real state in the chip and the panel, "Awake after N s", the
daemon's reason on a refused Save, a good one kept after a reload.
**Last:** 2026-11-26 at `42253e3`, 0.4.1~ynh2, on loopback in a browser,
from asleep: chip `asleep`, "The AI server is asleep"; Wake gave "Awake after
10 s.", chip `awake`, last wake "answered after 10 s"; a 5 s timeout refused
with "The wake timeout is 10 to 540 seconds.", a Save of the same values
"Saved. The next wake uses them." and 120 kept after a reload. Only console
errors: the mascot, font and favicons nginx serves, and the refused Save's
400. Logged out through the domain: `/`, `/whoopy/status`, `/ui/`,
`/key/list` 302 to the portal. Still to run: the portal as an admin.

### T-M-11 · the page's MCP section

`todo` `playtest` `reopened 2026-11-23`

**Do:** through the portal as an admin, on the MCP Servers tab, signed out
of `/ui/` then signed in; add a test server, use it, remove it.
**Pass:** signed out, a sign-in message and no form; signed in, the servers
listed, `whoopy` marked from the package; the test server answers at
`/mcp/<name>` with a key, and is gone once removed.
**Last:** 2026-11-26 at `42253e3`, 0.4.1~ynh2, on loopback with a real
sign-in, listing only (no server added or removed): signed out, "Sign in to
the admin page first" and no form; signed in, `docs`, `flambyno`, `web` open
to the internet with Remove, `whoopy` "from the package, every key (config
panel)" with only its internet box, the Add form shown. Only console errors:
the assets nginx serves. Still to run: the portal as an admin, and an add
and remove (last 2026-11-24 at `a292199`, passed).

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

- `B-P-7` closed 2026-11-28 · the docs and the hook's comment name the Speaches route `froaky-speaches/*`, and `doc/api.md` says why the prefix must not begin an owner name: no file names `speaches/` as the route any more
- `B-M-2` closed 2026-11-27 · the wake hook refuses with 403 a model request whose `tools` name an MCP server of the gateway, by LiteLLM's own `should_use_litellm_mcp_gateway`, whatever the origin (0.5.0~ynh2, upgraded 2026-11-27). Before: `/v1/responses` with a `litellm_proxy/mcp/docs` tool ran `docs-list_folder` and handed the model `docs`' write tools. After, master key on loopback: `/v1/responses`, `/v1/chat/completions` and `/v1/messages`, each with `litellm_proxy/mcp/docs` and `https://<domain>/mcp/docs`, 403 in under 1 s, background responses too, and through the domain from the LAN; the same routes without MCP tools 200. Not tried from outside the LAN: the refusal does not depend on the origin
- `F-M-17` done 2026-11-27 · `whoopy mcp list`, `add` and `update`, as root, the master key from `whoopy.env`, the token from a pipe, never a terminal (0.5.0~ynh1, released as `whoopy-0.5.0`, upgraded 2026-11-27): a `test` server added at `whoopy`'s own port answered at `/mcp/test` with 4 tools to an ungranted key; after `update --granted` that key was refused ("not allowed to access the requested MCP servers: test") and a granted key listed them; after `update --token-stdin` the granted key still did; an empty `update` and one on the package's `whoopy` refused; the server and both keys deleted
- `F-W-2` done 2026-11-27 · `whoopy settings` shows and changes the wake settings through the socket's `POST /settings`, root only by the peer's uid; `whoopy-settings` reads them over MCP (0.5.0~ynh1): as user `whoopy` 403 "Only root may change the settings here.", as root the timeout changed and put back, a 5 s timeout refused with the daemon's reason, the open servers kept; `whoopy-settings` answered through LiteLLM
- `T-M-8` done `playtest` 2026-11-27 · 0.5.0~ynh2, on loopback: `/mcp/whoopy` 401 without a token, and with the master key lists `whoopy-models`, `whoopy-settings`, `whoopy-status`, `whoopy-wake`, and `whoopy-status` answers. Before, at `f2d7dac` (0.5.0~ynh1): the daemon's MCP port 401 without its token; `/mcp/whoopy` with the master key lists `whoopy-models`, `whoopy-settings`, `whoopy-status`, `whoopy-wake`, and `whoopy-settings` answers. Not rerun through the domain
- `F-M-16` done 2026-11-26 · the status page's Token button gives an MCP server a new token in place (`PUT /v1/mcp/server`, same `server_id`), so the keys granted it keep their grant (0.4.1~ynh5, upgraded 2026-11-26): through the portal the button took `web`'s current token, then a new one from `token --new`, and a granted key still searched through `/mcp/web`; against a stub in a browser at 390 and 1366 wide, the form sends `server_id`, `bearer_token` and the token, none for the package's server; `check_page.py` no fault at six sizes, signed out
- `B-P-6` closed 2026-11-26 · LiteLLM's user reaches on loopback only DNS, PostgreSQL, Prisma's query engine (pinned to `port_engine`), `port_mcp` and the panel's `loopback_ports`, by an nftables table on its uid (0.4.1~ynh4, at `02ab67e`): as user `whoopy` 5432, 8790, 8770 and 8792 open, 389, 6379, 8384, 443 and `::1` refused in 2 ms; `docs`, `web`, `flambyno` and `whoopy` list their tools, `embeddinggemma` on 8771 embeds. 0.4.1~ynh3 without the pin crash-looped LiteLLM for about 10 minutes (Prisma's random port refused). The only refusals left are Prisma trying `::1` before 127.0.0.1
- `T-P-1` done `playtest` 2026-11-26 at `02ab67e` · 0.4.1~ynh4 with the loopback table: through the domain `/v1/models` 401 without a token, 200 with; `/key/list` and `/v1/mcp/server` 302 to the portal even with a token. Migrations not rerun (an upgrade, not a first install)
- `T-P-2` done 2026-11-26 at `02ab67e` · 0.4.1~ynh4: no download in the journal since start-up (the only URLs are LiteLLM's banner); the outbound rule unchanged since 2026-11-04, the pypi.org probe not rerun
- `F-W-1` done 2026-11-26 · the hook wakes the AI server for any model on its host, not only Ollama's; the `speaches/*` database model on froaky's model routes transcribed through the domain (200, segments and language), the test from asleep is T-W-1
- `F-P-10` done 2026-11-24 · the status page in the pokestyle look: Whoopy's dark palette, square pieces, the mascot header with the state as a chip (awake, waking, asleep, offline), LiteLLM links after Config with the new-page icon, the MCP servers beside the status from 75em; mascot and Fredoka served by nginx from `icons/` (0.4.0~ynh2, upgraded 2026-11-24): `check_page.py` no fault at six sizes on every tab against a stub in the up, waking and down states, offline only the stub's 500s, screenshots read; on the live daemon the same apart from the assets nginx serves; through the portal: T-P-5
- `F-M-15` done 2026-11-24 · the status page opens each MCP server to the internet with a box and switches it between every key and granted keys; the panel chooses who may use `/mcp/whoopy` (all, granted, off); the open list sits in the daemon's file, checked by nginx through `auth_request` with SSOwat's pass replaced in that location (0.4.0~ynh1, released as `whoopy-0.4.0`, upgraded 2026-11-24; `web docs` carried over): T-M-10, T-M-8, T-W-1, T-W-2, T-P-4, T-P-3 passed; the page's boxes checked in a browser (T-M-11)
- `T-M-10` done `playtest` 2026-11-24 at `a292199` · 0.4.0~ynh1, `web` and `docs` opened on the page (carried from the panel by the upgrade): from the off-site backup server through the domain, `/mcp/web` and `/mcp/docs` 401 without a key and list 2 and 7 tools with the master key; `/mcp/whoopy` 403 with it until its box was ticked (then 401 without a key, applied at once), 403 again once unticked; `/mcp`, `/mcp/sse`, `/mcp/proxy`, `/mcp/enabled`, `/mcp/web,whoopy`, `/mcp/web%2Cwhoopy`, `/mcp/web/x`, `/mcp/docs%2F..%2Fweb`, `/web/mcp`, `/mcp-rest`, `/v1/mcp/server`, `/ui/mcp/`, `/public/mcp_hub` 403, `/whoopy-mcp-public` 404; `/v1/models` 200. From the LAN over IPv4 and IPv6, every server lists its tools with a key and 401s without; `/key/list` and `/v1/mcp/server` sent to the portal
- `F-M-14` done 2026-11-23 · the panel's `mcp_public` opens chosen MCP servers to the internet at `/mcp/<name>` exactly, a key still required, the rest LAN-only; checked on a throwaway nginx against the live LiteLLM with `web docs` from outside the LAN ranges: `/mcp/web`, `/mcp/docs` 401 without a key and list their tools with one, `/mcp`, `/mcp/whoopy` (even with the master key), `/mcp/web,whoopy`, `/mcp/web%2Cwhoopy`, `/mcp/web/x`, `/mcp/docs%2F..%2Fweb`, `/mcp/sse`, `/web/mcp`, `/v1/mcp/server`, `/ui/mcp/`, `/mcp-rest`, `/public/mcp_hub` 403 (0.3.0~ynh5, not yet installed: T-M-10)
- `B-M-1` closed 2026-11-23 · the status page's MCP section sends the `key` claim of LiteLLM's sign-in cookie, a JWT LiteLLM refuses as a Bearer (0.3.0~ynh2, upgraded 2026-11-23): with a real `/ui/` sign-in it lists, adds and removes servers, where 0.3.0~ynh1 said "session has expired"
- `F-P-9` done 2026-11-22 · the wake settings and `keep_alive` moved from the panel to the status page, kept by the daemon in `/var/lib/whoopy/settings.json` and applied live (0.3.0~ynh1, released as `whoopy-0.3.0`, upgraded 2026-11-22): the upgrade carried the panel's values (on, MAC, 120 s) and dropped them; the panel holds five fields and the master key button; `tests/wake-playtest.sh` passed T-P-4 (page wake in 10 s), T-W-1 (chat 200 in 27 s, one resume, `keep_alive` 3m from the page: the model expired 180 s later) and T-W-2 (503 in 21 s, then 0 s) with settings changed through the page and no restart; `/mcp/whoopy`, `web`, `docs` list 3, 2, 7 tools after it
- `T-P-3` done `playtest` 2026-11-24 at `a292199` · 0.4.0~ynh1, backup, remove, restore: 147 s down; the 26 settings identical but the order of two permission URL lists, `whoopy.env`, `wake.env` and `settings.json` identical; both services up, `docs`, `web`, `whoopy` with their ids and key modes; from outside `/mcp/web` 2 tools, `/mcp/docs` 7, `/mcp/whoopy` 403; from the LAN 2, 7, 3; `/v1/models` 200; embeddings 768 dimensions
- `B-P-5` closed 2026-11-23 · a restore or reinstall after a remove gives the left data folder to the new user (0.3.0~ynh3): with its 1,287 files set to a foreign id, an upgrade handed them back and LiteLLM started with no permission error. On 0.3.0~ynh2 a restore failed and rolled back on them
- `F-M-12` done 2026-11-22 · the package names no side app: web search, documents and `embeddinggemma` left the panel, the settings and `config.yaml` (0.2.0~ynh5, upgraded 2026-11-22); re-created in LiteLLM's database, `docs` with its old id: `/mcp/web` 401 without a token and searched with an ordinary key, `/mcp/docs` refused an ordinary key and listed its seven tools to a granted one, `embeddinggemma` 768 dimensions in 0.2 s with the desktop asleep, `/mcp/whoopy` lists its three tools; no token in YunoHost's logs. T-P-3 reopened
- `F-M-13` done 2026-11-22 · the status page lists, adds and removes MCP servers with the admin page's sign-in cookie (0.2.0~ynh5); checked offline in a browser against the live listing, T-M-11 open for the portal run
- `F-M-11` done 2026-11-22 · the gateway docs name every tool `<server>-<tool>`, as LiteLLM lists them (separator `-`); `/mcp/docs` lists its seven tools, write ones included, and links each side app's own `doc/mcp.md`
- `F-P-8` done 2026-11-22 · the master key button is `new_master_key`, like the side apps' `new_token`; also run as `yunohost app action run whoopy main.master_key.new_master_key`
- `F-M-9` done 2026-11-22 · the MCP routes (`/mcp`, `/mcp-rest`, `/v1/mcp`, `/<server>/mcp`, `/toolset/*/mcp`, `/public/mcp_hub`, `/api/v1/inspect/mcp`, and `/ui/mcp/`) answer only the panel's `lan_ranges`, 403 elsewhere; checked on a throwaway nginx against the live LiteLLM (0.2.0~ynh4, not yet installed: T-M-10)
- `B-P-4` closed 2026-11-21 · the token defaults and generated secrets run untraced, and so does the rotate button (0.2.0~ynh3, upgraded 2026-11-21): the new MCP token and the new master key have 0 lines in `yunohost-cli.log`; old master key 401, new 200, `/mcp/whoopy` lists its tools. 93 traced lines purged from `yunohost-cli.log`, `.1` and `.2.gz`. Salt key kept (nothing stored encrypted), search token not rotated
- `F-P-1` done 2026-11-21 · the hook empties LiteLLM's static Ollama list, so a failed `/api/tags` lists nothing instead of `ollama/llama2` (0.2.0~ynh2); checked against the installed LiteLLM with a dead address, not yet on the server. A listing is cached 5 min, so models appear up to 5 min after a wake
- `F-P-7` done 2026-11-21 · Whoopy's status page at `/`, served by the wake daemon: state, last wake, a Wake button (needs `X-Whoopy: 1`), tabs to `/ui/` and `/docs`, where `DOCS_URL` moved Swagger (0.2.0~ynh1, released as `whoopy-0.2.0`, upgraded 2026-11-20); T-P-5 passed
- `F-P-6` done 2026-11-20 · the wake, the panel's Wake button and an MCP server (`wake`, `status`, `models` at `/mcp/whoopy`) in one Rust daemon, `whoopy-wake.service`, the hook its client (0.1.0~ynh1, released as `whoopy-0.1.0`); T-W-1, T-W-2, T-P-3, T-P-4, T-M-8 passed; `tests/wake-playtest.sh` reruns the wake tests
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
