#!/bin/sh
# Foreground normal Apps child only: stock owns the menu and its return path.
# This is not a boot launcher and never stops or restarts stock processes.
set -eu
root=/mnt/SDCARD/Slot
refuse() { printf '%s\n' "Slot stock safety check: $*" >&2; exit 78; }
[ "$#" -eq 0 ] || refuse 'extra arguments refused'
for dir in /mnt /mnt/SDCARD "$root" "$root/Logs" "$root/System"; do
    [ ! -L "$dir" ] && [ -d "$dir" ] || refuse 'missing or linked root directory'
done
mounts=0
while read -r source target type options rest; do
    case "$target" in
        /mnt/SDCARD)
            mounts=$((mounts + 1))
            case "$source" in /dev/*) ;; *) refuse 'SD is not a device mount' ;; esac
            case "$type" in exfat|vfat) ;; *) refuse 'SD is not FAT/exFAT' ;; esac
            case ",$options," in *,rw,*) ;; *) refuse 'SD is not writable' ;; esac
            case ",$options," in *,ro,*) refuse 'SD is read-only' ;; esac
            ;;
        /mnt/SDCARD/Slot|/mnt/SDCARD/Slot/*) refuse 'nested Slot mount' ;;
    esac
done </proc/mounts
[ "$mounts" -eq 1 ] || refuse 'SD mount missing or ambiguous'
check_tree() {
    for item in "$1"/* "$1"/.[!.]* "$1"/..?*; do
        [ ! -L "$item" ] || refuse 'linked Slot entry'
        [ -e "$item" ] || continue
        if [ -d "$item" ]; then check_tree "$item"
        elif [ ! -f "$item" ]; then refuse 'special Slot entry'
        fi
    done
}
check_tree "$root"
[ -f "$root/System/slot" ] && [ -x "$root/System/slot" ] || refuse 'Slot binary unavailable'
cd "$root" || refuse 'cannot enter Slot root'
umask 077
# Stock BusyBox ash and the host fixture dash support this required limit.
# shellcheck disable=SC3045
ulimit -c 0 || refuse 'cannot disable core dumps'
# No inherited library/core override; use the installed matched vendor stack.
unset SLOT_CORE LD_LIBRARY_PATH LD_PRELOAD LD_AUDIT
export SLOT_ROOT="$root" SLOT_BOARD=tg5040 SLOT_STOCK_APP=1
export PATH=/usr/bin:/bin:/usr/sbin:/sbin
# Never truncate an earlier log, even if a PID is reused after boot.
set -C
exec >"$root/Logs/stock-app-$$.log" 2>&1
printf '%s\n' 'Slot stock foreground test started; full presentation unverified'
set +e
"$root/System/slot"
status=$?
printf 'Slot child exit=%s; returning to stock Apps caller\n' "$status"
exit "$status"
