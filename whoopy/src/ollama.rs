//! The AI server: its HTTP API and its Wake-on-LAN packet.

use std::{
    io,
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};

use serde::Deserialize;

use crate::wake::{Machine, parse_mac, send_magic_packet};

const PROBE_TIMEOUT: Duration = Duration::from_millis(1500);
const LIST_TIMEOUT: Duration = Duration::from_secs(5);
/// Model lists are cut to this many names before they reach a model.
const MAX_MODELS: usize = 200;

pub struct Ollama {
    /// scheme://host:port, no path, no trailing slash.
    url: String,
    http: reqwest::Client,
}

#[derive(Deserialize)]
struct ModelList {
    models: Vec<Model>,
}

#[derive(Deserialize)]
struct Model {
    name: String,
}

impl Ollama {
    pub fn new(url: &str) -> Self {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("an HTTP client with no TLS always builds");
        Self {
            url: url.trim_end_matches('/').into(),
            http,
        }
    }

    /// Installed models (`/api/tags`), or None when Ollama does not answer.
    pub async fn installed(&self) -> Option<Vec<String>> {
        self.list("/api/tags").await
    }

    /// Models in memory (`/api/ps`), or None when Ollama does not answer.
    pub async fn loaded(&self) -> Option<Vec<String>> {
        self.list("/api/ps").await
    }

    async fn list(&self, path: &str) -> Option<Vec<String>> {
        let resp = self
            .http
            .get(format!("{}{path}", self.url))
            .timeout(LIST_TIMEOUT)
            .send()
            .await;
        let list: ModelList = resp.ok()?.error_for_status().ok()?.json().await.ok()?;
        Some(
            list.models
                .into_iter()
                .take(MAX_MODELS)
                .map(|m| m.name)
                .collect(),
        )
    }
}

impl Machine for Ollama {
    // /api/version loads no model, so it does not keep the machine awake.
    async fn up(&self) -> bool {
        let resp = self
            .http
            .get(format!("{}/api/version", self.url))
            .timeout(PROBE_TIMEOUT)
            .send()
            .await;
        resp.is_ok_and(|r| r.status().is_success())
    }

    fn send_packet(&self, mac: &str, broadcast: Ipv4Addr) -> io::Result<()> {
        if mac.is_empty() {
            return Err(io::Error::other("no MAC address is saved"));
        }
        let mac = parse_mac(mac)
            .ok_or_else(|| io::Error::other(format!("not a MAC address: {mac:?}")))?;
        send_magic_packet(mac, SocketAddr::new(broadcast.into(), 9))
    }
}
