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

## Merged or in PR, not released

- ko.21 is released (2026-11-17): ranode recovers by itself when Tailscale stops answering (PR #33);
  the unreleased renames were reverted (#32).
- ko.22 (version `1.5.0+6822`, merged as PR #35): ranode reads Tailscale when opened from Finder,
  the Dock or at login (`TERM=dumb` for the CLI).
- ko.23 (version `1.5.0+6823`, merged): ranode also accepts incoming connections from the tailnet
  with its permanent password only; left pane "이 Mac에 접속받기"; `--set-permanent-password`.
- ko.24 (version `1.5.0+6824`, in PR to `feat/sync-upstream-1.5.0`, not merged; awaiting merge): ranode accepts ID connections like the main app (per-Mac
  whitelist and approve mode via `--set-incoming-policy`, floor adds
  `allow-remote-config-modification=N`); the left pane shows the ID.
- ko.23 and ko.24 ship as internal builds (operations, internal builds), signed with the stable
  "ranode internal signing" identity; a Developer ID release is optional and later. Rust compiled
  in the Windows/Linux CI build; Flutter tests green in CI. CI's Rust test workflow is disabled, so
  the `public_flavor` unit tests of ko.23 and ko.24 have never run.

## Remaining, in order

1. **ko.22 release (handed to the owner)**: build, sign and release the merge commit of PR #35
   (`version: 1.5.0+6822`) on the owner's release Mac, the same way as ko.21. Before marking it
   Latest, launch the signed ranode from Finder with Tailscale up (device list appears), stopped,
   restarting and absent (each shows its own state), and once from a terminal (unchanged). Mark
   Latest only after that check passes. Optional once ko.24 is installed internally.
2. **ko.24 internal build (machine-setup cycle)**, once the ko.24 PR is merged, on the company
   build Mac: first `cargo test --lib public_flavor` on the merge commit (must pass; first run of
   the ko.23 and ko.24 tests), then `ranode-build` signed with the stable internal identity (`codesign -dv` shows it,
   never ad-hoc). ko.23 is not built separately: ko.24 contains it.
3. **ko.23 live checks, on that internal build, before the cutover**:
   1. another tailnet Mac connects by this Mac's Tailscale IP + permanent password and controls it;
   2. a connection relayed through the public server (non-allowed address) is refused;
   3. with the main app running, ranode works outgoing and does not take 21118;
   4. after quitting the main app and relaunching ranode, ranode accepts;
   5. the left pane shows the address and password status; missing Screen Recording /
      Accessibility grants show their cards; with the service stopped it says
      "지금은 접속받지 않음 (서비스 중지됨)";
   6. `--set-permanent-password` with the password on stdin sets it (exit 0), exits non-zero with
      ranode quit, and is refused under sudo;
   7. a one-time password is refused; the permanent password works.
4. **Machine-setup cutover**, per Mac: install the internal build, grant Screen Recording,
   Accessibility and Input Monitoring, run the cutover (operations), add ranode as a login item.
   The main app is removed from every Mac — done in the machine-setup repo.
5. **ko.24 live checks, on that internal build, after the cutover** (operations, "Cutover keeping
   the Mac's ID"):
   1. Craft Mac: by the kept ID from inside the office, password only, no click offered; by
      Tailscale IP;
   2. Craft Mac: by ID from outside the office → refused;
   3. own Mac: Tailscale IP works and click-accept is offered;
   4. the card shows the ID; `--set-incoming-policy` prints the expected line, exits 2 with ranode
      running and 1 for a public subnet;
   5. after a rebuild signed with the stable identity, the macOS grants are still on.

## Blockers

- None. The release Mac that signed ko.21 is not reachable from the company tailnet; releases run
  where ko.21 was built.
