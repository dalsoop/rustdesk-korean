//! Korean fork: the "RustDesk Public" build flavor, chosen at build time with
//! `RUSTDESK_FLAVOR=public` (build.rs turns it into `RUSTDESK_BUILD_FLAVOR`).
//!
//! A separate app for reaching peers through the public RustDesk server. It runs under its own
//! app name, so its config, socket and log folders never mix with the main app's. It accepts
//! incoming connections only from the Tailscale range and only with its permanent password
//! (set with `--set-permanent-password`). A build without the flavor never enters any of this.

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

pub const APP_NAME: &str = "RustDeskPublic";
// Tailscale's IPv4 and IPv6 ranges, in the whitelist's comma-separated form.
const TAILSCALE_RANGE: &str = "100.64.0.0/10,fd7a:115c:a1e0::/48";
const INTERNAL_ID_DOMAIN: &str = "@rustdesk.internal.kr";
const CLEANUP_DONE_OPTION: &str = "public-flavor-cleanup-done";
pub const DISCOVERY_PROBE_TIMEOUT: Duration = Duration::from_millis(1500);

#[inline]
pub fn is_public() -> bool {
    option_env!("RUSTDESK_BUILD_FLAVOR") == Some("public")
}

/// Must run before anything reads the config: the app name decides where the config lives.
pub fn init() {
    if !is_public() {
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
        (keys::OPTION_DIRECT_SERVER.to_owned(), "Y".to_owned()),
        (keys::OPTION_WHITELIST.to_owned(), TAILSCALE_RANGE.to_owned()),
        (
            keys::OPTION_VERIFICATION_METHOD.to_owned(),
            "use-permanent-password".to_owned(),
        ),
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

/// JSON `{"id","hostname","platform"}` for the Public home cards, or "" when nothing answered.
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

fn in_tailscale_range(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let [a, b, ..] = v4.octets();
            a == 100 && b & 0xc0 == 64
        }
        IpAddr::V6(v6) => v6.segments()[..3] == [0xfd7a, 0x115c, 0xa1e0],
    }
}

// macOS names every tunnel `utunN` (the Tailscale app and Homebrew's tailscaled alike); on Linux
// the interface is `tailscale0`.
fn is_tailscale_interface_name(name: &str) -> bool {
    name.starts_with("utun") || name.starts_with("tailscale")
}

/// Whether `ip` is assigned to a Tailscale interface in `addrs` (interface name, address): a
/// tunnel that also carries a Tailscale IPv6 address, which other VPNs using 100.64.0.0/10 on a
/// `utun` (Cloudflare WARP, for one) do not have. A tailnet with IPv6 turned off fails closed.
fn is_tailscale_interface_address(addrs: &[(String, IpAddr)], ip: IpAddr) -> bool {
    addrs.iter().any(|(name, a)| {
        *a == ip
            && is_tailscale_interface_name(name)
            && addrs
                .iter()
                .any(|(n, b)| n == name && b.is_ipv6() && in_tailscale_range(*b))
    })
}

/// This computer's addresses with their interface names, straight from getifaddrs(3): cheap,
/// unlike a full interface listing that also looks up gateways.
#[cfg(unix)]
fn interface_addresses() -> Vec<(String, IpAddr)> {
    use hbb_common::libc;
    let mut out = Vec::new();
    let mut head: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: getifaddrs allocates the list, freeifaddrs below frees it; nodes are read only in
    // between, and only through the pointers the list itself holds.
    if unsafe { libc::getifaddrs(&mut head) } != 0 {
        return out;
    }
    let mut cur = head;
    while !cur.is_null() {
        let ifa = unsafe { &*cur };
        cur = ifa.ifa_next;
        if ifa.ifa_addr.is_null() || ifa.ifa_name.is_null() {
            continue;
        }
        let ip = match unsafe { (*ifa.ifa_addr).sa_family } as libc::c_int {
            libc::AF_INET => {
                let sin = unsafe { &*(ifa.ifa_addr as *const libc::sockaddr_in) };
                IpAddr::from(sin.sin_addr.s_addr.to_ne_bytes())
            }
            libc::AF_INET6 => {
                let sin6 = unsafe { &*(ifa.ifa_addr as *const libc::sockaddr_in6) };
                IpAddr::from(sin6.sin6_addr.s6_addr)
            }
            _ => continue,
        };
        let name = unsafe { std::ffi::CStr::from_ptr(ifa.ifa_name) };
        out.push((name.to_string_lossy().into_owned(), ip));
    }
    unsafe { libc::freeifaddrs(head) };
    out
}

#[cfg(not(unix))]
fn interface_addresses() -> Vec<(String, IpAddr)> {
    Vec::new()
}

fn is_on_tailscale_interface(ip: IpAddr) -> bool {
    is_tailscale_interface_address(&interface_addresses(), ip)
}

