# PS1 and PICO-8 core builds

The `stock-tsp-0.3.0-test.9` follow-up combines these cores with native shelves,
controller UI, disc handling, isolated save directories and RomM download
routing. Host checks and a combined FAT32 test package are separate from
handheld acceptance, which remains pending. See [the test checklist](TEST-CHECKLIST.md).

**Latest status:** test.9 was installed with separate owner approval, verified
backup/readback and normal safe eject. The owner now reports PICO still does
not work (exact failing action unknown), and Swan is a bit better but at
30–40 fps with choppy audio (game/scene/metric unconfirmed). Neither platform
has full hardware acceptance. See [current status and provenance](STATUS.md).
This draft work makes no new SD/device change or stable release.

The earlier report that PICO-8 and brightness were good after test.4 was scoped,
not acceptance of every cart, save-state path, stock OSD or sleep/wake.
Test.6 keeps those core/brightness fixes and corrects GameBit artwork:
PS1 uses an original neutral square sleeve; PICO-8 uses a portrait virtual
card at the 160:205 cart format ratio. Supplied labels fit without cropping.
Defaults contain platform/title text and simple geometry, not borrowed logos
or game artwork. Native shelf rendering and user bezel files remain unchanged.
The native Default bezels now use original coordinated geometric side panels:
grey vents/rings for PS1 and muted teal/coral pixel geometry for PICO-8. They
are generated under Slot's licence, with no downloaded art, marks or textures.
None and custom PNG remain selectable. Native bezel textures use the actual
panel size, avoiding a 720×480 cover-crop before display on a 1280×720 panel.

## Test.9: optional SwanStation offline comparison milestone

Test.9 retains local PICO-8 labels and PS1 rewind preferences from test.8 and
adds the opt-in software-renderer Swan core. PCSX remains the default; no saves
are imported between cores. Swan states, quick states and automatic resume
are disabled to prevent snapshot restoration from replacing newer card data.
The core contains licensed MIT OpenBIOS replacement firmware, not a Sony BIOS.

Offline authored-game checks pass boot, input-controlled video/audio, real
disc reads and card progress across core and full Slot exit/restart cycles.
Core-advertised DualShock IDs are used instead of assuming PCSX's subclass ID.
These checks do not establish commercial-game compatibility, analog coverage,
handheld speed or a Bloody Roar II performance fix. See [the evidence](SWANSTATION.md).
The frozen partial test.8 ZIP is unchanged. No SD update or publication is
authorised by this candidate alone; the later test.9 installation had separate
approval. No further installation is part of this draft.

## Test.8 local candidate: embedded labels and PS1 rewind preference

Fetch Missing Labels, Cart Studio and the shared fetch-label endpoint extract
local PICO-8 `__label__` pixels or standard `.p8.png` visible labels without an
account, network or code execution. Existing labels are preserved. Nintendo
catalogue fetching is unchanged; PS1 artwork still uses manual PNG upload.
Inputs over 2 MiB, malformed labels and unsupported PNG layouts fail explicitly.

PS1 rewind defaults off when unset, independently of Nintendo. An explicit
legacy global rewind setting is honoured until PS1 is changed separately;
the new `rewind_ps` preference persists across PS1 core selection. PICO-8 and
unsupported-core capability gates remain in effect. This is not a speed claim.

The frozen test.8 candidate omitted the optional Swan runtime: its authored
gameplay gate had not yet passed. Test.9 supersedes that gate as described
above; states/resume remain disabled because restoration can replace newer
memory-card data. See [SwanStation evidence and gates](SWANSTATION.md).
No SD update, handheld acceptance or publication follows from this host build.

## Test.7: bezel collections, art coverage and diagnostics

GameBit lists an empty PS1/PICO-8 bezel collection before its first upload,
rather than returning 404. Upload, download and handheld bezel selection use
the existing paths; unknown paths and missing individual files still fail.

