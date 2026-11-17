This file is written in English only.

# 0005 KOREAN.md and README's Korean section stay Korean

## Context
The owner's rule is that Markdown is English. `KOREAN.md` and the Korean part of `README.md` are the
product's guide and release notes for Korean users.

## Decision
Those two are exempt and stay Korean; agent-facing docs are English.

## Alternatives
- Convert the parts touched to English, as the general rule says: rejected by the owner (2026-11-17).

## Consequences
- Release notes and user-visible behaviour are written in Korean in `KOREAN.md`.
- English paragraphs already inside it may stay; new ones should be Korean.
