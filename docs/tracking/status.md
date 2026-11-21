This file is written in English only.

# Status (2026-11-21)

## Released

| Area | State | Evidence |
|---|---|---|
| Android Korean soft keyboard (grapheme diff, 두벌식 to Linux, F19/F24, 한/영 button) | released since ko.1–ko.4 | 3 Flutter test files run in CI on push (green 2026-11-17); checked by hand on a Galaxy Z Fold to macOS (ko.1) and to Pop!_OS COSMIC/fcitx5 (ko.3) |
| Desktop 한/영 → peer OS key, macOS input-source switch, local hotkey pause | released ko.5–ko.10 | Rust unit tests exist (26 in `hangul_key`, 7 in `macos_input_source`, hotkey-flag tests); not run in CI, last pass not recorded; never tested with a Windows peer |
| Fork update check | released ko.11 | Rust unit tests exist (`fork_update`, `updater`); not run in CI |
| ranode (on-prem + internet tabs, onboarding, sections, identity, optional Tailscale, public sign-in) | released up to ko.22 (latest release, 2026-11-21) | Flutter tests `tailscale_test`, `public_onprem_test`: 8 passed locally at PR #33 (2026-11-17); run in CI on push from ko.21. ko.22 signed build checked live on m5m1 (opened with `open -b` and from a terminal: device list and "Tailscale: … 연결됨") and on m5m2 after install (device list) |
| Main app shown as "RustDesk (for korean)" | released ko.19 | installed on the owner's Macs |

## Merged, not released

- ko.22 is released (2026-11-21, release `1.5.0-ko.22` on fa319e09, the merge of PR #35, marked
  Latest): ranode reads Tailscale when opened from Finder, the Dock or at login (`TERM=dumb` for
  the CLI). All four platforms, built, signed and published on m5m1. Live-checked only with
  Tailscale up; the stopped, restarting and absent states were not exercised, because stopping
  Tailscale on a Mac cuts the SSH path the agents and other sessions use.
- ko.23 (version `1.5.0+6823`, merged): ranode also accepts incoming connections from the tailnet
  with its permanent password only; left pane "이 Mac에 접속받기"; `--set-permanent-password`.
  Built: Rust compiled in the Windows/Linux CI build; Flutter tests green in CI; the Rust unit
  tests (`public_flavor`) are compiled and run first on the release Mac (`m5m1`).

## Remaining, in order

1. **ko.23 release**: on the release Mac `m5m1`, from the merge commit of
   the ko.23 PR (`version: 1.5.0+6823`): run `cargo test --lib public_flavor` (must pass; first
   time these tests compile), build both macOS apps, sign, then on the signed ranode check:
   1. another tailnet Mac connects by this Mac's Tailscale IP + permanent password and controls it;
   2. a connection through the public server (non-Tailscale address) is refused;
   3. with the main app running, ranode works outgoing and does not take 21118;
   4. after quitting the main app and relaunching ranode, ranode accepts;
   5. the left pane shows the address and password status; missing Screen Recording /
      Accessibility grants show their cards; with the service stopped it says
      "지금은 접속받지 않음 (서비스 중지됨)";
   6. `--set-permanent-password` with the password on stdin sets it (exit 0), exits non-zero with
      ranode quit, and is refused under sudo;
   7. a one-time password is refused; the permanent password works.
   Mark Latest only after all pass.
2. **Machine-setup cutover (after ko.23 is released)**, for each owner Mac: install ko.23 ranode;
   grant it Screen Recording, Accessibility and Input Monitoring; quit and remove the main app
   (and its LaunchAgent/LaunchDaemon where installed) before (re)starting ranode, because ranode retakes a busy 21118 only at its next start; start
   ranode and add it as a login item; set the vault password with `--set-permanent-password`
   (stdin, as the user, ranode running); verify from another tailnet Mac.
3. Removal of the main app from the owner's Macs and the switch of their setup (permissions,
   options, password) to ranode — done in the machine-setup repo after ko.23.

## Blockers

- None. The release Mac is `m5m1`; agents build, sign and release there (`docs/operations.md`).
