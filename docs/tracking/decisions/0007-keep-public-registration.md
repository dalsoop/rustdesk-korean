This file is written in English only.

# 0007 ranode stays registered with the public RustDesk server while it accepts incoming

## Context
From ko.23 ranode accepts incoming connections. Like before, it registers with the public RustDesk
server under an ID nobody is shown, because its outgoing internet tab uses that server.

## Decision
Keep the registration. Incoming safety rests on the whitelist (100.64.0.0/10), which upstream
checks on the connecting address of every path, direct or through the server.

## Alternatives
- Stop ranode from registering: rejected (2026-11-17). It needs a hook in upstream's rendezvous
  code, cannot be tested before a signed build, and could break the internet tab.

## Consequences
- The whitelist is the only barrier against a public-server peer; it must never be widened in
  ranode.
- ranode keeps a hidden public ID.

## Addendum (2026-11-17, ko.24)
ranode's ID is now shown in the left pane and accepts connections, and the whitelist may carry
private office subnets per Mac (decision 0002 addendum). The whitelist still checks the peer's own
address on every path, so a relayed public-server peer is refused. Registering with an own server
instead was rejected: on these Macs ID access runs through the public server, and receiving on a
second server needs new registration code in upstream files.
