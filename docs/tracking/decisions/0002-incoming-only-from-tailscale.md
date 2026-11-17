This file is written in English only.

# 0002 Incoming connections only from the Tailscale range

## Context
The owner's devices must be reachable by the owner and invited people, but not from the internet.

## Decision
Devices accept connections only from 100.64.0.0/10 (whitelist) on the direct port 21118 or via the
owner's server inside Tailscale. People without Tailscale are invited to the tailnet. ranode pins
the whitelist in code so its users cannot widen it.

## Alternatives
- Letting people without Tailscale in from the internet: rejected; they are invited to the tailnet instead.

## Consequences
- No feature may open a device to a non-Tailscale address.
- A device without Tailscale can connect out (ranode internet tab) but can never be reached.
- ranode's incoming (ko.23) needs no RustDesk ID: peers connect by Tailscale IP.
