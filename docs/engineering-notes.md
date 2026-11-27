This file is written in English only.

# Engineering notes

## The Fn restore event opened Dictation (fixed in ko.27)

- Symptom: on a Mac controlled through RustDesk, two 한/영 presses close together opened the macOS
  Dictation prompt (unified log on two company Mac Studios, 2026-11-26 and 2026-11-27, about one
  second after an input-source switch; the machine setup turned the Dictation shortcut off as a
  stopgap, its decision 0040).
- Cause: `MacPostHotKey` ended each switch with a `kCGEventFlagsChanged` event whose keycode was
  `kVK_Function` to clear the SecondaryFn left by F18. A flags-changed event names a modifier key, so
  macOS took it as an Fn press; two switches were "Fn pressed twice", the default Dictation shortcut.
- Response: `MacHotKeyEvents` lists the posted events, the key-down with the shortcut's flags and
  the key-up with `MacFlagsAfterHotKey`'s restored state; the key-up's flags clear SecondaryFn and
  nothing else is posted (tests `a_switch_posts_no_fn_key_event`,
  `the_key_up_keeps_a_physically_held_fn`).
- Rejected: a flags-changed restore with another keycode (still a modifier press to macOS); F18
  without SecondaryFn (the window server does not match an F-key shortcut without it).

## Caps Lock unreadable at the first focus (fixed in ko.27)

`MacReadCapsLockSwitch` and `MacReadFnUsageType` answer from a cache filled on the main queue; the
first call off it returned -1, so at a remote window's first focus Caps Lock (and 🌐) did not count
as 한/영 ("unreadable settings: caps-lock-switch" in the log). `main_init_input_source` now reads
both once when the window opens, before it can take the keyboard.

## One Caps Lock press arrives as two key-downs (fixed in ko.26)

- Symptom: from a controlling Mac whose "Use Caps Lock to switch to and from ABC" is on, Caps Lock
  does not change the remote Mac's input source. The controlling Mac's log shows "Local 한/영 key
  Caps Lock sent to macos peer" twice per press, 7 to 15 ms apart, and the controlled Mac's log
  "Remote 한/영 key Keycode(79): switched the input source (x2)" (ranode ko.20 on macOS 26.5 through
  the Flutter key path, no Input Monitoring, to the main app ko.19 on macOS 27, 2026-11-26).
- Cause: macOS reports that press as two Caps Lock key-downs; each became a 한/영 tap, so the peer
  switched twice and ended where it started.
- Response: `mac_key_action` treats Caps Lock key-downs less than `CAPS_LOCK_SAME_PRESS` (80 ms)
  apart as one press (test `caps_lock_double_key_down_of_one_press_toggles_once`). A shortcut
  route (Caps Lock mapped to F18) was never affected.

## Tailscale CLI run from a GUI app fails (fixed in ko.22)

- Symptom: ranode launched from Finder/Dock says "Tailscale이 꺼져 있습니다" (or not responding)
  while Tailscale is up; launched from a terminal, or with `open --env TERM=dumb -a "<app>"`, the
  device list appears.
- Cause: the standalone Tailscale app binary (`/Applications/Tailscale.app/Contents/MacOS/Tailscale`,
  bundle `io.tailscale.ipn.macsys`, seen on 1.104.x) acts as a CLI only when its environment looks
  like a shell (`TERM`, `TERM_PROGRAM` or `SHLVL` set). Otherwise it tries to start a second GUI,
  prints "The Tailscale GUI failed to start: … (Tailscale.CLIError error 3.)" and exits non-zero.
  A Finder-launched app has none of those variables, and `Process.start` passes them through.
  A `~/.local/bin/tailscale` wrapper that `exec`s the bundle binary fails the same way.
- Response: `_run` passes `tailscaleCliEnvironment(Platform.environment)`, which adds `TERM=dumb`
  when `TERM` is missing or empty and keeps a terminal's own `TERM`. Any new CLI call must go
  through `_run`. Verify by launching the app from Finder, never only from a terminal (a terminal
  launch hides the bug).
