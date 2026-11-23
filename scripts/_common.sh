#!/bin/bash

# Shared steps between install, upgrade, restore and the config panel.

# Settings with no install question. Defaults only: an upgrade keeps whatever
# the config panel has since set.
whoopy_default_settings() {
    ynh_app_setting_set_default --key=outbound --value="lan"
    # The shared admin/password login. On at install: no other admin exists yet.
    ynh_app_setting_set_default --key=env_login --value=1
    # Who may reach the MCP routes: private networks only by default. The
    # server's own public IPv6 prefix is added in the panel.
    ynh_app_setting_set_default --key=lan_ranges --value="127.0.0.0/8 ::1 10.0.0.0/8 172.16.0.0/12 192.168.0.0/16 fc00::/7"
    # The tokens and secrets below are untraced: the trace prints each value,
    # in the -z checks and in set_default's own check, and YunoHost redacts
    # the operation log, not yunohost-cli.log.
    set +o xtrace

    # Generated once. The salt key must never change: it encrypts the
    # provider credentials stored in the database.
    if [ -z "${master_key:-}" ]; then
        master_key="sk-$(ynh_string_random --length=48)"
        ynh_app_setting_set --key=master_key --value="$master_key"
    fi
    if [ -z "${salt_key:-}" ]; then
        salt_key=$(ynh_string_random --length=48)
        ynh_app_setting_set --key=salt_key --value="$salt_key"
    fi
    # Between LiteLLM and the wake daemon's MCP server, which refuses one
    # under 32 characters.
    if [ -z "${mcp_token:-}" ]; then
        mcp_token=$(ynh_string_random --length=48)
        ynh_app_setting_set --key=mcp_token --value="$mcp_token"
    fi
    set -o xtrace
}

# uv is told not to look for configuration files: Yunohost runs scripts from a
# directory under /var/cache/yunohost, and uv would walk up from it.
whoopy_uv() {
    env HOME="$build_home" UV_CACHE_DIR="$build_home/uv" UV_NO_CONFIG=1 \
        "$install_dir/.uv/uv" "$@"
}

# Code is root-owned and read by the app user through its group: a stolen
# process cannot rewrite what it runs. It is rebuilt rather than backed up.
whoopy_install_code() {
    ynh_safe_rm "$install_dir/.uv"
    ynh_setup_source --dest_dir="$install_dir/.uv" --source_id="uv"
    cp "$YNH_APP_BASEDIR/conf/requirements.txt" "$install_dir/requirements.txt"
}

# The wake daemon, a checked download. Only with both services stopped: the
# running binary cannot be overwritten.
whoopy_install_binary() {
    mkdir -p "$install_dir/bin"
    ynh_setup_source --dest_dir="$install_dir/bin" --source_id="whoopy"
    chmod 750 "$install_dir/bin/whoopy"
}

