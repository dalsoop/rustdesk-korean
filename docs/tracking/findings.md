This file is written in English only.

# Findings

## Unmerged branches kept after the 2026-11-21 and 2026-11-22 cleanups

- State on 2026-11-22 (owner's word "진행", session m5m1): 10 more remote branches were deleted, so
  the remote holds only the default branch, the release branches tied to tags (`release/ko.9`,
  `release/ko.10`, `release/ko19`, `release/ko23`, whose merge commits carry `1.5.0-ko.9`,
  `1.5.0-ko.10`, `1.5.0-ko.20` and `1.5.0-ko.23`) and three branches kept on purpose:
  - `fix-texture-lifetime` (24a16d9a): its upstream PR #15851 is still open.
  - `keyboard-shortcuts` (fe93cbe5): upstream #14933 is merged and arrives with the next upstream
    sync; delete it then.
  - `fix/wayland-pipewire-latch` (b2612262): a fork-authored fix never proposed upstream; keep it
    until someone decides whether to propose it.

  Deleted on 2026-11-22 (restore with `git push origin <sha>:refs/heads/<branch>`; also in
  `~/tmp/rdk-hygiene/deleted-branches-20261122.tsv` on m5m1):
  `auto-update-webpki-roots` 18f5f72bf4ce1f6ee0ec8cf765431b7137ae88c5 (bot commit of the disabled
  "Update webpki-roots" workflow), `dxgi-reuse-staging-texture`
  9343affe0bf90110a494134a8076c4060d41bbd5, `fix-unzoomed-cursor-dpi`
  8da629c57de7baf97127448089100cd75e8cb939, `fix/16073-ipc-exe-check-eacces`
  9dd427d49fb903c7a664ad412267d8cb495cd939 and `svt-av1` e17efa947477f9eaf0fcb9d1cb013732fb38811a
  (each: upstream PR closed without merge), `sciterjs` e86d0a0fc14f0bff4cb1ecd94a84eae42818da80
  (2022 upstream branch, unused), and, fully merged into the default branch with their PRs merged,
  `docs/ko23-released` e416681a80c82289a0846bb063668e6e09729b04,
  `feat/ko24-ranode-id-incoming` c68083f85fa63a0cb38eb091b282b49edd09c13c,
  `fix/qa-public-client` 28e801def0ed25a0af9ff4072a335ec6736fcea0,
  `fix/qa-ranode-incoming` f7372b66712fe644212243bc70fc1cb2429cdb05.
- Earlier state, 2026-11-21 (owner's go-ahead in session m5m1: "그거 pr 이랑 이슈 다 해소했나봐봐")
  33 remote branches were deleted: every branch fully merged into the default branch, and six
  unmerged ones whose content is superseded by merged work (list below). PR #19 was already closed
  (2026-11-12). 14 branches remain: the default branch; `release/ko.9` and `release/ko.10` (merged
  release PR branches whose merge commits carry tags `1.5.0-ko.9` and `1.5.0-ko.10`);
  `release/ko19` (tag `1.5.0-ko.20`, hotfix base); `feat/ko24-ranode-id-incoming` (open PR #37);
  and nine unmerged branches with no merged replacement:
  - `auto-update-webpki-roots` (18f5f72b): a commit from the now-disabled "Update webpki-roots"
    workflow; upstream merged its own updates (#16713, #17074), which arrive with the next sync.
  - `fix/wayland-pipewire-latch` (b2612262): a fork-authored Wayland fix, never proposed or merged.
  - Upstream feature mirrors whose upstream PR was closed unmerged or is still open:
    `dxgi-reuse-staging-texture` (#15957 closed), `fix-texture-lifetime` (#15851 open),
    `fix-unzoomed-cursor-dpi` (#16144 closed), `fix/16073-ipc-exe-check-eacces` (#16088 closed),
    `svt-av1` (#15656 closed), `sciterjs` (2022, no merged PR), `keyboard-shortcuts` (upstream #14933
    merged, but that merge is not in the default branch yet).
- Blast radius: an agent may still pick one of these as a base.
- Why not now: each of the three kept branches still waits on something outside this fork.
- Approach: delete `keyboard-shortcuts` once an upstream sync contains #14933 and
  `fix-texture-lifetime` once upstream #15851 is closed or merged; decide whether to propose
  `fix/wayland-pipewire-latch` upstream, then drop or keep it.
- Restore record of the deleted branches (`git push origin <sha>:refs/heads/<branch>`; also in
  `~/tmp/rdk-hygiene/deleted-branches.tsv` on m5m1). Merged into the default branch:
  `ci/desktop-build-cache` b20dcbdee63cad0c39ef826229e7ec5a479df6b1,
  `docs/fork-project-docs` 74db0f52d28d19a51e7979377a16600773a4bfdf,
  `feat/android-korean-toggle-key` a578c38b83fea55da3450548763a79ebe39db421,
  `feat/for-korean-connect` ad80258b864cf0030fc8aea70cd6481d55cbbf02,
  `feat/for-korean-identifiers` 430d060b6fbee5b4e49f5d64f4c4f80c1fda30f7,
  `feat/fork-update-check` 8b6766f76d3d303e85f56fa66131b63275b61e44,
  `feat/keyboard-settings-clarity` 0fb5279f8a797b5014b127a05778c4d3997683e3,
  `feat/ko23-ranode-incoming` a29d06b6534c7ecd3303bc09f9602b307a672a4e,
  `feat/macos-hangul-keys` 12fcffdb17384a243dbacc62fc7ff0c49b723ccd,
  `feat/macos-korean-toggle` 711b26ba2edaf1ba34fc3bcf7b1d3ad382d4b611 (also tag `1.5.0-ko.8`),
  `feat/main-identity` eca1a6060ff4983ca74144c7de4b5e698c602455,
  `feat/public-device-sections` fb7283b9c719e2eaece070b045d3913036f3a30d,
  `feat/public-display-name` 5aa099401427579753053ea4e66f5d6b4f6c2b85,
  `feat/public-identity` 86488aacd6ed5aee17ad1a59b192687a37fad572,
  `feat/public-onboarding` 1721454cf2f23e467fd39e67946031ed6492ee5a,
  `feat/public-tailscale-optional` dbd145f457b2fdc448b437e6f0fbfad4e60ffee5,
  `fix-android-korean-ime` a9c5ae7c57c1f2db1a915681aefb396d37afb453,
  `fix/for-korean-public-login` 9e7edf20061acdeb1f938ce450c6c8473c84da88,
  `fix/input-source-2-grab-race` 2d00c18d2b3b313da98dfe807beb471a7538ff4c,
  `fix/ko21-tailscale-term` 86db16f58a327ac75c2c82778ae177b518afcdfd,
  `fix/ko9-deb-version` 47899a7b37b6679629410a62e635259ff397eca3,
  `fix/macos-remote-hangul-hotkey` 5fe20fbb2c692b921fc8906fa6b9fdb7b75674fa,
  `fix/macos-server-keepalive` 5a1fd20f700f881d7358c7b2c05ddb68cfe1660a,
  `fix/main-app-name` c2da16cb06b31dacceb80cdb9969d8c3191ea932,
  `fix/main-app-name-for-korean` fda258b6d602cd823f924243631a778955cad6a0,
  `fix/soft-keyboard-grapheme-diff` ba9c8a691b6e4f33c0f840cee63d51edaf31eb5e,
  `master` a9c5ae7c57c1f2db1a915681aefb396d37afb453 (same head as `fix-android-korean-ime`).
  Unmerged but superseded:
  `fix/macos-tailscale-localapi` 4faad961949c6245bdfd50a9cad143fff56c6c55 (PR #19, closed,
  superseded by #35), `hotfix/ko20-public-login` fdc7349d4415084b762ede8684b16e98f79c1276 (merged
  into `release/ko19` by #30, ported by #31), `webrtc` 7dc127c00faf8103e973423c94eadc453d33fe85 and
  `worktree-port-forward-shared-conn` c2927046f80ec7b360663f6f41c81e6974f0cc4f (each the final head
  of upstream #15684 / #16062, whose merges ae6af2de / 3fc11c0f are in the default branch),
  `hdr-tonemap` ce968c02dd62c9246ad30f047f7f68b60243513a and
  `temporary-password-rotates-at-authorization` 2637003859127891a0dcf369fd3e0d25f67cc494 (earlier
  iterations of upstream #16033 / #16080, merged later as 58ff78a8 / d4ac2c07, in the default
  branch).

## ranode on m5m2 could not open a connection to a LAN address

- Condition: during the m5m1 cutover checks (2026-11-22), ranode ko.22 on m5m2 opened
  `rustdeskpublic://connection/new/<m5m1's LAN address>` twice and showed
  "Failed to connect to <address>:21118" without reaching m5m1 (ranode there logged no
  incoming connection), while `nc` from an SSH session on m5m2 reached the same port. Likely
  macOS Local Network privacy for the GUI app on m5m2 (not checked), or a client-side path choice.
- Blast radius: none for security (the receiving side never saw the attempt); the check "a LAN
  address of the receiving Mac is refused by the whitelist" stays unproven from a GUI client.
- Why not now: m5m2 is the owner's working Mac; its privacy settings were not changed.
- Approach: at the next live check, run the LAN-address attempt from a Mac whose ranode has Local
  Network access, and look for "Your ip is blocked by the peer" on the client and the refusal on
  the receiving Mac.

## A "RustDesk 없음" memo written before ko.23 outlives a timing-out device

- Condition: up to ko.22 a port check that timed out was remembered as "RustDesk 없음"
  (`public-onprem-rustdesk-memos`). ko.23 never remembers a timeout, but it also does not clear
  such an old memo, so the device shows "RustDesk 없음" again while each new check runs, then
  "접속 불가". Seen on m5m1 on 2026-11-21: `win-build` (port 21118 times out) still had the memo
  `missing` after ko.23 ran.
- Blast radius: cosmetic, for a few seconds per check, on Macs that ran ko.22 or earlier.
- Why not now: clearing it means changing the memo rules (new state handling); found during the
  ko.23 release.
- Approach: when the memo code is next touched, drop a device's memo when its check ends
  `unreachable`; until then the memo is overwritten as soon as the device answers or refuses.

## Stale "owner's own server" claims about the main app

- Condition: `docs/architecture.md` (Rendezvous row, main-app cell) and `docs/security-policy.md`
  ("Who may connect in", main-app row) say the main app uses the owner's own RustDesk server; the
  machine setup points it at the default public server. The comment in
  `flutter/lib/desktop/pages/desktop_home_page.dart` that says ranode makes only outgoing
  connections is stale since ko.23.
- Blast radius: an agent planning the main-app removal or ID access reads the wrong server.
- Why not now: outside ko.24's change scope (found in its final review).
- Approach: reword the two rows and the comment in the next ranode cycle.

## The office subnet trusts any LAN with the same numbering

- Condition: from ko.24 a Craft Mac's ranode accepts peers from its listed office subnet when the
  connection arrives on the Mac's own address in that subnet on a non-tunnel interface, with the
  route back through it (the decision 0009 rule applied to the subnet). A private range does not
  prove which LAN it is: on another network with the same numbering, devices that know the Mac's
  permanent password could connect. Virtual interfaces other than `utun*`, `tailscale*`, `ipsec*`
  and `ppp*` (`bridge*` of VMs, `gif*`, `stf*`) count as LAN for this rule.
- Blast radius: only a Craft Mac moved out of the office onto such a network, or an office subnet
  that overlaps a VM bridge; the owner's own Macs list no office subnet. The permanent password is
  still required (approve mode `password`).
- Why not now: binding the subnet to one network (gateway MAC, Wi-Fi SSID) needs new state; the
  Craft Macs are fixed office Macs.
- Approach: revisit if a Craft Mac ever leaves the office; the machine setup can clear the subnet
  with `--set-incoming-policy --extra-subnets none`.

## A refused relay makes the controller wait for its timeout

- Condition: from ko.24 ranode refuses relayed incoming in `create_relay_connection_`, after
  upstream has already answered the rendezvous server; the controller waits for its relay timeout
  instead of seeing "Your ip is blocked by the peer", and each refusal logs one error line.
- Blast radius: cosmetic (no access is granted); repeated attempts add log lines.
- Why not now: failing faster means changing upstream's relay handshake in `rendezvous_mediator.rs`.
- Approach: leave as is unless the log volume becomes a problem.
