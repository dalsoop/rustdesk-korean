This file is written in English only.

# 0003 Fork only: upstream merged in, nothing sent upstream, minimal diff

## Context
The fork tracks upstream RustDesk and must keep merging it while carrying Korean and ranode work.

## Decision
Upstream changes are merged in; fork changes are not proposed upstream. Upstream's minimal-diff and
coding rules still bind fork work so merges stay clean.

## Alternatives
- Offering fork fixes upstream as PRs: rejected by the owner (2026-11-17).

## Consequences
- Fork logic sits in fork-owned files with thin hooks in upstream files.
- Fork behaviour that upstream would never accept (fixed app identities, fork update URL) is fine.
