This file is written in English only.

# Operations

## Machines (by role)

- **macOS build Mac**: an Apple Silicon Mac with Rust 1.81+, Flutter 3.24.5, Xcode; builds both
  macOS apps (arm64 only).
- **Signing Mac**: holds the age key for the SOPS repo with the owner's Developer ID and notary
  key, `bwsx` access to the Bitwarden Secrets Manager project `android-signing`, `rcodesign`, `sops`,
  Xcode command-line tools, Android build-tools 34.0.0 and JDK 17.
- Both roles are played by the owner's Mac Studio `m5m1` (checked 2026-11-21: it holds `~/work/sdk`
  with Flutter 3.24.5, vcpkg, Android build-tools and JDK 17, the SOPS checkout and age key, the
  Developer ID and `bwsx`; ko.21 to ko.23 were built, signed and released there by agents). The
  owner's device table in the machine-setup repo stays the binding if this changes.
- The signing scripts' defaults are set for `m5m1`. On any other Mac override:

| Variable | Script | Role | Default (`m5m1`) |
|---|---|---|---|
| `PRINCIPAL_SECRETS_ROOT` | osx-sign-notarize | checkout of the SOPS repo with `secrets/macbook-tools/` | `~/.environment-secret-broker/principal-secrets-mono` |
| `SOPS_AGE_KEY_FILE` | osx-sign-notarize | age key that decrypts those files | `~/.config/sops/age/m5m1-se.txt` |
| `BWSX` | android-sign-release | path to `bwsx` | newest `bin/bwsx` under the `agent-secret-controller-skills` plugin cache |
| `BWS_HANDLE` | android-sign-release | `bwsx` handle for the Secrets Manager access token | `ranode-m5m1` |
| `AGENT_SECRET_SE_IDENTITY` | android-sign-release | identity file `bwsx` uses | `~/.config/sops/age/$(hostname -s)-se.txt` |
| `ANDROID_BUILD_TOOLS`, `JAVA_HOME` | android-sign-release | apksigner 34.0.0, JDK 17 | under `~/work/sdk/` |

  Both scripts create their private temp folder under `~/tmp`.

## Setup

```sh
git clone https://github.com/dalsoop/rustdesk-korean.git && cd rustdesk-korean
git remote add upstream https://github.com/rustdesk/rustdesk.git
git submodule update --init --recursive      # libs/hbb_common
```

Toolchains: the versions pinned in `.github/workflows/build-desktop-ko.yml` (`FLUTTER_VERSION`
3.24.5, `RUST_VERSION` 1.75, `MAC_RUST_VERSION` 1.81, vcpkg commit) and upstream's vcpkg
dependencies. The release builds keep the same pins: `res/release-ko-build-windows.ps1` (Windows)
and the Linux job of `gujoai/apps/upstream-mono`'s CI.

The Flutter↔Rust bridge files are generated and git-ignored; without them `flutter analyze` and
every build fail. Generate them after cloning and after any change to `src/flutter_ffi.rs`:

```sh
cargo install flutter_rust_bridge_codegen --version 1.80.1 --features uuid --locked
(cd flutter && flutter pub get)
~/.cargo/bin/flutter_rust_bridge_codegen --rust-input ./src/flutter_ffi.rs \
  --dart-output ./flutter/lib/generated_bridge.dart --c-output ./flutter/macos/Runner/bridge_generated.h
cp ./flutter/macos/Runner/bridge_generated.h ./flutter/ios/Runner/bridge_generated.h
```

## Verify

```sh
for m in hangul_key fork_update public_flavor updater; do cargo test --lib $m; done   # any OS
cargo test --lib platform::macos                                 # on a Mac: input source + hotkey tests
cd flutter && flutter pub get && flutter analyze <changed files> && \
  flutter test test/soft_keyboard_input_utils_test.dart test/hangul_dubeolsik_test.dart \
    test/korean_toggle_key_test.dart test/tailscale_test.dart test/public_onprem_test.dart
```

CI runs the first line (`fork-rust-tests.yml`, Linux x64) and the Flutter tests
(`flutter-unit-test.yml`) on pushes and on pull requests to the default branch. Upstream's "CI",
"Full Flutter CI", "Flutter Nightly Build" and "Update webpki-roots" are disabled in the
repository's Actions settings (`gh workflow enable <name> -R dalsoop/rustdesk-korean` undoes it);
their files stay byte-identical to upstream.


## Build the macOS apps

```sh
python3 build.py --flutter                          # main app → flutter/build/macos/Build/Products/Release/RustDesk.app
RUSTDESK_FLAVOR=public python3 build.py --flutter   # ranode  → …/Release/RustDeskPublic.app
```

