# Shared environment: keeps every Android artefact inside this repository.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

export ANDROID_HOME="$ROOT/.android-sdk"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_USER_HOME="$ROOT/.android"
export ANDROID_EMULATOR_HOME="$ROOT/.android"
export ANDROID_AVD_HOME="$ROOT/.android/avd"
# Dedicated adb server so this emulator never talks to a host-wide one.
export ADB_SERVER_PORT=5038

export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/build-tools/34.0.0:$PATH"

NDK="$ANDROID_HOME/ndk/26.3.11579264"
export NDK
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android26-clang"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android26-clang"

export PACKAGE="dev.buffy.clck"
export ACTIVITY="$PACKAGE/android.app.NativeActivity"
export APK="$ROOT/build/clck.apk"
export AVD_NAME="clck"
