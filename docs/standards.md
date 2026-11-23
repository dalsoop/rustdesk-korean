This file is written in English only.

# Standards

## Relationship to upstream

- Upstream (rustdesk/rustdesk, remote `upstream`) is merged into the fork; nothing from the fork is
  proposed upstream. Upstream's coding, logging, Tokio, editing-hygiene, test and localization rules
  (the upstream part of `AGENTS.md`) apply to every change here.
- MUST: fork logic lives in fork-owned files; an upstream file gets at most a thin hook (a call, a
  `#[cfg]`-gated block, a flag check) marked `// Korean fork:` where the reason is not obvious.
  MUST NOT: refactor, reformat or reorder upstream code, or fix unrelated upstream bugs in a fork
  change. Violation test: `git diff $(git merge-base HEAD upstream/master) HEAD -- <upstream file>`
  shows lines that are not required by a fork feature.
- MUST NOT edit the upstream part of `AGENTS.md`, `CLAUDE.md` or `GEMINI.md`; the fork owns only
  the block at the top of `AGENTS.md`.
- MUST NOT change `libs/hbb_common` (a submodule shared with the server); fork option keys go in
  `libs/base/src/config/keys.rs`.

## Branches, commits, PRs

- Default and release branch: `feat/sync-upstream-1.5.0`. A hotfix to an older release may branch
  from that release's tag (`release/<name>`), and must be ported to the default branch in a
  separate PR before the next release.
- Work happens on a topic branch (`fix/…`, `feat/…`) merged by PR on `dalsoop/rustdesk-korean`;
  `gh` must target `-R dalsoop/rustdesk-korean` (a bare `gh pr` resolves to upstream).
- A commit that changes user-visible behaviour also updates `KOREAN.md`.

## Versioning

- MUST bump the release only in the `version:` line of `flutter/pubspec.yaml`, as
  `1.5.0+<6800+N>`, in the PR that ships ko.N. MUST NOT edit derived versions (Android gradle,
  Windows, .deb, About) by hand, and MUST NOT change `Cargo.toml`'s `1.5.0`.
- One tag per release, `1.5.0-ko.N`, on the commit both macOS apps and all platforms are built from.

## Fixed identities

- MUST NOT change: `com.carriez.rustdesk`, app name `RustDesk`, scheme `rustdesk`, bundle file
  `RustDesk.app` (main); `net.ranode.rustdesk-public`, `RustDeskPublic`, `rustdeskpublic` (ranode).
- The shown name "RustDesk (forked ranode)" is written in two places that must stay equal:
  `RUSTDESK_DISPLAY_NAME` in `build.py` and `kPublicAppDisplayName` in `flutter/lib/common.dart`.
- `RUSTDESK_FLAVOR` accepts only unset or `public`; `build.py` fails on anything else.

## Flavor gating

- ranode-only behaviour MUST be gated by `public_flavor::is_public()` (Rust) or
  `bind.mainIsPublicFlavor()` (Flutter); a main-app build MUST run upstream's path unchanged.
- Settings ranode must hold regardless of the user go into `override_settings()` in
  `src/public_flavor.rs` (applied through `OVERWRITE_SETTINGS`), never into defaults.

## Language

- Agent-facing Markdown (`AGENTS.md` fork block, `docs/`, module `AGENTS.md`) is English and starts
  with `This file is written in English only.`.
- `KOREAN.md` and the Korean section of `README.md` are product docs for Korean users and stay in
  Korean; English paragraphs already in them may stay.
- UI strings: Korean text goes in `src/lang/ko.rs`, English in `src/lang/en.rs` when it differs
  from the key; other languages get `""` per upstream's localization rule.

## Verification gates

Before merge (pull requests to the default branch run "fork Rust unit tests" and "flutter unit
tests (soft keyboard / Korean)" when their paths change; both must be green):

- Rust: `cargo test --lib` covering the fork modules that changed (`hangul_key`, `fork_update`,
  `public_flavor`, `updater`, `platform::macos_input_source`, macOS hotkey tests in
  `platform::macos`). CI runs the first four on Linux; the macOS-only tests still run locally on
  a Mac.
- Flutter: `flutter analyze` on the changed files (local) and `flutter test` on the fork tests
  (`soft_keyboard_input_utils_test`, `hangul_dubeolsik_test`, `korean_toggle_key_test`,
  `tailscale_test`, `public_onprem_test`; CI).
After merge, before release (CI on push to the default branch):

- "flutter unit tests (soft keyboard / Korean)" is green when its paths changed. The Windows and
  Linux release builds are made by the release driver (docs/operations.md "Release ko.N"), not by a
  workflow of this repository.
- A change to ranode or the Tailscale module is also checked live on a Mac launched from Finder
  (not from a terminal) before release.

## Release gates

- macOS assets MUST pass `spctl` as "Notarized Developer ID"; the APK MUST carry the pinned
  certificate; every asset ships with its `.sha256`.
- MUST NOT set `ANDROID_SIGN_EXPECTED_SHA256` (it replaces the pinned certificate).
- The APK is signed only by `res/android-sign-release.sh` on the signing Mac; no workflow signs
  or publishes an APK.
- Signing runs only on the owner's signing Mac with the owner's keys; MUST NOT add signing keys
  to GitHub Actions or any CI.
