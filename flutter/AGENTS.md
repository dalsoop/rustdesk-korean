This file is written in English only.

# flutter/ — fork-owned Flutter

## Scope
ranode UI: `lib/desktop/pages/public_home.dart` (tabs, on-prem sections, cards, left pane),
`public_onboarding.dart` (first-run guide; `PublicIntroCard` is the left pane's "이 Mac에 접속받기"
card: this Mac's RustDesk ID with a copy button, shown only when `gFFI.serverModel.serverId.text`
is non-empty and not `translate('Generating ...')`, the card listening to `gFFI.serverModel`;
own Tailscale IPv4 from `Tailscale.latest`; password status and dialog; stopped-service line; never
the one-time password), `lib/common/tailscale.dart` (CLI runner, states,
reachability), `public_onprem.dart` (recents/favourites records), `public_login.dart`,
`tailscale_range.dart` (no `dart:io`, compiled for web too). Korean input:
`lib/models/soft_keyboard_input_utils.dart`, `hangul_dubeolsik.dart`, `korean_toggle_key.dart`,
`lib/mobile/pages/remote_page.dart`, `lib/desktop/widgets/remote_toolbar.dart` (input source 1/2
explanations, Input Monitoring state). Fork edits in shared pages: `desktop_setting_page.dart` (About "1.5.0 (ko.N)", the 한/영
setting and its status), `common/widgets/peers_view.dart` (Tailscale-range filter),
`common/widgets/login.dart` (third-party-only sign-in), desktop/mobile `connection_page.dart`,
`models/input_model.dart` (remote-keyboard-active hook), `models/model.dart` (login-required
prompt), `models/peer_tab_model.dart`, `common.dart`, `consts.dart`, `web/bridge.dart` stubs.
macOS app identity: `macos/Runner/Configs/AppInfo.xcconfig`,
`Info.plist` variables, `AppIconMain.icns`, `AppIconPublic.icns`; version line in `pubspec.yaml`.

Not this module's scope: upstream pages beyond thin `bind.mainIsPublicFlavor()` branches;
`generated_bridge.dart` (generated); `Flavor.xcconfig` (written by `build.py`, git-ignored).

## Invariants
- Every ranode-only branch checks `bind.mainIsPublicFlavor()`; the main app renders upstream's UI.
- `Tailscale._run` never throws, returns null on any failure, stops a hung CLI (SIGTERM, SIGKILL
  after 1 s), and decodes output leniently; `Tailscale.read()` shares one in-flight read.
- The CLI is located by `findBinary()` only (app bundle, `PATH`, `/opt/homebrew/bin`,
  `/usr/local/bin`). A Finder-launched app has no shell environment, and the Tailscale app binary
  then refuses to act as a CLI; `_run` therefore starts every CLI with
  `tailscaleCliEnvironment(...)` (`TERM=dumb` added only when `TERM` is missing or empty).
- `notFound` means no binary; `notResponding` means a binary that did not answer in time; neither
  is reported as "stopped".
- The page's loading flag is cleared on every path of a refresh.
- Option names `public-onprem-records`, `public-onprem-rustdesk-memos`, `public-start-tab`,
  `public-welcome-done`, `korean-toggle-key` are stored user data; renaming them loses it.
- `kPublicAppDisplayName` equals `RUSTDESK_DISPLAY_NAME` in `build.py`.
- Soft keyboard: F19 is never re-sent inside a word; diffs are by grapheme cluster.

## Tests
- `test/tailscale_test.dart`, `test/public_onprem_test.dart`: state parsing, records, public API
  server detection. Add a case for each new Tailscale state or CLI argument/environment change.
  Both files run in CI with the soft-keyboard tests.
- `test/soft_keyboard_input_utils_test.dart`, `hangul_dubeolsik_test.dart`,
  `korean_toggle_key_test.dart`: composition diffs, 두벌식 mapping, auto key per OS.
- Live check for ranode changes: launch from Finder with Tailscale up, stopped, restarting, absent.
