//! Korean fork: the "RustDesk Public" build flavor, chosen at build time with
//! `RUSTDESK_FLAVOR=public` (build.rs turns it into `RUSTDESK_BUILD_FLAVOR`).
//!
//! A separate app for reaching peers through the public RustDesk server. It runs under its own
//! app name, so its config, socket and log folders never mix with the main app's, and it only
//! accepts incoming connections from the Tailscale range. A build without the flavor never
//! enters any of this.

use base::config::keys;
use hbb_common::config::{self, Config, LocalConfig, PeerConfig};
use std::collections::HashMap;

pub const APP_NAME: &str = "RustDeskPublic";
const TAILSCALE_RANGE: &str = "100.64.0.0/10";
const INTERNAL_ID_DOMAIN: &str = "@rustdesk.internal.kr";
const CLEANUP_DONE_OPTION: &str = "public-flavor-cleanup-done";

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
}
