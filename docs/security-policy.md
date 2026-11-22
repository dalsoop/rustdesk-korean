This file is written in English only.

# Security

## Protected assets

1. The owner's devices (Macs, the office Linux server) as remote-control targets.
2. The release signing keys, all the owner's **personal** assets (not crosswall company assets):
   the Apple Developer ID Application certificate and key (team V2Z46YY386), the App Store Connect
   API key used for notarization, and the Android release signing key (certificate SHA-256
   `43:7D:FD:91:3F:F7:9A:4A:CB:26:46:65:F9:F0:3E:8D:36:C9:78:D2:28:F9:E7:27:5B:0E:0E:B4:53:9E:AD:7D`).
3. The integrity of what the owner's devices install: notarized macOS zips, internal macOS builds
   signed with the stable internal identity, the pinned-certificate APK, and the `.sha256` beside
   every release file.
4. The stable self-signed code-signing identity "ranode internal signing" for internal builds: a
   machine-setup build asset, kept only in the vault and the build Mac's keychain; never in the
   repo or CI. It is not one of the owner's personal release keys.

## Who may connect in

| Subject | Action | Allowed when | Explicitly not allowed |
|---|---|---|---|
| A device inside the owner's tailnet (source in 100.64.0.0/10) | Connect to the main app on an owner's Mac | Direct port 21118 is on (`direct-server=Y`) and whitelist is `100.64.0.0/10`, both set with `sudo RustDesk --option` on that Mac; or through the owner's own RustDesk server, reachable only inside Tailscale; then the permanent password | — |
| Any address outside the Tailscale ranges and the Mac's listed office subnet (internet, other LANs, public relay) | Connect in | never | the whitelist refuses any connection whose source address is outside it |
| A device inside the owner's tailnet | Connect in to ranode (from ko.23), by Tailscale IP | ranode is running in the logged-in session, its source is in 100.64.0.0/10 or fd7a:115c:a1e0::/48, the connection arrived on this Mac's Tailscale interface (decision 0009), and it gives ranode's permanent password (or someone at the Mac clicks accept, unless approve mode is `password`); `direct-server=Y`, the whitelist, `use-permanent-password` and `allow-remote-config-modification=N` are pinned in code (`OVERWRITE_SETTINGS`), which saved settings cannot override | the one-time password; changing the pinned settings through the UI, config files or `--option`; a 100.64.0.0/10 source that reaches the Mac on another interface (a carrier or hotel LAN using that range) |
| A device on the office network (source in the office subnet) | Connect in to ranode on a company Craft Mac (from ko.24), by ID or address | the Mac's policy lists the office subnet (`--set-incoming-policy --extra-subnets`, approve mode `password`), the connection arrived on this Mac's own address in that subnet on a non-tunnel interface (not `utun*`, `tailscale*`, `ipsec*`, `ppp*`) with the route back to the peer through it (the decision 0009 rule applied to the office subnet), and the peer gives the permanent password | a click to accept; any public address; any relayed connection; an office-range source reaching another address of the Mac (its Tailscale IP, a VPN tunnel, a Tailscale subnet route) |
| Someone without Tailscale | Reach an owner's device | only by being invited into the tailnet | opening a port or a public-server path for them |

- The office Linux server gets `direct-server=Y` from its own infrastructure repo, not from this fork.
- Unattended access at the login screen is not part of the policy: the owner's Macs run RustDesk as
  a logged-in user app; ranode refuses to install a LaunchDaemon/LaunchAgent at all.

## Incoming authentication

1. The connection's source address is checked against the whitelist; outside it the peer is refused
   before any password prompt. In ranode a source in a Tailscale range is then refused too unless
   the accepted socket's local address is a Tailscale address of this Mac's Tailscale interface
   and the route back to the peer leaves from that address (`public_flavor::incoming_path_allowed`,
   decision 0009), with the same "Your ip is blocked by the peer" error; from ko.24 a source in a
   listed office subnet is refused unless the local address is this Mac's own address in that
   subnet on a non-tunnel interface with the route back through it.
2. The peer sends the permanent password; a wrong one is refused and counted by upstream's
   attempt limiter. The fork adds no lockout, approval click or second factor of its own.
3. Accepted sessions show upstream's connection window on the Mac; the fork keeps no extra log.

