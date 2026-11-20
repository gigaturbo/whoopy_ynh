# CLAUDE.md

A YunoHost package that wraps the LiteLLM proxy as an AI gateway in front of
an Ollama machine that sleeps, and wakes it by Wake-on-LAN on demand.

The host's own `CLAUDE.md` still applies, and its rules win: installing or
upgrading this app touches a running service and a database, so ask first.

## Layout

- `manifest.toml`, `scripts/`: packaging format 2, helpers 2.1. Shared steps in
  `scripts/_common.sh`. `config_panel.toml` and `scripts/config` hold every
  setting that changes after install, and the two buttons.
- `conf/requirements.in` → `conf/requirements.txt`: the hash-locked wheels.
  The re-lock command is at the top of `requirements.in`.
- `conf/config.yaml` (jinja): LiteLLM's config. No secret in it.
- `conf/app.env`: secrets and wake settings, rendered root 600 as
  `whoopy.env`. Never `.env`: LiteLLM loads any `.env` above its code at
  import, and crashes on one it may not read.
- `conf/whoopy_wake.py`: the hook LiteLLM loads; asks the wake daemon.
- `whoopy/`: the wake daemon and its CLI, in Rust. `conf/wake.service`
  and `conf/wake.env` (root 600, wake settings and MCP token only) run it.
- `conf/logo.png`: the portal tile and LiteLLM's logo. `conf/icons/`: browser
  favicons, served by nginx. `doc/logo.svg`: the README.

## Architecture

- **Two services**, both user `whoopy`. `whoopy.service`: LiteLLM, one
  uvicorn worker on loopback; it applies pending database migrations at
  each start. `whoopy-wake.service`: `whoopy serve`, started first, with
  the socket `/run/whoopy/wake.sock` and an MCP server on loopback
  (`port_mcp`), registered in LiteLLM as `whoopy`, at `/mcp/whoopy`. It
  never reaches the internet, whatever the outbound setting.
- **Code** in `/var/www/whoopy`, root-owned, group `whoopy` read-only: venv,
  `prisma/` (CLI, query engine, schema engine), `config/` (config.yaml and the
  hook, which LiteLLM loads from the config's directory).
- **State** in PostgreSQL. `/home/yunohost.app/whoopy` is the service's HOME
  and holds only what LiteLLM copies there (migrations, admin page files).
- **Three permissions.** `main` (`/`, portal) and `api` (`/v1`, `/chat/completions`,
  `/completions`, `/embeddings`, `/models`, `/mcp`, visitors).
  A third, `admin_api` (admins), takes back the management routes those
  prefixes would expose: `/v1/mcp`, `/mcp-rest`, `/v1/tool`, `/v1/traces`,
  `/v1/decisions`. `auth_header` is off on all three: the portal would
  overwrite the Bearer token.
- **The MCP side apps** (`cheruby-search-mcp`, `cheruby-docs-mcp`) are
  reached by an address and a token pasted in the panel. Whoopy never reads
  their settings: restore must not depend on them, and the token is the only
  contract. The upgrade to ~ynh9 read them once, to carry the tokens over.
- **The wake hook** has two entry points: `async_pre_call_check` sees the
  deployment the router chose and wakes only for Ollama's api_base;
  `async_pre_call_hook` sees MCP tool calls and wakes only for a server in
  `WAKE_MCP_SERVERS`. Both `POST /ensure` on the socket. The daemon holds
  the one shared wake and its timers; the panel's Wake button and status
  line call the same binary (`whoopy wake`, `whoopy status`).
- **The binary is downloaded, never compiled on install**: a release asset
  of this repository, url and sha256 in `resources.sources.whoopy`. It is
  installed at each upgrade, with both services stopped.

## Commands

Before committing:

```bash
for f in scripts/*; do bash -n "$f"; done
python3 -c 'import ast,sys; ast.parse(open(sys.argv[1]).read())' conf/whoopy_wake.py
(cd whoopy && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cargo audit)
```

A new binary: bump `version` in `whoopy/Cargo.toml`, `cargo build --release`,
publish `target/release/whoopy` as the asset `whoopy-aarch64-linux` of a
release `whoopy-<version>`, then put its url and `sha256sum` in the
manifest. The toolchain is rustup's, pinned by `whoopy/rust-toolchain.toml`.

The wake playtests (T-P-4, T-W-1, T-W-2) run on the server against the
installed app: `tests/wake-playtest.sh <ssh alias of the Ollama machine> ollama/<chat model>`.
Lower froaky's idle time there first (`froaky settings --idle 2m`), and put
it back after (30 min).

Set config panel values from the command line one key at a time:
`yunohost app config set whoopy main.wol.wol_mac -v <value>`.

A button sits alone in its section. Save skips any section holding one, and
its fields reach the script empty: a `bind = "null"` one as the word `None`.
That stored `None` as the admin password at each webadmin Save, until ~ynh11.

On the server, the advisory check covers the lock file:
`~/Documentation/bin/check-whoopy-advisories.sh conf/requirements.txt`.

## Packaging

The version is the `whoopy/` crate's, then `~ynhN` for packaging changes:
`0.1.0~ynh1` onwards. Up to `1.106.6~ynh14` it was LiteLLM's, so a `~ynhN`
cited in BACKLOG or here before 2026-11-20 belongs to that series.

The whole repository ships except `CLAUDE.md`, `BACKLOG.md`, `TODO.md` and
`whoopy/target/`.
Backup holds the database dump and the nginx and systemd files; the secrets
travel in the app settings. Restore rebuilds the code and needs the network.
