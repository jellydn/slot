#!/bin/sh
# Stock Apps entry only. The official launcher remains unchanged.
fail() { printf '%s\n' "Grout refused: $*" >&2; exit 78; }
[ "$#" -eq 0 ] || fail 'arguments are not supported'
[ -z "${LD_PRELOAD-}${LD_AUDIT-}" ] || fail 'loader overrides are not supported'
case "$0" in
    /mnt/SDCARD/Apps/Grout/slot-launch.sh) ;;
    *) fail 'use the stock Apps entry at /mnt/SDCARD/Apps/Grout' ;;
esac
for path in /mnt /mnt/SDCARD /mnt/SDCARD/Apps /mnt/SDCARD/Apps/Grout; do
    [ -d "$path" ] && [ ! -L "$path" ] || fail 'linked or missing app root'
done
ROOT=$(CDPATH='' cd -P /mnt/SDCARD/Apps/Grout && pwd -P) || fail 'cannot resolve app root'
[ "$ROOT" = /mnt/SDCARD/Apps/Grout ] || fail 'wrong canonical app root'
[ ! -L "$0" ] || fail 'linked wrapper'
for path in "$ROOT/grout" "$ROOT/grout/lib"; do
    [ -d "$path" ] && [ ! -L "$path" ] || fail 'linked or missing runtime directory'
done
for path in "$ROOT/launch.sh" "$ROOT/grout/grout" "$ROOT/grout/lib/libSDL2_gfx-1.0.so.0"; do
    [ -f "$path" ] && [ ! -L "$path" ] || fail 'linked or missing release file'
done
[ ! -e "$ROOT/lib" ] && [ ! -L "$ROOT/lib" ] || fail 'unexpected outer library directory'
[ ! -e "$ROOT/../.update" ] && [ ! -L "$ROOT/../.update" ] || fail 'Apps/.update requires manual review'
PATH=/usr/bin:/bin
export PATH
printf '%s\n' \
    '5b51e2149a192823662a07a6200d86b1f43343bee81e9ea742aa2e7433951e12  /mnt/SDCARD/Apps/Grout/launch.sh' \
    '9d5c6a51683462aa81ce83baef65940c65105603e6a7b3ef588ef815b6ed6958  /mnt/SDCARD/Apps/Grout/grout/grout' \
    '8f124e2666901f68913fcce997816c167ab28a5723c709803eed4d91ad1f24ad  /mnt/SDCARD/Apps/Grout/grout/lib/libSDL2_gfx-1.0.so.0' \
    | sha256sum -c - >/dev/null 2>&1 || fail 'official release hashes changed'
# A stale lock is a safe refusal. Never kill another app or remove its lock.
LOCK=/tmp/slot-grout.lock
mkdir "$LOCK" 2>/dev/null || fail 'another Grout launch or stale lock exists'
trap 'rmdir "$LOCK" 2>/dev/null' EXIT
[ ! -e "$ROOT/../.update" ] && [ ! -L "$ROOT/../.update" ] || fail 'Apps/.update requires manual review'
LD_LIBRARY_PATH="$ROOT/grout/lib:/usr/trimui/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export LD_LIBRARY_PATH
cd "$ROOT" || fail 'cannot enter app root'
/bin/sh "$ROOT/launch.sh"
status=$?
exit "$status"
