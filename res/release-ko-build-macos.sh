#!/bin/bash
# Korean fork: build both macOS apps of one release commit on the release Mac (m5m1). Run by
# res/release-ko.py through the build queue, never by hand on an M1 MacBook:
#
#   build-pipeline submit "bash <src>/res/release-ko-build-macos.sh <src> <out> <commit> <build>" \
#       --workdir <src> --host m5m1 --wait
#
# <src> is a clean checkout of <commit> (the script refuses any other HEAD or tracked changes),
# <out> receives RustDesk.app (main app) and RustDeskPublic.app (ranode), unsigned, plus
# source-commit / source-tree. <build> is the expected CFBundleVersion (68NN, the pubspec build
# number). Signing is a separate step (res/osx-sign-notarize.sh); no key is read here.
#
# Toolchains: the SDK folder ($RDK_SDK, default ~/work/sdk on m5m1) with Flutter 3.24.5, cargo-home
# (flutter_rust_bridge_codegen 1.80.1), pub-cache, cocoapods and vcpkg; Rust 1.81 (rustup).
set -euo pipefail

src=${1:?checkout}
out=${2:?output folder}
want=${3:?commit}
build=${4:?build number}
lsreg=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
products="$src/flutter/build/macos/Build/Products/Release"

sdk=${RDK_SDK:-$HOME/work/sdk}
export CARGO_HOME="$sdk/cargo-home"
export RUSTUP_TOOLCHAIN=1.81
export PUB_CACHE="$sdk/pub-cache"
export CP_HOME_DIR="$sdk/cocoapods"
export VCPKG_ROOT="$sdk/vcpkg"
export VCPKG_DEFAULT_BINARY_CACHE="$sdk/vcpkg-cache"
export FLUTTER_ROOT="$sdk/flutter-3.24.5"
export TMPDIR="$HOME/tmp/rustdesk-release-build"
export PATH="$CARGO_HOME/bin:$HOME/.cargo/bin:$FLUTTER_ROOT/bin:/opt/homebrew/opt/llvm/bin:/opt/homebrew/bin:$PATH"
export LANG=en_US.UTF-8
mkdir -p "$CARGO_HOME" "$PUB_CACHE" "$CP_HOME_DIR" "$VCPKG_DEFAULT_BINARY_CACHE" "$TMPDIR"

cd "$src"
[ "$(git rev-parse HEAD)" = "$want" ] || { echo "HEAD is not $want" >&2; exit 1; }
git submodule update --init --recursive
if [ -n "$(git status --porcelain --untracked-files=no -- . ':!flutter/pubspec.lock' ':!flutter/macos/Podfile.lock')" ]; then
  echo "source has local changes" >&2
  exit 1
fi
grep -q "^version: 1\.5\.0+$build\$" flutter/pubspec.yaml || { echo "pubspec version is not 1.5.0+$build" >&2; exit 1; }

rm -rf "$out"
mkdir -p "$out"
git rev-parse HEAD >"$out/source-commit"
git rev-parse 'HEAD^{tree}' >"$out/source-tree"

bridge() {
  (cd flutter && flutter pub get >/dev/null)
  flutter_rust_bridge_codegen --rust-input ./src/flutter_ffi.rs --dart-output ./flutter/lib/generated_bridge.dart \
    --c-output ./flutter/macos/Runner/bridge_generated.h >/dev/null
}

# Local-toolchain adjustment: Xcode 27 rejects macOS deployment targets below 12. The same edits
# upstream CI makes for arm64, applied to this build checkout only and never committed (the driver
# resets the checkout before every build).
mac_target() {
  local v=12.3
  sed -i '' -E "s/MACOSX_DEPLOYMENT_TARGET=[0-9]+\.[0-9]+/MACOSX_DEPLOYMENT_TARGET=$v/g" build.py
  sed -i '' -E "s/platform :osx, '[^']*'/platform :osx, '$v'/" flutter/macos/Podfile
  sed -i '' -E "s/osx_minimum_system_version = \"[0-9]+\.[0-9]+\"/osx_minimum_system_version = \"$v\"/" Cargo.toml
  sed -i '' -E "s/MACOSX_DEPLOYMENT_TARGET = [0-9]+\.[0-9]+;/MACOSX_DEPLOYMENT_TARGET = $v;/g" flutter/macos/Runner.xcodeproj/project.pbxproj
  if ! grep -q "config.build_settings\['MACOSX_DEPLOYMENT_TARGET'\]" flutter/macos/Podfile; then
    /usr/bin/python3 -I -c '
import pathlib, sys
p = pathlib.Path("flutter/macos/Podfile"); t = p.read_text(); v = sys.argv[1]
old = "    flutter_additional_macos_build_settings(target)\n"
new = old + "    target.build_configurations.each do |config|\n      config.build_settings[\x27MACOSX_DEPLOYMENT_TARGET\x27] = \x27" + v + "\x27\n    end\n"
p.write_text(t.replace(old, new, 1))
' "$v"
  fi
}

# xcodebuild registers every built .app with LaunchServices. The unsigned copy has the bundle id of
# the installed app, so `open -b` and the URL schemes could start it instead (it dies at launch).
# Unregister both spellings of the path (~/work may be a link to the external drive).
unregister() {
  local a=$1 real
  real=$(cd "$(dirname "$a")" && pwd -P)/$(basename "$a")
  "$lsreg" -u "$a" >/dev/null 2>&1 || true
  "$lsreg" -u "$real" >/dev/null 2>&1 || true
}

build_flavor() {
  local flavor=$1 app=$2
  rm -rf "$products"
  if [ -n "$flavor" ]; then
    RUSTDESK_FLAVOR=$flavor python3 ./build.py --flutter --hwcodec --unix-file-copy-paste
  else
    python3 ./build.py --flutter --hwcodec --unix-file-copy-paste
  fi
  [ -d "$products/$app" ] || { echo "build produced no $app" >&2; exit 1; }
  ditto "$products/$app" "$out/$app"
  for a in "$products"/*.app; do
    [ -d "$a" ] && unregister "$a"
  done
  rm -rf "$products"
}

bridge
mac_target
build_flavor "" RustDesk.app
build_flavor public RustDeskPublic.app
for a in "$out"/*.app; do
  unregister "$a"
done

check() {
  local app=$1 id=$2 got_id got_build
  got_id=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$out/$app/Contents/Info.plist")
  got_build=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$out/$app/Contents/Info.plist")
  [ "$got_id" = "$id" ] || { echo "$app: bundle id $got_id, expected $id" >&2; exit 1; }
  [ "$got_build" = "$build" ] || { echo "$app: CFBundleVersion $got_build, expected $build" >&2; exit 1; }
  echo "built $app $got_id $got_build"
}
check RustDesk.app com.carriez.rustdesk
check RustDeskPublic.app net.ranode.rustdesk-public
# Written last: the driver signs only a folder whose `complete` names the commit (a build cut off mid-copy has none).
cp "$out/source-commit" "$out/complete"
echo "source $(cat "$out/source-commit") tree $(cat "$out/source-tree")"
