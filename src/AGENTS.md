This file is written in English only.

# src/ — fork-owned Rust

## Scope
Fork modules (registered in `lib.rs` and `platform/mod.rs`): `hangul_key.rs` (peer 한/영 mapping), `public_flavor.rs` (ranode identity, pinned
options, cleanup, discovery probe), `fork_update.rs` (fork release check; std-only because
`build.rs` includes it), `platform/macos_input_source.rs` and the fork parts of `platform/macos.mm`
(TIS switch, HID shortcut post, local hotkey pause, Fn flag fix). Hooks in upstream files:
`keyboard.rs`, `server/input_service.rs`, `updater.rs`, `common.rs` (`is_rustdesk`,
`is_custom_client`), `core_main.rs` (`public_flavor::init()` first), `platform/macos.rs` (service
refusal, hotkey tests), `flutter_ffi.rs` (`main_is_public_flavor`, `main_public_probe_rustdesk`,
`main_public_direct_port_busy`,
`session_set_remote_keyboard_active`, `main_init_input_source` which restores the Mac's hotkeys
after a crash, the `local-hangul-keys-status` option), `server/connection.rs` (`on_open`:
`public_flavor::incoming_path_allowed` after the whitelist), `rendezvous_mediator.rs`
(`direct_server`: `public_flavor::listen_direct` in place of `listen_any`, and
`public_flavor::set_direct_listener` on bind success and failure), the fork keys appended to
every `lang/*.rs`,
`platform/privileges_scripts/agent.plist` (`KeepAlive = true`).

Not this module's scope: upstream's protocol, codecs, server services, `libs/hbb_common`
(submodule, never edited), the `src/lang/*` key set beyond adding fork keys.

## Invariants
- `public_flavor::init()` runs before anything reads config; it sets `APP_NAME = RustDeskPublic`
  and the overrides. A main-app build never enters it (`is_public()` is a compile-time false).
- `override_settings()` holds every ranode setting the user must not change (`direct-server=Y`,
  whitelist `100.64.0.0/10,fd7a:115c:a1e0::/48`, `use-permanent-password`); overrides are not saved and win over saved
  values.
- `handle_cli` handles only `--set-permanent-password` in ranode: the password comes from stdin,
  never argv; root is refused; exit 0 / 1 (refused input) / 2 (ranode did not apply it).
- `incoming_path_allowed` (ranode only, every incoming path): a peer address in 100.64.0.0/10 or
  fd7a:115c:a1e0::/48 is accepted only when the accepted socket's local address is in the same
  ranges, assigned to a tunnel interface (`utun*`, `tailscale*`) that also carries a
  fd7a:115c:a1e0::/48 address, and is the source address of the route back to the peer
  (decision 0009). It never widens what the whitelist allows. Interfaces come from getifaddrs
  (`interface_addresses`), not `default_net`, which also probes gateways.
- `listen_direct`: ranode on macOS binds 21118 without `SO_REUSEPORT`, so a port another app
  holds fails (card: busy); every other build calls upstream's `listen_any`.
- `cleanup_once()` touches only own-server keys and ids containing `@rustdesk.internal.kr`, once.
- `probe_rustdesk` accepts a pong only from the probed IP and never blocks past its timeout.
- `fork_update::is_newer_release` compares `(version, ko)` tuples; a non-fork tag is `None`, never
  "newer". `HAS_MACOS_UPDATE_DMG` stays false while macOS ships zips.
- Windows → Windows and Linux → Linux key events go through upstream's path untouched in every
  mode; Mac → Mac turns the input-source shortcut into F18 (Hangul in legacy mode); nothing is
  replaced when `enable-peer-hangul-key` is off.
- `macos_input_source` never injects the 한/영 key-up and switches once per press.

## Patterns
- New fork option keys: constant in `libs/base/src/config/keys.rs`, added to the key lists there.
- Platform code lives in `platform/`; shared files get one-line hooks.
- No `unwrap()`/`expect()` outside tests; log per-event paths at `trace`.

## Tests
- `hangul_key` (26 tests): every OS pair in the mapping table, exact-match shortcut rule, modifier
  release/re-press, hold-once.
- `macos_input_source`, `platform::macos` hotkey-flag and agent-plist tests run only on macOS.
- `public_flavor`: overrides pinned to `Y`, `100.64.0.0/10,fd7a:115c:a1e0::/48` and the permanent
  password; internal-id detection; pong origin.
- `public_flavor` interface rule: a Tailscale-range peer on a non-Tailscale local address (LAN that
  hands out 100.64.0.0/10) or with its route back over the LAN is refused; a CGNAT `utun` without
  a Tailscale IPv6 address is not Tailscale; IPv4 and IPv6 Tailscale paths pass; getifaddrs
  decoding (loopback).
- `fork_update`/`updater`: tag parsing, newer-than, URLs limited to this repo's releases.
- Edge cases to cover when changing: Mac with no Korean input source; shortcut turned off; legacy
  keyboard mode; tag with an upstream version change.