# The venv from the hash-locked wheels, then Prisma's client and engines.
# Everything is fetched here, as root, into a throwaway HOME; the running
# service downloads nothing.
whoopy_build_venv() {
    build_home=$(mktemp -d)
    ynh_safe_rm "$install_dir/venv"
    ynh_safe_rm "$install_dir/prisma"

    pushd "$install_dir"
        ynh_hide_warnings whoopy_uv venv --python /usr/bin/python3 "$install_dir/venv"
        ynh_hide_warnings whoopy_uv pip install --python "$install_dir/venv/bin/python" \
            --require-hashes --no-build --requirement "$install_dir/requirements.txt"

        # The generator is a console script of the venv; Node runs the CLI.
        local schema
        schema=$("$install_dir/venv/bin/python" -c 'import litellm, os; print(os.path.dirname(litellm.__file__))')/proxy/schema.prisma
        ynh_hide_warnings env HOME="$build_home" PATH="$install_dir/venv/bin:$path_with_nodejs" \
            PRISMA_BINARY_CACHE_DIR="$install_dir/prisma" CHECKPOINT_DISABLE=1 PRISMA_HIDE_UPDATE_MESSAGE=1 \
            "$install_dir/venv/bin/python" -m prisma generate --schema "$schema"
    popd

    # The generated client records absolute engine paths. Refuse a build
    # whose engine would have to come from anywhere else at runtime.
    "$install_dir/venv/bin/python" -c '
import sys, prisma.client as c
paths = c.BINARY_PATHS.query_engine.values()
sys.exit(0 if paths and all(p.startswith(sys.argv[1]) for p in paths) else 1)' "$install_dir/prisma/" \
        || ynh_die "Prisma's query engine was not built under $install_dir/prisma."

    # The schema engine runs the migrations. The service names it, so the
    # CLI never looks for one in a cache.
    [ -x "$install_dir/prisma/node_modules/@prisma/engines/schema-engine-linux-arm64-openssl-3.0.x" ] \
        || ynh_die "Prisma's schema engine is missing from $install_dir/prisma."

    ynh_safe_rm "$build_home"
    ynh_app_setting_set --key=build_fingerprint --value="$(whoopy_build_fingerprint)"
}

# What a build depends on: the lock file and uv's download. An upgrade that
# changes neither keeps the venv, and needs no network.
whoopy_build_fingerprint() {
    { cat "$YNH_APP_BASEDIR/conf/requirements.txt"; ynh_read_manifest "resources.sources.uv.arm64.url"; } \
        | sha256sum | cut -d' ' -f1
}

# Before an upgrade stops anything: a failed download mid-upgrade makes
# Yunohost remove the app, database included, then restore from the network.
whoopy_check_downloads() {
    local url
    for url in https://pypi.org/simple/ https://files.pythonhosted.org/ https://github.com/ \
        https://registry.npmjs.org/ https://binaries.prisma.sh/; do
        curl --silent --show-error --output /dev/null --max-time 15 "$url" \
            || ynh_die "$url cannot be reached; nothing was changed. Retry the upgrade later."
    done
}

