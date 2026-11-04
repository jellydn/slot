#!/bin/sh
set -e
HERE=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
/etc/init.d/S31emulationstation stop || true
export SLOT_ROOT="$HERE"
export LD_LIBRARY_PATH=/usr/lib:${LD_LIBRARY_PATH:-}
# pad map: SLOT_BOARD or /boot/boot/knulli.board (binary already reads both)
if [ -z "${SLOT_BOARD:-}" ] && [ -f /boot/boot/knulli.board ]; then
  SLOT_BOARD=$(tr -d '[:space:]' < /boot/boot/knulli.board)
  export SLOT_BOARD
fi
"$HERE/System/slot"
rc=$?
/etc/init.d/S31emulationstation start || true
exit $rc
