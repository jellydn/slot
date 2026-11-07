#!/bin/sh
# Opt-in Linux host diagnostic. Does not build Slot, Swan or device packages.
set -eu
source_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/slot-transport-host.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

"${CC:-cc}" -std=c11 -O2 -Wall -Wextra -Werror \
    "$source_dir/transport-host.c" -o "$scratch/probe" -pthread -lEGL -lGLESv2
"$scratch/probe" --quick
"$scratch/probe"

for fault in producer-fence consumer-fence generation origin timeout oversize publish-after-loss capture-after-loss; do
    if "$scratch/probe" --fault "$fault" > "$scratch/control.log" 2>&1; then
        status=0
    else
        status=$?
    fi
    cat "$scratch/control.log"
    if [ "$status" -ne 2 ]; then
        printf 'FAIL control=%s expected_exit=2 actual_exit=%s\n' "$fault" "$status" >&2
        exit 1
    fi
    case "$fault" in
        producer-fence|consumer-fence) expected='reason=missing-fence' ;;
        generation) expected='reason=stale-generation' ;;
        origin) expected='reason=frame-pattern' ;;
        timeout) expected='reason=fence-timeout' ;;
        oversize) expected='reason=unsupported-geometry' ;;
        publish-after-loss|capture-after-loss) expected='reason=invalid-session' ;;
    esac
    case "$(cat "$scratch/control.log")" in
        *"$expected"*) printf 'PASS expected_rejection=%s exit=2\n' "$fault" ;;
        *) printf 'FAIL unexpected control failure: %s\n' "$fault" >&2; exit 1 ;;
    esac
done
printf 'PASS all eight negative controls rejected at their intended boundary\n'