- ranode stays registered with the public RustDesk server (it was before ko.23); a peer coming
  through it on a direct path is checked by its own address like any other. On the relay path the
  peer address is only what the rendezvous message claims (its sender is not checked), so from
  ko.24 ranode refuses every relayed incoming connection before opening it
  (`public_flavor::relayed_incoming_allowed`, decision 0009 addendum). Only a direct path from an
  allowed network gets in.
- `--set-incoming-policy` accepts only private IPv4 networks inside 10.0.0.0/8, 172.16.0.0/12 or
  192.168.0.0/16 without host bits, at most 8; a public range, `0.0.0.0/0`, a Tailscale-range
  entry or IPv6 is refused and nothing is saved. Approve mode is `password` or `both` only.
- Known limit of the office subnet: a private range proves only that the peer is on a LAN with
  that numbering. A Craft Mac taken to another network that uses the same subnet would accept that
  network's devices that know its permanent password (no click to accept there, approve mode
  `password`). The Craft Macs stay in the office; the owner's own Macs list no office subnet.

## Outgoing

- ranode's internet tab talks to the public RustDesk server; that server requires the controlling
  user to sign in with a third-party account. Sign-in tokens stay in ranode's own config folder.
- ranode's on-prem tab runs only the local Tailscale CLI and connects to Tailscale IPs; it stores no
  Tailscale key or server address.

## Credentials

| Credential | Owner | Where it lives | Used where | Never |
|---|---|---|---|---|
| Developer ID Application key + cert | owner (personal) | SOPS file `secrets/macbook-tools/developer-id-application.enc.yaml` in the SOPS repo `principal-secrets-mono`, decrypted with an age key on the signing Mac | `res/osx-sign-notarize.sh` only, into a 0700 temp folder removed on exit | printed, committed, put in GitHub/GitLab CI secrets |
| App Store Connect API key (p8, key id, issuer) | owner (personal) | same SOPS repo, `app-store-connect-api-key.enc.yaml` | `xcrun notarytool` in the same script | same |
| Android signing key | owner (personal) | Bitwarden Secrets Manager project `android-signing`, read through `bwsx` into the signing script's environment only | `res/android-sign-release.sh` on the signing Mac | given to CI runners; the CI APK is unsigned |
| Each Mac's permanent RustDesk password | owner | the main app's config on that Mac; for ranode, ranode's own config, set by the user in its dialog or by `RustDeskPublic --set-permanent-password` reading stdin as the logged-in user | incoming authentication | passed on the command line, committed or logged |
| Other own devices' permanent passwords, saved in ranode (from ko.25, decision 0010) | owner | ranode's `RustDeskPublic_saved_peer_passwords.toml` (0600), encrypted with the Mac's key like upstream's stored passwords, one entry per Tailscale address, written by `RustDeskPublic --save-peer-password <address>` reading stdin as the logged-in user (machine setup pipes it from the vault) | the preset password of a new session to that address, hashed with the peer's salt | passed on the command line, printed, logged, given to the Flutter UI, committed |
| Each Mac's RustDesk ID key pair | owner | the main app's config; at cutover copied into ranode's config, so the ID is kept | public-server registration | read, printed or committed |
| "ranode internal signing" identity (self-signed, internal builds) | machine setup (company build asset) | the vault, imported into the build Mac's keychain by the machine-setup build role | the machine-setup `ranode-build` script on the build Mac | committed, put in CI, printed |

- Rotation: the Android key cannot rotate without breaking updates (every device would have to
  uninstall first); the signing script refuses a certificate other than the pinned SHA-256 unless
  `ANDROID_SIGN_EXPECTED_SHA256` overrides it, which must never be set.
- A leaked macOS key or notary key is revoked and reissued in the owner's Apple developer account
  and replaced in the SOPS file; releases already notarized stay valid until Apple revokes them.

## Audit evidence (releases only; connections are logged only as upstream logs them)

- Each release: the `.sha256` of every asset, the notarization verdict (`spctl` must say
  "Notarized Developer ID"), and the APK certificate check printed by the signing script.
- Each internal build: `codesign` shows the "ranode internal signing" identity (never ad-hoc).
- The owner's Macs pin release version + sha256 in their setup repo and refuse a mismatch.
