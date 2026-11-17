This file is written in English only.

# Findings

## The macOS release path in the release procedure is not available

- Condition/symptom (2026-11-17): the Mac that signed releases up to ko.20 has no SOPS repo
  checkout, no age key and no release folders; the former build Mac is not in the tailnet; no
  `principal-secrets-mono` repo resolves under `dalsoop` or `crosswell-corp`.
- Blast radius: no macOS release (ko.21 onward) can be signed or notarized.
- Why not now: which Mac signs and where the owner keeps the keys is the owner's decision.
- Approach: the owner names a signing Mac, restores the SOPS repo and age key there (or moves the
  keys to a personal store), and overrides the signing scripts' defaults.

## Personal signing keys were used on a company machine

- Condition: the former signing Mac is a crosswall company Mac Studio, while the Developer ID,
  notary key and Android key are the owner's personal assets. Who owns `principal-secrets-mono` and
  the Secrets Manager project `android-signing` could not be checked (the repo was not found; the
  Bitwarden account used for company machine setup has no organization).
- Blast radius: personal and company assets mixed on one machine.
- Why not now: owner decision; nothing was moved.
- Approach: sign on one of the owner's own Macs; confirm both stores are in the owner's personal
  accounts.

## Obsolete Android workflows can still be run

- Condition: `sign-apk.yml` (manual trigger) signs with a throwaway keystore (password 123456) and
  uploads to release `v1.5.0-korean-fix`; `build-android-arm64.yml` triggers on the stale branch
  `fix-android-korean-ime`.
- Blast radius: one manual run publishes an APK whose certificate differs from the pinned one, so it
  cannot install as an update on the owner's phone.
- Why not now: removing workflows needs the owner's OK.
- Approach: delete both workflows in a separate change.

## Fork Rust tests are not in CI, and CI does not run on pull requests

- Condition: `flutter-unit-test.yml` runs the 5 fork Flutter test files on push, but no job runs the
  fork's Rust tests, and `ci.yml` is set for `pull_request` but no pull-request run appears in the
  Actions history.
- Blast radius: regressions in the Rust key mapping, ranode core and update check reach the default
  branch unnoticed.
- Why not now: a Rust test job needs the vcpkg/toolchain setup of the desktop build; not part of the
  ko.21 fix.
- Approach: add a `cargo test --lib` job for the fork modules in a separate change.

## Upstream scheduled workflows fail on the fork

- Condition: "Flutter Nightly Build" ends in `startup_failure` on every schedule; "Update
  webpki-roots" fails.
- Blast radius: failures that matter are hidden among the scheduled ones in the Actions list.
- Why not now: disabling them means editing upstream workflow files or the repository settings;
  owner's call.
- Approach: disable the two schedules in the repository's Actions settings (no file change).

## Stale branches and a superseded PR

- Condition: 45 remote fork branches, 16 of them unmerged (upstream-feature names and finished
  fork topics); PR #19
  (Tailscale LocalAPI) is superseded by the ko.21 fix and carries the unmerged `macos-public.yml`.
- Blast radius: an agent may pick an old branch as a base.
- Why not now: deleting branches cannot be undone; PR #19 is closed as part of ko.21.
- Approach: the owner approves a list of branches to delete.
