This file is written in English only.

# Status (2026-11-17)

## Released

| Area | State | Evidence |
|---|---|---|
| Android Korean soft keyboard (grapheme diff, 두벌식 to Linux, F19/F24, 한/영 button) | released since ko.1–ko.4 | 3 Flutter test files run in CI on push (green 2026-11-17); checked by hand on a Galaxy Z Fold to macOS (ko.1) and to Pop!_OS COSMIC/fcitx5 (ko.3) |
| Desktop 한/영 → peer OS key, macOS input-source switch, local hotkey pause | released ko.5–ko.10 | Rust unit tests exist (26 in `hangul_key`, 7 in `macos_input_source`, hotkey-flag tests); not run in CI, last pass not recorded; never tested with a Windows peer |
| Fork update check | released ko.11 | Rust unit tests exist (`fork_update`, `updater`); not run in CI |
| ranode (on-prem + internet tabs, onboarding, sections, identity, optional Tailscale, public sign-in) | released up to ko.21 (latest release, 2026-11-17) | Flutter tests `tailscale_test`, `public_onprem_test`: 8 passed locally at PR #33 (2026-11-17); run in CI on push from ko.21 |
| Main app shown as "RustDesk (for korean)" | released ko.19 | installed on the owner's Macs |

## Merged, not released

- ko.21 is released (2026-11-17): ranode recovers by itself when Tailscale stops answering (PR #33);
  the unreleased renames were reverted (#32).
- ko.22 (version `1.5.0+6822`, merged as PR #35): ranode reads Tailscale when opened from Finder,
  the Dock or at login (`TERM=dumb` for the CLI).
- ko.23 (version `1.5.0+6823`, merged): ranode also accepts incoming connections from the tailnet
  with its permanent password only; left pane "이 Mac에 접속받기"; `--set-permanent-password`.
  Built: Rust compiled in the Windows/Linux CI build; Flutter tests green in CI; the Rust unit
  tests (`public_flavor`) are compiled and run first on the owner's release Mac.

## Remaining, in order

1. **ko.22 release (handed to the owner)**: build, sign and release the merge commit of PR #35
   (`version: 1.5.0+6822`) on the owner's release Mac, the same way as ko.21. Before marking it
   Latest, launch the signed ranode from Finder with Tailscale up (device list appears), stopped,
   restarting and absent (each shows its own state), and once from a terminal (unchanged). Mark
   Latest only after that check passes.
2. **ko.23 release (handed to the owner)**: on the owner's release Mac, from the merge commit of
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
   Mark Latest only after all pass. Release ko.22 first (or fold it in: ko.23 contains it).
3. **Machine-setup cutover (after ko.23 is released)**, for each owner Mac: install ko.23 ranode;
   grant it Screen Recording, Accessibility and Input Monitoring; quit and remove the main app
   before (re)starting ranode, because ranode retakes a busy 21118 only at its next start; start
   ranode and add it as a login item; set the vault password with `--set-permanent-password`
   (stdin, as the user, ranode running); verify from another tailnet Mac.
4. Removal of the main app from the owner's Macs and the switch of their setup (permissions,
   options, password) to ranode — done in the machine-setup repo after ko.23.

## Blockers

- None. The release Mac that signed ko.21 is not reachable from the company tailnet; releases run
  where ko.21 was built.
