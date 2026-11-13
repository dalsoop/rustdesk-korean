//! Korean fork: the for-korean build flavor ("RustDesk (for korean) 접속"), chosen at build time
//! with `RUSTDESK_FLAVOR=for-korean` (build.rs turns it into `RUSTDESK_BUILD_FLAVOR`).
//!
//! A separate app for reaching peers through the public RustDesk server. It runs under its own
//! app name, so its config, socket and log folders never mix with the main app's, and it only
//! accepts incoming connections from the Tailscale range. A build without the flavor never
//! enters any of this.

use base::config::keys;
use hbb_common::{
    config::{self, Config, LocalConfig, PeerConfig, RENDEZVOUS_PORT},
    log,
    protobuf::Message as _,
    rendezvous_proto::{rendezvous_message, PeerDiscovery, RendezvousMessage},
};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr, UdpSocket},
    time::{Duration, Instant},
};

pub const APP_NAME: &str = "RustDeskForKorean";
const TAILSCALE_RANGE: &str = "100.64.0.0/10";
const INTERNAL_ID_DOMAIN: &str = "@rustdesk.internal.kr";
const CLEANUP_DONE_OPTION: &str = "for-korean-flavor-cleanup-done";
pub const DISCOVERY_PROBE_TIMEOUT: Duration = Duration::from_millis(1500);

#[inline]
pub fn is_for_korean() -> bool {
    option_env!("RUSTDESK_BUILD_FLAVOR") == Some("for-korean")
}

/// Must run before anything reads the config: the app name decides where the config lives.
pub fn init() {
    if !is_for_korean() {
        return;
    }
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        *config::APP_NAME.write().unwrap() = APP_NAME.to_owned();
        // Overrides win over saved settings and cannot be saved over, so users cannot unlock them.
        config::OVERWRITE_SETTINGS
            .write()
            .unwrap()
            .extend(override_settings());
        cleanup_once();
    });
}

fn override_settings() -> HashMap<String, String> {
    HashMap::from([
        (keys::OPTION_DIRECT_SERVER.to_owned(), "N".to_owned()),
        (keys::OPTION_WHITELIST.to_owned(), TAILSCALE_RANGE.to_owned()),
    ])
}

fn is_internal_peer_id(id: &str) -> bool {
    id.contains(INTERNAL_ID_DOMAIN)
}

/// Settings left over from the internal server would keep this app off the public server.
fn cleanup_once() {
    if LocalConfig::get_option(CLEANUP_DONE_OPTION) == "Y" {
        return;
    }
    for k in [
        keys::OPTION_CUSTOM_RENDEZVOUS_SERVER,
        keys::OPTION_RELAY_SERVER,
        keys::OPTION_API_SERVER,
        keys::OPTION_KEY,
    ] {
        Config::set_option(k.to_owned(), String::new());
    }
    // Removing the peer file drops both the recent session and its remembered password.
    for (id, _, _) in PeerConfig::get_vec_id_modified_time_path(&None) {
        if is_internal_peer_id(&id) {
            PeerConfig::remove(&id);
        }
    }
    let fav = LocalConfig::get_fav();
    let kept: Vec<String> = fav
        .iter()
        .filter(|id| !is_internal_peer_id(id))
        .cloned()
        .collect();
    if kept.len() != fav.len() {
        LocalConfig::set_fav(kept);
    }
    if is_internal_peer_id(&LocalConfig::get_remote_id()) {
        LocalConfig::set_remote_id("");
    }
    LocalConfig::set_option(CLEANUP_DONE_OPTION.to_owned(), "Y".to_owned());
}

#[derive(Debug, PartialEq)]
pub struct RustDeskPeer {
    pub id: String,
    pub hostname: String,
    pub platform: String,
}

// Same port as lan.rs `get_broadcast_port()`, where RustDesk's discovery listener answers.
fn discovery_port() -> u16 {
    (RENDEZVOUS_PORT + 3) as _
}

fn discovery_ping(our_id: String) -> Vec<u8> {
    let mut msg = RendezvousMessage::new();
    msg.set_peer_discovery(PeerDiscovery {
        cmd: "ping".to_owned(),
        id: our_id,
        ..Default::default()
    });
    msg.write_to_bytes().unwrap_or_default()
}

