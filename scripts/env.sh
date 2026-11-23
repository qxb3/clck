# Shared environment: keeps every Android artefact inside this repository.
# ANDROID_HOME and NDK may be overridden, which is how CI uses a hosted SDK.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

export ANDROID_HOME="${ANDROID_HOME:-$ROOT/.android-sdk}"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_USER_HOME="${ANDROID_USER_HOME:-$ROOT/.android}"
export ANDROID_EMULATOR_HOME="${ANDROID_EMULATOR_HOME:-$ROOT/.android}"
export ANDROID_AVD_HOME="${ANDROID_AVD_HOME:-$ROOT/.android/avd}"
# Dedicated adb server so this emulator never talks to a host-wide one.
export ADB_SERVER_PORT=5038

export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/build-tools/34.0.0:$PATH"

NDK="${NDK:-$ANDROID_HOME/ndk/26.3.11579264}"
export NDK
CLANG="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$CLANG/x86_64-linux-android26-clang"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CLANG/aarch64-linux-android26-clang"
export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="$CLANG/armv7a-linux-androideabi26-clang"
export CARGO_TARGET_I686_LINUX_ANDROID_LINKER="$CLANG/i686-linux-android26-clang"

export PACKAGE="dev.buffy.clck"
export ACTIVITY="$PACKAGE/android.app.NativeActivity"
export APK="$ROOT/build/clck.apk"
export AVD_NAME="clck"
