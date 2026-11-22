This file is written in English only.

# 0010 ranode keeps each own device's access password, encrypted on the Mac

## Context
The owner connects from ranode's on-prem tab to the owner's own devices by Tailscale address and had
to type each device's permanent password. Every device's permanent password is a Bitwarden item;
nobody (the owner included) should have to see or type it. ranode on the Macs is not signed in to an
API server and has no address book, so upstream's shared or personal address-book passwords do not
apply. The owner chose a fork feature (2026-11-22, session m5m1: "fork에 기능 추가").

## Decision
- `RustDeskPublic --save-peer-password <Tailscale address>` (ranode only, run as the logged-in user)
  reads one password from stdin and stores it in `RustDeskPublic_saved_peer_passwords.toml` in
  ranode's config folder (0600), encrypted with the Mac's key exactly as upstream encrypts its own
  stored passwords (`password_security`, version `00`). Only a Tailscale-range address is accepted
  as the key; a value that does not decrypt back to the same password is never written. The app
  need not be running; the file is read on every new session, never cached.
- When ranode starts a session to that exact address without a password from the caller (the
  on-prem card, the `rustdeskpublic://` link, a typed address), the saved value becomes upstream's
  preset password (`session_add` hook): upstream's `handle_hash` hashes it with the peer's salt and
  challenge, so it never leaves the Mac. A wrong or stale value ends in upstream's "wrong password"
  prompt, where a password can still be typed.
- The value is never printed, logged, put in argv, or handed to the Flutter UI. Machine setup fills
  it from the vault through `bw-agent exec … --stdin` piped into the command over ssh.

## Alternatives
- Upstream's shared address book on the own RustDesk API server: rejected; ranode on the Macs is not
  signed in there and its on-prem tab does not use address books.
- Passing the password from Flutter to `connect(..., password:)`: rejected; the value would cross the
  Dart side and the multi-window call arguments.
- Storing upstream's salted hash per peer (the "remember password" form): rejected; it needs the
  peer's salt, i.e. a connection, before saving.

## Consequences
- Anyone who can run code as the Mac's user can read the saved passwords, as with upstream's own
  stored passwords; the file is useless on another Mac.
- A device whose Tailscale address changes needs its entry saved again under the new address.
- A stale value costs one failed attempt in the peer's attempt limiter before the prompt appears.
