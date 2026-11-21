This file is written in English only.

# 0009 ranode trusts a Tailscale-range peer only on the Tailscale interface

## Context
ko.23's ranode listens on 21118 on every interface (upstream's `listen_any`), keeps
`direct-server=Y` pinned even while Tailscale is off, and the whitelist checks only the peer's
source address. 100.64.0.0/10 is the shared carrier-grade NAT range: a carrier, hotel or office
network can hand it out on its LAN, so a host there passed the whitelist and, before a permanent
password was set, could raise the click-to-accept dialog (QA 2026-11-21, finding 6). Tailscale
peers that connect over IPv6 (fd7a:115c:a1e0::/48) were refused.

## Decision
ranode checks, on every incoming path (`Connection::on_open`, right after the whitelist), that a
peer whose address is in 100.64.0.0/10 or fd7a:115c:a1e0::/48:

1. reached this Mac on a local address in the same ranges,
2. that is assigned to a tunnel interface (`utun*` on macOS, `tailscale*` elsewhere) that also
   carries a fd7a:115c:a1e0::/48 address, and
3. that is the address this Mac's routing table would send from to reach the peer (a connected,
   unsent UDP socket), so the way back runs through the Tailscale tunnel.

Otherwise it is refused with upstream's "Your ip is blocked by the peer". The pinned whitelist
becomes `100.64.0.0/10,fd7a:115c:a1e0::/48`. The listener keeps binding all interfaces.

## Alternatives
- Bind the direct listener to this Mac's Tailscale IP: rejected. The IP is unknown while Tailscale
  is off or starting and can change with a tailnet switch, so upstream's listener loop would need a
  rebind-on-change path (new state in an upstream file). macOS follows the weak host model: a
  packet for the Tailscale IP that arrives on Ethernet or Wi-Fi is still delivered to such a socket,
  so binding alone does not stop a LAN host that uses 100.64.0.0/10 and sends to the Tailscale IP;
  check 3 does.
- Local address in 100.64.0.0/10 only: rejected; a LAN that uses that range gives the Mac such an
  address on its Ethernet or Wi-Fi interface.
- Interface name alone (`utun*`): rejected; other VPNs (Cloudflare WARP uses 100.96.0.0/12) put a
  CGNAT address on a `utun`. Requiring the Tailscale IPv6 prefix on the same interface tells
  Tailscale's tunnel apart.

## Consequences
- Tailscale IPv6 peers can connect; the left pane still shows the IPv4 address.
- A tailnet whose admin turned IPv6 off has no fd7a address on the tunnel, so ranode refuses its
  peers (fails closed); the live check before release covers the owner's tailnets.
- The rule only narrows: an address the whitelist refuses stays refused. Peers outside the
  Tailscale ranges are left to the whitelist, so a later per-Mac LAN subnet (ko.24 PR #37) is not
  affected by this check.
- A LAN whose own subnet overlaps a tailnet peer's address breaks that peer's Tailscale path anyway;
  ranode refuses it (check 3), which is the safe side.
- Each incoming connection costs one interface listing and one unsent UDP socket, before any
  password prompt.