/// The address this computer sends from to reach `peer`, i.e. which interface its route uses.
/// A connected UDP socket sends nothing.
fn route_source(peer: IpAddr) -> Option<IpAddr> {
    let bind: SocketAddr = if peer.is_ipv4() {
        ([0, 0, 0, 0], 0).into()
    } else {
        ([0u16; 8], 0).into()
    };
    let socket = UdpSocket::bind(bind).ok()?;
    socket.connect((peer, 9)).ok()?;
    Some(socket.local_addr().ok()?.ip())
}

/// A peer address in the Tailscale range is trusted only when the connection reached this
/// computer on a Tailscale address of its Tailscale interface and the way back to the peer runs
/// through that same address. The whitelist alone checks only the peer's address, which a
/// carrier or hotel network handing out 100.64.0.0/10 on its LAN could also present; the route
/// check covers such a peer that sends to this Mac's Tailscale IP over the LAN (macOS accepts a
/// packet for any of its addresses on any interface).
fn tailscale_path_allowed(
    peer: IpAddr,
    local: IpAddr,
    on_tailscale_interface: impl Fn(IpAddr) -> bool,
    route_source: impl Fn(IpAddr) -> Option<IpAddr>,
) -> bool {
    !in_tailscale_range(peer)
        || (in_tailscale_range(local)
            && on_tailscale_interface(local)
            && route_source(peer) == Some(local))
}

/// Korean fork hook for `Connection::on_open`, next to the whitelist check. `true` outside ranode.
pub fn incoming_path_allowed(peer: SocketAddr, local: SocketAddr) -> bool {
    if !is_public() {
        return true;
    }
    tailscale_path_allowed(
        hbb_common::try_into_v4(peer).ip(),
        hbb_common::try_into_v4(local).ip(),
        is_on_tailscale_interface,
        route_source,
    )
}

// The direct listener's last bind attempt: 0 none yet, 1 listening, 2 failed (port in use).
static DIRECT_LISTENER: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Korean fork hook for upstream's direct listener; only ranode records it.
pub fn set_direct_listener(listening: bool) {
    if is_public() {
        DIRECT_LISTENER.store(
            if listening { 1 } else { 2 },
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}

/// For the left pane card: the direct port could not be opened (another app holds it), so this
/// app does not receive until it is started again with the port free.
pub fn direct_listener_failed() -> bool {
    DIRECT_LISTENER.load(std::sync::atomic::Ordering::Relaxed) == 2
}

/// Korean fork hook for upstream's direct listener. Upstream's `listen_any` sets `SO_REUSEPORT`,
/// so a second app binds 21118 too and gets nothing while the first takes every connection. In
/// ranode on macOS the bind is exclusive, so a port another app holds fails here (and the card
/// says so). Every other build uses `listen_any` unchanged.
pub async fn listen_direct(
    port: u16,
) -> hbb_common::ResultType<hbb_common::tokio::net::TcpListener> {
    #[cfg(target_os = "macos")]
    {
        if is_public() {
            return Ok(listen_exclusive(port)?);
        }
    }
    hbb_common::tcp::listen_any(port).await
}

// macOS opens IPv6 sockets dual-stack (net.inet6.ip6.v6only is 0), so [::] takes IPv4 too.
#[cfg(target_os = "macos")]
fn listen_exclusive(port: u16) -> std::io::Result<hbb_common::tokio::net::TcpListener> {
    use hbb_common::tokio::net::TcpSocket;
    let (socket, addr) = match TcpSocket::new_v6() {
        Ok(socket) => (socket, SocketAddr::from(([0u16; 8], port))),
        Err(_) => (TcpSocket::new_v4()?, SocketAddr::from(([0, 0, 0, 0], port))),
    };
    socket.set_reuseaddr(true)?;
    socket.bind(addr)?;
    socket.listen(128)
}

/// The password for `--set-permanent-password`: the first stdin line without its line ending.
/// `rest` is what follows the flag; a password there is refused so it never sits in the process
/// list. Refusal texts never contain the password.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn read_password_input(
    rest: &[String],
    stdin: &mut impl std::io::BufRead,
) -> Result<String, &'static str> {
    if !rest.is_empty() {
        return Err("Refused: pass the password on stdin, not as an argument.");
    }
    let mut line = String::new();
    if stdin.read_line(&mut line).is_err() {
        return Err("Refused: could not read the password from stdin.");
    }
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
    if line.is_empty() {
        return Err("Refused: the password is empty.");
    }
    Ok(line)
}

/// Exit code: 0 applied, 1 refused input, 2 the running app did not apply it.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn run_set_permanent_password(rest: &[String]) -> i32 {
    if crate::platform::is_root() {
        eprintln!("Refused: run as the logged-in user, not as root.");
        return 1;
    }
    let password = match read_password_input(rest, &mut std::io::stdin().lock()) {
        Ok(password) => password,
        Err(reason) => {
            eprintln!("{reason}");
            return 1;
        }
    };
    match crate::ipc::set_permanent_password(password) {
        Ok(()) => {
            println!("Done!");
            0
        }
        Err(e) => {
            let reason = e.to_string().replace(['\r', '\n'], " ");
            eprintln!("Password not set (is {APP_NAME} running?): {reason}");
            2
        }
    }
}