fn parse_discovery_pong(from: IpAddr, target: IpAddr, bytes: &[u8]) -> Option<RustDeskPeer> {
    if from != target {
        return None;
    }
    match RendezvousMessage::parse_from_bytes(bytes).ok()?.union {
        Some(rendezvous_message::Union::PeerDiscovery(p)) if p.cmd == "pong" => Some(RustDeskPeer {
            id: p.id,
            hostname: p.hostname,
            platform: p.platform,
        }),
        _ => None,
    }
}

/// Asks the RustDesk LAN discovery listener at `ip` who it is, so a peer whose direct port is
/// closed can still be told apart from one without RustDesk. Blocks up to `timeout`; the target
/// only answers while its `enable-lan-discovery` option is on.
pub fn probe_rustdesk(ip: IpAddr, timeout: Duration) -> Option<RustDeskPeer> {
    let bind: SocketAddr = if ip.is_ipv4() {
        ([0, 0, 0, 0], 0).into()
    } else {
        ([0u16; 8], 0).into()
    };
    let result = (|| -> std::io::Result<Option<RustDeskPeer>> {
        let socket = UdpSocket::bind(bind)?;
        socket.send_to(&discovery_ping(Config::get_id()), (ip, discovery_port()))?;
        let deadline = Instant::now() + timeout;
        let mut buf = [0u8; 2048];
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Ok(None);
            }
            socket.set_read_timeout(Some(left))?;
            let (len, from) = socket.recv_from(&mut buf)?;
            if let Some(peer) = parse_discovery_pong(from.ip(), ip, &buf[..len]) {
                return Ok(Some(peer));
            }
        }
    })();
    match result {
        Ok(peer) => peer,
        Err(e) => {
            log::trace!("RustDesk discovery probe to {ip}: {e}");
            None
        }
    }
}

/// JSON `{"id","hostname","platform"}` for the for-korean home cards, or "" when nothing answered.
pub fn probe_rustdesk_json(ip: &str) -> String {
    let Ok(ip) = ip.parse::<IpAddr>() else {
        return String::new();
    };
    match probe_rustdesk(ip, DISCOVERY_PROBE_TIMEOUT) {
        Some(p) => serde_json::json!({
            "id": p.id,
            "hostname": p.hostname,
            "platform": p.platform,
        })
        .to_string(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_removes_only_internal_server_ids() {
        assert!(is_internal_peer_id(
            "77561364@rustdesk.internal.kr?key=OeVuKk5nlHiXp+APNn0Y3pC1Iwpwn44JGqrQCsWqmBw="
        ));
        assert!(is_internal_peer_id("77561364@rustdesk.internal.kr"));
        assert!(!is_internal_peer_id("1703636484"));
        assert!(!is_internal_peer_id("1703636484@rustdesk.com"));
    }

    #[test]
    fn incoming_is_pinned_to_tailscale_without_direct_port() {
        let settings = override_settings();
        assert_eq!(
            settings.get(keys::OPTION_DIRECT_SERVER).map(String::as_str),
            Some("N")
        );
        assert_eq!(
            settings.get(keys::OPTION_WHITELIST).map(String::as_str),
            Some("100.64.0.0/10")
        );
    }

    #[test]
    fn discovery_pong_is_read_only_from_the_probed_ip() {
        let target: IpAddr = "100.91.226.87".parse().unwrap();
        let other: IpAddr = "100.64.0.9".parse().unwrap();
        let mut msg = RendezvousMessage::new();
        msg.set_peer_discovery(PeerDiscovery {
            cmd: "pong".to_owned(),
            id: "123456789".to_owned(),
            hostname: "m5m2".to_owned(),
            platform: "Mac OS".to_owned(),
            ..Default::default()
        });
        let pong = msg.write_to_bytes().unwrap();
        assert_eq!(
            parse_discovery_pong(target, target, &pong),
            Some(RustDeskPeer {
                id: "123456789".to_owned(),
                hostname: "m5m2".to_owned(),
                platform: "Mac OS".to_owned(),
            })
        );
        assert_eq!(parse_discovery_pong(other, target, &pong), None);
        let ping = discovery_ping("987".to_owned());
        assert_eq!(parse_discovery_pong(target, target, &ping), None);
        match RendezvousMessage::parse_from_bytes(&ping).unwrap().union {
            Some(rendezvous_message::Union::PeerDiscovery(p)) => {
                assert_eq!((p.cmd.as_str(), p.id.as_str()), ("ping", "987"));
            }
            _ => panic!("not a discovery message"),
        }
    }
}
