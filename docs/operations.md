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
3.24.5, `MAC_RUST_VERSION` 1.81, vcpkg commit) and upstream's vcpkg dependencies.

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
is delegated to agents (owner, 2026-11-06): an agent on `m5m1` builds, signs, publishes and installs
a release from the merge commit on `feat/sync-upstream-1.5.0` without waiting for the owner's word,
and runs the live checks on the signed build before marking it Latest. macOS builds run through the
build queue (`build-pipeline submit … --host m5m1`), never on the M1 MacBooks.

1. Merge the PR that sets `version: 1.5.0+68NN` in `flutter/pubspec.yaml` (and the KOREAN.md notes)
   into `feat/sync-upstream-1.5.0`. CI then builds Windows/Linux artifacts.
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
5. Download the CI artifacts of the release commit and rename them to the published names
   `rustdesk-1.5.0-ko.N-x86_64.exe`, `…-x86_64.msi`, `…-x86_64.deb` (CI names them
   `rustdesk-1.5.0-ko-windows-x86_64` etc.; the Windows updater needs the exact names), write a
   `.sha256` for each, then create GitHub release `1.5.0-ko.N` on that commit with the macOS zips,
   Windows/Linux files, APK, and every `.sha256`; title in Korean naming what changed. Mark it Latest only when it is
   meant to reach the update check.
6. Roll out: the owner's Macs pin version + sha256 per app in their machine-setup repo; the office
   Linux server pins `versions.rustdesk` (`1.5.0+koN`), sha256 and URL in its infrastructure repo.

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

## Install by hand on a Mac

```sh
ditto -x -k rustdesk-public-1.5.0-ko.N-macos-aarch64.zip /tmp/r && \
  ditto "/tmp/r/RustDesk (forked ranode).app" "/Applications/RustDesk (forked ranode).app"
```

Check the sha256 first. The main app needs Accessibility, Screen Recording and Input Monitoring;
ranode needs Input Monitoring only (to send ⌘Tab and system shortcuts). Main-app incoming options:
`sudo /Applications/RustDesk.app/Contents/MacOS/RustDesk --option direct-server Y` and
`--option whitelist 100.64.0.0/10`.
