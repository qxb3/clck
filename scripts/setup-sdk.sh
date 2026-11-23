#!/usr/bin/env bash
# Installs the Android SDK, NDK and a small x86_64 system image into ./.android-sdk
set -euo pipefail
source "$(dirname "$0")/env.sh"

CMDLINE_TOOLS_VERSION="11076708"
SDKMANAGER="$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager"

if [ ! -x "$SDKMANAGER" ]; then
    echo "==> downloading command line tools"
    mkdir -p "$ANDROID_HOME/cmdline-tools"
    tmp="$(mktemp -d)"
    curl -sSL -o "$tmp/cmdline-tools.zip" \
        "https://dl.google.com/android/repository/commandlinetools-linux-${CMDLINE_TOOLS_VERSION}_latest.zip"
    unzip -q "$tmp/cmdline-tools.zip" -d "$tmp"
    mkdir -p "$ANDROID_HOME/cmdline-tools/latest"
    mv "$tmp/cmdline-tools/"* "$ANDROID_HOME/cmdline-tools/latest/"
    rm -rf "$tmp"
fi

echo "==> accepting licenses"
yes | "$SDKMANAGER" --licenses >/dev/null 2>&1 || true

echo "==> installing packages"
"$SDKMANAGER" \
    "platform-tools" \
    "build-tools;34.0.0" \
    "platforms;android-34" \
    "ndk;26.3.11579264" \
    "emulator" \
    "system-images;android-30;aosp_atd;x86_64"

echo "==> rust targets"
rustup target add x86_64-linux-android aarch64-linux-android

echo "==> done"
