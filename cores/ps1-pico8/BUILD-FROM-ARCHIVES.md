# Build the supplied sources

The ZIP contains exact frontend code and original pinned core sources, including
the two required submodules. The core patches are separate and must be applied
once. Uncompiled vendor sample carts are omitted, not build inputs. Archive
hashes are in the payload manifest. No BIOS or ROM is needed to build.

## AArch64 Linux toolchain

Use the image recorded in `Slot/BUILD-INFO.json`:
`rust@sha256:4730e387a220a08a365c77da3096544dde214f9d796c16284d4be45438cad4a9`.
It supplies Debian GCC/G++ 10.2.1, make, patch, binutils and Rust 1.98.0.
Run as an ordinary user when testing permission failures, not container root.
Mount only a disposable host work directory; do not mount a card or device.

From a new directory, unpack `combined-frontend-source.tar.gz`,
`fake-08-source.tar.gz` and `pcsx_rearmed-source.tar.gz`. This creates `source`,
`fake-08` and `pcsx_rearmed` beside one another. Use this frontend snapshot,
not the separate older stable 0.2.0 frontend source download. The unchanged
Game Boy core sources/build materials are retained in this candidate.

The precompiled `crates/slot/tests/rtc_probe/probe.gba` test ROM is also omitted;
its C, assembly and linker-script sources are retained. It is not a frontend
or core build input. Tests that read that optional ROM need it regenerated
with an ARM7TDMI bare-metal toolchain before running that fixture.

## Cores from archives (no Git metadata required)

Run these commands from that new directory on AArch64 Linux. The pinned PCSX
Git string is supplied explicitly because the archive has no Git repository.
Core source notices remain applicable after applying the patches.

```sh
patch -p1 -d fake-08 < source/cores/ps1-pico8/fake08-host-streams.patch
patch -p1 -d fake-08 < source/cores/ps1-pico8/fake08-slot-state.patch
abi=$({
  cat source/cores/ps1-pico8/fake08-host-streams.patch \
      source/cores/ps1-pico8/fake08-slot-state.patch
  printf '%s\n' 814991a2571ad3970e386cef48f3b148aa1c27b9 \
    e6928578d46b61fd5ea30cfcf547e855a30a0553 tg5040 unix \
    -mcpu=cortex-a53 "$(uname -m)" "$(cc --version | head -1)"
} | sha256sum | cut -d ' ' -f1)
CFLAGS="-mcpu=cortex-a53 -DSLOT_STATE_ABI=\\\"$abi\\\"" CXXFLAGS=-mcpu=cortex-a53 \
  make -C fake-08/platform/libretro platform=unix arch=arm -j2

patch -p1 -d pcsx_rearmed < source/cores/ps1-pico8/pcsx-slot-boundaries.patch
CFLAGS=-mcpu=cortex-a53 CXXFLAGS=-mcpu=cortex-a53 \
  make -C pcsx_rearmed -f Makefile.libretro platform=unix \
  DYNAREC=ari64 BUILTIN_GPU=neon HAVE_CHD=1 HAVE_PHYSICAL_CDROM=0 \
  'GIT_VERSION=" 7b0e8f8"' -j2
```

Outputs are `fake-08/platform/libretro/fake08_libretro.so` and
`pcsx_rearmed/pcsx_rearmed_libretro.so`. `build.sh` is the alternate recipe for
clean Git checkouts at the recorded pins; it exports fresh build trees and
checks the submodule revisions before applying exactly these patches.

## Frontend and web

From `source`, run the command below with Rust 1.98.0. Cargo dependencies are
pinned by `Cargo.lock`; provide a matching Cargo registry cache for offline
mode, or download them in advance with `cargo fetch --locked`.

```sh
SLOT_VERSION=stock-tsp-0.3.0-test.3 \
  cargo build --locked --offline -p slot --no-default-features \
  --features stock-tg5040 --profile device
```

The output is `target/device/slot`. Source archives have no Git stamp; their
displayed checkpoint may be `unknown` unless built in the recorded repository.
The supplied frontend snapshot manifest and SHA256 identify the actual source,
not a promise that a different checkout/date/path produces identical bytes.

Build web assets in `source/web` with Bun: `bun install --frozen-lockfile`,
`bun run test`, then `bun run build`. The candidate uses Bun 1.4.4. Copy only
the resulting `dist` files to a new package's `Slot/System/web` directory.

The original Game Boy core sources and build materials remain under
`Slot/Sources/`; their binaries were not rebuilt for this candidate. Their
licences are retained under `Slot/System/licenses/`. Repackaging must retain
all corresponding sources, notices and the current test limitations.