Order matters: each run writes or deletes `flutter/macos/Runner/Configs/Flavor.xcconfig`; copy the
first `.app` away before the second build overwrites the Release folder.

## Release ko.N

Releases are made on the release Mac `m5m1` (it holds `~/work/sdk` and the signing keys). Shipping
is delegated to agents (owner, 2026-11-06), and from 2026-11-22 the release runs by itself (owner,
"진행", 2026-11-22, session m5m1: "버전이 올라간 커밋이 병합되면 m5m1이 빌드부터 GitHub 등록까지
진행합니다. 서명 키는 지금처럼 m5m1 밖으로 나가지 않습니다."). macOS builds run through the build
queue (`build-pipeline submit … --host m5m1`), never on the M1 MacBooks.

Where each part is built (from 2026-11-23; the owner's choice "둘 다 옮김" in session m5m1, which moved
Windows and Linux off GitHub-hosted runners): macOS on m5m1, and every signature there; Linux (the
deb) and Android (the unsigned APK) in the GitLab CI of `gujoai/apps/upstream-mono` (jobs
`rustdesk:linux-x64` and `rustdesk:android-arm64`); Windows (exe and msi, unsigned as before) on the
Windows build VM `win-build` with `res/release-ko-build-windows.ps1`. Neither CI nor the VM holds a
signing key or a publishing token; assembly, checks and the GitHub release happen on m5m1. A run's
`result.json` lists the host of each part under `build_hosts`.

### Automatic release

To release ko.N, merge the PR that sets `version: 1.5.0+68NN` in `flutter/pubspec.yaml` (with the
KOREAN.md notes and, optionally, `res/release-notes/1.5.0-ko.N.md`) into `feat/sync-upstream-1.5.0`.
Nothing else is needed:

- **Trigger**: the m5m1 LaunchAgent `net.ranode.dotfiles.rustdesk-release-watch` (machine-setup
  repo, README "RustDesk fork release watch") reads the branch head every 15 minutes (anonymous
  `git ls-remote` and the raw `pubspec.yaml`). When GitHub has no published release `1.5.0-ko.N`,
  it runs `res/release-ko.py <head>` from a checkout of that head. No workflow of this public
  repository runs on m5m1 (no self-hosted runner): only code merged to the default branch is run.
- **Driver** `res/release-ko.py` does the steps of "By hand" below, each resumable (a step whose
  result is already on disk and passes its checks is skipped): the upstream-mono MR that
  moves `rustdesk/` to the commit, put in its merge queue (a blocked, held or draft MR stops the run
  with its link; it is never re-queued), and the unsigned APK of the newest `rustdesk:android-arm64`
  job on a main pipeline whose `rustdesk/` equals the commit's tree file by file, signed with
  `res/android-sign-release.sh`; the deb and the flutter_rust_bridge output of the
  `rustdesk:linux-x64` job of such a pipeline; the Windows build on `win-build`
  (`res/release-ko-build-windows.ps1`, copied there with that bridge output and run as a scheduled
  task, so it outlives the SSH session; the exe, msi and the commit and MSI version it reports are
  copied back and compared by sha256; a failed build is started again on the next attempt); both
  macOS apps built through the build queue (`res/release-ko-build-macos.sh`) and signed, notarized
  and zipped with `res/osx-sign-notarize.sh`; checks of every file; `.sha256` files; notes; a draft
  release with the 12 files, GitHub's sha256 of each compared with ours; then publish and Latest.
- **Checks before Latest** (all automatic): macOS zips hold exactly `RustDesk.app` / `RustDesk
  (forked ranode).app` with the bundle id, `1.5.0` and build `68NN`, `codesign --verify --deep
  --strict`, team `V2Z46YY386`, `spctl` "Notarized Developer ID", stapled; the APK has versionName
  `1.5.0-ko.N`, versionCode `68NN+2000`, the pinned certificate and v1–v3 signatures; the deb's
  package version is `1.5.0+koN`; LaunchServices lists no build or signing copy of either bundle id;
  the process listening on TCP 21118 (the Mac's receiving RustDesk) is the same as at the start.
  The UI check of the new ranode is **skipped** on m5m1: its installed ranode receives remote
  sessions, and a second ranode would share its IPC socket (`/tmp/RustDeskPublic-<uid>/`), ID
  and settings, while the main-app bundle would register the same ID; so the live checks happen
  after the machine-setup repo installs the release (operations of that repo).
- **Notes**: `res/release-notes/1.5.0-ko.N.md` at the release commit when present (first line
  `# <short title>`, the rest is "what changed"); otherwise the titles of the PRs merged since the
  previous release. The version numbers and the file list are always added.
- **Waiting and failures**: a run waits at most 10 minutes for the upstream-mono merge queue, a
  CI job or the Windows build, then ends with exit 75; the next tick continues. A failed run is
  retried after an hour, a refused one (commit not on the branch, bad version line) not until the
  head changes. Start, waiting, failure and success are posted in ranode-corp `#970-에이전트-작업`,
  thread `1795280360.472489`.
- **Work folder** `~/work/rdk-release` (external drive): `repo/` (fork clone), `build/` (build
  checkout, `target/` kept between releases), `upstream-mono/`, `tools/` (the trigger's checkout),
  `1.5.0-ko.N/` (`assets/`, `logs/run-*.log`, `notes.md`, `result.json`), `last-result.json`.
- **Stop it**: `touch ~/.config/rustdesk-release-watch/off` on m5m1 (remove the file to resume).

### By hand

Run the driver on m5m1 from any checkout of the default branch (it waits up to 3 hours per wait by
default; `--wait-seconds` changes that):

```sh
/usr/bin/python3 -I res/release-ko.py <commit>                 # full release, same as the trigger
/usr/bin/python3 -I res/release-ko.py --dry-run <commit>       # build, sign, check; no GitHub/GitLab writes
/usr/bin/python3 -I res/release-ko.py --draft-only <commit>    # stop after the draft and its checks
/usr/bin/python3 -I res/release-ko.py --draft-only --release-tag 1.5.0-ko.N-test <commit>   # throwaway draft; delete it after
```

`--dry-run` also runs for a release that exists and compares its files with the published ones.
Before a hand run on a ko.N the watch has not released yet, create the stop file above: otherwise the
watch publishes a `--draft-only` draft at its next tick, and a run on another commit of the same ko.N
moves the watch's release folder aside (kept as `1.5.0-ko.N.<commit>-<time>`). The driver accepts
only first-parent commits of the branch (merge results), never a commit from inside a merged PR.
Exit 0 done, 1 failed (see `~/work/rdk-release/1.5.0-ko.N/logs/`), 2 refused, 75 waiting.
`launchctl kickstart gui/$(id -u)/net.ranode.dotfiles.rustdesk-release-watch` runs one tick now.

The steps it automates:

1. Merge the PR that sets `version: 1.5.0+68NN` in `flutter/pubspec.yaml` (and the KOREAN.md notes)
   into `feat/sync-upstream-1.5.0`.
2. Build both macOS apps from that commit on the build Mac; move them to the signing Mac.
3. On the signing Mac:
   ```sh
   res/osx-sign-notarize.sh <…/RustDesk.app> RustDesk.app rustdesk-1.5.0-ko.N-macos-aarch64.zip
   res/osx-sign-notarize.sh <…/RustDeskPublic.app> "RustDesk (forked ranode).app" \
       rustdesk-public-1.5.0-ko.N-macos-aarch64.zip
   ```
   Each run fails unless `spctl` reports "Notarized Developer ID", and writes `<zip>.sha256`.
   `PRINCIPAL_SECRETS_ROOT` (default `~/.environment-secret-broker/principal-secrets-mono`) and
   `SOPS_AGE_KEY_FILE` point at the SOPS repo and its age key.
4. Android: the unsigned arm64 APK comes from the GitLab mirror `gujoai/apps/upstream-mono`
   (an MR moving its `rustdesk/` subtree to the release commit), then on the signing Mac:
   `res/android-sign-release.sh <unsigned.apk> rustdesk-1.5.0-ko.N-aarch64.apk` (checks the pinned
   certificate, writes `.sha256`).
5. Linux: the deb (and the bridge output) from the `rustdesk:linux-x64` job of the same
   upstream-mono main pipeline. Windows: on `win-build`, `res/release-ko-build-windows.ps1 -Action
   start -Commit <commit>` after copying the script to `C:\build\rdk\runs\<commit>\build.ps1` and
   the bridge files to its `bridge\` (`-Action status` reports the result). Rename the files to the
   published names `rustdesk-1.5.0-ko.N-x86_64.exe`, `…-x86_64.msi`, `…-x86_64.deb` (the builds
   name them `rustdesk-1.5.0-x86_64.*`; the Windows updater needs the exact names), write a
   `.sha256` for each, then create GitHub release `1.5.0-ko.N` on that commit with the macOS zips,
   Windows/Linux files, APK, and every `.sha256`; title in Korean naming what changed. The driver
   creates it as a draft and marks it Latest after the checks above (every release is meant to reach
   the update check; to hold one back, stop the watch before merging the version PR).
6. Roll out: the owner's Macs pin version + sha256 per app in their machine-setup repo; the office
   Linux server pins `versions.rustdesk` (`1.5.0+koN`), sha256 and URL in its infrastructure repo.

## Internal builds (from ko.24)

- ko.24 and later may ship as internal builds (ko.23 was released as a notarized Developer ID
  release on 2026-11-21): made on the company build Mac by the machine-setup
  repo's `ranode-build` script from a merge commit on `feat/sync-upstream-1.5.0`, installed only on
  the owner's and the company's Macs. A notarized Developer ID release is optional and later.
- Signing rule: every internal build is signed with the one stable self-signed code-signing
  identity "ranode internal signing" (created once, kept in the vault, imported into the build
  Mac's keychain by the machine-setup build role). Never ad-hoc: macOS ties Screen Recording,
  Accessibility and Input Monitoring grants to the signing identity, and an ad-hoc signature
  changes with every build. Check with `codesign -dv` before installing.
- Before the first internal build of a commit, run the Rust tests of the changed fork modules on
  the build Mac (CI runs them on Linux only; `platform::macos` and the macOS listener need a Mac).

## ranode as the receiving app (from ko.23)

Order on a Mac that still has the main app (ranode retakes a busy 21118 only when it starts). If
the main app's service is installed (`/Library/LaunchDaemons/com.carriez.RustDesk_service.plist`,
`/Library/LaunchAgents/com.carriez.RustDesk_server.plist`), its `--server` keeps 21118 after the
window quits: unload and remove both plists (admin) before starting ranode.

```sh
osascript -e 'quit app "RustDesk"'             # the main app releases 21118
rm -rf /Applications/RustDesk.app               # after the cutover is approved
open -a "/Applications/RustDesk (forked ranode).app"
printf '%s\n' "$PASSWORD" | "/Applications/RustDesk (forked ranode).app/Contents/MacOS/RustDeskPublic" --set-permanent-password
```

The last command runs as the logged-in user (never `sudo`), needs ranode running, prints `Done!`
and exits 0; exit 2 means ranode was not running or did not apply it. Take `$PASSWORD` from the
vault without echoing it. ranode then needs Screen Recording, Accessibility and Input Monitoring.

### Saved passwords of the own devices (from ko.25, decision 0010)

On each controller Mac, as the logged-in user (ranode may be running), once per target device and
again after that device's permanent password or Tailscale address changes:

```sh
printf '%s\n' "$PASSWORD" | "/Applications/RustDesk (forked ranode).app/Contents/MacOS/RustDeskPublic" --save-peer-password <target Tailscale IPv4>
```

It prints `Saved for <address>.` and exits 0; 1 means refused input (no Tailscale address, password
on the command line, empty, root), 2 not saved. Take `$PASSWORD` from the vault without echoing it
(the machine setup pipes `bw-agent exec … --stdin` into it over ssh). From then on a click on that
device's on-prem card connects without the password prompt; ranode's log says only "using the saved
password for <address>". To drop an entry, remove its line from
`~/Library/Preferences/com.carriez.RustDeskPublic/RustDeskPublic_saved_peer_passwords.toml`.

### Cutover keeping the Mac's ID (from ko.24; run by the machine-setup repo)

1. Quit the main app and ranode. Remove the main app, or at least disable everything that starts
   it (its login LaunchAgent, and its service plists where installed): a main app running with the
   same ID as ranode would clash.
2. Back up ranode's `~/Library/Preferences/com.carriez.RustDeskPublic/RustDeskPublic.toml`, then
   copy the main app's `~/Library/Preferences/com.carriez.RustDesk/RustDesk.toml` over it. This
   keeps the Mac's ID, key pair and permanent password; never read or print the file's values.
   ranode's local settings (`RustDeskPublic_local.toml`) are not touched.
3. `RustDeskPublic --set-incoming-policy`, ranode quit, as the user: Craft Macs
   `--extra-subnets <office subnet> --approve-mode password`; the owner's own Macs: skip.
4. Start ranode, then `--set-permanent-password` from the vault (above).
5. Verify once by ID from inside the office (Craft Macs) and by Tailscale IP.

## Install by hand on a Mac

```sh
ditto -x -k rustdesk-public-1.5.0-ko.N-macos-aarch64.zip /tmp/r && \
  ditto "/tmp/r/RustDesk (forked ranode).app" "/Applications/RustDesk (forked ranode).app"
```

Check the sha256 first. The main app needs Accessibility, Screen Recording and Input Monitoring;
ranode needs Input Monitoring only (to send ⌘Tab and system shortcuts). Main-app incoming options:
`sudo /Applications/RustDesk.app/Contents/MacOS/RustDesk --option direct-server Y` and
`--option whitelist 100.64.0.0/10`.
