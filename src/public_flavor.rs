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
const TAILSCALE_RANGE: &str = "100.64.0.0/10";
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
            Some("100.64.0.0/10")
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
            "whitelist=100.64.0.0/10 approve-mode=both"
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
            Some("100.64.0.0/10,10.0.0.0/24")
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
            "whitelist=100.64.0.0/10,10.0.0.0/24 approve-mode=password"
        );
        let edited = build_overrides("0.0.0.0/0, 192.168.1.0/24 ,8.8.8.0/24", "both");
        assert_eq!(
            edited.get(keys::OPTION_WHITELIST).map(String::as_str),
            Some("100.64.0.0/10,192.168.1.0/24")
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
}
