# Optional SwanStation comparison core — offline integration gate

PCSX ReARMed remains the PS1 default. SwanStation is an explicit per-game
choice (`swanstation` in `Config/selected_core.txt`). It is not a replacement,
release dependency, or claim of higher speed or handheld compatibility.

## Source and licence boundary

Upstream: <https://github.com/libretro/swanstation>, revision
`631f0fb31b809e0f889d7c551fe695788e39c1d3`.
The local checkout used for the prior build is
`/tmp/amp-librarian/libretro_swanstation`; its working tree was clean when
checked. The exported build tree and output are under
`.amp/in/ps1-alternative/swan-build/`.

SwanStation supplies the GNU GPL version 3 in `LICENSE`. Keep that licence,
all source copyright notices and the complete corresponding build source.
Do not label the entire dependency tree GPL-only: preserve the separate
notices for `dep/libchdr` and its compression libraries, `dep/libretro-common`,
`dep/openbios`, `dep/vixl`, `dep/glad`, `dep/stb`, `dep/xxhash`,
`dep/vulkan-loader`, and any other compiled or redistributed dependencies.
The Makefile uses VIXL on AArch64; the presence of a dependency directory alone
does not prove it was linked. Preserve the exact dependency sources and build
flags rather than infer attribution from a single umbrella licence.

The optional package inputs retain the pinned source, licences, build recipe,
toolchain identity and per-file source manifest beside the runtime. Candidate
readiness is recorded in its verification receipts, separately from hardware acceptance.
No Sony BIOS or game is supplied. A user-supplied BIOS remains the user's
responsibility; authored contract fixtures are not BIOS boot evidence.

### Licensed embedded replacement firmware

This revision contains the MIT-licensed PCSX-Redux OpenBIOS binary and generated
C++ array under `dep/openbios/`. `src/core/system.cpp` includes
`openbios.bin.h` and calls `Bus::SetBIOS(openbios, sizeof(openbios))` when
no external BIOS image was found. This is not a Sony BIOS or user file, but it
is BIOS code embedded in the core. Omitting just `openbios.bin` from a source
archive does not remove the compiled firmware or the generated header.

The package excludes user/proprietary game ROMs, Sony BIOS and user data.
The upstream replacement firmware has a separate MIT licence; no specific
owner approval of OpenBIOS is claimed. Preserve `dep/openbios/LICENSE.txt`
(MIT, copyright 2019
PCSX-Redux authors), the binary, generated header and conversion/build files.
The complete pinned upstream tree is retained without dependency substitutions.
Existing test.7 source snapshots and payloads are not modified; test.8 remains
local and hardware-unverified.

Independent identity checks (hash the complete bytes, not display strings):

| Input | SHA-256 |
| --- | --- |
| `git archive --format=tar 631f0fb31b809e0f889d7c551fe695788e39c1d3` | `f5ead949860a003e928e581c4a2b154574d91672350f2887d152985cb122bb68` |
| `dep/openbios/openbios.bin` (524288 bytes) | `0a9685d9895b422f091a9bcceed8f73cf67a414c09eb8310c714f27ccdfdb349` |
| `dep/openbios/openbios.bin.h` | `bef0f79598bd867fb310d4acfbbbfcacf6c316aecb803a9ee062d0547e5aa0ed` |

The raw unfiltered Git tar is 113612800 bytes and includes upstream firmware.
It includes tracked vendored dependencies; this revision has no Git submodules.
Recompute from a separately obtained pinned checkout with:

```sh
git archive --format=tar 631f0fb31b809e0f889d7c551fe695788e39c1d3 | shasum -a 256
shasum -a 256 dep/openbios/openbios.bin dep/openbios/openbios.bin.h
shasum -a 256 /path/to/swanstation_libretro.so
```

For the parent's final package command, add both optional arguments:

```sh
--swan-source /tmp/amp-librarian/libretro_swanstation \
--swan-core .amp/in/ps1-alternative/swan-build/swanstation_libretro.so
```

The recipe rejects an unreviewed source/runtime hash before it writes output.
It archives only pinned Git content, not dirty/untracked checkout files, user
BIOS or saves. The output includes `Sources/swanstation-source.tar.gz`,
`Sources/SWANSTATION-MANIFEST.json`, extracted licence texts and runtime metadata.
`BUILD-INFO.json` records the compressed source archive hash, original Git tar
hash, runtime hash, embedded firmware and the state-support limitation. The
outer package manifest also covers those files. Omitting both arguments leaves
Swan out of the package; PCSX is default in either case.

## Prior build evidence and recipe

The prior build used offline Docker image
`rust@sha256:4730e387a220a08a365c77da3096544dde214f9d796c16284d4be45438cad4a9`
on `linux/arm64`, with no network and a read-only source mount. The command was:

