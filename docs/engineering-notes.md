This file is written in English only.

# Engineering notes

## Tailscale CLI run from a GUI app fails (open; fix planned for ko.21, not in the code yet)

- Symptom: ranode launched from Finder/Dock says "Tailscale이 꺼져 있습니다" (or not responding)
  while Tailscale is up; launched from a terminal, or with `open --env TERM=dumb -a "<app>"`, the
  device list appears.
- Cause: the standalone Tailscale app binary (`/Applications/Tailscale.app/Contents/MacOS/Tailscale`,
  bundle `io.tailscale.ipn.macsys`, seen on 1.104.x) acts as a CLI only when its environment looks
  like a shell (`TERM`, `TERM_PROGRAM` or `SHLVL` set). Otherwise it tries to start a second GUI,
  prints "The Tailscale GUI failed to start: … (Tailscale.CLIError error 3.)" and exits non-zero.
  A Finder-launched app has none of those variables, and `Process.start` passes them through.
  A `~/.local/bin/tailscale` wrapper that `exec`s the bundle binary fails the same way.
- Response: give the CLI process `TERM=dumb` in its environment. Verify by launching the app from
  Finder, never from a terminal (a terminal launch hides the bug).
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
  the machine setup's `--option`.

## Release on macOS: the nested `service` binary

Notarization rejects the archive unless `Contents/MacOS/service` also has the hardened runtime;
`osx-sign-notarize.sh` passes a scoped `--code-signature-flags` for it. Keep that when changing signing.

## Same-source APKs differ

Rebuilding the same commit gives a different APK sha256; the published `.sha256` is of the
released file, not reproducible.
