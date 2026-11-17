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
- ko.22 (version `1.5.0+6822`, in PR, not merged): ranode reads Tailscale when opened from Finder,
  the Dock or at login (`TERM=dumb` for the CLI).

## Remaining, in order

1. **ko.22 release (handed to the owner)**: build, sign and release the merge commit of PR #35
   (`version: 1.5.0+6822`) on the owner's release Mac, the same way as ko.21. Before marking it
   Latest, launch the signed ranode from Finder with Tailscale up (device list appears), stopped,
   restarting and absent (each shows its own state), and once from a terminal (unchanged). Mark
   Latest only after that check passes.
2. **ko.23**: ranode also accepts incoming connections (Tailscale range only, direct port 21118, no
   RustDesk ID, no service, same permanent password), taking over port 21118 once the main app is
   gone; its left pane stops saying "접속하는 쪽 전용".
3. Removal of the main app from the owner's Macs and the switch of their setup (permissions,
   options, password) to ranode — done in the machine-setup repo after ko.23.

## Blockers

- None. The release Mac that signed ko.21 is not reachable from the company tailnet; releases run
  where ko.21 was built.
