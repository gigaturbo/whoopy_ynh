# Security

What Whoopy does to limit the damage from a bad LiteLLM release or a leaked
token.

- **LiteLLM is pinned** (1.106.6) and installed from a hash-checked lock file.
  The pin moves only for a fix.
- **Code is owned by root**, readable by the app. Secrets sit in root-only
  files, read by systemd before it drops privileges.
- **The service is confined**: no `/home` beyond its own data, no `/mnt`,
  `/media` or `/srv`. Memory capped at 1.5 GB.
- **The MCP tools are not on the internet** by default: their routes answer
  the local networks only, so a leaked token alone does not reach the tools.
  A server opened from the status page is the exception: for it, the key is
  the only guard. That list sits in the wake daemon's file, which LiteLLM's
  user can write too: harmless, since LiteLLM already answers every outside
  call to `/v1` and holds every server's token. Whoever may open the status
  page (the install's admin group by default) may open a server.
- **Other services on this machine are out of reach**: LiteLLM may connect
  only to its database, DNS, its own MCP server and the ports listed in the
  panel, not to LDAP, Redis, the YunoHost API or another app.
- **No internet by default**: LiteLLM reaches local networks only until the
  panel's outbound setting says otherwise.
- **Nothing is fetched at runtime**: price list, blog feed and similar
  downloads are off. Prisma's engines are built at install.
- **The wake service is separate and smaller**: written in Rust, it reads only
  the wake settings and its own token, never LiteLLM's secrets, and never
  reaches the internet. Its binary is downloaded from this repository's
  releases and checked against the sha256 in the manifest.
- **Secrets stay out of YunoHost's logs**: tokens and keys are generated and
  rotated without being traced.
