//! `whoopy mcp`: lists, adds and updates LiteLLM's MCP servers, as the
//! status page's MCP section does, from the command line.
//!
//! Run as root: the admin key is LiteLLM's master key, read from the
//! root-600 whoopy.env with LiteLLM's loopback address. The daemon never
//! holds it. A server's token comes from stdin only, never the command line,
//! which every local user sees in `ps`.

use std::{
    io::{IsTerminal, Read},
    path::Path,
};

use reqwest::{Method, Url};
use serde_json::{Value, json};

pub struct Admin {
    url: String,
    key: String,
    http: reqwest::Client,
}

/// The changes `update` makes; `None` leaves a field as it is.
pub struct Change {
    pub url: Option<String>,
    pub every_key: Option<bool>,
    pub token: bool,
}

impl Admin {
    pub fn from_env_file(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("{}: {e} (run as root)", path.display()))?;
        let get = |name: &str| {
            env_value(&text, name).ok_or_else(|| format!("{}: no {name}", path.display()))
        };
        Ok(Self {
            url: get("WHOOPY_LITELLM_URL")?.trim_end_matches('/').to_string(),
            key: get("LITELLM_MASTER_KEY")?,
            // No proxy from the environment, which sudo keeps: the master key
            // goes to loopback only.
            http: reqwest::Client::builder()
                .no_proxy()
                .build()
                .map_err(|e| e.to_string())?,
        })
    }

    async fn call(&self, method: Method, body: Option<Value>) -> Result<Value, String> {
        let mut req = self
            .http
            .request(method, format!("{}/v1/mcp/server", self.url))
            .bearer_auth(&self.key);
        if let Some(body) = body {
            req = req.json(&body);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("LiteLLM at {}: {e}", self.url))?;
        let code = resp.status();
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        if code.is_success() {
            return Ok(body);
        }
        Err(format!("LiteLLM refused ({code}): {}", reason(&body)))
    }

    async fn servers(&self) -> Result<Vec<Value>, String> {
        match self.call(Method::GET, None).await? {
            Value::Array(list) => Ok(list),
            other => Err(format!("unexpected server list: {other}")),
        }
    }

    pub async fn list(&self) -> Result<(), String> {
        for s in self.servers().await? {
            let who = match s["allow_all_keys"].as_bool() {
                Some(true) => "every key",
                _ => "granted keys",
            };
            let from = match s["is_config"].as_bool() {
                Some(true) => ", from the package",
                _ => "",
            };
            println!(
                "/mcp/{}  {}  ({who}{from})",
                path_name(&s),
                s["url"].as_str().unwrap_or("")
            );
        }
        Ok(())
    }

    pub async fn add(
        &self,
        name: &str,
        url: &str,
        every_key: bool,
        token: bool,
    ) -> Result<(), String> {
        check_name(name)?;
        check_url(url)?;
        let mut body = json!({
            "server_name": name,
            "url": url,
            "transport": "http",
            "allow_all_keys": every_key,
        });
        if token {
            body["auth_type"] = "bearer_token".into();
            body["credentials"] = json!({ "auth_value": read_token()? });
        }
        self.call(Method::POST, Some(body)).await?;
        println!("Added: /mcp/{name}.");
        Ok(())
    }

    /// A partial update: LiteLLM 1.106.6 changes only the fields sent, so
    /// the token stays unless a new one is given, and the grants stay always.
    pub async fn update(&self, name: &str, change: Change) -> Result<(), String> {
        let servers = self.servers().await?;
        let Some(s) = servers.iter().find(|s| path_name(s) == name) else {
            return Err(format!("no MCP server at /mcp/{name}"));
        };
        if s["is_config"].as_bool() == Some(true) {
            return Err(format!(
                "/mcp/{name} comes from the package: its access is set in the config panel"
            ));
        }
        let mut body = json!({ "server_id": s["server_id"] });
        if let Some(url) = &change.url {
            check_url(url)?;
            body["url"] = url.as_str().into();
        }
        if let Some(every_key) = change.every_key {
            body["allow_all_keys"] = every_key.into();
        }
        if change.token {
            body["auth_type"] = "bearer_token".into();
            body["credentials"] = json!({ "auth_value": read_token()? });
        }
        if body.as_object().is_some_and(|o| o.len() == 1) {
            return Err(
                "nothing to change: give --url, --every-key, --granted or --token-stdin".into(),
            );
        }
        self.call(Method::PUT, Some(body)).await?;
        println!("Updated: /mcp/{name}.");
        Ok(())
    }
}

