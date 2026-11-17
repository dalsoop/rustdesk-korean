This file is written in English only.

# Contracts

Consumers: the owner's machine-setup scripts, the office server's infrastructure repo, the apps'
own update check, and people downloading from the Releases page.

## Release assets (GitHub `dalsoop/rustdesk-korean`, tag `1.5.0-ko.N`)

| Asset | Content | Consumer expectation |
|---|---|---|
| `rustdesk-1.5.0-ko.N-macos-aarch64.zip` | `RustDesk.app` (main app), notarized, stapled | unzip with `ditto -x -k`; top-level entry is exactly `RustDesk.app` |
| `rustdesk-public-1.5.0-ko.N-macos-aarch64.zip` | ranode, notarized | top-level entry is exactly `RustDesk (forked ranode).app` (until ko.16 `RustDesk Public.app`) |
| `rustdesk-1.5.0-ko.N-aarch64.apk` | Android arm64, pinned certificate | installs over any earlier fork APK |
| `rustdesk-1.5.0-ko.N-x86_64.exe`, `rustdesk-1.5.0-ko.N-x86_64.msi` | Windows x64 from CI | exact names: the Windows updater downloads `rustdesk-<tag>-x86_64.{exe,msi}` |
| `rustdesk-1.5.0-ko.N-x86_64.deb` | Linux x64 from CI | package version `1.5.0+koN` |
| `<asset>.sha256` | `shasum -a 256` line for each asset | consumers compare and refuse a mismatch |

Errors: a missing asset or wrong sha256 means the consumer must not install; asset names never
change once published.

## Update check

- Input: `GET https://api.github.com/repos/dalsoop/rustdesk-korean/releases/latest`.
- Processing: tag parsed as `<version>-ko.<N>`; newer when `(version, N)` > the build's
  (`RUSTDESK_KO_RELEASE` from `flutter/pubspec.yaml`).
- Output: Windows downloads `…/releases/download/<tag>/rustdesk-<tag>-x86_64.{exe,msi}`; other
  platforms show `https://github.com/dalsoop/rustdesk-korean/releases/tag/<tag>`.
- Errors: a tag that does not parse, a prerelease/draft, or a download URL outside this repo's
  releases, or a `html_url` other than `…/releases/tag/<tag>` → no update offered.

## Command-line options (run with `sudo` for the main app's root config)

| Option | Values | Default | Effect |
|---|---|---|---|
| `enable-input-source-toggle` | Y/N | Y | controlled Mac switches input source on F18/Hangul/LANG1 |
| `direct-server` | Y/N | N | main app: Y on the owner's Macs; ranode: pinned Y from ko.23 (N before), and `--option` refuses on ranode (see below) |
| `verification-method` | `use-temporary-password` / `use-permanent-password` / both | both | ranode: pinned `use-permanent-password` from ko.23 |
| `whitelist` | CIDR list | empty | main app: `100.64.0.0/10` on the owner's Macs; ranode: pinned to `100.64.0.0/10` + the policy's extra subnets |
| `allow-remote-config-modification` | Y/N | N | ranode: pinned `N` from ko.24 |
| `approve-mode` | `password` / `click` / empty (both) | empty | ranode: pinned `password` when the policy says `password` (ko.24), otherwise upstream's, changeable in settings |

Local setting (per user, Settings → General → Keyboard; not settable with `--option`):
`enable-peer-hangul-key` (Y/N, default Y) — controlling side maps 한/영 to the peer OS's key.
Android: `korean-toggle-key` holds the 한/영 button's pinned key (unset = auto).

`--option` is accepted only from an installed app whose executable sits under
`/Applications/<app name>.app`; ranode's bundle is `RustDesk (forked ranode).app`, not
`RustDeskPublic.app`, so `--option` never applies to ranode.

## ranode password command (from ko.23)

- Call: `"/Applications/RustDesk (forked ranode).app/Contents/MacOS/RustDeskPublic" --set-permanent-password`,
  run as the logged-in user while ranode is running; the password is the first line of stdin.
- Output: `Done!` and exit 0 when ranode applied it.
- Errors: exit 1 for a password given as an argument, an empty password, or running as root; exit 2
  when ranode is not running or did not apply it. The password is never printed.

## ranode incoming policy command (from ko.24)

- Call: `"/Applications/RustDesk (forked ranode).app/Contents/MacOS/RustDeskPublic" --set-incoming-policy [--extra-subnets <list>|none] [--approve-mode password|both]`,
  run as the logged-in user while ranode is **not** running; the policy applies at ranode's next start.
- At least one flag, each at most once. `<list>`: 1–8 comma-separated `a.b.c.d/n` entries (spaces
  around entries ignored), each without host bits and wholly inside 10.0.0.0/8, 172.16.0.0/12 or
  192.168.0.0/16; `none` clears the extras. A flag not given leaves its stored value unchanged.
- Stored as ranode local options `public-incoming-extra-subnets` (comma list, no spaces) and
  `public-incoming-approve-mode` (`password` / `both`).
- Output: one stdout line with the effective policy, e.g.
  `whitelist=100.64.0.0/10,10.0.0.0/24 approve-mode=password` (unset prints `approve-mode=both`); exit 0.
- Errors (stderr, nothing saved): exit 1 for an unknown, duplicate, missing or invalid argument or
  any refused entry, or running as root; exit 2 when ranode is running (its IPC answers). Zero vs
  non-zero is the contract.

## URL schemes

`rustdesk://` opens the main app, `rustdeskpublic://` opens ranode; both take upstream's
`rustdesk://<id>` forms.

## Ports

TCP 21118 direct access and UDP 21119 LAN discovery (upstream defaults). Peers' on-prem cards in
ranode assume 21118; a device listening elsewhere shows as "직접 접속 꺼짐". ranode does not answer
UDP 21119, so a Mac whose only RustDesk is a closed ranode shows as "RustDesk 없음".
