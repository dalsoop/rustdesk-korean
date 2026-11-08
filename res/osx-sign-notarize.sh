#!/bin/bash
# Korean fork: sign, notarize and zip a macOS build (main app or RustDesk Public) on m5m1.
# Not a build step: run it on m5m1 after build.py produced the .app on m5m1/m5m2.
#
#   res/osx-sign-notarize.sh <built .app> <bundle name in the zip> <output zip>
#   e.g. res/osx-sign-notarize.sh flutter/build/macos/Build/Products/Release/RustDeskPublic.app \
#          "RustDesk Public.app" ~/work/release-ko11/rustdesk-public-1.5.0-ko.11-macos-aarch64.zip
#
# Keys come from principal-secrets-mono (sops, decrypted only into a private temp folder under
# ~/tmp that is removed on exit): secrets/macbook-tools/developer-id-application.enc.yaml
# (private_key_pem, certificate_pem) for rcodesign, and app-store-connect-api-key.enc.yaml
# (p8, key_id, issuer_id) for notarytool. Values are never printed.
set -euo pipefail

app=${1:?built .app}
name=${2:?bundle name in the zip}
zip=${3:?output zip}
here=$(cd "$(dirname "$0")/.." && pwd)
entitlements="$here/flutter/macos/Runner/Release.entitlements"
secrets="${PRINCIPAL_SECRETS_ROOT:-$HOME/.environment-secret-broker/principal-secrets-mono}/secrets/macbook-tools"
export SOPS_AGE_KEY_FILE="${SOPS_AGE_KEY_FILE:-$HOME/.config/sops/age/m5m1-se.txt}"

mkdir -p "$HOME/tmp"
tmp=$(mktemp -d "$HOME/tmp/osx-sign.XXXXXX")
chmod 700 "$tmp"
trap 'rm -rf "$tmp"' EXIT

out=$(dirname "$zip")
mkdir -p "$out"
stage="$tmp/stage"
mkdir -p "$stage"
ditto "$app" "$stage/$name"

sops -d --extract '["private_key_pem"]' "$secrets/developer-id-application.enc.yaml" >"$tmp/key.pem"
sops -d --extract '["certificate_pem"]' "$secrets/developer-id-application.enc.yaml" >"$tmp/cert.pem"
rcodesign sign --pem-source "$tmp/key.pem" --pem-source "$tmp/cert.pem" \
  --code-signature-flags runtime --entitlements-xml-path "$entitlements" "$stage/$name"
rm -f "$tmp/key.pem" "$tmp/cert.pem"

sops -d --extract '["p8"]' "$secrets/app-store-connect-api-key.enc.yaml" >"$tmp/key.p8"
key_id=$(sops -d --extract '["key_id"]' "$secrets/app-store-connect-api-key.enc.yaml")
issuer=$(sops -d --extract '["issuer_id"]' "$secrets/app-store-connect-api-key.enc.yaml")
ditto -c -k --keepParent "$stage/$name" "$tmp/submit.zip"
xcrun notarytool submit "$tmp/submit.zip" --key "$tmp/key.p8" --key-id "$key_id" --issuer "$issuer" --wait
rm -f "$tmp/key.p8"
xcrun stapler staple "$stage/$name"
spctl -a -vv -t exec "$stage/$name" 2>&1 | grep -q "Notarized Developer ID"

rm -f "$zip"
ditto -c -k --keepParent "$stage/$name" "$zip"
(cd "$out" && shasum -a 256 "$(basename "$zip")" >"$(basename "$zip").sha256")
cat "$zip.sha256"
