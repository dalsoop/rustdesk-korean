This file is written in English only.

# Business rules

## Terms

- **Controlling side**: the device whose user types; it sends keys. **Controlled side** (host): the
  device that receives the connection and injects keys.
- **한/영 key**: the key that switches Korean/English. Per OS: Windows `VK_HANGUL` (also right Alt on
  a Korean layout); Linux the Hangul key (`KEY_HANGEUL`, keysym `Hangul`); a Mac has no Hangul key,
  so its 한/영 is whatever its "Select the previous / next input source" shortcuts are (F18 when Caps
  Lock is remapped, ⌃Space by default), plus LANG1, Caps Lock when "Use Caps Lock to switch to and
  from ABC" is on, and 🌐 tapped alone when "Press 🌐 to" is "Change Input Source".
- **Main app** "RustDesk (for korean)" and **ranode** "RustDesk (forked ranode)": the two macOS apps.
  "RustDesk Public" / "public flavor" in code, file names and release zips means ranode.
- **ko.N**: the fork release number. Release tag `1.5.0-ko.N`.

## 한/영 across operating systems (desktop controlling side)

Option `enable-peer-hangul-key`, default **on**. When on, the local 한/영 key is replaced by the peer's
own whenever the OSes differ, and also Mac → Mac:

| Local → peer | Sent |
|---|---|
| Windows or Linux → Mac | F18 |
| Mac (its input-source shortcut) or Windows → Linux | Hangul key (X keycode 130) |
| Mac or Linux → Windows | `VK_HANGUL` as a legacy control key (the Windows IME only toggles on the virtual key) |
| Mac (its input-source shortcut) → Mac | F18 (the controlled Mac switches itself) |
| legacy keyboard mode, different OSes or Mac → Mac | Hangul key |

- Windows → Windows and Linux → Linux: the key goes exactly as upstream sends it, in every
  keyboard mode, whether the option is on or off.
- With the option off, every key goes as upstream sends it.
- On a controlling Mac, a shortcut press sends only the 한/영 key, not ⌃ or Space; modifiers already
  sent are released before it and pressed again after, so the peer's modifier state matches.
- Only an exact match counts: Space alone, Shift+Space, or ⌃ with another key go unchanged.
  Holding the key switches once.
- The Mac's shortcuts are re-read each time a remote window takes the keyboard.
- While a remote window holds the keyboard (focused, pointer over the remote screen) and the option
  is on, the controlling Mac turns its own previous/next input-source shortcuts off and turns them
  back on when focus leaves or the window closes; after a crash they are restored when a remote
  window next opens. This needs no Input Monitoring permission.

## 한/영 on a controlled Mac

Option `enable-input-source-toggle`, default **on** (turned off only with
`sudo <app binary> --option enable-input-source-toggle N`).

- Arriving F18, Hangul (legacy mode) or LANG1 (Mac keycode 104) is not injected. Instead the Mac's
  own "previous input source" shortcut is posted at HID level, so the front app switches in key
  order and the next typed character already uses the new source.
- If that shortcut is off (or paused by this Mac's own remote window), the input source is switched
  directly through TIS: from a Korean source to ABC, from any other source to the first enabled
  Korean source. This direct path may apply one key late in the front app.
- Holding the key switches once; the key-up is never injected.
- When the shortcut is off and the Mac has no Korean input source, the key is injected unchanged;
  with the shortcut on, the shortcut is posted regardless of the installed sources.

## Android controlling side

- Soft-keyboard edits are diffed by grapheme cluster and re-sent as delete + insert, so a composing
  Hangul syllable never arrives as loose jamo.
- To a Linux peer, Hangul text is sent as 두벌식 keys (안 → d k s) for the peer's IME to compose.
  F19 is sent before the first Hangul key after a space, Latin letter or digit; F24 once before each
  edit that contains Latin letters; F19 is never re-sent inside one word (fcitx5 commits the composing syllable on it).
- The 한/영 toolbar button defaults to "auto": F18 to a Mac, Hangul to Windows/Linux; a long press
  pins one of Hangul, F18, right Alt, Shift+Space, Ctrl+Space.

## The two macOS apps

- Both apps accept incoming connections (ranode from ko.23); the main app is to be removed from the
  owner's Macs, after which ranode alone receives. Only one app can listen on 21118 at a time; a
  ranode started while the main app holds 21118 connects out normally and starts receiving only
  after it is started again with the port free.
- ranode settings that the user cannot change: `direct-server=Y`, whitelist `100.64.0.0/10`,
  `verification-method=use-permanent-password` (the one-time password is never accepted; a click
  to accept on the Mac still works).
- ranode's left pane "이 Mac에 접속받기" shows this Mac's Tailscale IPv4 (from the latest Tailscale
  read; "Tailscale이 켜져 있어야 접속받을 수 있습니다" when Tailscale is not running), whether a
  permanent password is set with a "비밀번호 설정" button, and "지금은 접속받지 않음 (서비스 중지됨)"
  when `stop-service` is `Y`. It never shows a RustDesk ID or one-time password. A busy port is not
  shown.
- When ranode on a Mac is closed, other Macs' on-prem tabs show that Mac as "RustDesk 없음".
- ranode's first launch clears, once, the own-server settings (custom rendezvous, relay, API
  server, key) and every peer, favourite and remembered remote id containing `@rustdesk.internal.kr`.
  Option `public-flavor-cleanup-done=Y` marks it done; it never runs again.
