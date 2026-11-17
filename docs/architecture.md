This file is written in English only.

# Architecture (fork-owned parts)

Upstream RustDesk 1.5.0 is the base: a Rust core (`src/`, `libs/`) compiled into a library that the
Flutter UI (`flutter/`) calls through flutter_rust_bridge (`src/flutter_ffi.rs`). The fork adds
the components below on top of it.

## Component map

| Component | Where | Role | Depends on |
|---|---|---|---|
| Peer 한/영 key mapping | `src/hangul_key.rs`, hooks in `src/keyboard.rs` | Controlling side: replaces the local 한/영 key (or the Mac's input-source shortcut) with the peer OS's own 한/영 key before it is sent | upstream key event pipeline (`rdev`), option `enable-peer-hangul-key` |
| macOS input-source switch | `src/platform/macos_input_source.rs`, `src/platform/macos.mm`, hook in `src/server/input_service.rs` | Controlled Mac: an arriving F18 / Hangul / LANG1 key triggers the Mac's own "previous input source" shortcut at HID level, or a direct TIS switch | option `enable-input-source-toggle`, macOS TIS / CGEvent |
| Local hotkey pause | `src/platform/macos.mm`, `src/platform/macos_input_source.rs` | Controlling Mac: turns off its own input-source shortcuts while a remote window holds the keyboard, restores them after | macOS symbolic hotkeys |
| Android Korean soft keyboard | `flutter/lib/models/soft_keyboard_input_utils.dart`, `hangul_dubeolsik.dart`, `korean_toggle_key.dart`, `flutter/lib/mobile/pages/remote_page.dart` | Phone: diffs keyboard edits by grapheme, sends 두벌식 keys + F19/F24 to Linux, 한/영 button | upstream input model |
| Fork update check | `src/fork_update.rs`, edits in `src/updater.rs`, `build.rs` | Compares this build's ko number with the fork's GitHub "Latest" release | GitHub releases API |
| Build flavor | `build.rs` (`RUSTDESK_BUILD_FLAVOR`), `build.py` (`RUSTDESK_FLAVOR`, `Flavor.xcconfig`), `flutter/macos/Runner/Configs/AppInfo.xcconfig` | Chooses main app or ranode at build time | — |
| ranode core | `src/public_flavor.rs`, gates in `src/common.rs`, `src/platform/macos.rs`, `src/core_main.rs` | App name `RustDeskPublic`, pinned options, no service install, one-time cleanup, LAN discovery probe | `hbb_common::config` |
| ranode home | `flutter/lib/desktop/pages/public_home.dart`, `public_onboarding.dart`, `flutter/lib/common/public_onprem.dart`, `public_login.dart`, `tailscale_range.dart` | On-prem tab (Tailscale devices) and internet tab (public server), first-run guide | Tailscale module, ranode core via FFI |
| Tailscale module | `flutter/lib/common/tailscale.dart` | Runs the local `tailscale` CLI (status, profiles, switch), probes TCP 21118 on peers | the Tailscale app / CLI on the same Mac |
| Release tooling | `res/osx-sign-notarize.sh`, `res/android-sign-release.sh`, `.github/workflows/build-desktop-ko.yml`, `flutter-unit-test.yml` | Sign/notarize macOS, sign Android, build Windows/Linux in CI | owner's signing keys, GitHub Actions |

Dependency direction: fork Flutter code → FFI (`mainIsPublicFlavor`, `mainPublicProbeRustdesk`) → fork
Rust modules → upstream. Upstream code calls fork code only through one-line hooks.

## The two macOS apps

One source tree, two builds. `RUSTDESK_FLAVOR=public` is read by `build.py` (writes
`flutter/macos/Runner/Configs/Flavor.xcconfig`, copies the globe icon) and by `build.rs` (emits
`RUSTDESK_BUILD_FLAVOR`, so `public_flavor::is_public()` is a compile-time constant). Unset → main app.

| | Main app | ranode |
|---|---|---|
| Display name | "RustDesk (for korean)" (localized display name added by `build.py` after the build) | "RustDesk (forked ranode)" |
| Bundle file | `/Applications/RustDesk.app` | `/Applications/RustDesk (forked ranode).app` |
| Bundle id / app name / scheme | `com.carriez.rustdesk` / `RustDesk` / `rustdesk` | `net.ranode.rustdesk-public` / `RustDeskPublic` / `rustdeskpublic` |
| Config, IPC socket, logs | under `RustDesk` | under `RustDeskPublic` (never shared) |
| Rendezvous server | the owner's own RustDesk server, reachable only inside Tailscale | public RustDesk server (internet tab) |
| Incoming | yes; direct port 21118 + whitelist 100.64.0.0/10 set on each Mac with `sudo RustDesk --option` | from ko.23 yes, in its own GUI process: pinned `direct-server=Y`, whitelist 100.64.0.0/10, permanent password only; until ko.22 no |
| Service (LaunchDaemon/Agent) | upstream's service can be installed per Mac; the owner does not need login-screen access | refused in code |

Both use TCP 21118 (direct access) and UDP 21119 (LAN discovery) when they listen, so only one of
them can listen at a time.

## Representative flow: on-prem connect from ranode

1. `public_home.dart` asks `Tailscale.read()`; `tailscale.dart` finds the binary
   (`/Applications/Tailscale.app/Contents/MacOS/Tailscale`, then `PATH`, then `/opt/homebrew/bin`,
   `/usr/local/bin`) and runs `status --json` and `switch --list` with a 5 s timeout.
2. The JSON becomes `TailscaleState` (`notFound`, `notResponding`, `stopped`, `needsLogin`,
   `needsApproval`, `running`) plus devices and profiles.
3. For every device with an IPv4 the tab opens TCP to `<tailscale ip>:21118` (1.5 s); to each
   online one it also sends a UDP discovery ping to 21119 through `mainPublicProbeRustdesk` to read the peer's RustDesk ID and hostname.
4. The device lands in a section ("바로 접속", "직접 접속 꺼짐", "확인 중", "RustDesk 없음", "오프라인").
5. A click connects to the device's current Tailscale IP as the peer id; the peer accepts because
   the IP is inside its whitelist and its direct server is on.

## Boundaries

- What crosses into the owner's devices: only connections from 100.64.0.0/10 to port 21118, or via
  the owner's server inside Tailscale. ranode stays registered with the public RustDesk server, but
  a peer reaching it that way has a non-Tailscale address and is refused by the whitelist.
- ranode does not answer LAN discovery on UDP 21119 (upstream starts that listener only for an
  "installed" app), so other Macs find a ranode-only Mac through TCP 21118 alone.
- The Tailscale module never talks to Tailscale's servers or LocalAPI; it only runs the local CLI.
- Release keys never enter GitHub Actions; CI builds Windows/Linux unsigned and runs Flutter tests.
