#!/usr/bin/env bash
# Boots the isolated, low-spec headless emulator and waits until Android is up.
set -euo pipefail
source "$(dirname "$0")/env.sh"

AVD_DIR="$ANDROID_AVD_HOME/$AVD_NAME.avd"
BOOT_TIMEOUT="${BOOT_TIMEOUT:-1800}"

if [ ! -f "$AVD_DIR/config.ini" ]; then
    echo "==> creating AVD $AVD_NAME"
    mkdir -p "$ANDROID_AVD_HOME"
    echo no | avdmanager create avd \
        -n "$AVD_NAME" \
        -k "system-images;android-30;aosp_atd;x86_64" \
        --device "10.1in WXGA (Tablet)" \
        --force >/dev/null
fi

# Keep the emulator small and disposable; this image only ever runs the clock.
set_avd() {
    sed -i "/^$1=/d" "$AVD_DIR/config.ini"
    echo "$1=$2" >> "$AVD_DIR/config.ini"
}
set_avd hw.ramSize 1536
set_avd vm.heapSize 128
set_avd hw.lcd.width 1280
set_avd hw.lcd.height 800
set_avd hw.lcd.density 160
set_avd hw.initialOrientation landscape
set_avd hw.gpu.enabled yes
set_avd hw.gpu.mode swiftshader_indirect
set_avd fastboot.forceFastBoot no
set_avd fastboot.forceColdBoot yes
set_avd firstboot.bootFromLocalSnapshot no
set_avd firstboot.saveToLocalSnapshot no
set_avd disk.dataPartition.size 1G

if ! adb get-state >/dev/null 2>&1; then
    echo "==> starting emulator (software TCG, no KVM needed)"
    # setsid detaches the emulator from this shell's process group so it keeps
    # running after the script (or a CI wrapper) exits.
    ANDROID_I_WANT_MY_TCG=yes setsid emulator \
        -avd "$AVD_NAME" \
        -no-window -no-audio -no-boot-anim -no-snapshot \
        -gpu swiftshader_indirect \
        -accel off \
        -memory 1536 -cores 2 \
        -no-metrics \
        >"$ROOT/.emulator.log" 2>&1 </dev/null &
    echo $! >"$ROOT/.emulator.pid"
fi

echo "==> waiting for device"
adb wait-for-device

echo "==> waiting for boot_completed (timeout ${BOOT_TIMEOUT}s)"
start=$(date +%s)
until [ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ]; do
    if ! adb get-state >/dev/null 2>&1; then
        echo "emulator disappeared; see .emulator.log" >&2
        exit 1
    fi
    if [ $(( $(date +%s) - start )) -gt "$BOOT_TIMEOUT" ]; then
        echo "boot timed out after ${BOOT_TIMEOUT}s" >&2
        exit 1
    fi
    sleep 5
done

# Landscape, no animations, screen awake: deterministic screenshots.
adb shell settings put global window_animation_scale 0 >/dev/null 2>&1 || true
adb shell settings put global transition_animation_scale 0 >/dev/null 2>&1 || true
adb shell settings put global animator_duration_scale 0 >/dev/null 2>&1 || true
adb shell input keyevent 82 >/dev/null 2>&1 || true

echo "==> emulator ready: $(adb shell getprop ro.product.model | tr -d '\r') ($(adb shell wm size | tr -d '\r'))"
