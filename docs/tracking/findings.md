This file is written in English only.

# Findings

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
  Finder fix (ko.22, released 2026-11-21).
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

## The macOS signing script leaves LaunchServices records behind

- Condition (2026-11-17, again 2026-11-21): `res/osx-sign-notarize.sh` copies the app to a stage
  folder under `~/tmp/osx-sign.*`, which macOS registers with LaunchServices, and deletes the folder
  on exit without unregistering it. Each run leaves a dead record per bundle id (two after ko.22;
  removed by hand with `lsregister -u`).
- Blast radius: a stale record with the installed app's bundle id can make `open -b`, Gujo Cloud
  Apps or `rustdeskpublic://` links resolve to a missing or unsigned copy.
- Why not now: a script change is outside a release; the records were cleaned after the run.
- Approach: unregister the stage copy (`lsregister -u`) in the script's exit trap.