/// ranode-only CLI commands; `None` leaves `args` to upstream's handling.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn handle_cli(args: &[String]) -> Option<i32> {
    match args.split_first() {
        Some((cmd, rest)) if is_public() && cmd == "--set-permanent-password" => {
            Some(run_set_permanent_password(rest))
        }
        _ => None,
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
    fn incoming_is_pinned_to_tailscale_with_permanent_password() {
        let settings = override_settings();
        assert_eq!(
            settings.get(keys::OPTION_DIRECT_SERVER).map(String::as_str),
            Some("Y")
        );
        assert_eq!(
            settings.get(keys::OPTION_WHITELIST).map(String::as_str),
            Some("100.64.0.0/10,fd7a:115c:a1e0::/48")
        );
        assert_eq!(
            settings
                .get(keys::OPTION_VERIFICATION_METHOD)
                .map(String::as_str),
            Some("use-permanent-password")
        );
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    #[test]
    fn password_is_read_from_the_first_stdin_line_only() {
        let read = |input: &str| read_password_input(&[], &mut input.as_bytes());
        assert_eq!(read("s3cret pw \n"), Ok("s3cret pw ".to_owned()));
        assert_eq!(read(" s3cret\r\nnext\n"), Ok(" s3cret".to_owned()));
        assert_eq!(read("s3cret"), Ok("s3cret".to_owned()));
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    #[test]
    fn password_on_the_command_line_or_empty_is_refused() {
        let argv = ["s3cret".to_owned()];
        assert!(read_password_input(&argv, &mut "s3cret\n".as_bytes()).is_err());
        assert!(read_password_input(&[], &mut "".as_bytes()).is_err());
        assert!(read_password_input(&[], &mut "\n".as_bytes()).is_err());
        assert!(read_password_input(&[], &mut "\r\n".as_bytes()).is_err());
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

    #[test]
    fn tailscale_peers_are_trusted_only_on_the_tailscale_interface() {
        let ip = |s: &str| s.parse::<IpAddr>().unwrap();
        let utun = |a: IpAddr| a == ip("100.101.2.3") || a == ip("fd7a:115c:a1e0::3");
        // This Mac sits on a hotel LAN 100.70.0.0/16 (address 100.70.0.3) and in a tailnet.
        let route = |peer: IpAddr| {
            Some(match peer {
                IpAddr::V4(v4) if v4.octets()[..2] == [100, 70] => ip("100.70.0.3"),
                IpAddr::V4(v4) if v4.octets()[0] == 100 => ip("100.101.2.3"),
                IpAddr::V4(_) => ip("192.168.0.5"),
                IpAddr::V6(_) => ip("fd7a:115c:a1e0::3"),
            })
        };
        let allowed =
            |peer: &str, local: &str| tailscale_path_allowed(ip(peer), ip(local), utun, route);
        // Tailscale peers to this Mac's Tailscale address on its Tailscale interface.
        assert!(allowed("100.64.0.9", "100.101.2.3"));
        assert!(allowed("fd7a:115c:a1e0::9", "fd7a:115c:a1e0::3"));
        // A 100.64.0.0/10 source on the LAN, to the LAN address or to the Tailscale address.
        assert!(!allowed("100.70.0.9", "100.70.0.3"));
        assert!(!allowed("100.70.0.9", "100.101.2.3"));
        assert!(!allowed("100.64.0.9", "192.168.0.5"));
        assert!(!allowed("fd7a:115c:a1e0::9", "2001:db8::5"));
        // Outside the Tailscale range this check has no say; the whitelist decides.
        assert!(allowed("192.168.0.9", "192.168.0.5"));
        assert!(!in_tailscale_range(ip("100.128.0.1")));
        assert!(!in_tailscale_range(ip("100.63.255.255")));
        assert!(!in_tailscale_range(ip("fd7a:115c:a1e1::1")));
        let addrs = [
            ("en0", "100.70.0.3"),
            ("utun4", "100.101.2.3"),
            ("utun4", "fd7a:115c:a1e0::3"),
            // Cloudflare WARP: a CGNAT address on a tunnel without a Tailscale IPv6 address.
            ("utun6", "100.96.0.7"),
            ("tailscale0", "100.64.0.20"),
            ("tailscale0", "fd7a:115c:a1e0::20"),
        ]
        .map(|(name, a)| (name.to_owned(), ip(a)));
        assert!(is_tailscale_interface_address(&addrs, ip("100.101.2.3")));
        assert!(is_tailscale_interface_address(
            &addrs,
            ip("fd7a:115c:a1e0::3")
        ));
        assert!(is_tailscale_interface_address(&addrs, ip("100.64.0.20")));
        assert!(!is_tailscale_interface_address(&addrs, ip("100.70.0.3")));
        assert!(!is_tailscale_interface_address(&addrs, ip("100.96.0.7")));
        // getifaddrs decoding, byte order included: every host has 127.0.0.1.
        #[cfg(unix)]
        assert!(interface_addresses()
            .iter()
            .any(|(_, a)| *a == ip("127.0.0.1")));
    }
}
