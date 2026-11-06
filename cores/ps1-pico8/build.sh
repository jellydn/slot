#!/bin/sh
# Build reviewed source only; no downloader and no device installation.
set -eu

here="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
if [ "$#" -ne 5 ]; then
    echo "usage: sh $0 fake08|pcsx_rearmed host|tg5040 SOURCE NEW_WORKDIR OUT" >&2
    exit 2
fi
core=$1 profile=$2 source=$3 work=$4 out=$5
case "$core" in
fake08)
    commit=814991a2571ad3970e386cef48f3b148aa1c27b9
    sub=libs/z8lua
    dependency=e6928578d46b61fd5ea30cfcf547e855a30a0553
    ;;
pcsx_rearmed)
    commit=7b0e8f8fadfbce3d170d195b9d9eecaabfa08c8e
    sub=frontend/libpicofe
    dependency=a2697389e76744cfa0c56c13a172845c45ede719
    ;;
*) echo "unknown core: $core" >&2; exit 2 ;;
esac
case "$profile" in
host)
    flags=
    case "$(uname -s)" in Darwin) platform=osx; ext=dylib ;; Linux) platform=unix; ext=so ;; *) exit 2 ;; esac
    dynarec=0
    ;;
tg5040)
    [ "$(uname -s):$(uname -m)" = Linux:aarch64 ] || { echo 'TG5040 builds require an AArch64 Linux toolchain' >&2; exit 2; }
    flags=-mcpu=cortex-a53
    platform=unix ext=so dynarec=ari64
    ;;
*) echo "unknown profile: $profile" >&2; exit 2 ;;
esac
[ "$(git -C "$source" rev-parse HEAD)" = "$commit" ]
[ "$(git -C "$source/$sub" rev-parse HEAD)" = "$dependency" ]
# A fresh work directory prevents stale objects or local source edits entering the build.
mkdir "$work"
git -C "$source" archive "$commit" | tar -xf - -C "$work"
git -C "$source/$sub" archive "$dependency" | tar -xf - -C "$work/$sub"
if [ "$core" = fake08 ]; then
    patch -p1 --batch --forward -d "$work" < "$here/fake08-host-streams.patch"
    patch -p1 --batch --forward -d "$work" < "$here/fake08-slot-state.patch"
    abi=$({
        cat "$here/fake08-host-streams.patch" "$here/fake08-slot-state.patch"
        printf '%s\n' "$commit" "$dependency" "$profile" "$platform" "$flags" "$(uname -m)" "$(cc --version | head -1)"
    } | if command -v sha256sum >/dev/null 2>&1; then sha256sum; else shasum -a 256; fi | cut -d ' ' -f1)
    CFLAGS="$flags -DSLOT_STATE_ABI=\\\"$abi\\\"" CXXFLAGS="$flags" make -C "$work/platform/libretro" \
        platform="$platform" arch=arm -j"${JOBS:-2}"
    binary="$work/platform/libretro/fake08_libretro.$ext"
else
    patch -p1 --batch --forward -d "$work" < "$here/pcsx-slot-boundaries.patch"
    CFLAGS="$flags" CXXFLAGS="$flags" make -C "$work" -f Makefile.libretro \
        platform="$platform" DYNAREC="$dynarec" BUILTIN_GPU=neon HAVE_CHD=1 \
        GIT_VERSION="\" $(printf %s "$commit" | cut -c1-7)\"" \
        HAVE_PHYSICAL_CDROM=0 -j"${JOBS:-2}"
    binary="$work/pcsx_rearmed_libretro.$ext"
fi
mkdir -p "$(dirname "$out")"
cp "$binary" "$out"
{
    echo "core=$core"
    echo "commit=$commit"
    echo "submodule=$sub@$dependency"
    echo "profile=$profile platform=$platform cflags=$flags"
    [ "$core" != fake08 ] || echo "patch=fake08-host-streams.patch"
    [ "$core" != fake08 ] || echo "patch=fake08-slot-state.patch state_abi=$abi"
    [ "$core" != pcsx_rearmed ] || echo "patch=pcsx-slot-boundaries.patch"
    [ "$core" != pcsx_rearmed ] || echo "dynarec=$dynarec gpu=neon chd=1 physical_cdrom=0"
    echo "compiler=$(cc --version | head -1)"
    if command -v sha256sum >/dev/null 2>&1; then sha256sum "$out"; else shasum -a 256 "$out"; fi
} > "$out.meta"
