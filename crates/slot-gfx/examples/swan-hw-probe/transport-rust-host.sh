#!/bin/sh
# Opt-in synthetic Linux host proof; no Slot/Swan or device/package operations.
set -eu
source_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/slot-transport-rust.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
export EGL_PLATFORM=surfaceless LIBGL_ALWAYS_SOFTWARE=1
export GALLIUM_DRIVER=llvmpipe MESA_SHADER_CACHE_DISABLE=true
"${RUSTC:-rustc}" --version
"${RUSTC:-rustc}" --edition=2021 -D warnings -C opt-level=2 \
    "$source_dir/transport-host.rs" -o "$scratch/probe"
"$scratch/probe" --quick
"$scratch/probe"
for fault in producer-fence consumer-fence generation origin timeout oversize publish-after-loss capture-after-loss rewind; do
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
        producer-fence|consumer-fence) expected='STOP reason=missing-fence' ;;
        generation) expected='STOP reason=stale-generation' ;;
        origin) expected='STOP reason=frame-pattern' ;;
        timeout) expected='STOP reason=fence-timeout' ;;
        oversize) expected='STOP reason=unsupported-geometry' ;;
        publish-after-loss|capture-after-loss) expected='STOP reason=invalid-session' ;;
        rewind) expected='STOP reason=late-frame-must-not-rewind' ;;
    esac
    case "$(cat "$scratch/control.log")" in
        *"$expected"*) printf 'PASS expected_rejection=%s exit=2\n' "$fault" ;;
        *) printf 'FAIL unexpected control failure: %s\n' "$fault" >&2; exit 1 ;;
    esac
done
printf 'PASS all nine Rust negative controls rejected at their intended boundary\n'