```sh
CFLAGS="-mcpu=cortex-a53" CXXFLAGS="-mcpu=cortex-a53" \
  make -f Makefile.libretro platform=unix CPU_ARCH=aarch64 -j4
```

The exact wrapper is `.amp/in/ps1-alternative/build-probe.sh`; its ELF/version
report is `.amp/in/ps1-alternative/build.log`. The existing
`swan-build/swanstation_libretro.so` SHA-256 is
`3b5d66f649aa1a1d53feaf102a14d6787ae337f758199d89cfd42ba0f9da94a8`.
That report identifies AArch64 and a maximum GLIBC requirement of 2.29.
This is build/ABI evidence, not successful device loading or frame timing.
For a repeat, export the pinned full source/dependencies into a new disposable
directory and use the wrapper's same offline image and flags. Never clean or
overwrite the existing evidence tree. Installation is owner-controlled:
`System/swanstation_libretro.so` on Linux (host extension differs).

## Runtime policy and unresolved decision

The frontend presets the software renderer before `retro_init`, native 1x
resolution, PGXP off, card 1 `Libretro`, and card 2 `None`. No hardware context
or new rendering backend is introduced. In upstream
`src/libretro/libretro_host_interface.cpp`, `RequestHardwareRendererContext`
returns without a hardware request for `Software`.

**Swan save states and resume are disabled.** Upstream
`src/core/pad.cpp::DoStateMemcard` reads card contents from a snapshot into the
live card. That can roll back a newer card even with separate save directories.
The owner approved comparison-only operation with states, quick states and
automatic resume disabled; no card reconciliation is part of this milestone.
Do not silently restore an old card or copy a PCSX state. Enabling states later
would require a separate decision and validation.

Cards use `Saves/PS/<core>/<SHA256(game-stem)>/memory.sav`. The same per-core,
per-game directory is supplied to the core as its save directory. Existing PCSX
paths stay unchanged. There is no automatic import, copy, fallback, or migration
between cores. The state ring remains `States/PS/<core>/<game-stem>/` for both
manual and resume paths; those paths do not imply Swan state support is enabled.

The authored C fixture checks pre-init options, save directory, native
software-frame transport, card memory, refusal of states and unchanged card
bytes after refusal. Real-core acceptance uses the original MIPS program
`acceptance.S`, built by `build-acceptance.py` into two MODE2 discs and an M3U.
No SDK, proprietary game, external BIOS or network is needed for these checks.

### Latest offline acceptance

The unchanged pinned Swan runtime passes authored boot, RIGHT-controlled
red/green output, game-generated audio with silence on release, disc reads
0→1→0 with distinct data on each disc, and a checked card counter across three
core reopen cycles. These are emulated-game assertions, not firmware frame or
audio-callback counts. The source's RGB555→RGB565 conversion yields green 251;
the colour assertion permits one quantisation step, but not the wrong colour.
The audio gate observed 44234 held samples with absolute energy 379946594,
versus zero energy while idle and after release.

The frontend now selects the core-advertised DualShock subclass: Swan uses 261,
whereas PCSX uses 517. A contract regression advertises both an analog joystick
and DualShock to catch accidental use of the wrong device. PCSX remains default.

Real Slot worker boot/exit/restart also passes: the authored counter advances
to 4 then 5 across two full sessions, with the PCSX sentinel unchanged. Quick
Save, Quick Load and resume attempts produce no Swan state files. Log:
`.amp/in/ps1-alternative/authored-slot-lifecycle.log`.
Earlier failures below are historical evidence, not the current boot result.
The combined gate also requires broad regressions, a rebuilt executable and
independent package/source verification; see the candidate's receipts.
No SD or handheld action is part of this offline work. No commercial-game,
hardware compatibility, analog-stick coverage or speed conclusion follows.

## Historical failed boot gate and fixture diagnosis

`crates/slot/tests/native_software.rs` now provides an ignored, explicit real-core
gate, `swan_authored_disc_must_execute_gameplay_not_only_firmware_frames`.
It uses Slot's actual core loader, presets and state capability gate, the pinned
AArch64 runtime, the existing authored CUE/BIN and no proprietary/user BIOS.
It requires sustained authored red gameplay and the emulated card-write byte;
firmware frames or audio callbacks cannot satisfy the gameplay assertion.

The offline Linux ARM64 run failed: after 1200 frames the geometry was 640x448,
there were zero authored red frames and 1769340 audio samples. The card-write
assertion was not reached. This is a retained failing acceptance gate, not a
passing regression test. Log: `.amp/in/ps1-alternative/slot-real-boot-gate.log`.

