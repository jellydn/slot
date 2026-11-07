# Optional-core test candidate notices

This is an independent Slot fork and a test package, not an official Sony,
Lexaloffle or TrimUI product. No commercial game, Sony BIOS, official PICO-8 runtime,
personal configuration or credential is included. Existing stable core notices
and corresponding sources are retained alongside the new ones.

## Optional SwanStation integration

The package recipe can explicitly include SwanStation as an optional
software-renderer comparison core. PCSX remains default. See `SWANSTATION.md`
for the pinned revision, source/runtime hashes, licence/source materials and
the state/card safety limitation. SwanStation retains its GPL v3 licence and
separate dependency notices. The exact pinned source tree is supplied with the
optional runtime; it includes MIT-licensed PCSX-Redux OpenBIOS replacement
firmware, its binary/header and conversion files. Preserve
`dep/openbios/LICENSE.txt`, copyright 2019 PCSX-Redux authors. This licensed
embedded replacement is not a Sony BIOS or user BIOS. No source dependencies
are substituted or removed from the Swan archive.

## Slot

Slot is GPL-3.0-or-later. See `Slot/LICENSE`. The exact working-tree source
snapshot, lockfiles, frontend build command, source manifest and local diff
are supplied under `Slot/Sources/`. The source snapshot, not the unchanged
checkpoint Git identifier alone, identifies this uncommitted test build.

## PCSX ReARMed

Source: <https://github.com/libretro/pcsx_rearmed>, revision
`7b0e8f8fadfbce3d170d195b9d9eecaabfa08c8e`; libpicofe submodule
`a2697389e76744cfa0c56c13a172845c45ede719`.

PCSX ReARMed is GPL-2.0-or-later. Its separate shared library keeps its own
terms and copyright notices. The libpicofe README offers GPL-2.0-or-later,
LGPL-2.1-or-later or the MAME licence; this build uses the GPL option. Applicable
libchdr, miniz, zstd, public-domain LZMA SDK, NEON/filter and libretro-common
notices remain in the corresponding source and the extracted licence tree.
Physical CD support and enhanced resolution are not enabled.

Local changes: `pcsx-slot-boundaries.patch` bounds serialization streams and
uses CUE-relative tracks rather than unrelated working-directory files. The
exact original sources, submodule, patch and build recipe are supplied together.

## fake-08

Source: <https://github.com/jtothebell/fake-08>, revision
`814991a2571ad3970e386cef48f3b148aa1c27b9`; z8lua submodule
`e6928578d46b61fd5ea30cfcf547e855a30a0553`.

fake-08 is MIT, with separately licensed dependencies and borrowed code. Do
not treat every file as MIT. Preserve `LICENSE.MD` and source notices for
z8lua/Lua, Zepto 8, tac08, utf8-util, PicoLove, LodePNG, simpleini, miniz and
other attributed code. Platform-specific dependency listings in that licence
do not mean this libretro build links every listed platform library.

The oval algorithm is attributed to **Michael Dorgan**,
<https://stackoverflow.com/a/8448181>, under **CC BY-SA 3.0 Unported**,
<https://creativecommons.org/licenses/by-sa/3.0/>. The legal code is included
as `CC-BY-SA-3.0.txt`; preserve attribution and share-alike terms for that
material. It is not claimed as Slot-original code. The Slot patch does not
change the oval implementation.

Local change: `fake08-host-streams.patch` gives core file streams independent
handles so cart-data writes cannot replace the frontend diagnostic stream.
`fake08-slot-state.patch` repairs bounded build-specific Lua/VM/audio/input
snapshots and error propagation, initializes memory-loaded text-cart sections
and uses a restorable noise PRNG. Original file licences still apply; Slot's
new modifications to MIT-licensed fake-08/z8lua files are provided under MIT,
and synth modifications retain that file's WTFPL terms. Preserve the upstream
MIT attribution for the ps4-p8-derived persistence code. This is not the
official PICO-8 runtime and does not claim full cartridge compatibility.

## Source and content boundary

The PS1/PICO-8 GameBit fallback representations and procedural native Default
bezels are new original geometry in the Slot source, under GPL-3.0-or-later.
No GermanTacos or SkyWalker overlay, game art, console logo, manufacturer mark
or external texture is included in these defaults. Research of MIT/CC0 overlay
collections did not establish third-party trademark clearance; none was used.
User-supplied labels and custom bezel files retain their separate rights.

Core archives include exact code and pinned submodules used by the builds.
Uncompiled vendor sample/test cartridges are omitted and listed in provenance;
no code required to build either shared library is omitted. Frontend authored
test generators are included as source, not installed games. Core and frontend
sources, patches, licence texts and build materials travel in the same test ZIP.
