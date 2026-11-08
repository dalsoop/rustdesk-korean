#!/bin/bash
# Korean fork: zipalign, sign and verify the Android release APK on m5m1.
# Not a build step: gitlab gujoai/apps/upstream-mono (job rustdesk:android-arm64) builds
# apk/rustdesk-<versionName>-<commit>-aarch64-unsigned.apk without any key; CI runners never hold
# the release key. Run this on m5m1 with that artifact.
#
#   res/android-sign-release.sh <unsigned.apk> <out.apk>
#   e.g. res/android-sign-release.sh ~/work/rdk/ci/rustdesk-1.5.0-ko.12-09f4b5aa-aarch64-unsigned.apk \
#          ~/work/rdk/release-ko12-assets/rustdesk-1.5.0-ko.12-aarch64.apk
#
# The key comes from Bitwarden Secrets Manager project android-signing through bwsx (m5m1's
# machine-account handle): UPSTREAM_MONO_RELEASE_KEYSTORE_JKS_B64, _STORE_PASSWORD, _KEY_PASSWORD
# and _KEY_ALIAS reach only this script's own child environment. The keystore is written to a
# private temp folder under ~/tmp that is removed on exit. Values are never printed.
#
# The signer certificate must be the one every earlier fork APK was signed with (otherwise the
# APK does not install as an update): its SHA-256 is checked with apksigner and the script fails
# on any other certificate. Writes <out.apk> and <out.apk>.sha256.
#
# Tools (m5m1): Android build-tools 34.0.0 (zipalign, apksigner, aapt2) under
# ~/work/sdk/android-sdk and a JDK 17 under ~/work/sdk/jdk-17. Override with ANDROID_BUILD_TOOLS
# and JAVA_HOME.
set -euo pipefail

project_id=5c3cb98f-bb51-4d10-9cdc-b4eb013bed54 # Secrets Manager project android-signing
expected=${ANDROID_SIGN_EXPECTED_SHA256:-43:7D:FD:91:3F:F7:9A:4A:CB:26:46:65:F9:F0:3E:8D:36:C9:78:D2:28:F9:E7:27:5B:0E:0E:B4:53:9E:AD:7D}

in=${1:?unsigned .apk}
out=${2:?output .apk}

# First pass: re-run this script under bwsx so the four values exist only in its environment.
if [ -z "${ANDROID_SIGN_RELEASE_INNER:-}" ]; then
  [ -f "$in" ] || { echo "no such file: $in" >&2; exit 1; }
  bwsx=${BWSX:-$(command -v bwsx || true)}
  if [ -z "$bwsx" ]; then
    for f in "$HOME"/.claude/plugins/cache/agent-secret-controller-skills/agent-secret-controller/*/bin/bwsx; do
      [ -x "$f" ] && bwsx=$f
    done
  fi
  [ -n "$bwsx" ] || { echo "bwsx not found (agent-secret-controller plugin); set BWSX" >&2; exit 1; }
  bwsx_dir=$(dirname "$bwsx")
  export PATH="$bwsx_dir:$PATH" # bwsx calls bws-token next to it
  export VAULT_OPS_SE_IDENTITY="${VAULT_OPS_SE_IDENTITY:-$HOME/.config/sops/age/m5m1-se.txt}"
  export ANDROID_SIGN_RELEASE_INNER=1
  self="$(cd "$(dirname "$0")" && pwd)/$(basename "$0")"
  exec "$bwsx" "${BWS_HANDLE:-ranode-m5m1}" run --project-id "$project_id" -- "$self" "$in" "$out"
fi

: "${UPSTREAM_MONO_RELEASE_KEYSTORE_JKS_B64:?not injected}"
: "${UPSTREAM_MONO_RELEASE_STORE_PASSWORD:?not injected}"
: "${UPSTREAM_MONO_RELEASE_KEY_PASSWORD:?not injected}"
: "${UPSTREAM_MONO_RELEASE_KEY_ALIAS:?not injected}"

bt=${ANDROID_BUILD_TOOLS:-$HOME/work/sdk/android-sdk/build-tools/34.0.0}
export JAVA_HOME="${JAVA_HOME:-$HOME/work/sdk/jdk-17/Contents/Home}"
export PATH="$JAVA_HOME/bin:$PATH"
for t in zipalign apksigner aapt2; do
  [ -x "$bt/$t" ] || { echo "missing $bt/$t (Android build-tools 34.0.0)" >&2; exit 1; }
done

mkdir -p "$HOME/tmp"
tmp=$(mktemp -d "$HOME/tmp/android-sign.XXXXXX")
chmod 700 "$tmp"
trap 'rm -rf "$tmp"' EXIT

# printf is a shell builtin: the keystore never appears on a command line.
printf '%s' "$UPSTREAM_MONO_RELEASE_KEYSTORE_JKS_B64" | base64 -d >"$tmp/release.jks"
chmod 600 "$tmp/release.jks"

# Page-align uncompressed native libraries, then sign. minSdk is 22, so v1 (JAR) signing is
# needed next to v2/v3, the same schemes as the published APKs. No v4 .idsig file.
"$bt/zipalign" -p -f 4 "$in" "$tmp/aligned.apk"
"$bt/apksigner" sign --ks "$tmp/release.jks" --ks-key-alias "$UPSTREAM_MONO_RELEASE_KEY_ALIAS" \
  --ks-pass env:UPSTREAM_MONO_RELEASE_STORE_PASSWORD --key-pass env:UPSTREAM_MONO_RELEASE_KEY_PASSWORD \
  --v1-signing-enabled true --v2-signing-enabled true --v3-signing-enabled true \
  --v4-signing-enabled false --out "$tmp/signed.apk" "$tmp/aligned.apk"
rm -f "$tmp/release.jks"

"$bt/apksigner" verify --verbose --print-certs "$tmp/signed.apk" >"$tmp/verify.txt" 2>&1 || {
  cat "$tmp/verify.txt" >&2; echo "apksigner verify failed" >&2; exit 1; }
want=$(printf '%s' "$expected" | tr -d ':' | tr 'A-F' 'a-f')
got=$(sed -n 's/^Signer #1 certificate SHA-256 digest: //p' "$tmp/verify.txt")
signers=$(sed -n 's/^Number of signers: //p' "$tmp/verify.txt")
if [ "$signers" != 1 ] || [ "$got" != "$want" ]; then
  echo "wrong signer: $signers signer(s), certificate SHA-256 $got, expected $want" >&2
  exit 1
fi
for scheme in "v1 scheme (JAR signing)" "v2 scheme (APK Signature Scheme v2)" "v3 scheme (APK Signature Scheme v3)"; do
  grep -qF "Verified using $scheme: true" "$tmp/verify.txt" || { echo "not signed with $scheme" >&2; exit 1; }
done
"$bt/zipalign" -c -p 4 "$tmp/signed.apk" >/dev/null || { echo "signed APK is not aligned" >&2; exit 1; }

"$bt/aapt2" dump badging "$tmp/signed.apk" 2>/dev/null | sed -n 's/^package: /package: /p'
echo "signer certificate SHA-256: $expected (matches)"

mkdir -p "$(dirname "$out")"
mv "$tmp/signed.apk" "$out"
(cd "$(dirname "$out")" && shasum -a 256 "$(basename "$out")" >"$(basename "$out").sha256")
cat "$out.sha256"
