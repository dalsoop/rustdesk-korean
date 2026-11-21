This file is written in English only.

# Security

## Protected assets

1. The owner's devices (Macs, the office Linux server) as remote-control targets.
2. The release signing keys, all the owner's **personal** assets (not crosswall company assets):
   the Apple Developer ID Application certificate and key (team V2Z46YY386), the App Store Connect
   API key used for notarization, and the Android release signing key (certificate SHA-256
   `43:7D:FD:91:3F:F7:9A:4A:CB:26:46:65:F9:F0:3E:8D:36:C9:78:D2:28:F9:E7:27:5B:0E:0E:B4:53:9E:AD:7D`).
3. The integrity of what the owner's devices install: notarized macOS zips, the pinned-certificate
   APK, and the `.sha256` beside every release file.

## Who may connect in

| Subject | Action | Allowed when | Explicitly not allowed |
|---|---|---|---|
| A device inside the owner's tailnet (source in 100.64.0.0/10) | Connect to the main app on an owner's Mac | Direct port 21118 is on (`direct-server=Y`) and whitelist is `100.64.0.0/10`, both set with `sudo RustDesk --option` on that Mac; or through the owner's own RustDesk server, reachable only inside Tailscale; then the permanent password | — |
| Any address outside 100.64.0.0/10 (internet, LAN, public relay) | Connect in | never | the whitelist refuses any connection whose source address is outside the range |
| A device inside the owner's tailnet | Connect in to ranode (from ko.23) | ranode is running in the logged-in session, its source is in 100.64.0.0/10 or fd7a:115c:a1e0::/48, the connection arrived on this Mac's Tailscale interface (decision 0009), and it gives ranode's permanent password (or someone at the Mac clicks accept); `direct-server=Y`, the whitelist and `use-permanent-password` are pinned in code (`OVERWRITE_SETTINGS`), which saved settings cannot override | the one-time password; changing the pinned settings through the UI, config files or `--option`; a 100.64.0.0/10 source that reaches the Mac on another interface (a carrier or hotel LAN using that range) |
| Someone without Tailscale | Reach an owner's device | only by being invited into the tailnet | opening a port or a public-server path for them |

- The office Linux server gets `direct-server=Y` from its own infrastructure repo, not from this fork.
- Unattended access at the login screen is not part of the policy: the owner's Macs run RustDesk as
  a logged-in user app; ranode refuses to install a LaunchDaemon/LaunchAgent at all.

## Incoming authentication

1. The connection's source address is checked against the whitelist; outside it the peer is refused
   before any password prompt. In ranode a source in a Tailscale range is then refused too unless
   the accepted socket's local address is a Tailscale address of this Mac's Tailscale interface
   and the route back to the peer leaves from that address (`public_flavor::incoming_path_allowed`,
   decision 0009), with the same "Your ip is blocked by the peer" error.
2. The peer sends the permanent password; a wrong one is refused and counted by upstream's
   attempt limiter. The fork adds no lockout, approval click or second factor of its own.
3. Accepted sessions show upstream's connection window on the Mac; the fork keeps no extra log.

- ranode stays registered with the public RustDesk server (it was before ko.23); that path cannot
  get in because the whitelist checks the connecting address on every path.

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

- Rotation: the Android key cannot rotate without breaking updates (every device would have to
  uninstall first); the signing script refuses a certificate other than the pinned SHA-256 unless
  `ANDROID_SIGN_EXPECTED_SHA256` overrides it, which must never be set.
- A leaked macOS key or notary key is revoked and reissued in the owner's Apple developer account
  and replaced in the SOPS file; releases already notarized stay valid until Apple revokes them.

## Audit evidence (releases only; connections are logged only as upstream logs them)

- Each release: the `.sha256` of every asset, the notarization verdict (`spctl` must say
  "Notarized Developer ID"), and the APK certificate check printed by the signing script.
- The owner's Macs pin release version + sha256 in their setup repo and refuse a mismatch.