- Rejected alternative: Tailscale's LocalAPI over `127.0.0.1:<ipnport>` with the token files under
  `/Library/Tailscale` (PR #19); unofficial and macOS-variant specific.

## Tailscale read that never finished (fixed in ko.21, PR #33)

- Symptom: after Tailscale restarted, ranode stayed on "응답하지 않습니다" and "다시 확인" was dead
  until relaunch.
- Cause: an exception in the CLI call (malformed UTF-8 on stdout) left the loading flag set, which
  stopped re-checks; a CLI that did not answer got only SIGTERM and piled up.
- Response: `_run` never throws, decodes leniently, kills with SIGTERM then SIGKILL after 1 s; one
  shared read at a time; the page clears its loading flag on every path. Keep these properties
  when editing `tailscale.dart`.

## GUI apps get a minimal PATH

A Finder-launched app's `PATH` lacks `/opt/homebrew/bin` and `/usr/local/bin`; the Tailscale
lookup appends them explicitly. Any new external CLI call from Flutter needs the same.

## macOS 한/영 shortcuts are decided at HID level

- Symptom: an injected ⌃Space or F18 does not switch the remote Mac's input source, or the first
  character after 한/영 lands in the old mode.
- Cause: macOS evaluates input-source shortcuts before the session event tap where RustDesk injects
  keys; a direct TIS switch reaches the front app only at its next activation or switch.
- Response: post the Mac's own "previous input source" shortcut at HID level; fall back to TIS only
  when that shortcut is off. Reading the TIS 한/영 settings must happen on the main queue (fixed ko.12).
- Trap: after a synthesized F18+Fn the Fn flag stayed set for later keys; `MacFlagsAfterHotKey`
  clears flags the shortcut added unless the key is physically held (tests in `platform::macos`).
  From ko.27 those flags ride on the shortcut's key-up, never on a separate Fn event (next entry).

## The main app's `--server` agent must restart on any exit

`agent.plist` uses `KeepAlive = true`. With upstream's `{ SuccessfulExit = false }` a `--server`
that exited with 0 stayed down and incoming stopped after the main window closed (fixed ko.13).

## rdev grab left on

Switching keyboard mode to "input source 2" on Windows left rdev's grab and unicode lookup on, so
local keys kept going to the peer; release both when the mode changes (fixed ko.10).

## Upstream sync checklist

1. `git fetch upstream`, merge `upstream/master` into a topic branch, never rebase the fork.
2. Conflicts in upstream files: keep upstream's version and re-apply the fork's hook lines only.
3. `AGENTS.md`: take upstream's text below the fork block unchanged. `README.md`: keep the fork's
   Korean section on top and take upstream's text below it. `KOREAN.md` is fork-only.
4. List every file the fork changes with
   `git diff --stat $(git merge-base HEAD upstream/master) HEAD -- ':!docs' ':!*.md'` taken before
   the merge; each conflict there is a fork hook to re-apply. The largest surface is the fork keys
   appended to every `src/lang/*.rs`. Hooks that must still fire afterwards: `public_flavor::init()`
   first in `core_main()`, the macOS toggle hook in `server/input_service.rs`, the 한/영 mapping in
   `keyboard.rs`, the fork URL check in `updater.rs`, `bind.mainIsPublicFlavor()` branches in the
   desktop home, and the flavor/branding steps in `build.py`/`build.rs`.
5. Run every verification gate; build both macOS flavors once.
6. If upstream's version leaves 1.5.0, the build-number base (68) and the release tag prefix change
   with it — decide that before the next release.

## `--option` never reaches ranode

- Symptom: `sudo "/Applications/RustDesk (forked ranode).app/Contents/MacOS/RustDeskPublic" --option …`
  prints "Installation and administrative privileges required!".
- Cause: `--option` needs `platform::is_installed()`, which checks that the executable sits under
  `/Applications/<app name>.app`; ranode's app name is `RustDeskPublic` but its bundle is
  `RustDesk (forked ranode).app`. The value is also handed over IPC to the running app, which
  writes its own config, so the app must be running.
- Response: ranode settings that must be set come from `override_settings()` in code, never from
  the machine setup's `--option`; its permanent password goes in with
  `RustDeskPublic --set-permanent-password` (stdin, as the user, ranode running).

## ranode does not take over a busy 21118 by itself

- Symptom: with the main app running, ranode never accepts incoming, even after the main app quits.
- Cause: upstream's `listen_any` sets `SO_REUSEPORT`, so a second app's bind of 21118 succeeds
  and the app that bound first takes every connection (checked on m5m1, 2026-11-21). From ko.23
  ranode binds without it (`public_flavor::listen_direct`), so its bind fails while another app
  holds the port and the left pane says so; upstream's listener then waits only for the port
  number to change and never retries the same port.
- Response: quit (and remove) the main app first, then start ranode; verify by connecting from
  another tailnet Mac.
- The other way round holds too from ko.23: while ranode holds 21118 the main app's bind fails,
  and after ranode quits the main app receives only once it is restarted (before, its idle shared
  listener took over at once). This matches the cutover, which removes the main app.

## A running ranode overwrites its local settings

- Symptom: a value written into ranode's local settings (`RustDeskPublic_local.toml`) while ranode runs
  is lost, or not applied.
- Cause: the running process keeps its own copy of the local settings and writes it back; the
  overrides are built once in `public_flavor::init()` at start.
- Response: `--set-incoming-policy` refuses (exit 2) while ranode's IPC answers; quit ranode, run
  it, start ranode. ranode installs no service, so only its GUI process can answer that IPC.

## Ad-hoc signing drops the macOS privacy grants

- Symptom: after installing a rebuilt ranode, Screen Recording, Accessibility and Input Monitoring
  are off again although System Settings still lists the app.
- Cause: macOS ties those grants to the code-signing identity; an ad-hoc signature differs with
  every build.
- Response: sign every internal build with the stable "ranode internal signing" identity
  (operations, internal builds); never ad-hoc. A Developer ID build is a different identity and
  needs the grants once again.

## Release on macOS: the nested `service` binary

Notarization rejects the archive unless `Contents/MacOS/service` also has the hardened runtime;
`osx-sign-notarize.sh` passes a scoped `--code-signature-flags` for it. Keep that when changing signing.

## Same-source APKs differ

Rebuilding the same commit gives a different APK sha256; the published `.sha256` is of the
released file, not reproducible.