# Settings that change from the config panel: secrets, LiteLLM's config,
# the unit (outbound rule) and nginx.
whoopy_configure() {
    # The trace would print the password, quoted in a way Yunohost's log
    # redaction does not recognise. systemd reads whoopy.env with escapes, so the
    # value is written double-quoted with \ and " escaped.
    set +o xtrace
    [ -n "${ui_password:-}" ] || ynh_die "No admin password for LiteLLM's page."
    ui_password_env=${ui_password//\\/\\\\}
    ui_password_env=${ui_password_env//\"/\\\"}
    set -o xtrace

    # Not named .env: LiteLLM loads any .env above its code at import, and
    # crashes on one it may not read.
    # Its checksum is dropped first: a file changed since its last render is
    # copied to /var/cache/yunohost/appconfbackup and diffed, tokens included,
    # into the operation log.
    ynh_delete_file_checksum "$install_dir/whoopy.env"
    ynh_config_add --template="app.env" --destination="$install_dir/whoopy.env"
    chown root:root "$install_dir/whoopy.env"
    chmod 600 "$install_dir/whoopy.env"

    # The wake daemon's own: wake settings and the MCP token, nothing else.
    ynh_delete_file_checksum "$install_dir/wake.env"
    ynh_config_add --template="wake.env" --destination="$install_dir/wake.env"


    # LiteLLM loads the hook from config.yaml's directory, so both are
    # root-owned: the app user must not be able to change code it runs.
    mkdir -p "$install_dir/config"
    ynh_config_add --jinja --template="config.yaml" --destination="$install_dir/config/config.yaml"
    install -m 644 "$YNH_APP_BASEDIR/conf/whoopy_wake.py" "$install_dir/config/whoopy_wake.py"

    # The browser icons, and the full badge LiteLLM shows as its logo.
    mkdir -p "$install_dir/icons"
    cp "$YNH_APP_BASEDIR"/conf/icons/* "$install_dir/icons/"
    cp "$YNH_APP_BASEDIR/conf/logo.png" "$install_dir/icons/whoopy.png"

    # The status page, which the wake daemon reads at each request.
    mkdir -p "$install_dir/web"
    cp "$YNH_APP_BASEDIR/conf/index.html" "$install_dir/web/index.html"

    chown -R root:"$app" "$install_dir"
    chmod -R u=rwX,g=rX,o= "$install_dir"
    chmod 600 "$install_dir/whoopy.env" "$install_dir/wake.env"
    # nginx serves the icons itself: it may cross the install dir, not list it.
    chmod o+x "$install_dir" "$install_dir/icons"
    chmod o+r "$install_dir"/icons/*

    # The data dir survives a remove, with its files still owned by the old
    # user's id; a reinstall or restore gives the user a new one, and LiteLLM
    # could no longer overwrite what it copies there at each start.
    chown -R "$app:$app" "$data_dir"

    # "lan": loopback and private networks only, so a stolen process cannot
    # reach the internet. "any": cloud providers become reachable.
    if [ "$outbound" = "lan" ]; then ip_deny="any"; else ip_deny=""; fi
    ynh_config_add_systemd
    ynh_config_add_systemd --service="$app-wake" --template="wake.service"
    lan_allow=$(whoopy_lan_allow "$lan_ranges") \
        || ynh_die "The LAN ranges hold something that is not an address or a network: $lan_ranges"
    ynh_config_add_nginx

    yunohost service add "$app" --description="Whoopy, the AI gateway (LiteLLM)"
    yunohost service add "$app-wake" --description="Whoopy, wakes the Ollama machine"
}

# The nginx `allow` lines for the MCP routes, from a space-separated list of
# addresses and networks. Fails on anything else: the result lands in nginx's
# config, so a stray `;` or `}` must never get through.
whoopy_lan_allow() {
    python3 -I -c '
import ipaddress, re, sys
raw = sys.argv[1].split()
# Address characters only: a zone such as "fe80::1%x;}" parses, and would
# reach nginx with its ";" and "}".
if any(not re.fullmatch(r"[0-9A-Fa-f:./]+", r) for r in raw):
    sys.exit(1)
nets = [ipaddress.ip_network(r, strict=False) for r in raw]
if not nets:
    sys.exit(1)
print(" ".join(f"allow {n};" for n in nets))' "$1" 2>/dev/null
}

# The tile gets the package's logo only while it has none, so a logo chosen
# in the webadmin survives upgrades. The portal reads PNG only.
whoopy_set_logo() {
    yunohost tools shell -c "
from yunohost.permission import user_permission_info
from yunohost.user import user_permission_update as update_tile
if not user_permission_info('$app.main').get('logo_hash'):
    update_tile('$app.main', logo=open('$YNH_APP_BASEDIR/conf/logo.png', 'rb'))
"
}

whoopy_start() {
    # The hook asks the wake daemon before each Ollama request: it goes first.
    ynh_systemctl --service="$app-wake" --action="restart" --log_path="systemd" \
        --wait_until="whoopy: wake socket at" --timeout=30
    # The wait above only warns. Without the daemon every Ollama request fails.
    timeout 5 "$install_dir/bin/whoopy" --socket "/run/$app/wake.sock" status >/dev/null \
        || ynh_die "The wake daemon did not start: systemctl status $app-wake"
    # An older binary starts fine but serves no page: / would answer 502.
    curl -fsS -o /dev/null --max-time 5 "http://127.0.0.1:$port_web/whoopy/status" \
        || ynh_die "The status page did not start: systemctl status $app-wake"
    # The first start replays every database migration: allow for it.
    ynh_systemctl --service="$app" --action="restart" --log_path="systemd" \
        --wait_until="Uvicorn running on" --timeout=900
}
