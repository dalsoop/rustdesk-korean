This file is written in English only.

# Status (2026-11-17)

## Released

| Area | State | Evidence |
|---|---|---|
| Android Korean soft keyboard (grapheme diff, 두벌식 to Linux, F19/F24, 한/영 button) | released since ko.1–ko.4 | 3 Flutter test files run in CI on push (green 2026-11-17); checked by hand on a Galaxy Z Fold to macOS (ko.1) and to Pop!_OS COSMIC/fcitx5 (ko.3) |
| Desktop 한/영 → peer OS key, macOS input-source switch, local hotkey pause | released ko.5–ko.10 | Rust unit tests exist (26 in `hangul_key`, 7 in `macos_input_source`, hotkey-flag tests); not run in CI, last pass not recorded; never tested with a Windows peer |
| Fork update check | released ko.11 | Rust unit tests exist (`fork_update`, `updater`); not run in CI |
| ranode (on-prem + internet tabs, onboarding, sections, identity, optional Tailscale, public sign-in) | released up to ko.20 (latest release) | Flutter tests `tailscale_test`, `public_onprem_test`: 8 passed locally at PR #33 (2026-11-17); not in CI |
| Main app shown as "RustDesk (for korean)" | released ko.19 | installed on the owner's Macs |

## Merged, not released

- ko.21 (version `1.5.0+6821` on the default branch): ranode recovers by itself when Tailscale stops
  answering (PR #33); reverts of the unreleased renames (#32).

## Remaining, in order

1. **ko.21 release**: also fix the Tailscale CLI failing when ranode is launched from Finder (give
   the CLI `TERM=dumb`), close PR #19, release, verify live from Finder.
2. **ko.22**: ranode also accepts incoming connections (Tailscale range only, direct port 21118, no
   RustDesk ID, no service, same permanent password), taking over port 21118 once the main app is
   gone; its left pane stops saying "접속하는 쪽 전용".
3. Removal of the main app from the owner's Macs and the switch of their setup (permissions,
   options, password) to ranode — done in the machine-setup repo after ko.22.

## Blockers

- The Mac that signed releases up to ko.20 no longer holds the owner's macOS signing keys, and the
  former build Mac is gone; the ko.21 release needs a decided signing Mac with those keys.
