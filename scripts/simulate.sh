#!/usr/bin/env bash
# Jump the clock forward/back to exercise transitions without waiting for them.
#
#   scripts/simulate.sh ampm     # 11:20 AM -> 11:20 PM
#   scripts/simulate.sh minute   # roll the minute (odometer effect)
#   scripts/simulate.sh day      # next calendar day
#   scripts/simulate.sh month    # next month, same day-of-month
#   scripts/simulate.sh year     # next year
#   scripts/simulate.sh "-2 hours"
#   scripts/simulate.sh reset    # back to real time
#
# Jumps stack: each call offsets the currently displayed time.
set -euo pipefail
source "$(dirname "$0")/env.sh"

jump="${1:-reset}"
delta=""

case "$jump" in
    reset | now) offset=0 ;;
    minute | min) delta="1 minute" ;;
    hour) delta="1 hour" ;;
    ampm | period) delta="12 hours" ;;
    day) delta="1 day" ;;
    month) delta="1 month" ;;
    year) delta="1 year" ;;
    -*) delta="${jump#-} ago" ;;
    +*) delta="${jump#+}" ;;
    *) delta="$jump" ;;
esac

# debug.* properties need root on the emulator images.
if [ "$(adb shell id -u 2>/dev/null | tr -d '\r')" != "0" ]; then
    adb root >/dev/null 2>&1 || true
    adb wait-for-device
fi

if [ "${offset:-}" = "0" ]; then
    adb shell setprop debug.clck.offset_secs 0
    echo "clock: live time restored"
    exit 0
fi

real_now=$(adb shell date +%s | tr -d '\r')
current=$(adb shell getprop debug.clck.offset_secs | tr -d '\r')
if [[ ! "$current" =~ ^-?[0-9]+$ ]]; then
    current=0
fi

# Shift whatever is currently displayed, so repeated calls accumulate.
# Do the arithmetic in UTC: GNU date would otherwise read "+12" as a UTC offset.
base=$(date -u -d "@$((real_now + current))" '+%Y-%m-%d %H:%M:%S')
effective=$(date -u -d "$base $delta" +%s)
offset=$((effective - real_now))
adb shell setprop debug.clck.offset_secs "$offset"

echo "clock: $(date -u -d "@$effective" '+%A %b %d %Y %H:%M') UTC  (offset ${offset}s)"