- ranode shows the on-prem tab only when a Tailscale binary exists on the Mac; it re-checks every
  5 s, so installing Tailscale is the switch. Without it the internet tab is the whole home.
- On-prem states and their next step: not installed → download link; stopped or signed out →
  open the Tailscale app; `NeedsMachineAuth` → waiting for admin approval; running with no other
  devices → say so. While not running the tab re-checks every 5 s; an installed binary that does not
  answer in 5 s is "not responding" and is re-checked after 5, 10, 20, then every 30 s, and at once
  when the window comes to the front. One read runs at a time; callers share it.
- Device sections, in order: 즐겨찾기, RustDesk 있음 · 바로 접속, RustDesk 있음 · 직접 접속 꺼짐, 확인 중,
  RustDesk 없음 (folded), 오프라인 (folded). Results are remembered per tailnet + device name
  (`public-onprem-rustdesk-memos`); an offline device never uses a remembered result.
- Recents and favourites of the on-prem tab are keyed by tailnet + device name
  (`public-onprem-records`) and never mix with internet ids.
- Switching to a non-active Tailscale profile needs the user's confirmation first.
- First-run guide: "팀 기기에 접속" (on-prem) or "인터넷으로 접속" (internet) sets the start tab
  (`public-start-tab`); "나중에" keeps on-prem; `public-welcome-done` marks it shown.
- Public server sign-in: with the public API server the sign-in dialog shows only third-party
  buttons (Google, GitHub, …), because that server refuses passwords; with any other server the
  dialog is upstream's. A "login required" refusal offers "로그인" and reconnects after it.

## Versions

- Build number = 68 × 100 + N (upstream 1.5.0 is build 68): ko.9 → `1.5.0+6809`. N runs 1–99:
  the ko number is read as build % 100, so ko.100 would read as ko.0; a build number below 100
  means "no ko number" (upstream versioning).
- Derived versions: Linux .deb `1.5.0+koN` (sorts after `1.5.0` for apt); Windows file and MSI
  version `1.5.0.68NN`; Android versionName `1.5.0-ko.N`, versionCode build + 2000 (arm64 split);
  macOS `CFBundleShortVersionString 1.5.0`, `CFBundleVersion 68NN`; About shows "1.5.0 (ko.N)".
- RustDesk protocol version stays `1.5.0` so the fork interoperates with official RustDesk.
- Update check: only this fork's GitHub "Latest" release (no prerelease, no draft); a tag
  `X-ko.N` is newer when `(X, N)` is greater than the build's. Windows installs the `.exe`/`.msi`;
  macOS, Linux and Android show a link to the release page only. The macOS root updater never runs
  (fork releases have no `.dmg`). A download URL outside `dalsoop/rustdesk-korean` releases is refused, and so is a "Latest" whose
  page URL is not `…/releases/tag/<tag>`.

## Interoperability invariants

- Never change the RustDesk protocol version or message formats.
- Korean-key behaviour Windows → Windows and Linux → Linux stays upstream's.
- The Android signing certificate never changes, so every APK installs over the previous one.
- The two macOS bundle ids never change, so permissions and settings carry over between releases.
