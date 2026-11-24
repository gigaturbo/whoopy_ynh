//! Waking the AI server: one wake at a time, shared by every caller.
//!
//! A wake probes Ollama, sends a Wake-on-LAN packet while it does not answer,
//! resends it every `RESEND`, and gives up after the timeout. A success is
//! trusted for `FRESH`, so a burst of requests costs one probe; a failure is
//! remembered for `DOWN`, so a burst during an outage fails fast.

use std::{
    io,
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use tokio::{sync::watch, time::Instant};

use crate::settings::Settings;

pub const POLL: Duration = Duration::from_secs(2);
// One UDP packet can be lost, and a machine still finishing a suspend ignores
// one sent too early.
pub const RESEND: Duration = Duration::from_secs(15);
pub const FRESH: Duration = Duration::from_secs(10);
pub const DOWN: Duration = Duration::from_secs(15);

/// What a wake needs from the outside world, so tests can fake it.
pub trait Machine: Send + Sync + 'static {
    /// True when Ollama answers. Must load no model.
    fn up(&self) -> impl Future<Output = bool> + Send;
    fn send_packet(&self, mac: &str, broadcast: Ipv4Addr) -> io::Result<()>;
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Outcome {
    pub ok: bool,
    /// Seconds the caller waited, or the wake took when it ran.
    pub seconds: u64,
    /// Why it failed; empty on success.
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct LastWake {
    /// Unix time the wake started.
    pub started: u64,
    #[serde(flatten)]
    pub outcome: Outcome,
}

pub struct Waker<M> {
    machine: M,
    /// Read at each wake, so a change applies to the next one.
    settings: Mutex<Settings>,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    up_until: Option<Instant>,
    down_until: Option<Instant>,
    wake: Option<watch::Receiver<Option<Outcome>>>,
    last: Option<LastWake>,
}

impl<M: Machine> Waker<M> {
    pub fn new(machine: M, settings: Settings) -> Arc<Self> {
        Arc::new(Self {
            machine,
            settings: Mutex::new(settings),
            state: Mutex::default(),
        })
    }

    pub fn machine(&self) -> &M {
        &self.machine
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    /// A wake already running keeps the settings it started with.
    pub fn set_settings(&self, settings: Settings) {
        *self.settings.lock().unwrap() = settings;
    }

    pub fn waking(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.wake.as_ref().is_some_and(|rx| rx.borrow().is_none())
    }

    pub fn last(&self) -> Option<LastWake> {
        self.state.lock().unwrap().last.clone()
    }

    /// Returns once Ollama answers or the wake gives up. `manual` is a wake
    /// asked for by hand (panel, CLI, MCP): it runs even when waking on
    /// requests is off. A caller that gives up does not cancel the wake.
    pub async fn ensure_awake(self: &Arc<Self>, manual: bool) -> Outcome {
        if !manual && !self.settings().wol_enabled {
            return Outcome {
                ok: true,
                seconds: 0,
                reason: String::new(),
            };
        }
        let start = Instant::now();
        let mut rx = {
            let mut state = self.state.lock().unwrap();
            if state.up_until.is_some_and(|t| start < t) {
                return Outcome {
                    ok: true,
                    seconds: 0,
                    reason: String::new(),
                };
            }
            // A wake asked for by hand always tries again.
            if !manual && state.down_until.is_some_and(|t| start < t) {
                return Outcome {
                    ok: false,
                    seconds: 0,
                    reason: format!("a wake failed less than {} s ago", DOWN.as_secs()),
                };
            }
            match &state.wake {
                // A wake still running; has_changed fails if its task died.
                Some(rx) if rx.borrow().is_none() && rx.has_changed().is_ok() => rx.clone(),
                _ => {
                    let (tx, rx) = watch::channel(None);
                    state.wake = Some(rx.clone());
                    tokio::spawn(Arc::clone(self).run(tx));
                    rx
                }
            }
        };
        let outcome = match rx.wait_for(Option::is_some).await {
            Ok(done) => done.clone().unwrap(),
            Err(_) => Outcome {
                ok: false,
                seconds: 0,
                reason: "the wake stopped".into(),
            },
        };
        Outcome {
            seconds: start.elapsed().as_secs(),
            ..outcome
        }
    }

    async fn run(self: Arc<Self>, tx: watch::Sender<Option<Outcome>>) {
        let started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let settings = self.settings();
        let timeout = Duration::from_secs(settings.wake_timeout_s);
        let start = Instant::now();
        let mut last_sent: Option<Instant> = None;
        let mut reason = format!("Ollama did not answer within {} s", timeout.as_secs());
        while start.elapsed() < timeout {
            if self.machine.up().await {
                reason.clear();
                break;
            }
            if last_sent.is_none_or(|t| t.elapsed() >= RESEND) {
                if let Err(err) = self
                    .machine
                    .send_packet(&settings.wol_mac, settings.wol_broadcast)
                {
                    reason = format!("the wake packet could not be sent: {err}");
                    break;
                }
                last_sent = Some(Instant::now());
            }
            tokio::time::sleep(POLL).await;
        }

        let ok = reason.is_empty();
        let outcome = Outcome {
            ok,
            seconds: start.elapsed().as_secs(),
            reason,
        };
        {
            let mut state = self.state.lock().unwrap();
            let now = Instant::now();
            if ok {
                state.up_until = Some(now + FRESH);
            } else {
                state.down_until = Some(now + DOWN);
            }
            // A run that found Ollama up sent nothing: not a wake to report.
            if last_sent.is_some() || !ok {
                state.last = Some(LastWake {
                    started,
                    outcome: outcome.clone(),
                });
            }
        }
        tx.send_replace(Some(outcome));
    }
}

/// `aa:bb:cc:dd:ee:ff` or with dashes.
pub fn parse_mac(mac: &str) -> Option<[u8; 6]> {
    let parts: Vec<u8> = mac
        .split([':', '-'])
        // from_str_radix alone would take "+a".
        .map(|p| {
            let hex = p.len() == 2 && p.bytes().all(|b| b.is_ascii_hexdigit());
            if hex {
                u8::from_str_radix(p, 16).ok()
            } else {
                None
            }
        })
        .collect::<Option<_>>()?;
    parts.try_into().ok()
}

/// Broadcast, never unicast: a sleeping machine answers no ARP.
pub fn send_magic_packet(mac: [u8; 6], to: SocketAddr) -> io::Result<()> {
    let mut packet = [0xffu8; 102];
    for chunk in packet[6..].chunks_mut(6) {
        chunk.copy_from_slice(&mac);
    }
    let sock = UdpSocket::bind(("0.0.0.0", 0))?;
    sock.set_broadcast(true)?;
    sock.send_to(&packet, to)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Ollama answers once `up_after` probes have failed (never if None).
    struct Fake {
        up_after: Option<u32>,
        probes: AtomicU32,
        packets: AtomicU32,
        send_fails: bool,
    }

    impl Fake {
        fn new(up_after: Option<u32>) -> Self {
            Self {
                up_after,
                probes: 0.into(),
                packets: 0.into(),
                send_fails: false,
            }
        }
    }

    impl Machine for Fake {
        async fn up(&self) -> bool {
            let n = self.probes.fetch_add(1, Ordering::SeqCst);
            self.up_after.is_some_and(|k| n >= k)
        }
        fn send_packet(&self, _: &str, _: Ipv4Addr) -> io::Result<()> {
            if self.send_fails {
                return Err(io::Error::other("no route"));
            }
            self.packets.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    const T: Duration = Duration::from_secs(90);

    fn tuned(enabled: bool) -> Settings {
        Settings {
            wol_enabled: enabled,
            wake_timeout_s: T.as_secs(),
            ..Settings::default()
        }
    }

    #[tokio::test(start_paused = true)]
    async fn wakes_and_shares_one_wake() {
        let w = Waker::new(Fake::new(Some(5)), tuned(true));
        let (a, b) = tokio::join!(w.ensure_awake(false), w.ensure_awake(false));
        assert!(a.ok && b.ok);
        assert_eq!(w.machine().probes.load(Ordering::SeqCst), 6);
        assert_eq!(w.machine().packets.load(Ordering::SeqCst), 1);
        // Fresh: no new probe.
        assert!(w.ensure_awake(false).await.ok);
        assert_eq!(w.machine().probes.load(Ordering::SeqCst), 6);
    }

    #[tokio::test(start_paused = true)]
    async fn resends_every_resend_interval() {
        // Up after 40 s of probes every 2 s: packets at 0, 15, 30 s.
        let w = Waker::new(Fake::new(Some(20)), tuned(true));
        assert!(w.ensure_awake(false).await.ok);
        assert_eq!(w.machine().packets.load(Ordering::SeqCst), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn fails_after_timeout_then_fails_fast() {
        let w = Waker::new(Fake::new(None), tuned(true));
        let out = w.ensure_awake(false).await;
        assert!(!out.ok);
        assert!(out.seconds >= T.as_secs());
        let probes = w.machine().probes.load(Ordering::SeqCst);
        let again = w.ensure_awake(false).await;
        assert!(!again.ok && again.seconds == 0);
        assert_eq!(w.machine().probes.load(Ordering::SeqCst), probes);
        // After DOWN, a new wake runs.
        tokio::time::sleep(DOWN).await;
        w.ensure_awake(false).await;
        assert!(w.machine().probes.load(Ordering::SeqCst) > probes);
    }

    #[tokio::test(start_paused = true)]
    async fn a_failed_send_stops_the_wake() {
        let mut fake = Fake::new(None);
        fake.send_fails = true;
        let w = Waker::new(fake, tuned(true));
        let out = w.ensure_awake(true).await;
        assert!(!out.ok && out.reason.contains("could not be sent"));
        assert_eq!(out.seconds, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn disabled_skips_requests_not_manual_wakes() {
        let w = Waker::new(Fake::new(Some(1)), tuned(false));
        assert!(w.ensure_awake(false).await.ok);
        assert_eq!(w.machine().probes.load(Ordering::SeqCst), 0);
        assert!(w.ensure_awake(true).await.ok);
        assert_eq!(w.machine().packets.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_dropped_caller_does_not_cancel_the_wake() {
        let w = Waker::new(Fake::new(Some(3)), tuned(true));
        let _ = tokio::time::timeout(Duration::from_secs(1), w.ensure_awake(false)).await;
        assert!(w.ensure_awake(false).await.ok);
        assert_eq!(w.machine().packets.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_manual_wake_ignores_a_recent_failure() {
        let w = Waker::new(Fake::new(None), tuned(true));
        assert!(!w.ensure_awake(false).await.ok);
        let packets = w.machine().packets.load(Ordering::SeqCst);
        w.ensure_awake(true).await;
        assert!(w.machine().packets.load(Ordering::SeqCst) > packets);
    }

    #[tokio::test(start_paused = true)]
    async fn a_plain_check_keeps_the_last_wake() {
        let w = Waker::new(Fake::new(Some(4)), tuned(true));
        assert!(w.ensure_awake(false).await.ok);
        let woke = w.last().unwrap().outcome.seconds;
        assert!(woke >= 8);
        // Past FRESH, the next request probes again and finds Ollama up.
        tokio::time::sleep(FRESH).await;
        assert!(w.ensure_awake(false).await.ok);
        assert_eq!(w.last().unwrap().outcome.seconds, woke);
    }

    #[tokio::test(start_paused = true)]
    async fn new_settings_apply_to_the_next_wake() {
        let w = Waker::new(Fake::new(None), tuned(false));
        assert!(w.ensure_awake(false).await.ok);
        assert_eq!(w.machine().probes.load(Ordering::SeqCst), 0);
        w.set_settings(Settings {
            wake_timeout_s: 20,
            ..tuned(true)
        });
        let out = w.ensure_awake(false).await;
        assert!(!out.ok && out.reason.contains("within 20 s"), "{out:?}");
        assert!(out.seconds >= 20 && out.seconds < 30);
    }

    #[test]
    fn macs() {
        assert_eq!(
            parse_mac("aa:BB:0c:dd:ee:ff"),
            Some([0xaa, 0xbb, 0x0c, 0xdd, 0xee, 0xff])
        );
        assert_eq!(
            parse_mac("aa-bb-0c-dd-ee-ff"),
            Some([0xaa, 0xbb, 0x0c, 0xdd, 0xee, 0xff])
        );
        assert_eq!(parse_mac("aa:bb:cc:dd:ee"), None);
        assert_eq!(parse_mac("aa:bb:cc:dd:ee:ff:00"), None);
        assert_eq!(parse_mac("aabbccddeeff"), None);
        assert_eq!(parse_mac("+a:bb:cc:dd:ee:ff"), None);
    }
}
