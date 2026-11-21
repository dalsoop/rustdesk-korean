This file is written in English only.

# 0006 macOS releases are signed on the owner's Mac, not in CI

## Context
macOS releases need the owner's personal Developer ID and notary key. A GitHub workflow building
ranode unsigned (`macos-public.yml`, PR #19) existed but was never merged.

## Decision
macOS apps are built on a Mac and signed/notarized there with `res/osx-sign-notarize.sh`; keys never
enter CI. PR #19 and its workflow are dropped.

## Alternatives
- Building ranode in GitHub Actions (`macos-public.yml`): dropped with PR #19 (2026-11-17); its
  output was unsigned and could not be released anyway.

## Consequences
- Each release needs a reachable signing Mac with the keys.

## Addendum (2026-11-17, ko.24)
From ko.24 macOS builds may ship as internal builds (ko.23 was released as a notarized Developer
ID release on 2026-11-21), made on the company build Mac by the
machine-setup `ranode-build` script and signed with the one stable self-signed identity "ranode
internal signing" (vault + build Mac keychain; never the repo or CI, never ad-hoc, because macOS
ties privacy grants to the signing identity). The notarized Developer ID release on the owner's
Mac stays optional and later; the owner's personal keys still never leave that Mac.
