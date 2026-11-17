This file is written in English only.

# 0001 Two macOS apps from one tree, chosen at build time

## Context
The main app is bound to the owner's own RustDesk server and its account; the public RustDesk
server requires the controlling user to sign in with a public account. One app holds one server
configuration and one account, so a single app could not reach both.

## Decision
`RUSTDESK_FLAVOR=public` builds a second app (ranode) with its own bundle id, app name, config
folder, IPC socket and URL scheme. The flavor is a compile-time constant; the main-app build runs
upstream's paths unchanged.

## Alternatives
- One app for both servers: not possible, an app holds one server configuration and one account.

## Consequences
- Both apps ship from one commit and one tag; release zips per app.
- Only one of them can listen on 21118/21119 at a time.
- Bundle ids and app names stay fixed so macOS permissions and settings carry over; the owner kept
  ranode's released identity and the renames #28/#29 were reverted (2026-11-17).
- Folding incoming into ranode (planned ko.22) makes the main app removable; the flavor split stays.
