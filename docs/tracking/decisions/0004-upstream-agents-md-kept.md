This file is written in English only.

# 0004 Keep upstream's AGENTS.md byte for byte under a fork block

## Context
`AGENTS.md`, `CLAUDE.md` (`@AGENTS.md`) and `GEMINI.md` are upstream files that upstream edits
often. The fork needs its own entry point for agents.

## Decision
The fork adds one block at the top of `AGENTS.md` and leaves upstream's text below it unchanged;
`CLAUDE.md` stays `@AGENTS.md`, so Claude reads the same content; `GEMINI.md` is untouched.

## Alternatives
- Move upstream's rules into the fork's own docs and rewrite `AGENTS.md`: rejected by the owner
  (2026-11-17), because upstream edits that file and syncs would conflict.

## Consequences
- On upstream sync, conflicts in `AGENTS.md` are resolved by taking upstream's lines below the block.
- Upstream's rules apply to the fork as written, including where they mention upstream maintainers.
