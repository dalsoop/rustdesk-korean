//! Korean fork: the "RustDesk Public" build flavor, chosen at build time with
//! `RUSTDESK_FLAVOR=public` (build.rs turns it into `RUSTDESK_BUILD_FLAVOR`).
//!
//! A separate app for reaching peers through the public RustDesk server. It runs under its own
//! app name, so its config, socket and log folders never mix with the main app's. It accepts
//! incoming connections only from the Tailscale range plus the private subnets set per Mac with
//! `--set-incoming-policy`, and only with its permanent password (set with
//! `--set-permanent-password`). A build without the flavor never enters any of this.

use base::config::keys;
use hbb_common::{
    config::{self, Config, LocalConfig, PeerConfig, RENDEZVOUS_PORT},
    log,
    protobuf::Message as _,
    rendezvous_proto::{rendezvous_message, PeerDiscovery, RendezvousMessage},
};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::{Duration, Instant},
};

pub const APP_NAME: &str = "RustDeskPublic";
// Tailscale's IPv4 and IPv6 ranges, in the whitelist's comma-separated form.
const TAILSCALE_RANGE: &str = "100.64.0.0/10,fd7a:115c:a1e0::/48";
const INTERNAL_ID_DOMAIN: &str = "@rustdesk.internal.kr";
const CLEANUP_DONE_OPTION: &str = "public-flavor-cleanup-done";
pub const DISCOVERY_PROBE_TIMEOUT: Duration = Duration::from_millis(1500);
const EXTRA_SUBNETS_OPTION: &str = "public-incoming-extra-subnets";
const APPROVE_MODE_OPTION: &str = "public-incoming-approve-mode";
#[cfg(not(any(target_os = "android", target_os = "ios")))]
const MAX_EXTRA_SUBNETS: usize = 8;
const PRIVATE_RANGES: [(Ipv4Addr, u8); 3] = [
    (Ipv4Addr::new(10, 0, 0, 0), 8),
    (Ipv4Addr::new(172, 16, 0, 0), 12),
    (Ipv4Addr::new(192, 168, 0, 0), 16),
];

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
    build_overrides(
        &LocalConfig::get_option(EXTRA_SUBNETS_OPTION),
        &LocalConfig::get_option(APPROVE_MODE_OPTION),
    )
}

/// The fixed floor plus the per-Mac policy stored by `--set-incoming-policy`. A stored entry that
/// is not valid is skipped, so a hand-edited value can never weaken the floor.
fn build_overrides(extra_subnets: &str, approve_mode: &str) -> HashMap<String, String> {
    // Upstream splits the whitelist on ',' without trimming, so entries are joined without spaces.
    let whitelist = std::iter::once(TAILSCALE_RANGE.to_owned())
        .chain(extra_subnets.split(',').filter_map(parse_private_subnet))
        .collect::<Vec<_>>()
        .join(",");
    let mut overrides = HashMap::from([
        (keys::OPTION_DIRECT_SERVER.to_owned(), "Y".to_owned()),
        (keys::OPTION_WHITELIST.to_owned(), whitelist),
        (
            keys::OPTION_VERIFICATION_METHOD.to_owned(),
            "use-permanent-password".to_owned(),
        ),
        (
            keys::OPTION_ALLOW_REMOTE_CONFIG_MODIFICATION.to_owned(),
            "N".to_owned(),
        ),
    ]);
    if approve_mode == "password" {
        overrides.insert(keys::OPTION_APPROVE_MODE.to_owned(), "password".to_owned());
    }
    overrides
}

fn ipv4_mask(prefix: u8) -> u32 {
    if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - u32::from(prefix))
    }
}