Direct libretro controls also failed with the existing disc, a GPU-only disc,
and a GPU-only standalone EXE. The standalone EXE's instructions were present
in RAM, but no red frame appeared with either recompiler or interpreter. The
original CUE control produced 1197 software frames from 1200 calls and zero
hardware-context requests. An inspected capture was black. Nonzero audio
samples do not prove game audio because firmware can produce sound.

A separately generated ISO/MODE1 control failed its file lookup (invalid seek)
and is not counted as valid disc-boot evidence. Do not attribute all failures to
that control: the original MODE2 CUE and standalone EXE fail independently.
The precise OpenBIOS/core boot cause is not established; no speculative firmware,
renderer, BIOS patch or default change has been applied. Relevant direct logs
are `real-cue-v2.log`, `real-minimized.log` and `real-full-iso-interpreter.log`.

Real gameplay/input/game-audio/disc-switch/card-persistence acceptance remains
incomplete. The owner authorised SD update only after all milestones pass and
explicitly ruled out substituting the partial test.8 package. Therefore no new
combined Swan package, SD identification/write/eject operation or live profiling
was performed. The frozen PICO-label/PS1-rewind test.8 candidate is unchanged.

### Offline follow-up: disc boot reached; card acceptance still blocked

The original authored ISO omitted both ISO9660 path tables and the volume-set
and volume-sequence fields. PCSX HLE tolerated this. A separate diagnostic Swan
build exposed OpenBIOS's terminal `Unimplemented A0:A1` / `halting` output.
Instruction tracing with the corrected path tables reached `0x80010000` and
executed the authored instructions. This establishes a fixture defect, not a
production Swan renderer or CPU fix. `crates/slot-retro/tests/software.rs` now
generates the missing fields and little-/big-endian root path tables in sectors
18/19; the original program, assertions and core runtime are unchanged.

Controls through Slot's real loader, using the **unchanged baseline** Swan:

- Corrected disc with original card code: still zero red frames after 1200
  frames. The diagnostic interpreter reaches the game's BIOS card-open call
  after `InitCARD`; it does not reach the drawing code. Whether the fixture
  needs a fuller asynchronous card initialisation sequence or OpenBIOS lacks
  the required behavior is not established.
- Corrected disc with just the first 42 card-instruction words replaced by
  NOPs: ten authored red frames at 320x224. The unchanged acceptance test then
  correctly fails the card assertion (255 versus 1). This is disc boot and
  GPU execution evidence only, not card/input/game-audio acceptance.
- The bounded PCSX ARM64 core passes the original authored MIPS/card/state/
  multidisc test with the corrected fixture. The generated raw disc SHA-256
  matches the independent corrected control:
  `54cd91ea98746dcb041f90d50d14e72780515a5f6af87481a4d048ae1ecf5614`.
  The GPU-only control hash is
  `e22c3d92ad610a8008166c29c93e2e873d3edc41fde296714194b94c015d098b`.

Logs under `.amp/in/ps1-alternative/`: `trace-original-disc.log`,
`trace-path-table.log`, `baseline-complete-card.log`,
`baseline-complete-gpu.log`, and `fixture-arm-bounded-check.log`.
Do not interpret the earlier standalone EXE failure as disproving this disc
result: its upstream boot path applies a separate fixed-address BIOS patch.
No BIOS patch or replacement was made in this follow-up.

The missing card startup was resolved by adding `StartCARD` after `InitCARD`
while retaining BIOS interrupts. CREATE is not open-or-create on OpenBIOS;
the new acceptance program opens existing files first. An additional disc-test
defect called unimplemented B0:40 instead of CD-driver A0:64. Correcting those
fixture calls, not changing the emulator, produced the latest passing results.
Logs: `card-marker-gate.log`, `authored-pad-trace.log` (historical strict-colour
failure), and `authored-full-v2.log` under `.amp/in/ps1-alternative/`.
Baseline Swan and frozen test.8 hashes remain unchanged. Diagnostic
instrumentation is not a candidate runtime and is removed from the frontend.

## Owner-controlled same-scene A/B plan (not executed)

1. Back up the content root and record hashes. Use disposable copies only.
   Give each core an independent empty card; never reuse original cards/states.
2. Record exact frontend/core hashes, options, BIOS/disc hashes, display mode,
   power/clock/temperature settings and test scene/input sequence.
3. Start each core clean, reach the same scene through the same inputs or an
   independently created in-game save. Do not exchange emulator states.
4. Run PCSX/Swan/Swan/PCSX with equal warm-up and duration. Record frame-time
   distribution, sustained speed, audio faults, visual faults and temperature
   only after the owner authorises profiling on the target device.
5. Verify separate cards survive exit/reopen and PCSX hashes are unchanged by
   Swan runs. Reject missing-core, unsupported-content or save-read errors;
   never present a fallback run as Swan evidence.
6. Report results for that scene/device only. No general faster claim or
   release/default change follows from this experiment.
