#!/bin/sh
# HOST CANDIDATE ONLY. Official stock startup owns service launch/deduplication.
# Every SD entry restores the matched installed runtime's paths before exec.
set -eu
runtime=/usr/trimui
sd=/mnt/SDCARD/trimui
root=/mnt/SDCARD/Slot
refuse() { printf 'Slot frontend refused: %s\n' "$*" >&2; exit 78; }
[ "$#" -eq 1 ] || refuse 'one fixed entrypoint required'
for dir in /mnt /mnt/SDCARD "$sd" "$sd/app" "$sd/osd" "$sd/lib" "$runtime" "$runtime/bin" "$runtime/lib" "$runtime/osd"; do
    [ -d "$dir" ] && [ ! -L "$dir" ] || refuse 'runtime directory missing or linked'
done
[ -f "$sd/runtime.sha256" ] && [ ! -L "$sd/runtime.sha256" ] || refuse 'manifest missing or linked'
[ -f "$sd/runtime.links" ] && [ ! -L "$sd/runtime.links" ] || refuse 'link manifest missing or linked'
sha256sum -c "$sd/runtime.sha256" >/dev/null 2>&1 || refuse 'installed runtime hash mismatch'
while read -r target path; do
    [ "$(readlink "$path")" = "$target" ] || refuse 'installed runtime link mismatch'
done < "$sd/runtime.links"
unset LD_PRELOAD LD_AUDIT
export LD_LIBRARY_PATH="$runtime/lib"
export PATH="$runtime/bin:/usr/bin:/bin:/usr/sbin:/sbin"
cd "$runtime/bin"
case "$1" in
    keymon|trimui_inputd|trimui_scened|trimui_btmanager|hardwareservice|musicserver)
        exec "$runtime/bin/$1" ;;
    trimui_osdd)
        cd "$runtime/osd"
        exec "$runtime/osd/trimui_osdd" ;;
    preload)
        # Preserve stock's normal post-frontend cleanup, including its CPU policy.
        # Its final rm may return 1 when no stay-awake file existed. That must
        # not start a second fallback menu after this dispatcher already ran it.
        set +e
        /bin/sh "$runtime/bin/preload.sh"
        status=$?
        printf 'Slot frontend: stock preload returned %s\n' "$status" >&2
        exit 0 ;;
    MainUI) ;;
    *) refuse 'unknown entrypoint' ;;
esac
stock_menu() {
    printf 'Slot frontend: stock MainUI foreground; Slot is not running\n' >&2
    exec "$runtime/bin/MainUI"
}
# Offline recovery: either marker bypasses Slot. No flag is removed at runtime.
if [ -e "$root/Config/stock-menu-only" ] || [ -L "$root/Config/stock-menu-only" ] ||
   [ -e /tmp/slot-frontend-return-to-stock ] || [ -L /tmp/slot-frontend-return-to-stock ]; then
    stock_menu
fi
services_ready() {
    running=0
    for comm in /proc/[0-9]*/comm; do
        [ -f "$comm" ] || continue
        IFS= read -r name < "$comm" || continue
        case "$name" in
            keymon) running=$((running | 1)) ;;
            trimui_inputd) running=$((running | 2)) ;;
            trimui_scened) running=$((running | 4)) ;;
            hardwareservice) running=$((running | 8)) ;;
            musicserver) running=$((running | 16)) ;;
            trimui_osdd) running=$((running | 32)) ;;
        esac
    done
    # Bluetooth manager may exit when Bluetooth is disabled. Do not start it
    # ourselves or turn Bluetooth on to satisfy a frontend readiness check.
    [ "$running" -eq 63 ] || return 1
    for event in /sys/class/input/event*; do
        IFS= read -r name < "$event/device/name" 2>/dev/null || continue
        [ "$name" = 'TRIMUI Player1' ] || continue
        [ -c "/dev/input/${event##*/}" ] && [ -r "/dev/input/${event##*/}" ] || continue
        IFS= read -r caps < "$event/device/capabilities/ev" 2>/dev/null || continue
        case "$caps" in ''|*[!0-9a-fA-F]*) continue ;; esac
        [ "${#caps}" -le 8 ] || continue
        [ "$((0x$caps & 10))" -ne 10 ] || return 0
    done
    return 1
}
# Unlike System/starts, this point is AFTER all official service launch requests.
tries=0
until services_ready; do
    if [ "$tries" -ge 10 ]; then
        printf 'Slot frontend: service/controller readiness timed out; stock fallback\n' >&2
        stock_menu
    fi
    sleep 0.5
    tries=$((tries + 1))
done
for dir in /mnt/SDCARD/Apps /mnt/SDCARD/Apps/SlotStockTest; do
    [ -d "$dir" ] && [ ! -L "$dir" ] || stock_menu
done
launcher=/mnt/SDCARD/Apps/SlotStockTest/launch.sh
[ -f "$launcher" ] && [ ! -L "$launcher" ] || stock_menu
# One automatic attempt per boot; errors and normal exits never relaunch Slot.
# This is a tmpfs directory only. No firmware, settings or SD hook is modified.
mkdir /tmp/slot-frontend-return-to-stock 2>/dev/null || stock_menu
set +e
/bin/sh "$launcher"
status=$?
printf 'Slot frontend: foreground child returned %s; no automatic retry\n' "$status" >&2
stock_menu
