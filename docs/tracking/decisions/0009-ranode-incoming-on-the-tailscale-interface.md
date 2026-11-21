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
becomes `100.64.0.0/10,fd7a:115c:a1e0::/48`. The listener keeps binding all interfaces, but
without `SO_REUSEPORT` (`listen_direct`), so it cannot silently share 21118 with the main app.

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
  Tailscale ranges are left to the whitelist and, from ko.24, to the office-subnet rule in the
  addendum below.
- A LAN whose own subnet overlaps a tailnet peer's address breaks that peer's Tailscale path anyway;
  ranode refuses it (check 3), which is the safe side.
- Each incoming connection costs one getifaddrs(3) call and one unsent UDP socket (both plain
  syscalls, no I/O wait), before any password prompt. `default_net`'s interface listing is not
  used: on macOS it also probes the default gateway, which can block for seconds.

## Addendum (2026-11-21, ko.24 PR #37)
ko.24 lets a Mac's policy append private office subnets to the whitelist. Those subnets get the
same three checks, adapted: a peer whose address lies in one of the private subnets of the
enforced whitelist must have reached this Mac on a local address in that same subnet, assigned to
a non-tunnel interface (not `utun*`, `tailscale*`, `ipsec*` or `ppp*`), that is also the source
address of the route back to the peer (`office_path_allowed`, after the Tailscale check). So an
office-range source cannot reach the Mac through its Tailscale IP, a VPN tunnel or a Tailscale
subnet route, or on a second address of the Mac. The Tailscale rule above is unchanged and still
applies first. A private subnet cannot prove which LAN it is; that limit is in `findings.md`.

These checks trust the peer address the connection layer is given. On upstream's relay path that
address is only what the rendezvous message claims (its sender is not checked), and the local
address is this Mac's own socket to the relay server, so a forged claim of an office address would
pass the office rule (and, when the relay socket runs through a Tailscale exit node, a Tailscale
address would pass the rule above). ranode therefore refuses every relayed incoming connection
before it opens the relay (`relayed_incoming_allowed`, a one-line hook in
`server::create_relay_connection_`). No legitimate peer is lost: a relayed peer's real address is
public and the whitelist refused it anyway. The direct listener, the LAN intranet accept, the TCP
punch and the IPv4 KCP punch (a UDP socket connected to the peer, then a key exchange a blind
sender cannot finish) carry the address the socket actually talks to. The IPv6 KCP path passes the
claimed IPv4 address, but its local address is a global IPv6 one, so both rules refuse any
Tailscale or office claim there; WebRTC has local `0.0.0.0` and fails both rules the same way.
