#!/usr/bin/env bash
# Builds one signed APK per supported Android ABI:
#   build/dist/clck-arm64-v8a.apk
#   build/dist/clck-armeabi-v7a.apk
#   build/dist/clck-x86.apk
#   build/dist/clck-x86_64.apk
set -euo pipefail
source "$(dirname "$0")/env.sh"

DIST="$ROOT/build/dist"
rm -rf "$DIST"
mkdir -p "$DIST"

for target in \
    aarch64-linux-android \
    armv7-linux-androideabi \
    x86_64-linux-android \
    i686-linux-android
do
    case "$target" in
        aarch64-linux-android) abi="arm64-v8a" ;;
        armv7-linux-androideabi) abi="armeabi-v7a" ;;
        x86_64-linux-android) abi="x86_64" ;;
        i686-linux-android) abi="x86" ;;
    esac
    echo "==> $target ($abi)"
    TARGETS="$target" "$ROOT/scripts/build-apk.sh" >/dev/null
    cp "$APK" "$DIST/clck-$abi.apk"
done

echo "==> release assets"
ls -lh "$DIST"
