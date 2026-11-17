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

## Addendum (2026-11-17, ko.24)
On the company Craft Macs the whitelist also includes the office subnet and approve mode is
password-only, as the owner decided for the main app there (the machine-setup repo's decision 0006
on unattended RustDesk on the Craft Macs). The owner's own Macs stay Tailscale only. The fork
enforces the floor (`100.64.0.0/10` first, permanent password only, never a public address); the
per-Mac part comes from the machine setup through `--set-incoming-policy`. ranode's incoming now
also works by RustDesk ID, from allowed networks only.