/// LiteLLM's reason: `{"detail": "..."}`, `{"error": {"message": "..."}}`,
/// or a validation error's `msg` fields only, since its `input` would echo
/// the token sent.
fn reason(body: &Value) -> String {
    if let Some(s) = body["detail"]
        .as_str()
        .or(body["detail"]["error"].as_str())
        .or(body["error"]["message"].as_str())
    {
        return s.to_string();
    }
    match body["detail"].as_array() {
        Some(items) => items
            .iter()
            .filter_map(|i| i["msg"].as_str())
            .collect::<Vec<_>>()
            .join("; "),
        None => "no reason given".into(),
    }
}

/// Where the server answers: its alias if it has one, as the page shows it.
fn path_name(s: &Value) -> &str {
    s["alias"]
        .as_str()
        .filter(|a| !a.is_empty())
        .or(s["server_name"].as_str())
        .unwrap_or("")
}

/// The page's rule: lower-case letters, digits and _, up to 32.
fn check_name(name: &str) -> Result<(), String> {
    if (1..=32).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        return Ok(());
    }
    Err("a server name is 1 to 32 lower-case letters, digits or _".into())
}

/// An http(s) URL; a loopback one gets a reminder of the firewall table.
fn check_url(url: &str) -> Result<(), String> {
    let parsed = Url::parse(url).map_err(|e| format!("{url}: {e}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(format!("{url}: not an http or https address"));
    }
    if matches!(parsed.host_str(), Some("127.0.0.1" | "localhost" | "[::1]")) {
        eprintln!(
            "Note: LiteLLM reaches loopback only on the ports allowed in the config panel. \
             Add {} to loopback_ports if it is not there yet.",
            parsed.port_or_known_default().unwrap_or(80)
        );
    }
    Ok(())
}

/// From a pipe only: typed at a terminal, it would show on the screen.
fn read_token() -> Result<String, String> {
    if std::io::stdin().is_terminal() {
        return Err(
            "pipe the token into --token-stdin, e.g. read -rs t; printf '%s' \"$t\" | ...".into(),
        );
    }
    let mut token = String::new();
    std::io::stdin()
        .read_to_string(&mut token)
        .map_err(|e| format!("reading the token from stdin: {e}"))?;
    let token = token.trim_end_matches(['\r', '\n']);
    if token.is_empty() {
        return Err("the token read from stdin is empty".into());
    }
    Ok(token.to_string())
}

/// `NAME=value` from an env file as systemd reads it: `#` lines skipped,
/// surrounding double or single quotes stripped, `\"` and `\\` unescaped
/// within double quotes.
fn env_value(text: &str, name: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let value = line.trim_start().strip_prefix(name)?.strip_prefix('=')?;
        let value = value.trim();
        if let Some(inner) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
            return Some(inner.replace("\\\"", "\"").replace("\\\\", "\\"));
        }
        if let Some(inner) = value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')) {
            return Some(inner.to_string());
        }
        Some(value.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_file() {
        let text = "# LITELLM_MASTER_KEY=commented\nLITELLM_MASTER_KEY=sk-1\n\
                    UI_PASSWORD=\"a\\\"b\\\\c\"\nQ='x y'\nWHOOPY_LITELLM_URL=http://127.0.0.1:8000/\n";
        assert_eq!(
            env_value(text, "LITELLM_MASTER_KEY").as_deref(),
            Some("sk-1")
        );
        assert_eq!(env_value(text, "UI_PASSWORD").as_deref(), Some("a\"b\\c"));
        assert_eq!(env_value(text, "Q").as_deref(), Some("x y"));
        assert_eq!(env_value(text, "LITELLM"), None);
        assert_eq!(env_value(text, "MISSING"), None);
    }

    #[test]
    fn names_and_urls() {
        assert!(check_name("web_2").is_ok());
        for bad in ["", "Web", "a-b", "a/b", &"a".repeat(33)] {
            assert!(check_name(bad).is_err(), "{bad}");
        }
        assert!(check_url("http://10.0.0.2:8770/mcp").is_ok());
        assert!(check_url("ftp://x/mcp").is_err());
        assert!(check_url("not a url").is_err());
    }

    #[test]
    fn reasons_never_echo_input() {
        assert_eq!(reason(&json!({"detail": "no"})), "no");
        assert_eq!(reason(&json!({"detail": {"error": "taken"}})), "taken");
        assert_eq!(reason(&json!({"error": {"message": "bad key"}})), "bad key");
        let v = json!({"detail": [{"msg": "field required", "input": {"credentials": "s3cret"}}]});
        assert_eq!(reason(&v), "field required");
        assert_eq!(reason(&Value::Null), "no reason given");
    }

    #[test]
    fn alias_first() {
        let s = json!({"server_name": "a", "alias": "b"});
        assert_eq!(path_name(&s), "b");
        let s = json!({"server_name": "a", "alias": null});
        assert_eq!(path_name(&s), "a");
    }
}
