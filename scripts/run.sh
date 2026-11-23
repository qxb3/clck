#!/usr/bin/env bash
# Build, install and screenshot the clock on the local emulator.
set -euo pipefail
source "$(dirname "$0")/env.sh"

"$ROOT/scripts/emulator.sh"
"$ROOT/scripts/build-apk.sh"

echo "==> installing"
adb install -r "$APK" >/dev/null
adb shell am force-stop "$PACKAGE" || true
adb shell am start -W -n "$ACTIVITY" | tail -3
sleep 3

echo "==> capturing build/screenshot.png"
adb exec-out screencap -p >"$ROOT/build/screenshot.png"

echo "==> app log"
adb logcat -d -s clck:V | tail -20
