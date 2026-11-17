This file is written in English only.

# Operations

## Machines (by role)

- **macOS build Mac**: an Apple Silicon Mac with Rust 1.81+, Flutter 3.24.5, Xcode; builds both
  macOS apps (arm64 only).
- **Signing Mac**: holds the age key for the SOPS repo with the owner's Developer ID and notary
  key, `bwsx` access to the Bitwarden Secrets Manager project `android-signing`, `rcodesign`, `sops`,
  Xcode command-line tools, Android build-tools 34.0.0 and JDK 17.
- Which machines play these roles is not fixed in this repo; the owner's device table in the
  machine-setup repo is the binding.
- The signing scripts' defaults were set for the former signing Mac. On any other Mac override:

| Variable | Script | Role | Default (former signing Mac) |
|---|---|---|---|
| `PRINCIPAL_SECRETS_ROOT` | osx-sign-notarize | checkout of the SOPS repo with `secrets/macbook-tools/` | `~/.environment-secret-broker/principal-secrets-mono` |
| `SOPS_AGE_KEY_FILE` | osx-sign-notarize | age key that decrypts those files | `~/.config/sops/age/<former host>-se.txt` |
| `BWSX` | android-sign-release | path to `bwsx` | looked up in the agent-secret-controller plugin cache |
| `BWS_HANDLE` | android-sign-release | `bwsx` handle for the Secrets Manager access token | a handle named after the former signing Mac |
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


## Build the macOS apps

```sh
python3 build.py --flutter                          # main app → flutter/build/macos/Build/Products/Release/RustDesk.app
RUSTDESK_FLAVOR=public python3 build.py --flutter   # ranode  → …/Release/RustDeskPublic.app
```

Order matters: each run writes or deletes `flutter/macos/Runner/Configs/Flavor.xcconfig`; copy the
first `.app` away before the second build overwrites the Release folder.

## Release ko.N

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

## Install by hand on a Mac

```sh
ditto -x -k rustdesk-public-1.5.0-ko.N-macos-aarch64.zip /tmp/r && \
  ditto "/tmp/r/RustDesk (forked ranode).app" "/Applications/RustDesk (forked ranode).app"
```

Check the sha256 first. The main app needs Accessibility, Screen Recording and Input Monitoring;
ranode needs Input Monitoring only (to send ⌘Tab and system shortcuts). Main-app incoming options:
`sudo /Applications/RustDesk.app/Contents/MacOS/RustDesk --option direct-server Y` and
`--option whitelist 100.64.0.0/10`.
