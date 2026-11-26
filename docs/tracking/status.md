This file is written in English only.

# Status (2026-11-22)

## Released

| Area | State | Evidence |
|---|---|---|
| Android Korean soft keyboard (grapheme diff, 두벌식 to Linux, F19/F24, 한/영 button) | released since ko.1–ko.4 | 3 Flutter test files run in CI on push (green 2026-11-17); checked by hand on a Galaxy Z Fold to macOS (ko.1) and to Pop!_OS COSMIC/fcitx5 (ko.3) |
| Desktop 한/영 → peer OS key, macOS input-source switch, local hotkey pause | released ko.5–ko.10 | Rust unit tests: 26 in `hangul_key` run in CI (`fork-rust-tests.yml`, push and PRs); `platform::macos` (13, input source and hotkey flags) Mac-only; all passed on m5m1 2026-11-21 at 223d2e56; never tested with a Windows peer |
| Fork update check | released ko.11 | Rust unit tests `fork_update` (4), `updater` (3) run in CI (push and PRs); passed on m5m1 2026-11-21 |
| ranode (on-prem + internet tabs, onboarding, sections, identity, optional Tailscale, public sign-in) | released up to ko.24 (latest release, 2026-11-22) | Flutter tests `tailscale_test`, `public_onprem_test` run in CI on push and PRs. ko.22 signed build checked live on m5m1 (opened with `open -b` and from a terminal: device list and "Tailscale: … 연결됨") and on m5m2 after install (device list); ko.23 signed build checked live on m5m1 (below) |
| ranode incoming from the tailnet (Tailscale interface and return route, IPv6 tailnet range, permanent password only, exclusive 21118, left-pane card) | released ko.23 (2026-11-21); receiving on m5m1 and m5m2 since their cutovers (2026-11-22) | Rust tests `public_flavor` passed on m5m1 at 80d94a8b and 6b6ed6c7 and in CI. Live on m5m1 and m5m2 after the cutovers: below ("ko.24 release and m5m1 cutover", "m5m2 cutover") |
| ranode incoming by RustDesk ID, per-Mac incoming policy, relayed incoming refused, ID on the card | released ko.24 (2026-11-22) | Fork Rust tests (42) and `platform::macos` (13) passed on m5m1 at 6b6ed6c7; CI green. Live on m5m1: below |
| Windows msi Cancel fix (issue #10, error 2866) | released ko.23 | msi built in CI. Checked live on 2026-11-22 with the ko.24 msi on a Windows 11 VM over an exe-installed RustDesk: full UI showed the "uninstall it first" dialog and Cancel ended setup with 1602, no 2866 in the log or the MsiInstaller events; `/qn` failed with 1603 and the `BlockSelfInstalledApp` message (issue #10 comment) |
| Main app shown as "RustDesk (for korean)" | released ko.19 | installed on the owner's Macs |

## Merged, not released

- ko.26 (`version: 1.5.0+6826`): Caps Lock as the local 한/영 key on a controlling Mac toggles the
  peer once per press (two key-downs within 80 ms are one press; engineering note "One Caps Lock
  press arrives as two key-downs"). Rust: the new `hangul_key` test in CI (`fork-rust-tests.yml`).
  Live check after the machine-setup install: from a Mac with "Caps Lock switches to ABC" on, one
  "sent" line per press on the controlling Mac and one "switched" line on the controlled Mac.

- ko.25 (`version: 1.5.0+6825`): ranode keeps each own device's access password, encrypted on the
  Mac, and the on-prem card connects with it (`--save-peer-password`, decision 0010). Released by
  the watch job after the merge; the live check (card click without a password prompt) follows the
  machine-setup install. Rust: ranode `cargo check --features flutter` and the fork tests on m5m1.

## Automatic release (2026-11-22)

From ko.25 a merged version bump is released without a person: the m5m1 watch job (machine-setup
repo, `rustdesk-release-watch`) runs `res/release-ko.py` on the branch head (docs/operations.md
"Release ko.N"). Owner's word: "진행" (2026-11-22, session m5m1). Checked on m5m1 against the ko.24
commit 6b6ed6c7 without publishing:

| Check | Result |
|---|---|
| `--dry-run 6b6ed6c7` from scratch (Windows/Linux from run 44871878795, APK from upstream-mono job 18247266755 found by the `rustdesk/` tree, both macOS apps built through the build queue and signed/notarized) | exit 0; both zips CFBundleVersion 6824, team V2Z46YY386, "Notarized Developer ID", stapled; APK versionCode 8824 with the pinned certificate; deb `1.5.0+ko24`; source tree 09c49132 (as released); APK, exe, msi and deb byte-identical to the published ko.24 files (the zips differ only by their new signatures) |
| `--draft-only --release-tag 1.5.0-ko.24-autotest` | exit 0: draft on 6b6ed6c7, 12 files uploaded, GitHub sha256 equal to ours; LaunchServices clean; TCP 21118 still owned by the receiving ranode (same pid); a second run uploaded nothing; the draft was deleted (no tag left, Latest still ko.24) |
| release mode on 6b6ed6c7 | "already released", published release verified |
| a commit from inside merged PR #37 (c68083f8) | refused (exit 2): not a first-parent commit |
| watch job decision with the branch at ko.24 | "1.5.0-ko.24 is released", nothing run |

The UI check of the new ranode is skipped by design on m5m1 (its ranode receives remote sessions;
operations explains why); the live checks follow the machine-setup install of each release.

## ko.24 release and m5m1 cutover (2026-11-22)

Release `1.5.0-ko.24` on 6b6ed6c7 (merge of PR #37; `version: 1.5.0+6824`), marked Latest after the
live checks below, built, signed and published on m5m1 as a notarized Developer ID release (not an
internal build): both macOS zips "Notarized Developer ID", Windows/Linux from CI run 44871878795,
Android from upstream-mono MR !57 signed with the pinned certificate (versionCode 8824). Owner's
word for the release and the cutover: "진행" (2026-11-22, session m5m1).

m5m1 cutover (operations "Cutover keeping the Mac's ID"; m5m1 is an owner Mac, so no incoming
policy was set): ranode ko.24 installed; Screen Recording, Accessibility and Input Monitoring on
for `net.ranode.rustdesk-public`; the main app's user agent and root daemon unloaded and their
plists moved out; the main app's `RustDesk.toml` copied over ranode's; ranode started and took
21118; permanent password set from the vault with `--set-permanent-password` (exit 0); ranode added
as a login item; after the checks the main app bundle and its Dock item were moved out of
`/Applications` (kept in a backup on m5m1 with a one-step rollback, see the task record).
`RustDeskPublic --get-id` printed the main app's ID before and after, and the card shows it.
Tailscale was never stopped, restarted or switched; SSH stayed up throughout. m5m2 and the
MacBooks were not cut over then (m5m2 followed the same night, "m5m2 cutover" below).

Live checks (client: ranode ko.22 on m5m2; numbers of the ko.23 list in Remaining 1 and the ko.24
list in Remaining 4 of the previous status):

| Item | Result |
|---|---|
| ko.23 1: tailnet Mac connects by Tailscale IP + permanent password and controls it | passed: m5m2 → 100.x of m5m1, session opened with the permanent password, m5m1's screen shown on m5m2, video service started on m5m1; input not exercised |
| ko.23 2 / ko.24 2: connection through the public server refused | passed: by ID from m5m2, the peer arrived from the shared public address (WebRTC path) and got "귀하의 IP가 상대방에 의해 차단되었습니다" |
| ko.23 4: after the main app left, ranode accepts | passed (`lsof`: RustDeskPublic owns 21118) |
| ko.23 5: stopped-service line | not run: it needs the service toggled off in ranode on the only receiving Mac |
| ko.23 6: `--set-permanent-password` (stdin, as the user, ranode running) | passed: `Done!`, exit 0, ranode logged "permanent-password updated" |
| ko.23 7: one-time password refused; permanent password works | partly: two wrong passwords got "잘못된 비밀번호" and the permanent password worked; the one-time password itself cannot be read (ranode never shows it), so it was not tried |
| ko.23 8: Tailscale off/on, tailnet switch | not run: never stop or switch Tailscale on these Macs |
| ko.23 9: "RustDesk 없음" / "접속 불가" in the folded section | passed (m1m1, m1m2: "RustDesk 없음"; win-build, dns01, dns02, dns03: "접속 불가") |
| ko.23 10: update card names "RustDesk (forked ranode)" | not observed: ko.24 is the first newer Latest; m5m2's ko.22 ranode was not restarted (owner's working Mac) |
| ko.23 11: Tailscale IPv6; LAN 100.70.x alias refused | not run: the `rustdeskpublic://` link does not take an IPv6 address and ranode has no address field; the alias test would change a LAN host |
| ko.24 1, 2 (Craft Mac by ID inside the office; office-range peer at the Tailscale IP) | not run: no Craft Mac has been cut over |
| ko.24 3: own Mac, Tailscale IP works and click-accept is offered | passed: m5m1 showed the accept window ("수락") before the password was entered |
| ko.24 4: card shows the ID; `--set-incoming-policy` | passed: card shows the ID (the main app's, kept), the Tailscale address, "비밀번호: 설정됨"; with ranode running exit 2, a public or Tailscale-range subnet exit 1, nothing stored; the printed policy line not run (it would store a policy on an owner Mac, where the procedure sets none) |
| ko.24 5: grants survive a rebuild signed with the stable identity | not run: no internal build yet |
| LAN address from another Mac | not concluded: m5m2's ranode failed to open TCP to m5m1's LAN address before reaching it (no log line on m5m1), while `nc` from m5m2 reached 21118; see `findings.md` |

## m5m2 cutover (2026-11-22)

The other owner Mac, cut over by the same procedure at 00:58–01:09 KST after the owner's "진행"
(2026-11-22, session m5m1). Before: the main app (6819) owned 21118, ranode ko.22 (6822) ran for
outgoing use only, and no remote session was open in either app (no established connection on
21118 or of either app, no `--cm` process). The owner's other apps were not touched; Tailscale was
never stopped, restarted or switched; SSH stayed up throughout.

Steps: ranode ko.24 installed (6822 → 6824, "Notarized Developer ID"); Accessibility and Screen
Recording turned on for `net.ranode.rustdesk-public` in System Settings (Input Monitoring was
already on); the main app's agent and daemon unloaded and their plists moved out; its
`RustDesk.toml` copied over ranode's; ranode started and took 21118; `--get-id` printed the main
app's ID (219847905, before and after; ranode's own ID until then was a different one); a new
permanent password from the vault set with `--set-permanent-password` (`Done!`, exit 0); ranode
added as a login item; after the check below the main app bundle was moved out of `/Applications`
(kept in a backup on m5m2 with a one-step rollback; no Dock item). No incoming policy (owner Mac).

| Item | Result |
|---|---|
| ko.23 1: tailnet Mac connects by Tailscale IP + permanent password and controls it | passed: m5m1's ranode ko.24 → m5m2's Tailscale IP, session opened with the new permanent password, m5m2's screen shown on m5m1, video service started on m5m2; input exercised (two macOS prompts on m5m2 answered through the session) |
| ko.23 4: after the main app left, ranode accepts | passed (`lsof`: RustDeskPublic owns 21118 on m5m2) |
| First start of ranode ko.24 as the receiving app | macOS asked on m5m2 to let ranode bypass the private window picker and to find devices on the local network; both allowed (as on m5m1) |
| ko.23 10: update card | still not observed: m5m2 went from ko.22 straight to the ko.24 install |

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

1. **Cutover steps as a machine-setup script**: both owner Macs (m5m1, m5m2) are cut over and the
   machine-setup repo's list of Macs that keep the main app is now empty (its daily app check
   expects ranode on every Mac and the main app on none); the cutover steps (operations "Cutover
   keeping the Mac's ID") are not yet a machine-setup script.
2. **Live checks still open** (from the ko.23 and ko.24 lists; done ones are in the table above):
   1. with the service stopped the card says "지금은 접속받지 않음 (서비스 중지됨)";
   2. on a Mac where Tailscale may be stopped: Tailscale off/on and a tailnet switch;
   3. a newer Latest shows the update card naming "RustDesk (forked ranode)" (ranode ko.22/ko.23
      at its next start);
   4. another tailnet Mac connects by the Tailscale IPv6 address; a LAN host given a 100.70.0.x
      alias is refused at the LAN address and at the Tailscale IP routed over the LAN;
   5. Craft Mac: by the kept ID from inside the office, password only, no click offered; by
      Tailscale IP; by ID from outside the office refused; an office-range peer sent to its
      Tailscale IP refused (decision 0009 addendum);
   6. `--set-incoming-policy` prints the effective policy line (on a Craft Mac, ranode quit);
   7. after a rebuild signed with the stable internal identity, the macOS grants are still on.

## Blockers

- None. The release Mac is `m5m1`; agents build, sign and release there (`docs/operations.md`).
