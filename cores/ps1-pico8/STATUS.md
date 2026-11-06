# PS1/PICO-8 draft status — 6 November 2026

Test.9 is an owner-test milestone, **not hardware accepted or a stable release**.
The draft records existing work; it does not authorise another SD/device write,
live profiling, a release or a merge.

## Done and still limited

- Native PS1/PICO-8 shelves, bounded disc references, software frame geometry,
  controls, isolated persistence, GameBit and RomM native download validation.
- PCSX ReARMed remains default. SwanStation is explicit per-game selection,
  software/native 1x/PGXP off, with separate cards. Swan states, quick states,
  automatic resume and rewind are disabled; cards are not reconciled/imported.
- Authored Swan boot, input-controlled video/audio, silence after release,
  real disc reads 0 → 1 → 0 and card counters across core and full Slot
  exit/restart passed offline. Contract-library tests alone are not boot proof.
- fake-08 bounded build-specific state repair and staged-GC restore correction
  passed real-core and Slot lifecycle checks. This is not all-cart compatibility.
- Account-free local `.p8` label and `.p8.png` visible-label extraction, bounded
  input/decoding and explicit failures; preserve custom art and Nintendo fetches.
- Fresh/unset PS1 rewind defaults off; explicit legacy/global and per-PS1
  choices persist. PICO and Swan capability gates still refuse rewind.
- Stock volume stays firmware-owned with unity PCM gain; brightness uses live
  readback without startup replay. Existing preferences are retained.

## Owner feedback and next gates

- **PICO:** “still not work.” The failing cart/format/action is unknown: label
  fetch, launch, gameplay, exit, save or resume must be distinguished before
  changing a path. The earlier positive test.4 report does not resolve this.
- **Swan:** “a bit better,” reported 30–40 fps with choppy sound. This is a
  subjective improvement, not performance acceptance/full speed. The game,
  same-scene comparison and exact displayed metric are unconfirmed.
- No new live profiling was requested. Offline defaults inspection found no
  evidence that blind frameskip, overclocking, quality changes or larger audio
  buffers will repair a sustained frame-budget deficit. Slot has no libretro
  hardware-render context; a GPU renderer requires a separate architecture
  decision and verification, not an option toggle.
- Startup volume, decorative LEDs and full sleep/wake reliability remain
  unresolved. PS1 automatic artwork source/redistribution rights remain
  unresolved; manual PNG upload is retained. No accounts or mirror is added.

Owner-controlled A/B: use the same legal disc, region, scene, display/audio
settings and controller mode; record the exact core and settings; compare
normal-speed PCSX versus Swan from separate in-game card progress (never
cross-load states/cards). Observe sound, input and frame consistency for the
same interval. Record the HUD row/metric, not only a number. Exit and relaunch
each core to check its own card. Stop if saves or game behaviour differ.

## Frozen test.9 identity

The approved SD installation used verified affected-file backups, binary-last
replacement, independent candidate/preserved-file readback and normal safe
eject. Private backups, ROMs, BIOS, captures, packages and credentials stay local.
No SD operation is performed while preparing this draft.

| Item | SHA-256 |
| --- | --- |
| Test.9 FAT32 ZIP | `192a84579623e007e83f52173ebceeaf23ad2dd9c20d40cb9a211755ddcedff4` |
| Frontend executable | `0286b0055c67c8af34575a6f2c1930fa5c7921aa0e60fb21a69d29a4637b22ff` |
| Frontend source archive | `81d12d1c98467281da5f7f0d9a4a96d3b7956c6608bf00fc3c7991c718720d3f` |
| Swan runtime | `3b5d66f649aa1a1d53feaf102a14d6787ae337f758199d89cfd42ba0f9da94a8` |

The package was built from an **uncommitted snapshot** at checkpoint
`226de404bbac07b35a53edfa54f37c18f77cea0b`, against fork baseline
`7d14cc6c9b4017a5dc2739c5ea54279a70f7a14b`. The snapshot/manifest hashes,
not that checkpoint alone, identify the shipped source. The initial draft
commits organise that work and add current status documentation; a later
review correction is listed below. Neither rebuilds or changes the frozen
package. See [pinned inputs](README.md#pinned-inputs),
[Swan source/licences](SWANSTATION.md) and [notices](NOTICE.md).

### Draft-only review correction — not installed

Code review found that local labels selected the decoder by filename, while
RomM and fake-08 accept PNG cartridges named `.p8`. An authored real-worker
regression failed with `Encoded.P8: Malformed`, although the same PNG worked
as `.p8.png`. Local labels now detect the PNG signature after the existing
filename and size checks, then use the same bounded PNG decoder. The worker
checks exact cropped pixels, full output naming and preserved custom labels.
This change is not in test.9 and is not a diagnosis of the owner's unspecified
PICO symptom. No new package or SD update was made.

Swan revision `631f0fb31b809e0f889d7c551fe695788e39c1d3` retains GPL v3
and separate dependency notices, including MIT PCSX-Redux OpenBIOS replacement
firmware. No Sony BIOS is bundled. The exact upstream commit of the embedded
OpenBIOS binary is unverified; the pinned Swan tree retains its bytes, generated
header, licence and build inputs. No firmware-origin claim beyond that is made.

## Recorded verification and known non-green checks

Prior frozen-candidate receipts record 366 scoped Linux ARM64 checks, 58 host
libretro checks, 52 store checks and the real authored Swan acceptance run.
The offline Linux stock build passed (frontend maximum GLIBC 2.30; Swan 2.29).
These results do not establish TSP game speed or the new owner PICO symptom.

The full ARM64 suite failed
`fast_forward_runs_the_chosen_number_of_core_frames_per_present` twice; the
scoped run explicitly excluded it. The frozen test.8 and test.9 emulation
sources match for that follow-up; baseline timing failures are retained, not
silenced or asserted to be fixed. A Mac stock-only build cannot resolve the
Linux-only `stock_dhcp::System`; the Linux stock build passes. Broad strict
Clippy also reports an existing single-element loop in `render_quick_menu`.
There is no all-green workspace claim. Current draft-preparation check results
are listed separately in the PR, not backdated into package receipts.
