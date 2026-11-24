//! The page's settings: changed on the status page, applied at once, kept in
//! a JSON file the daemon owns (WHOOPY_SETTINGS, in its StateDirectory).
//!
//! Not in the YunoHost panel: they only tune the wake, plus the MCP servers
//! open to the internet. What guards Whoopy (outbound access, LAN ranges,
//! passwords) stays there, root-only, since this file is writable by the user
//! LiteLLM runs as too. The open servers are the one exception (F-M-15): that
//! gate never confined LiteLLM, which answers every outside /v1 call and holds
//! every server's token already.

use std::{
    io::{self, Write},
    net::Ipv4Addr,
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::wake::parse_mac;

pub const MIN_TIMEOUT_S: u64 = 10;
/// nginx gives a request 600 s, and the hook waits this plus 10.
pub const MAX_TIMEOUT_S: u64 = 540;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// Requests wake the machine; a manual wake works either way.
    pub wol_enabled: bool,
    pub wol_mac: String,
    /// Where the wake packet goes, on UDP port 9.
    pub wol_broadcast: Ipv4Addr,
    /// How long Ollama has to answer before a wake fails.
    pub wake_timeout_s: u64,
    /// How long Ollama keeps a model loaded after a request through Whoopy,
    /// e.g. `5m`; empty leaves Ollama's own setting. Read by the hook.
    pub keep_alive: String,
    /// The MCP servers nginx lets through from anywhere, a key still needed.
    /// Changed one at a time with `toggle_public`, never by a full save.
    pub mcp_public: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            wol_enabled: false,
            wol_mac: String::new(),
            wol_broadcast: Ipv4Addr::BROADCAST,
            wake_timeout_s: 90,
            keep_alive: String::new(),
            mcp_public: Vec::new(),
        }
    }
}

impl Settings {
    /// Why these settings cannot be used, in words for the page.
    pub fn check(&self) -> Result<(), String> {
        if !self.wol_mac.is_empty() && parse_mac(&self.wol_mac).is_none() {
            return Err(
                "The MAC address is six pairs of hex digits, e.g. aa:bb:cc:dd:ee:ff.".into(),
            );
        }
        if self.wol_enabled && self.wol_mac.is_empty() {
            return Err("A MAC address is needed to wake the Ollama machine.".into());
        }
        if !(MIN_TIMEOUT_S..=MAX_TIMEOUT_S).contains(&self.wake_timeout_s) {
            return Err(format!(
                "The wake timeout is {MIN_TIMEOUT_S} to {MAX_TIMEOUT_S} seconds."
            ));
        }
        if !keep_alive_ok(&self.keep_alive) {
            return Err(
                "How long a model stays loaded is a number then s, m or h, e.g. 5m, or empty."
                    .into(),
            );
        }
        if let Some(bad) = self.mcp_public.iter().find(|n| !server_name_ok(n)) {
            return Err(format!(
                "\"{bad}\" cannot be opened: a server name is up to 64 letters, digits, _ or -, and not sse, proxy or enabled."
            ));
        }
        Ok(())
    }

    /// Opens or closes one MCP server to the internet; the list stays sorted
    /// and without duplicates.
    pub fn toggle_public(&mut self, name: &str, public: bool) {
        self.mcp_public.retain(|n| n != name);
        if public {
            self.mcp_public.push(name.to_owned());
            self.mcp_public.sort();
        }
    }

    /// The saved settings, or the defaults when none were ever saved.
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        let settings: Self =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        settings
            .check()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(settings)
    }

    /// Written beside, synced, then renamed: a power cut never leaves an
    /// empty file, which would stop the daemon's next start.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let tmp = path.with_extension("tmp");
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(&serde_json::to_vec_pretty(self)?)?;
        file.sync_all()?;
        std::fs::rename(&tmp, path)
    }
}

/// What nginx's public location matches, minus LiteLLM's own routes under
/// /mcp: /mcp/sse and /mcp/proxy pick their servers from a header or the
/// key's grants, so opening them would open every server.
fn server_name_ok(n: &str) -> bool {
    (1..=64).contains(&n.len())
        && n.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        && !["sse", "proxy", "enabled"].contains(&n.to_ascii_lowercase().as_str())
}

/// Empty, or digits then s, m or h: what Ollama's keep_alive takes, minus
/// the negative and bare-number forms nobody needs here.
fn keep_alive_ok(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    let (num, unit) = s.split_at(s.len() - 1);
    matches!(unit, "s" | "m" | "h") && !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> Settings {
        Settings {
            wol_enabled: true,
            wol_mac: "aa:bb:cc:dd:ee:ff".into(),
            wol_broadcast: Ipv4Addr::new(192, 168, 1, 255),
            wake_timeout_s: 120,
            keep_alive: "5m".into(),
            mcp_public: vec!["docs".into(), "web".into()],
        }
    }

    #[test]
    fn checks() {
        assert_eq!(valid().check(), Ok(()));
        assert_eq!(Settings::default().check(), Ok(()));
        let bad = [
            Settings {
                wol_mac: "aa:bb".into(),
                ..valid()
            },
            Settings {
                wol_mac: String::new(),
                ..valid()
            },
            Settings {
                wake_timeout_s: 9,
                ..valid()
            },
            Settings {
                wake_timeout_s: 541,
                ..valid()
            },
            Settings {
                keep_alive: "5".into(),
                ..valid()
            },
            Settings {
                keep_alive: "m".into(),
                ..valid()
            },
            Settings {
                keep_alive: "-1m".into(),
                ..valid()
            },
            Settings {
                keep_alive: "5d".into(),
                ..valid()
            },
        ]
        .into_iter()
        .chain(
            [
                "",
                "sse",
                "Proxy",
                "enabled",
                "web,docs",
                "a/b",
                "a;}",
                &"a".repeat(65),
            ]
            .map(|n| Settings {
                mcp_public: vec![n.into()],
                ..valid()
            }),
        );
        for s in bad {
            assert!(s.check().is_err(), "{s:?}");
        }
        // A MAC kept while the wake is off is fine.
        assert_eq!(
            Settings {
                wol_enabled: false,
                ..valid()
            }
            .check(),
            Ok(())
        );
    }

    #[test]
    fn toggle() {
        let mut s = valid();
        s.toggle_public("abc", true);
        s.toggle_public("web", true);
        assert_eq!(s.mcp_public, ["abc", "docs", "web"]);
        s.toggle_public("docs", false);
        s.toggle_public("nope", false);
        assert_eq!(s.mcp_public, ["abc", "web"]);
    }

    #[test]
    fn round_trip_and_defaults() {
        let dir = std::env::temp_dir().join(format!("whoopy-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        assert_eq!(Settings::load(&path), Ok(Settings::default()));
        valid().save(&path).unwrap();
        assert_eq!(Settings::load(&path), Ok(valid()));
        assert!(!path.with_extension("tmp").exists());
        // A partial file takes the defaults for what it lacks.
        std::fs::write(&path, r#"{"wol_mac": "aa:bb:cc:dd:ee:ff"}"#).unwrap();
        assert_eq!(Settings::load(&path).unwrap().wake_timeout_s, 90);
        // A file that would not pass the page is refused, not half used.
        std::fs::write(&path, r#"{"wake_timeout_s": 5}"#).unwrap();
        assert!(Settings::load(&path).is_err());
        std::fs::write(&path, r#"{"wol_macc": ""}"#).unwrap();
        assert!(Settings::load(&path).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