/// `a.b.c.d/n` without host bits and wholly inside 10/8, 172.16/12 or 192.168/16, normalized.
fn parse_private_subnet(entry: &str) -> Option<String> {
    let (addr, prefix) = entry.trim().split_once('/')?;
    let addr: Ipv4Addr = addr.parse().ok()?;
    let prefix: u8 = prefix.parse().ok().filter(|n| *n <= 32)?;
    let bits = u32::from(addr);
    if bits & ipv4_mask(prefix) != bits {
        return None;
    }
    PRIVATE_RANGES
        .iter()
        .any(|(base, len)| prefix >= *len && bits & ipv4_mask(*len) == u32::from(*base))
        .then(|| format!("{addr}/{prefix}"))
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[derive(Debug, Default, PartialEq)]
struct IncomingPolicyArgs {
    /// `Some(empty)` clears the extras (`none`).
    extra_subnets: Option<Vec<String>>,
    approve_mode: Option<&'static str>,
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn parse_extra_subnets(list: &str) -> Result<Vec<String>, String> {
    if list.trim() == "none" {
        return Ok(Vec::new());
    }
    let entries: Vec<&str> = list.split(',').map(str::trim).collect();
    if entries.len() > MAX_EXTRA_SUBNETS {
        return Err(format!("at most {MAX_EXTRA_SUBNETS} subnets."));
    }
    entries
        .into_iter()
        .map(|e| {
            parse_private_subnet(e).ok_or_else(|| {
                format!("'{e}' is not a private IPv4 network (10/8, 172.16/12, 192.168/16).")
            })
        })
        .collect()
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn parse_incoming_policy_args(rest: &[String]) -> Result<IncomingPolicyArgs, String> {
    let mut args = IncomingPolicyArgs::default();
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        let value = match flag.as_str() {
            "--extra-subnets" | "--approve-mode" => it
                .next()
                .ok_or_else(|| format!("{flag} needs a value."))?,
            _ => return Err(format!("unknown argument '{flag}'.")),
        };
        if flag == "--extra-subnets" {
            if args.extra_subnets.is_some() {
                return Err(format!("{flag} given twice."));
            }
            args.extra_subnets = Some(parse_extra_subnets(value)?);
        } else {
            if args.approve_mode.is_some() {
                return Err(format!("{flag} given twice."));
            }
            args.approve_mode = Some(match value.as_str() {
                "password" => "password",
                "both" => "both",
                _ => return Err(format!("{flag} takes password or both.")),
            });
        }
    }
    if args == IncomingPolicyArgs::default() {
        return Err("give --extra-subnets and/or --approve-mode.".to_owned());
    }
    Ok(args)
}

fn incoming_policy_line(overrides: &HashMap<String, String>) -> String {
    let whitelist = overrides
        .get(keys::OPTION_WHITELIST)
        .map(String::as_str)
        .unwrap_or(TAILSCALE_RANGE);
    let approve_mode = match overrides.get(keys::OPTION_APPROVE_MODE).map(String::as_str) {
        Some("password") => "password",
        _ => "both",
    };
    format!("whitelist={whitelist} approve-mode={approve_mode}")
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
/// unlike a full interface listing that also looks up gateways. Not on Android, whose libc has it
/// only from API 24 (the app targets older ones); ranode is a macOS app.
#[cfg(any(target_os = "macos", target_os = "linux"))]
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

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
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

/// Whether `ip` lies in `entry`, an entry that `parse_private_subnet` accepts.
fn in_private_subnet(entry: &str, ip: IpAddr) -> bool {
    let (IpAddr::V4(v4), Some(net)) = (ip, parse_private_subnet(entry)) else {
        return false;
    };
    match net.split_once('/').map(|(a, p)| (a.parse::<Ipv4Addr>(), p.parse::<u8>())) {
        Some((Ok(addr), Ok(prefix))) => u32::from(v4) & ipv4_mask(prefix) == u32::from(addr),
        _ => false,
    }
}

/// Whether `ip` is held by an interface in `addrs` that is not a tunnel (`utun*`, `tailscale*`,
/// and macOS's IKEv2/L2TP `ipsec*` and `ppp*`).
fn is_lan_interface_address(addrs: &[(String, IpAddr)], ip: IpAddr) -> bool {
    addrs.iter().any(|(name, a)| {
        *a == ip
            && !is_tailscale_interface_name(name)
            && !name.starts_with("ipsec")
            && !name.starts_with("ppp")
    })
}

/// The office-subnet counterpart of `tailscale_path_allowed` (ko.24): a peer address in one of the
/// private subnets the whitelist lists for this Mac is trusted only when the connection reached an
/// address of this computer inside that same subnet, held by a non-tunnel interface (the office
/// LAN itself, not a VPN or a Tailscale subnet route), and the way back to the peer runs through
/// that address. Other peers are left to the whitelist and the Tailscale rule.
fn office_path_allowed(
    peer: IpAddr,
    local: IpAddr,
    subnets: &[String],
    on_lan_interface: impl Fn(IpAddr) -> bool,
    route_source: impl Fn(IpAddr) -> Option<IpAddr>,
) -> bool {
    let listed: Vec<&String> = subnets
        .iter()
        .filter(|s| in_private_subnet(s, peer))
        .collect();
    listed.is_empty()
        || (listed.iter().any(|s| in_private_subnet(s, local))
            && on_lan_interface(local)
            && route_source(peer) == Some(local))
}

/// Korean fork hook for upstream's relay path (`server::create_relay_connection_`). A relayed
/// peer's address is only what the rendezvous message claims; this computer never sees it, and the
/// local address is its own socket to the relay, so neither the whitelist nor the path rules above
/// can check it. ranode admits direct paths only and refuses every relay. `true` outside ranode.
pub fn relayed_incoming_allowed() -> bool {
    !is_public()
}

/// Korean fork hook for `Connection::on_open`, next to the whitelist check. `true` outside ranode.
pub fn incoming_path_allowed(peer: SocketAddr, local: SocketAddr) -> bool {
    if !is_public() {
        return true;
    }
    let (peer, local) = (
        hbb_common::try_into_v4(peer).ip(),
        hbb_common::try_into_v4(local).ip(),
    );
    // The enforced whitelist (pinned in `init`); its private entries are the office subnets.
    let subnets: Vec<String> = Config::get_option(keys::OPTION_WHITELIST)
        .split(',')
        .filter_map(parse_private_subnet)
        .collect();
    tailscale_path_allowed(peer, local, is_on_tailscale_interface, route_source)
        && office_path_allowed(
            peer,
            local,
            &subnets,
            |ip| is_lan_interface_address(&interface_addresses(), ip),
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

// One dual-stack [::] socket takes IPv4 too, with SO_REUSEADDR but never SO_REUSEPORT.
#[cfg(target_os = "macos")]
fn listen_exclusive(port: u16) -> std::io::Result<hbb_common::tokio::net::TcpListener> {
    use hbb_common::{libc, tokio::net::TcpSocket};
    use std::os::unix::io::AsRawFd;
    let (socket, addr) = match TcpSocket::new_v6() {
        Ok(socket) => {
            // Dual-stack whatever net.inet6.ip6.v6only says, like upstream's listen_any.
            let off: libc::c_int = 0;
            // SAFETY: a valid socket fd and a c_int option value of the size passed.
            unsafe {
                libc::setsockopt(
                    socket.as_raw_fd(),
                    libc::IPPROTO_IPV6,
                    libc::IPV6_V6ONLY,
                    &off as *const libc::c_int as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                );
            }
            (socket, SocketAddr::from(([0u16; 8], port)))
        }
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

/// Only the running ranode's GUI process answers its main IPC: ranode installs no service.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
#[hbb_common::tokio::main(flavor = "current_thread", crate = "hbb_common::tokio")]
async fn is_ranode_running() -> bool {
    crate::ipc::connect(300, "").await.is_ok()
}

/// Exit code: 0 saved, 1 refused input or root, 2 ranode is running.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn run_set_incoming_policy(rest: &[String]) -> i32 {
    if crate::platform::is_root() {
        eprintln!("Refused: run as the logged-in user, not as root.");
        return 1;
    }
    let args = match parse_incoming_policy_args(rest) {
        Ok(args) => args,
        Err(reason) => {
            eprintln!("Refused: {reason}");
            return 1;
        }
    };
    // A running ranode keeps its own copy of the local settings and would write it back.
    if is_ranode_running() {
        eprintln!("Refused: quit {APP_NAME} first, then run this again.");
        return 2;
    }
    if let Some(subnets) = args.extra_subnets {
        LocalConfig::set_option(EXTRA_SUBNETS_OPTION.to_owned(), subnets.join(","));
    }
    if let Some(mode) = args.approve_mode {
        LocalConfig::set_option(APPROVE_MODE_OPTION.to_owned(), mode.to_owned());
    }
    println!(
        "{}",
        incoming_policy_line(&build_overrides(
            &LocalConfig::get_option(EXTRA_SUBNETS_OPTION),
            &LocalConfig::get_option(APPROVE_MODE_OPTION),
        ))
    );
    0
}

/// ranode-only CLI commands; `None` leaves `args` to upstream's handling.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn handle_cli(args: &[String]) -> Option<i32> {
    match args.split_first() {
        Some((cmd, rest)) if is_public() && cmd == "--set-permanent-password" => {
            Some(run_set_permanent_password(rest))
        }
        Some((cmd, rest)) if is_public() && cmd == "--set-incoming-policy" => {
            Some(run_set_incoming_policy(rest))
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
        let settings = build_overrides("", "");
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
        assert_eq!(
            settings
                .get(keys::OPTION_ALLOW_REMOTE_CONFIG_MODIFICATION)
                .map(String::as_str),
            Some("N")
        );
        assert_eq!(settings.get(keys::OPTION_APPROVE_MODE), None);
        assert_eq!(
            incoming_policy_line(&settings),
            "whitelist=100.64.0.0/10,fd7a:115c:a1e0::/48 approve-mode=both"
        );
    }

    #[test]
    fn only_private_ipv4_networks_are_accepted_as_extra_subnets() {
        for ok in ["10.0.0.0/24", "172.16.5.0/24", "192.168.1.0/24", " 10.0.0.0/8 "] {
            assert!(parse_private_subnet(ok).is_some(), "{ok}");
        }
        for bad in [
            "8.8.8.0/24",
            "0.0.0.0/0",
            "100.64.0.0/10",
            "10.0.0.1/24",
            "fd00::/8",
            "",
            "10.0.0.0",
            "10.0.0.0/33",
            "172.0.0.0/8",
            "192.168.0.0/15",
        ] {
            assert_eq!(parse_private_subnet(bad), None, "{bad}");
        }
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    #[test]
    fn incoming_policy_flags_are_parsed_strictly() {
        let parse = |argv: &[&str]| {
            parse_incoming_policy_args(&argv.iter().map(|a| a.to_string()).collect::<Vec<_>>())
        };
        assert_eq!(
            parse(&[
                "--extra-subnets",
                "10.0.0.0/24, 192.168.1.0/24",
                "--approve-mode",
                "password",
            ]),
            Ok(IncomingPolicyArgs {
                extra_subnets: Some(vec!["10.0.0.0/24".to_owned(), "192.168.1.0/24".to_owned()]),
                approve_mode: Some("password"),
            })
        );
        assert_eq!(
            parse(&["--extra-subnets", "none"]),
            Ok(IncomingPolicyArgs {
                extra_subnets: Some(Vec::new()),
                approve_mode: None,
            })
        );
        assert!(parse(&[]).is_err());
        assert!(parse(&["--whitelist", "10.0.0.0/24"]).is_err());
        assert!(parse(&["--approve-mode", "both", "--approve-mode", "both"]).is_err());
        assert!(parse(&["--approve-mode", "click"]).is_err());
        assert!(parse(&["--extra-subnets"]).is_err());
        assert!(parse(&["--extra-subnets", ""]).is_err());
        assert!(parse(&["--extra-subnets", "10.0.0.0/24,8.8.8.0/24"]).is_err());
        let nine = vec!["10.0.0.0/24"; 9].join(",");
        assert!(parse(&["--extra-subnets", &nine]).is_err());
    }

    #[test]
    fn stored_policy_extends_the_floor_and_invalid_entries_are_ignored() {
        let office = build_overrides("10.0.0.0/24", "password");
        assert_eq!(
            office.get(keys::OPTION_WHITELIST).map(String::as_str),
            Some("100.64.0.0/10,fd7a:115c:a1e0::/48,10.0.0.0/24")
        );
        assert_eq!(
            office.get(keys::OPTION_APPROVE_MODE).map(String::as_str),
            Some("password")
        );
        assert_eq!(
            office.get(keys::OPTION_DIRECT_SERVER).map(String::as_str),
            Some("Y")
        );
        assert_eq!(
            incoming_policy_line(&office),
            "whitelist=100.64.0.0/10,fd7a:115c:a1e0::/48,10.0.0.0/24 approve-mode=password"
        );
        let edited = build_overrides("0.0.0.0/0, 192.168.1.0/24 ,8.8.8.0/24", "both");
        assert_eq!(
            edited.get(keys::OPTION_WHITELIST).map(String::as_str),
            Some("100.64.0.0/10,fd7a:115c:a1e0::/48,192.168.1.0/24")
        );
        assert_eq!(edited.get(keys::OPTION_APPROVE_MODE), None);
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
        // ko.24 office subnet 192.168.0.0/24, held by en1 (192.168.0.5); a VPN tunnel has 10.8.0.2.
        let office = ["192.168.0.0/24".to_owned()];
        let lan = |a: IpAddr| a == ip("192.168.0.5");
        let office_route = |peer: IpAddr| {
            Some(match peer {
                IpAddr::V4(v4) if v4.octets()[..3] == [192, 168, 0] => ip("192.168.0.5"),
                _ => ip("10.8.0.2"),
            })
        };
        let office_allowed = |peer: &str, local: &str| {
            office_path_allowed(ip(peer), ip(local), &office, lan, office_route)
        };
        assert!(office_allowed("192.168.0.9", "192.168.0.5"));
        // An office-range source reaching another address of this Mac is refused.
        assert!(!office_allowed("192.168.0.9", "100.101.2.3"));
        assert!(!office_allowed("192.168.0.9", "10.8.0.2"));
        assert!(!office_path_allowed(
            ip("192.168.0.9"),
            ip("192.168.0.5"),
            &office,
            lan,
            |_| Some(ip("10.8.0.2"))
        ));
        assert!(!office_path_allowed(
            ip("192.168.0.9"),
            ip("192.168.0.5"),
            &office,
            |_| false,
            office_route
        ));
        // Peers outside the listed subnets are left to the whitelist and the Tailscale rule.
        assert!(office_allowed("100.64.0.9", "100.101.2.3"));
        assert!(office_path_allowed(ip("192.168.0.9"), ip("10.8.0.2"), &[], lan, office_route));
        let ifs = [
            ("en1", "192.168.0.5"),
            ("utun7", "10.8.0.2"),
            ("ipsec0", "10.9.0.2"),
        ]
        .map(|(name, a)| (name.to_owned(), ip(a)));
        assert!(is_lan_interface_address(&ifs, ip("192.168.0.5")));
        assert!(!is_lan_interface_address(&ifs, ip("10.8.0.2")));
        assert!(!is_lan_interface_address(&ifs, ip("10.9.0.2")));
        // getifaddrs decoding, byte order included: every host has 127.0.0.1.
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        assert!(interface_addresses()
            .iter()
            .any(|(_, a)| *a == ip("127.0.0.1")));
    }
}
