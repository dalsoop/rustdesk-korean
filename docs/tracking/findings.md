This file is written in English only.

# Findings

## Unmerged branches kept after the 2026-11-21 cleanup

- Condition: on 2026-11-21 (owner's go-ahead in session m5m1: "그거 pr 이랑 이슈 다 해소했나봐봐")
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
- Why not now: none is superseded by something merged, so deleting them needs a decision on each.
- Approach: decide per branch (the webpki-roots bot branch can go once a sync brings upstream's
  roots; `fix/wayland-pipewire-latch` needs a keep-or-drop decision); `keyboard-shortcuts` can go
  once an upstream sync contains #14933.
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

## The msi fix for error 2866 is not checked on a Windows PC

- Condition: issue #10 (msi over an exe-installed RustDesk stopped with Windows Installer error
  2866 instead of showing "please uninstall it first") is fixed by publishing `EndDialog` `Exit`
  from `UI_AnotherAppDialog`'s Cancel (PR #43, 2026-11-21). The msi builds in
  the fork's Windows CI; no Windows PC was available to run it.
- Blast radius: if the dialog still fails, an msi install over an exe install ends in a generic
  failure (1603) as before; nothing else changes.
- Why not now: no Windows PC is available to this work.
- Approach: on the next Windows PC test, follow issue #10's "Verify a fix": on a PC with an
  exe-installed RustDesk, `msiexec /i <msi>` shows the dialog, Cancel ends setup with 1602 and no
  2866 appears in the MsiInstaller events; `msiexec /i <msi> /qn` still fails with the
  `BlockSelfInstalledApp` message in the log. Do not uninstall someone's existing RustDesk to test.

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
