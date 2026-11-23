#!/usr/bin/env bash
# Builds libmain.so for the requested ABIs and packages a signed APK.
set -euo pipefail
source "$(dirname "$0")/env.sh"

TARGETS="${TARGETS:-x86_64-linux-android aarch64-linux-android}"
# Release builds pass these in from the git tag.
VERSION_NAME="${VERSION_NAME:-0.1.0}"
VERSION_CODE="${VERSION_CODE:-1}"
BUILD="$ROOT/build"
STAGE="$BUILD/stage"

mkdir -p "$BUILD"
rm -rf "$STAGE"
mkdir -p "$STAGE/lib"

for target in $TARGETS; do
    case "$target" in
        x86_64-linux-android) abi="x86_64" ;;
        aarch64-linux-android) abi="arm64-v8a" ;;
        armv7-linux-androideabi) abi="armeabi-v7a" ;;
        i686-linux-android) abi="x86" ;;
        *) echo "unsupported target: $target" >&2; exit 1 ;;
    esac
    echo "==> cargo build --release --target $target"
    (cd "$ROOT" && cargo build --release --target "$target")
    mkdir -p "$STAGE/lib/$abi"
    cp "$ROOT/target/$target/release/libmain.so" "$STAGE/lib/$abi/libmain.so"
done

echo "==> aapt2 compile"
rm -f "$BUILD/res.zip"
"$ANDROID_HOME/build-tools/34.0.0/aapt2" compile \
    --dir "$ROOT/android/res" \
    -o "$BUILD/res.zip"

echo "==> aapt2 link"
rm -f "$BUILD/base.apk" "$BUILD/aligned.apk"
"$ANDROID_HOME/build-tools/34.0.0/aapt2" link \
    -o "$BUILD/base.apk" \
    -I "$ANDROID_HOME/platforms/android-34/android.jar" \
    --manifest "$ROOT/android/AndroidManifest.xml" \
    --min-sdk-version 26 \
    --target-sdk-version 34 \
    --version-code "$VERSION_CODE" \
    --version-name "$VERSION_NAME" \
    "$BUILD/res.zip"

echo "==> package native libraries"
(cd "$STAGE" && zip -q -r "$BUILD/base.apk" lib)

echo "==> zipalign"
"$ANDROID_HOME/build-tools/34.0.0/zipalign" -f -p 4 "$BUILD/base.apk" "$BUILD/aligned.apk"

KEYSTORE="$BUILD/debug.keystore"
if [ ! -f "$KEYSTORE" ]; then
    echo "==> generating debug keystore"
    keytool -genkeypair \
        -keystore "$KEYSTORE" \
        -storepass android -keypass android \
        -alias androiddebugkey \
        -keyalg RSA -keysize 2048 -validity 10000 \
        -dname "CN=Android Debug,O=Android,C=US" >/dev/null 2>&1
fi

echo "==> sign"
rm -f "$APK"
"$ANDROID_HOME/build-tools/34.0.0/apksigner" sign \
    --ks "$KEYSTORE" \
    --ks-pass pass:android \
    --key-pass pass:android \
    --out "$APK" \
    "$BUILD/aligned.apk"

echo "==> $APK"
"$ANDROID_HOME/build-tools/34.0.0/apksigner" verify --print-certs "$APK" | head -3
ls -lh "$APK"
