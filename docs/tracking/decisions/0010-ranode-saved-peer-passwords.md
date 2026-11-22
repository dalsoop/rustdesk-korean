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
  as the key; a value that does not decrypt back to the same password is never written; input from
  a terminal (which would echo it) is refused; a file that exists but cannot be parsed is never
  overwritten. The app need not be running; the file is read on every new session, never cached.
- The saved value is used only when the route to that address leaves through this Mac's Tailscale
  interface, so a LAN host in 100.64.0.0/10 (Tailscale off, carrier or hotel network) never gets an
  answer computed from it.
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
  stored passwords. The key is the Mac's machine UUID (upstream's scheme, which the owner chose to
  reuse); the UUID is not secret (upstream also sends it to the RustDesk server it registers with),
  so the file together with that UUID decrypts elsewhere. The file never leaves the Mac.
- The session's salted password hash never leaves Rust either: a session on a saved password
  turns off upstream's "remember password" for that peer (which also drops a hash remembered
  earlier, and with it the address-book sync event), and `get_conn_token` gives further windows of
  that session (file transfer, terminal) no token, so they log in with the saved password
  themselves instead of receiving the hash through Dart and the main window's call log.
- With a proxy set in ranode the saved password is not used (the proxy would carry the login past
  the route check). The route is checked when a session starts; a reconnect of that session reuses
  the result.
- The key is the address only, not the device's RustDesk identity (a direct IP connection does not
  verify it). Another tailnet the Mac switches to could hold a different device at the same
  address; it would receive a login answer derived from the saved password, which it cannot use to
  reach the real device unless it can also reach that device through Tailscale.
- A device whose Tailscale address changes needs its entry saved again under the new address.
- A stale value costs one failed attempt in the peer's attempt limiter before the prompt appears,
  and keeps taking precedence over a password remembered in the prompt until it is saved again.
- A password that starts with `00` followed by valid Base64 is taken by upstream's helper for
  ciphertext and cannot be saved (exit 2, nothing written).
- Two `--save-peer-password` runs at the same moment on one Mac may keep only one of the entries;
  machine setup runs them one after another.