Handheld Fetch Missing Labels now states the current server's GB/GBC/GBA
coverage and directs PS1/PICO-8 users to PNG upload in GameBit. This is a
provider limitation, not a new native download feature. RomM installation
uses that same label provider, not RomM cover media. Native label filenames
retain the complete game filename, including `.chd`, `.p8` or `.p8.png`.
No additional art provider or copyrighted assets are bundled.

In a running game, open Settings → Gameplay → Performance HUD. It is off by
default, is not saved, and measures only normal-speed play with menus closed.
The one-second window reports emulated core steps, fresh worker frames
uploaded before a successful swap, and successful presentation swaps per
second. Fresh is not a count of unique images; swaps are not physical scanout
or proof of a game's internal animation rate. Device late counts are the
existing pacer's >1.5-period estimates; host builds report late n/a.

Core, rewind serialization/queue wait, frame copy, and audio processing/ring
wait show mean wall milliseconds per core step. Rewind compression runs on a
separate worker and is not included in that row. Main work/swap are mean
milliseconds per presentation. These overlapping thread timings must not be
summed. Audio gaps/drop count interleaved i16 samples per second, not stereo
frames or callbacks. The HUD also logs one `slot: perf:` line per window.

Owner-confirmed Bloody Roar II USB capture verified the deployed test.6
executable and active PCSX GPU thread. In one 5.29-second sample, main used
82%, emulation 62%, rewind 57% and GPU 42% of one CPU. This is process CPU
evidence, not measured FPS or an isolated cause. Bloody Roar II remains slow
on hardware; no quality loss, rewind change or speculative tuning is applied.
Test.7 is a local diagnostic candidate; SD installation needs separate approval.

## PS1 performance follow-up

