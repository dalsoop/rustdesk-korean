This file is written in English only.

# 0008 ranode accepts only its permanent password

## Context
ranode never shows upstream's one-time password, yet with upstream defaults it would stay valid.
The owner keeps one permanent password per Mac, supplied from the vault.

## Decision
ranode pins `verification-method=use-permanent-password` in code, like `direct-server` and the
whitelist. A click to accept on the Mac still works.

## Alternatives
- Keep upstream defaults (one-time password valid but hidden): rejected (2026-11-17); a password
  nobody can see is only something for a peer to guess.

## Consequences
- Before a permanent password is set, ranode can be reached only by someone clicking accept on the
  Mac.
- The password goes in through ranode's dialog or `--set-permanent-password`, never `--option`.
