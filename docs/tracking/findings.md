This file is written in English only.

# Findings

## Where the release Macs are is not recorded

- Condition (2026-11-17): releases up to ko.21 were built and signed on a Mac with the toolchain
  under `~/work/sdk` (paths embedded in the ko.21 binaries; Developer ID signature time 15:13 KST).
  The company Mac Studio with the same host and user name has no `~/work/sdk`, no SOPS checkout and
  no age key, so the release Mac is a different machine, most likely on the owner's personal
  tailnet, and the agents that set up company machines cannot reach it.
- Blast radius: an agent planning a release cannot tell which Mac or session builds and signs, and
  may wrongly conclude that releases are blocked (this happened on 2026-11-17).
- Why not now: which Mac this is, and whether its keys sit in personal stores, is for the owner to
  confirm; nothing was moved.
- Approach: the owner confirms the release Mac and its tailnet; the role binding is then written
  where the owner keeps the device table, and this item closes.

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
  Finder fix (ko.22).
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
  (Tailscale LocalAPI) is superseded by the ko.22 Finder fix and carries the unmerged `macos-public.yml`.
- Blast radius: an agent may pick an old branch as a base.
- Why not now: deleting branches cannot be undone; PR #19 is closed when the ko.22 fix merges.
- Approach: the owner approves a list of branches to delete.

## Stale "owner's own server" claims about the main app

- Condition: `docs/architecture.md` (Rendezvous row, main-app cell) and `docs/security-policy.md`
  ("Who may connect in", main-app row) say the main app uses the owner's own RustDesk server; the
  machine setup points it at the default public server. The comment in
  `flutter/lib/desktop/pages/desktop_home_page.dart` that says ranode makes only outgoing
  connections is stale since ko.23.
- Blast radius: an agent planning the main-app removal or ID access reads the wrong server.
- Why not now: outside ko.24's change scope (found in its final review).
- Approach: reword the two rows and the comment in the next ranode cycle.