The installed core already contains ARM64 Ari64 and the NEON GPU. The pinned
[MinUI TSP recipe](https://github.com/shauninman/MinUI/blob/dbf89435a5179788418ff3573042cf9c7e75d9e0/workspace/tg5040/cores/patches/pcsx_rearmed.patch)
supports those existing choices; no core rebuild or compiler-flag experiment
is included. Slot now supplies `pcsx_rearmed_gpu_thread_rendering=auto`, the
[upstream default](https://github.com/libretro/pcsx_rearmed/blob/7b0e8f8fadfbce3d170d195b9d9eecaabfa08c8e/frontend/libretro_core_options.h#L349-L365).
PCSX enables its GPU worker when it detects at least two CPUs. Previously the
missing option left its zero-initialised setting off. Only this PS1 preset
changes; no generic core defaults, CPU timing, rewind, dithering, interlace,
frameskip, resolution, firmware, clock or voltage changes are included.

Exact-core Linux ARM64 checks confirm the GPU worker starts and compare saved
scenes with threading off/on. This is a supported default correction, not a
demonstrated cure for Bloody Roar II audio starvation or a full-speed claim.
Host timing is not TG5040 timing. The owner must compare the same combat scene
and Metal Slug X with unchanged settings. See `VERIFICATION.json` for measured
results and limits, including baseline save-state replay differences.

This revision adds visible RomM transfer bytes, server totals/percentages when
known and an explicit unknown-total state without an ETA. Installing, Installed,
failed and cancelled states remain distinct; B cancels, A retries. Progress is
coalesced at most ten times a second. PNG-encoded PICO-8 carts named `.p8` now
pass the same complete 160×205 PNG validation as `.p8.png`; text carts retain
their header/version checks. Empty files, HTML and invalid PNGs are rejected.
Container validation is not a guarantee of fake-08 gameplay compatibility.

## Pinned inputs

| Core | Source revision | Required submodule |
| --- | --- | --- |
| [fake-08](https://github.com/jtothebell/fake-08) | `814991a2571ad3970e386cef48f3b148aa1c27b9` | `libs/z8lua` at `e6928578d46b61fd5ea30cfcf547e855a30a0553` |
| [PCSX ReARMed](https://github.com/libretro/pcsx_rearmed) | `7b0e8f8fadfbce3d170d195b9d9eecaabfa08c8e` | `frontend/libpicofe` at `a2697389e76744cfa0c56c13a172845c45ede719` |

`build.sh` verifies these revisions, exports clean Git archives into a new work
directory, builds the library and writes its compiler/flags/SHA256 to a sibling
`.meta` file. The source checkouts are not modified. It does not download cores,
install files, or change a device.

```text
sh cores/ps1-pico8/build.sh fake08 host SOURCE NEW_WORKDIR OUT
sh cores/ps1-pico8/build.sh pcsx_rearmed host SOURCE NEW_WORKDIR OUT
```

Use absolute source, work and output paths. The work directory must not exist.
The host profile supports macOS and Linux. PCSX host builds use its interpreter;
they do not verify the target dynarec.

The `tg5040` profile requires AArch64 Linux and builds for Cortex-A53. PCSX uses
Ari64, the software NEON renderer and CHD support; physical CD-ROM support is
disabled. The candidate uses the Rust 1.98.0 AArch64 Debian image
`rust@sha256:4730e387a220a08a365c77da3096544dde214f9d796c16284d4be45438cad4a9`.
PCSX requires at most GLIBC 2.17, fake-08 2.29 and the frontend 2.30,
below the captured stock 2.33. fake-08 also needs GLIBCXX 3.4.26 and CXXABI
1.3.9. Saved stock metadata identifies `libstdc++.so.6.0.28`, whose standard
[GCC ABI contract](https://gcc.gnu.org/onlinedocs/libstdc++/manual/abi.html)
covers those versions; this is not a fresh target symbol/load test.
ELF dependencies are recorded in verification. These checks do not prove
target performance or driver behavior.
No GPU driver or firmware substitution is required by these software recipes.

## Runtime contracts and limits

- **fake-08:** native 128×128 video, cartdata and the pinned patch's genuine
  quick states/resume. States are exact-build-specific; legacy and cross-build
  imports are unsupported. Rewind remains disabled independently. It is not the
  official PICO-8 runtime. The stream patch keeps save handles independent of
  the frontend diagnostic stream. See the repair limits below.
- **PCSX:** use native video, libretro-managed first memory card and controller
  selection. DualShock requires both independent signed sticks; four face
  buttons, D-pad, Start/Select and L1/R1/L2/R2 remain distinct. The TSP has no
  physical L3/R3; do not assign a substitute silently. Enhanced resolution is
  outside the initial configuration. Use the core's reported geometry, aspect,
  frame rate and sample rate rather than Game Boy display timing.
- **BIOS:** user-provided PS1 BIOS files belong under `Slot/BIOS/`; recognised
  names include `scph5500.bin`, `scph5501.bin` and `scph5502.bin`. HLE is a
  separate mode, not evidence of real-BIOS compatibility. No BIOS is bundled.
- **Content:** local PS1 formats are CUE with its tracks, CHD,
  PBP and bounded M3U playlists. All four boot and persist a memory card through
  fresh real-core Slot sessions using authored HLE fixtures. This does not
  establish compatibility with every disc or real BIOS. RomM validates single-file CHD/PBP and PICO-8 `.p8`/`.p8.png`
  downloads, not multi-file disc sets. Completed downloads enter their native
  shelves; launching requires the matching library. FAT32 files must be smaller
  than 4 GiB.

## Tests

`crates/slot-retro/tests/software.rs` contains an authored C ABI fixture and
optional real-core tests. Set `SLOT_FAKE08_CORE` and `SLOT_PS1_CORE` to the built
libraries. Run the real-core contracts, including the repaired state gates:

```sh
cargo +1.96 test -p slot-retro --test software -- --include-ignored --test-threads=1
```

Run serially because libretro cores share process-global callback state. Without
the two environment variables, run the normal suite, which ignores real-core
tests explicitly. The fixtures are authored text-cart and MIPS executable data;
no commercial ROM, BIOS or third-party demo cart is needed.

The tests check fake-08 video/input/audio, state rollback/fresh reopen, explicit
bad-state rejection and cartdata. PCSX HLE executes authored MIPS code that
uses the emulated BIOS card file API to read/increment/write progress; a fresh
core observes the next value. `crates/slot/tests/native_software.rs` additionally
boots text/PNG carts and CUE/M3U/CHD/PBP through actual Slot sessions, flushes
saves, then reopens them and checks progress. Set `SLOT_SOFTWARE_FIXTURE_DIR`
when running the low-level test to export authored CUE/M3U/BIN fixtures;
`test-pbp.py` builds only that fixture's PBP and `chdman createcd` creates its CHD.
No commercial disc or BIOS is used. Controller ABI tests are not physical
control or sustained TG5040 performance evidence.

`pcsx-slot-boundaries.patch` bounds state stream reads/writes/seeks, rejects a
wrong state size and resolves CUE tracks beside their descriptor, not the
process working directory. Regression probes cover malformed/truncated states
and a conflicting same-name track. This is not a full malicious-state audit.

## PICO-8 quick-state repair

The owner approved `fake08-slot-state.patch`, superseding the cartdata-only
milestone. The original core failed authored Lua rollback despite reporting
success, and Slot's original capability gate correctly prevented false resume.
The repaired build passes five low-level contracts and two actual Slot Session
contracts on macOS and pinned ARM64 Linux. These are host tests, not handheld
or arbitrary-cartridge acceptance. Frozen test.2 is unchanged.

The patch initializes Eris with deterministic permanent function names, saves
full globals/string metatable and restores Lua into an independent interpreter.
It includes closures, shared tables, added/deleted globals and coroutine stacks;
the sandbox registry is rebound instead of falling back to stale gameplay.
Missing text-cart ROM sections are initialized in the memory-loading path.
The cartridge is loaded before an immediate frontend resume request.

A fixed-size versioned container checks its exact build fingerprint, cart
identity, runtime size, Lua length and checksum. Lua output is bounded to 1 MiB;
staged Lua allocation is bounded to 32 MiB and Eris nesting to 256. Parse failure
returns an error without replacing live Lua/gameplay. The runtime image covers
RAM, all audio channels, VM/pause state, input-repeat/mouse/keyboard state,
timing/frame parity and a restorable noise PRNG. Noise differs from the old
libc-rand sequence; deterministic continuation is tested sample-for-sample.

Save failure is explicit for oversized/unsupported state, including custom
Eris persistence callbacks and finalizers. This is not an audit for malicious
state imports: do not import foreign or edited snapshots. States are not
portable across builds, compilers or platforms. Cartdata remains separate.
Slot retains refused resume files for recovery and does not overwrite them
with a failed restore. Quick save/resume are enabled; rewind stays disabled.

The authored tests compare rendered gameplay and full audio continuation,
including held-input repeats, PRNG and VM time; reject checksum damage,
truncation/oversize and an invalid Lua stream with a valid outer checksum;
and show gameplay/prior good saves still work after failures. Actual Slot
tests create a manual state and resume, reopen and verify gameplay, and
preserve text/PNG cartdata after an invalid resume. Run them serially:

```sh
cargo +1.96 test -p slot-retro --test software fake08 -- --ignored --test-threads=1
cargo +1.96 test -p slot --test native_software -- --ignored --skip ps1 --test-threads=1
```

Set `SLOT_FAKE08_CORE` to the patched library. Do not use an older fake-08
binary with the newly enabled frontend state controls. Combined packaging and
manifest checks remain mandatory before the approved offline SD update.

### Test.3 crash follow-up — corrected in test.4

The owner reported a crash around **Exit Game**. Preserved card logs identify
`bubblegum_spin-4.p8`: exit completed, but its next resume aborted with
`malloc(): unaligned tcache chunk detected`. The affected cart and an authored
nested-function cart reproduce a heap use-after-free in Eris `u_proto` under
AddressSanitizer. Garbage collection could reclaim partially restored function
prototypes before Eris completed their links.

The follow-up pauses collection only in the independent staged interpreter
while its object graph is incomplete, then restarts it before gameplay. The
32 MiB allocator bound and error/rollback path remain in effect. The authored
Slot regression crashes with the old core and passes with this correction on
macOS; corrected macOS and pinned ARM64 runs cover manual save/load, fresh
resume and three Exit Game/shelf/reinsert cycles. Cartdata and invalid-state
preservation checks also pass. This is not a handheld acceptance claim.

The core build fingerprint changes. Test.3 states are not imported into the
new build; refused snapshots remain available for recovery and independent
cartdata remains separate. Test.3 ZIP/source/backups are unchanged. Test.4 was
installed with backup, independent readback and safe eject before owner testing.

Test.4 also enables stock LCD brightness through `/dev/disp` GET/SET ioctls,
with live readback and no startup replay or saved software dim. Knulli/nonstock
controls and stock-owned volume are unchanged. Open/read failures show
Unavailable. Host UI/ARM64 ioctl-fixture checks do not establish physical
brightness, stock OSD/persistence or suspend acceptance.

Bloody Roar II audio/performance remains unresolved on the handheld. The
captured card has rewind enabled and no files in `Slot/BIOS`, so the configured
automatic BIOS selection has no user BIOS available. A read-only, Mac-hosted
ARM64 probe of the saved scenes measures roughly six times more core CPU work
for Bloody Roar II than Metal Slug X. This is not a TG5040 benchmark and does
not isolate presentation, audio pacing or rewind compression. No speculative
clock, frameskip, BIOS download or global rewind change is included.

The test.5 investigation used the installed ARM64 core and captured scenes in
an offline Mac-hosted Linux VM, with an optimised frontend. It measured frame
publication, serialisation, XOR/LZ4 compression and the actual rewind queue.
Both scenes produced the expected audio sample count. A real emulator-worker
comparison with a simulated 48 kHz consumer ran near 60 fps with zero steady
underruns or overruns in both rewind modes. This did not reproduce the severe
handheld starvation. Rewind overhead remains a hypothesis, not a demonstrated
cause. No PS1 performance correction or full-speed promise is included; target
stage timings are still needed under separately approved device-test scope.

MIT GermanTacos and CC0 SkyWalker bezel sources were inspected. Both need
aspect-preserving recomposition for 1280x720; their licences do not clear Sony
or Lexaloffle marks. Imported or original-neutral design selection is separate
from this candidate. No artwork is silently replaced.

## Redistribution gate

The test ZIP includes [notices](NOTICE.md), licence texts, exact core/submodule
sources, frontend snapshot and build materials under `Slot/Sources/`.
`package-test.py` verifies the frozen baseline, refuses to replace an existing
output directory and checks every ZIP member against its assembled payload.

- Preserve fake-08's pinned `LICENSE.MD`, copyright notices and dependency
  licences, including z8lua, LodePNG and miniz. Its licence file also attributes
  the oval implementation to [Michael Dorgan's answer](https://stackoverflow.com/a/8448181)
  under [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/).
  Retain that attribution and the applicable licence terms; do not describe
  every bundled source file as MIT.
- Preserve PCSX's GPL notice, `COPYING`, and applicable libpicofe/libchdr and
  compiled dependency notices. Include the exact corresponding core and
  submodule sources, local patch and build materials for the shipped binaries.
- Exclude vendor firmware, BIOS, commercial ROMs, third-party demo carts,
  personal settings, logs and credentials from the install package.
- Produce a final source revision, build provenance, per-file hashes and ZIP
  checksums. Keep the stable 0.2.0 tag and assets unchanged. Publication does not
  establish handheld acceptance; the owner tests the combined ZIP afterward.
