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
  `whoopy.env`.
- `conf/whoopy_wake.py`: the hook LiteLLM loads; asks the wake daemon.
- `whoopy/`: the wake daemon and its CLI, in Rust. `conf/wake.service`
  and `conf/wake.env` (root 600, wake settings and MCP token only) run it.
- `conf/index.html`: the status page at `/`, served by the wake daemon.
- `conf/logo.png`: the portal tile and LiteLLM's logo. `conf/icons/`: browser
  favicons, served by nginx.
- `doc/`: the user documentation, and YunoHost's `DESCRIPTION.md`.
  `CHANGELOG.md`: a section per release.

## Architecture

- **Two services**, both user `whoopy`. `whoopy.service`: LiteLLM, one
  uvicorn worker on loopback; it applies pending database migrations at
  each start. `whoopy-wake.service`: `whoopy serve`, started first: the
  wake, on the socket `/run/whoopy/wake.sock`; an MCP server on loopback
  (`port_mcp`), registered in LiteLLM as `whoopy`; the status page at `/`
  (`port_web`).
- **The wake hook** (`conf/whoopy_wake.py`) runs inside LiteLLM and asks the
  daemon over the socket. The panel's Wake button and status line call the
  same binary (`whoopy wake`, `whoopy status`).
- **Code** in `/var/www/whoopy`, root-owned, group `whoopy` read-only: venv,
  `prisma/` (CLI, query engine, schema engine), `config/` (config.yaml and the
  hook, which LiteLLM loads from the config's directory).
- **State** in PostgreSQL. `/home/yunohost.app/whoopy` is the service's HOME
  and holds only what LiteLLM copies there (migrations, admin page files).
- **Three permissions**: `main` (`/`, portal), `api` (the API paths,
  visitors), `admin_api` (admins, the management routes under them).
- **The MCP routes are LAN-only** in `conf/nginx.conf`: one regex location
  with an `allow` per entry of the `lan_ranges` setting, then `deny all`.
- **The binary is downloaded, never compiled on install**, from a release of
  this repository.

The rules behind all of this, and the T check recipes, are in the `whoopy-kb`
skill.

## Commands

Before committing:

```bash
for f in scripts/*; do bash -n "$f"; done
python3 -c 'import ast,sys; ast.parse(open(sys.argv[1]).read())' conf/whoopy_wake.py
(cd whoopy && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cargo audit)
```

A new binary, the wake playtests and the advisory check: see `whoopy-kb`.
The toolchain is rustup's, pinned by `whoopy/rust-toolchain.toml`.

Set config panel values from the command line one key at a time:
`yunohost app config set whoopy main.wol.wol_mac -v <value>`.

## Packaging

The version is the `whoopy/` crate's, then `~ynhN` for packaging changes.
A `~ynhN` cited before 2026-11-20 belongs to the older LiteLLM-numbered series.

The whole repository ships except `CLAUDE.md`, `BACKLOG.md` and `whoopy/target/`.
Backup holds the database dump and the nginx and systemd files; the secrets
travel in the app settings. Restore rebuilds the code and needs the network.
