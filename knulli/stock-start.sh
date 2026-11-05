#!/bin/sh
# SD-only stock System/starts hook. Run once in the foreground, then let the
# existing official startup continue. No updater, takeover or service commands.
set -u
root=/mnt/SDCARD/Slot
launcher=/mnt/SDCARD/Apps/SlotStockTest/launch.sh

# An offline-created marker bypasses automatic Slot startup. Do not read it.
if [ -e "$root/Config/stock-menu-only" ] || [ -L "$root/Config/stock-menu-only" ]; then
    exit 0
fi
for dir in /mnt /mnt/SDCARD /mnt/SDCARD/Apps /mnt/SDCARD/Apps/SlotStockTest; do
    if [ -L "$dir" ] || [ ! -d "$dir" ]; then
        printf '%s\n' 'Slot startup skipped: unsafe or missing launcher directory' >&2
        exit 0
    fi
done
if [ -L "$launcher" ] || [ ! -f "$launcher" ]; then
    printf '%s\n' 'Slot startup skipped: launcher unavailable' >&2
    exit 0
fi

# Stock starts inputd after this synchronous hook. Never wait here for it.
inputd_ready=0
for comm in /proc/[0-9]*/comm; do
    [ -f "$comm" ] || continue
    IFS= read -r name < "$comm" || continue
    if [ "$name" = trimui_inputd ]; then
        inputd_ready=1
        break
    fi
done
if [ "$inputd_ready" -ne 1 ]; then
    printf '%s\n' 'Slot startup skipped: stock input service not ready' >&2
    exit 0
fi
controller_ready=0
for event in /sys/class/input/event*; do
    IFS= read -r name < "$event/device/name" 2>/dev/null || continue
    [ "$name" = 'TRIMUI Player1' ] || continue
    node=/dev/input/${event##*/}
    [ -c "$node" ] && [ -r "$node" ] || continue
    IFS= read -r caps < "$event/device/capabilities/ev" 2>/dev/null || continue
    case "$caps" in ''|*[!0-9a-fA-F]*) continue ;; esac
    [ "${#caps}" -le 8 ] || continue
    # EV_KEY and EV_ABS are required; volume/power/jack nodes are not a pad.
    if [ "$((0x$caps & 10))" -eq 10 ]; then
        controller_ready=1
        break
    fi
done
if [ "$controller_ready" -ne 1 ]; then
    printf '%s\n' 'Slot startup skipped: stock controller not ready' >&2
    exit 0
fi
/bin/sh "$launcher"
status=$?
printf 'Slot startup returned status=%s; continuing stock startup\n' "$status" >&2
exit 0
