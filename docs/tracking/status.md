This file is written in English only.

# Status (2026-11-21)

## Released

| Area | State | Evidence |
|---|---|---|
| Android Korean soft keyboard (grapheme diff, 두벌식 to Linux, F19/F24, 한/영 button) | released since ko.1–ko.4 | 3 Flutter test files run in CI on push (green 2026-11-17); checked by hand on a Galaxy Z Fold to macOS (ko.1) and to Pop!_OS COSMIC/fcitx5 (ko.3) |
| Desktop 한/영 → peer OS key, macOS input-source switch, local hotkey pause | released ko.5–ko.10 | Rust unit tests: 26 in `hangul_key` run in CI (`fork-rust-tests.yml`, push and PRs); `platform::macos` (13, input source and hotkey flags) Mac-only; all passed on m5m1 2026-11-21 at 223d2e56; never tested with a Windows peer |
| Fork update check | released ko.11 | Rust unit tests `fork_update` (4), `updater` (3) run in CI (push and PRs); passed on m5m1 2026-11-21 |
| ranode (on-prem + internet tabs, onboarding, sections, identity, optional Tailscale, public sign-in) | released up to ko.23 (latest release, 2026-11-21) | Flutter tests `tailscale_test`, `public_onprem_test` run in CI on push and PRs. ko.22 signed build checked live on m5m1 (opened with `open -b` and from a terminal: device list and "Tailscale: … 연결됨") and on m5m2 after install (device list); ko.23 signed build checked live on m5m1 (below) |
| ranode incoming from the tailnet (Tailscale interface and return route, IPv6 tailnet range, permanent password only, exclusive 21118, left-pane card) | released ko.23 (2026-11-21), not yet receiving on any Mac | Rust tests `public_flavor` (6 of the 39 fork tests) passed on m5m1 at the release commit 80d94a8b and in CI. Live: only the card and the busy-port path were checked (main app keeps 21118 on every Mac until the cutover); the incoming checks wait for the cutover (Remaining, item 1) |
| Windows msi Cancel fix (issue #10, error 2866) | released ko.23 | msi built in CI; not checked on a Windows PC (`findings.md`) |
| Main app shown as "RustDesk (for korean)" | released ko.19 | installed on the owner's Macs |

## Merged, not released

- Nothing besides open PRs: ko.23 (below) took everything merged up to 80d94a8b.

## ko.23 release (2026-11-21)

Release `1.5.0-ko.23` on 80d94a8b (merge of PR #44; `version: 1.5.0+6823`), marked Latest, built,
signed and published on m5m1: PRs #36, #41, #42 (ranode incoming from the tailnet and its
boundary), #43 (msi Cancel, signing-script LaunchServices cleanup), #40 (CI). Fork Rust tests (39,
incl. `public_flavor` 6) passed on m5m1 at 80d94a8b. After both signing runs `lsregister -dump`
listed no `~/tmp/osx-sign.*` path (the #43 cleanup works).

Live checks of the signed ranode on m5m1 (launched with `open -b`, main app running and holding
21118, Tailscale up; screenshot evidence on m5m1 under `~/work/rdk/release-ko23-out/`). Tailscale
was never stopped, restarted or switched (it carries SSH and other sessions), the main app was not
restarted (it receives this Mac's remote sessions), and connecting in was not required:

| Item (numbers of the ko.23 check list; the open ones are in Remaining, item 1) | Result |
|---|---|
| Finder-style launch, Tailscale up | passed: device list, "Tailscale: urit245@gmail.com 연결됨", label 1.5.0 (ko.23) |
| 3. main app running: ranode does not take 21118 | passed (`lsof`: the main app keeps 21118); outgoing session not run |
| 5. left pane | address and "비밀번호: 설정 안 됨" shown, Screen Recording card shown; stopped-service line not run |
| 6. `--set-permanent-password` | with ranode quit: exit 2; under sudo: exit 1 "Refused"; setting a password not run |
| 9. "RustDesk 없음" / "접속 불가" | partly: section counts matched a port probe (2 reachable, 15 no answer, 6 offline) and m1m1 (port refused) is remembered as missing; the folded section's card labels were not read (m5m1's screen was locked, so no click) |
| 12. busy-port line, password-not-set line | passed |
| 1, 2, 4, 7, 11 (incoming) | not run: the main app must keep 21118 until the cutover, so ranode cannot receive |
| 8 (Tailscale off/on, tailnet switch) | not run: never stop or switch Tailscale on these Macs |
| 10 (update card) | not run: needs a Latest release newer than ko.23 |

Installed: m5m1 ranode 6823 (not running, as before; backup
`~/.local/state/dotfiles/RustDesk (forked ranode).app.previous-20261121-140343`), main app 6819
still owns 21118. m5m2 keeps ranode ko.22.

## Remaining, in order

1. **ko.23 incoming live checks, at the cutover of the first Mac** (the main app must have left
   21118 there; never by stopping Tailscale on a Mac that carries SSH):
   1. another tailnet Mac connects by this Mac's Tailscale IP + permanent password and controls it;
   2. a connection through the public server (non-Tailscale address) is refused;
   4. after quitting the main app and relaunching ranode, ranode accepts;
   5. with the service stopped the card says "지금은 접속받지 않음 (서비스 중지됨)";
   6. `--set-permanent-password` with the password on stdin sets it (exit 0);
   7. a one-time password is refused; the permanent password works;
   8. on a Mac where Tailscale may be stopped: turning Tailscale off shows "Tailscale이 꺼져
      있습니다" within about 30 s, on again shows the list; a tailnet switch shows the new tailnet;
   9. with the window unlocked, open "RustDesk 응답 없음": a Mac with RustDesk closed shows
      "RustDesk 없음", a peer that drops 21118 shows "접속 불가";
   10. with a newer Latest release, the update card names "RustDesk (forked ranode)";
   11. another tailnet Mac connects by this Mac's Tailscale IPv6 address too; a LAN host given a
       100.70.0.x alias is refused both at this Mac's LAN address and at its Tailscale IP routed
       over the LAN (decision 0009).
2. **Machine-setup cutover**, for each owner Mac: install ko.23 ranode;
   grant it Screen Recording, Accessibility and Input Monitoring; quit and remove the main app
   (and its LaunchAgent/LaunchDaemon where installed) before (re)starting ranode, because ranode retakes a busy 21118 only at its next start; start
   ranode and add it as a login item; set the vault password with `--set-permanent-password`
   (stdin, as the user, ranode running); verify from another tailnet Mac.
3. Removal of the main app from the owner's Macs and the switch of their setup (permissions,
   options, password) to ranode — done in the machine-setup repo.

## Blockers

- None. The release Mac is `m5m1`; agents build, sign and release there (`docs/operations.md`).
