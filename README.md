# Slot for TrimUI Smart Pro

**Native frontend for TG5040 on internal stock Tina Linux.** [jellydn/slot-tsp](https://github.com/jellydn/slot-tsp) is an independent [Slot](https://github.com/BrandonKowalski/slot) fork. Other TrimUI models are unverified.

Install stock packages by **copying content to an existing FAT32 card**, not flashing an OS image. Stock kernel, drivers and services stay in place.

## Verified status — 5 November 2026

Stable stock package: **[stock-tsp-0.2.0](https://github.com/jellydn/slot-tsp/releases/tag/stock-tsp-0.2.0)** for stock firmware **1.1.1, UI 20251128, kernel 4.9.191**. Download `slot-tsp-stock-0.2.0-fat32.zip` and `SHA256SUMS`. Read the [release notes](knulli/RELEASE-STOCK-0.2.0.md) before installing. This combines built-in RomM and the Wi-Fi wake stale-busy correction. **The rebuilt combined executable is host-verified; hardware acceptance is pending.** “Stable” identifies the release channel, not a hardware pass. The earlier [0.1.0 release](https://github.com/jellydn/slot-tsp/releases/tag/stock-tsp-0.1.0) remains available.

| Evidence level | Status |
| --- | --- |
| Historical owner evidence — 0.1.0 | Direct boot, corrected A/B, scan, Tools, real Restart/Shutdown and selected SD splash; first-connect report “Wifi good now.” Physical Power off/on later exposed a stale Network In Use error. These reports do not establish acceptance of the new 0.2.0 executable. |
| Combined 0.2.0 | 616 pinned Linux test executions, startup mocks, ARM64 build/ABI and 229 clean host integration tests passed. No handheld acceptance of combined RomM or Wi-Fi wake correction yet. Full-suite exceptions are disclosed in the release notes. |
| Host-checked baseline | Mount/write boundaries, input/panel mapping, EGL cleanup, dispatcher/fallback and native power paths. Not proof of hardware gameplay, save durability or suspend/resume. |
| Included controls | **Device → Restart / Shutdown: select a row, A activates, B returns.** X/Y do not activate power actions. Save/flush/settle and visible failure handling remain in place. Real power actions passed on the earlier build; the current A-select mapping is host-tested, not a separate new hardware pass. |
| Wi-Fi scope | Explicit Connect owns one bounded wlan0 DHCP client after association; Disconnect preserves profiles. Competing clients still block takeover. After stock takes over, fresh association plus valid IPv4 can clear only the stale busy indicator without adopting its client. No Internet, repeated cold-boot/reconnect/wake reliability or persistence claim. |
| Unresolved | Physical LED-off, Fn and real sleep/wake (white/gray failure). Early internal artwork/LED flash can remain before the SD splash. |

**Current hardware limitation:** the owner still reports **Network In Use** after
physical Power off/on. A later read-only USB capture identifies the running
executable as the older `stock-tsp-0.2.0-rc.1` RomM preview, before the Wi-Fi
handoff patch. That identifies the failing build; it does not establish a
hardware pass for the patched build or explain every wake failure. Wi-Fi wake
remains unresolved on hardware. Current source already includes the existing
correction; the preview/current-source regression fails before it and passes
after it. No additional Wi-Fi patch is supported by this capture. A corrected
candidate still needs physical Power-button testing.

LED controls and stock **Lid Power Off** show **Unavailable**. Slot Fn actions are disabled; physical lights and stock Fn policy are not changed. The TG5040 has no lid; no inactivity timer is added.

Grout is not desired or required; historical proposals are not install advice, and removal of an existing installation is unconfirmed.

Details: [stock notes](knulli/STOCK-TG5040.md) and [frontend design](knulli/stock-frontend/README.md). These retain older candidate-stage records; they are not blanket hardware passes.

## Stock install and card layout

Use the stock FAT32 ZIP above. It contains the exact host-verified combined executable with unchanged baseline cores and startup adapters. It is not the owner-tested 0.1.0 binary. Build labels, hashes, source integration and pending hardware acceptance are disclosed in provenance. **Do not use the Knulli ZIP or a BaseOS/unslot image for stock installation.** No firmware, internal boot-logo writer, ROM, BIOS, personal settings/logs, pairing credentials or Grout runtime is bundled.

1. Power off normally. Back up the intended FAT32 card; no format is needed.
2. Verify `SHA256SUMS`, extract the ZIP on the host, then copy its `Slot`, `Apps`, `System` and `trimui` directories to the card root. Do not copy an enclosing ZIP-name folder. Preserve games, BIOS, saves, states, settings and splash. Back up and review an existing `trimui` tree; stop if an SD updater or a different frontend would conflict. Do not merge unknown startup scripts blindly.
3. Safely eject. On-device, the card is `/mnt/SDCARD`. The complete matched `trimui` adapters start Slot after stock services are ready. **Slot Stock Test** in Apps is an alternate entry. The early `System/starts/slot.sh` hook alone is insufficient.

For Wi-Fi, open a saved network and select **Connect → A**. An associated network without an IP also offers Connect; no Disconnect/Power recovery should be needed first. This explicitly selects the profile and starts one owned DHCP client with scoped wlan0 address/default-route and temporary DNS updates. Opening the page alone starts no DHCP. Expect Getting IP → Connected with IPv4. If Network In Use appears, stop rather than restarting stock clients. Disconnect uses A and retains the saved profile. B returns.

| Card-relative path | Purpose |
| --- | --- |
| `Slot/System/` | `slot` executable and matching `*_libretro.so` cores |
| `Apps/SlotStockTest/` | `launch.sh` and `config.json` |
| `trimui/` | Complete direct-boot adapters and runtime pins, if supplied |
| `Slot/BIOS/` | User BIOS; GBA path is `gba_bios.bin` |
| `Slot/Saves/`, `States/`, `Config/` | Persistent data below `Slot/`; preserve on update |
| `Slot/Logs/` | Diagnostics; check for private data before sharing |
| `Slot/Config/bootlogo.bmp` | SD splash override of `Slot/System/bootlogo.bmp` |

Use an uncompressed 24-bit BMP; invalid splash overrides are skipped. This does not replace internal boot artwork. Keep the card inserted and unchanged during use: native cores are not sandboxed, and interrupted writes can lose data. ROMs and BIOS are user-supplied, not bundled.

### Offline recovery

With the device powered off and the card on a host:

1. Create empty `Slot/Config/stock-menu-only` to bypass Slot in the launch hook/dispatcher.
2. To disable the SD frontend branch, back up and rename the **whole card-root `trimui` directory**. The matched setup then uses internal startup.
3. If an update failed, restore the prior verified card files.

These steps do not recover a running driver hang; a conflicting updater can bypass this route. Do not alter internal firmware, boot logos, services or unknown files. Sleep/wake is unreliable: use normal save/exit and power controls, and never remove the card during a run.

## Games today

**The frozen 0.2.0 package has GB/GBC/GBA only.** ROMs must be unzipped; `.zip`/`.7z` are unsupported. These systems use a 240 × 160 canvas, with GB/GBC centered inside it. The separate two-core test candidate is described below.

| System | Folder below `Slot/` | Core choices in source |
| --- | --- | --- |
| Game Boy | `Games/GB` | mGBA, Gambatte |
| Game Boy Color | `Games/GBC` | mGBA, Gambatte |
| Game Boy Advance | `Games/GBA` | mGBA, gpSP |

`Games/GBAV` is a GBA Video category, not another system. Core availability depends on the package; game/save/link compatibility is not fully hardware-verified.

## Built-in RomM client

Included in **[stock-tsp-0.2.0](https://github.com/jellydn/slot-tsp/releases/tag/stock-tsp-0.2.0)**.
Download `slot-tsp-stock-0.2.0-fat32.zip` and verify `SHA256SUMS`.
Read the [release notes](knulli/RELEASE-STOCK-0.2.0.md). This rebuilt executable
is host-checked, not handheld-tested. The earlier RomM-only RC remains available
as history; it does not contain the Wi-Fi wake correction.

Source builds include RomM in both `device` (Knulli) and `stock-tg5040`.
For desktop testing, add `--features romm`. This is Slot code, not Grout or
an app in `Apps/`. Existing release ZIPs do not gain this feature.

1. Connect the handheld to Wi-Fi. Open **Network → RomM**.
2. Set Protocol, Server and Port for your RomM server. Use its LAN address,
   for example Server `192.168.1.21`, Protocol `HTTP`, Port `8080`.
   `localhost` and loopback addresses refer to the handheld and are rejected
   for pairing. A blank port uses the protocol's default, not 8080.
3. Select **Pair** and approve the code on your phone. Slot saves the server
   and pairing in `Config/romm.conf` below the Slot root. Keep that file private.
   Approve both ROM and platform read access. If paired with an older build,
   Disconnect and pair again to grant the new `platforms.read` permission.
4. Open **Library**. Left/Right selects a system. Up/Down selects a
   game; A downloads it. Each page has up to 20 games. Use Next page or
   Previous page for more games; A on the heading reloads the page.
5. B cancels a download or returns to the RomM menu. The shelf refreshes
   when you leave the library. Downloads use `Games/GB`, `Games/GBC` or
   `Games/GBA` below the same Slot root on either OS.

Only individual ROMs and ZIPs with exactly one playable ROM are supported.
ZIPs are unpacked before installation; neither ZIP nor 7z archives are left
on the shelf. 7z and multi-file entries show **Unsupported**. Failed or
cancelled downloads remove temporary files; an existing game is not replaced.
Library errors allow retry. Remote receive polling pauses while the library
is open. Box art, save sync and collections are not part of this client.

**Unreleased source change:** the library selector also offers PS1 and PICO-8.
PS1 downloads accept single-file `.chd` and `.pbp` images into `Games/PS`;
PICO-8 downloads accept `.p8` text carts and `.p8.png` cart images into
`Games/PICO8`. ZIPs must contain exactly one accepted game file. Downloads
check file headers (and decode PNGs at the cart's 160×205 dimensions); this
does not prove that a game runs. Files larger than FAT32's 4-GiB-minus-1-byte
limit are rejected. CUE/BIN and M3U sets remain unsupported by the downloader:
it must not install a descriptor without its tracks or discs.

RomM's `psx`/`ps` platform names map to PS1, and `pico`/`pico8`/`pico-8` map
to PICO-8. `sega-pico` is a different system and is not mapped. In the development
source, completed downloads enter their native shelves. Launch requires the
matching PCSX ReARMed or fake-08 library; a missing native core reports an error,
not a mock game. The published stock 0.2.0 package still offers only GB/GBC/GBA.

These are source capabilities, not a hardware verification claim. Stock
Wi-Fi must have a working IP connection before browsing.

## New systems — development status

[Issue #2](https://github.com/jellydn/slot-tsp/issues/2) covers PICO-8, Neo Geo, Mega Drive, arcade/FBNeo/MAME, PS1, N64, NDS and Dreamcast. PS1 and PICO-8 are the first development milestones; the other systems are not implemented. The [implementation plan](knulli/ISSUE-2-PLAN.md) records the feasibility and acceptance gates.

The combined `stock-tsp-0.3.0-test.9` follow-up adds PCSX ReARMed and fake-08,
including the current fork's upstream sync, RomM and Wi-Fi handoff correction.
It is a test build, not a stable update or a handheld acceptance claim. The
stable 0.2.0 downloads are unchanged. Read the
[combined test checklist](cores/ps1-pico8/TEST-CHECKLIST.md) and
[core build and licence notes](cores/ps1-pico8/README.md).

**Test.9 is not hardware accepted.** It includes the PICO-8 exit/resume repair,
local labels, PS1 rewind defaults and optional SwanStation software comparison
core. The owner reports PICO still does not work, without identifying whether
the failure is labels, launch, exit or persistence. Swan is subjectively a bit
better but reported at 30–40 fps with choppy audio; the game, scene and metric
are not confirmed. This is not full-speed acceptance. See the
[latest status and provenance](cores/ps1-pico8/STATUS.md) and historical
[crash follow-up](cores/ps1-pico8/README.md#test3-crash-follow-up--local-source-not-deployed).
Keep existing states/backups; cross-build quick states are unsupported.

RomM downloads show received bytes and a total/percentage when the HTTP server
supplies a length. Unknown-length transfers show bytes without an invented
percentage or ETA. Installation, completion and errors are separate; B cancels
and A retries. PICO-8 validation inspects the content, so PNG-encoded carts
named `.p8` work too. Empty files, HTML responses and malformed/wrong-size PNGs
are rejected. A valid container does not guarantee fake-08 game compatibility.

| Test system | Folder below `Slot/` | Content / save scope |
| --- | --- | --- |
| PlayStation / PCSX ReARMed | `Games/PS` | CUE with sibling tracks, CHD, PBP, bounded M3U; first memory card, core-specific states and disc selection |
| PlayStation / optional SwanStation | `Games/PS` | Software comparison only; isolated cards; states, quick states, automatic resume and rewind disabled |
| PICO-8 / fake-08 | `Games/PICO8` | `.p8` and `.p8.png`; cart data, patched build-specific quick states and resume; no rewind |

PS1 uses software NEON video and the AArch64 Ari64 dynarec. User BIOS files go
under `Slot/BIOS/` (for example `scph5501.bin`); none are supplied. Authored HLE
fixtures pass on the host, not real-BIOS game compatibility or TG5040 speed.
Choose **Digital** or **DualShock** in the game's settings. Both sticks and
four face buttons are independent; the existing mapper remains available.

Read-only TG5040 input capture confirmed left X/Y, right
RX/RY, and Z/RZ triggers. The owner confirmed the current left-stick digital
navigation works and that the TSP has **no physical L3/R3**. These controls
remain unsupported; no substitute click mapping is assigned. Games that
require L3/R3 will need an agreed alternate binding. Keep the existing button
mapper and stock A/B normalization; analog gameplay is a separate test gate.
fake-08 supports cart data and the pinned patch's quick states/resume. States
are tied to the exact core build, not portable or legacy imports. Unsupported
or oversized Lua snapshots fail explicitly. Rewind stays disabled. This is not
the official PICO-8 runtime or a guarantee for every cartridge.

### Art, bezels and GameBit — local follow-up

The source now exposes PS1 and PICO-8 in the existing GameBit web manager.
Game art uses the existing cartridge presentation and label upload path, not
a new art service. Use `Labels/PS/<complete game filename>.png` or
`Labels/PICO8/<complete game filename>.png`; GameBit's game page can upload
the label. No commercial game art is bundled.

**Local PICO-8 labels (development source):** Fetch Missing Labels reads the
optional 128×128 `__label__` in `.p8` files or the visible label in standard
160×205 `.p8.png` cartridges. Cart Studio and the shared fetch-label endpoint
use the same local path. No account, Internet connection or cart execution is
needed. Existing/custom labels are kept. Both printer styles use the embedded
image; absent, malformed, oversized (over 2 MiB) or unsupported carts are not
guessed. Nintendo catalogue fetching is unchanged; PS1 art remains manual.
This is not permission to redistribute cartridge images or game artwork.

**PS1 rewind default (development source):** fresh/unset PS1 settings default
to off for both PCSX and the optional Swan comparison path. An existing explicit
global `rewind=1` or `rewind=0` is preserved until PS1 is changed separately;
`rewind_ps` then stores that PS1 choice without changing Nintendo settings.
PICO-8 rewind and cores without save-state support remain unavailable. This
reduces optional recording overhead, not a verified Bloody Roar II speed fix.

Each native system has an independent **Bezel** setting: None, Default or a
custom PNG under `Bezels/PS/` or `Bezels/PICO8/`. Default is a procedural dark
border. Native borders sit behind the complete game image, retain its reported
aspect and fill only unused space. GameBit's Bezel tab manages those custom
files. These changes and the serialization repair are not in the preserved
test.2 ZIP. See the [repair notes](cores/ps1-pico8/README.md#pico-8-quick-state-repair)
for limits and real-core/Slot tests. Test.9 was installed with separate owner
approval and verified backup/readback; this draft does not authorise another
SD update or publish a stable package.

### Stock TSP controls

Stock firmware owns hardware volume. Slot uses unity PCM gain and ignores its
own volume/mute shortcuts. The local brightness follow-up uses the TG5040
`/dev/disp` LCD brightness ioctls with live readback: ten hardware steps, no
startup replay, no automatic restoration and no saved Slot dim overlay. It
reads the current hardware level before each adjustment so changes made by
stock controls are not replaced by stale preferences. Unavailable now means
the display cannot be opened or its brightness cannot be read, not that stock
firmware lacks brightness support. Existing saved preferences and Knulli/
nonstock controls are unchanged. Blue-light/saturation/contrast remain software
pixel effects, not panel calibration. The owner reported brightness working
after test.4; this does not establish stock OSD or suspend acceptance. Startup
volume and decorative LED control remain unresolved.

## Knulli port

Experimental [knulli-tsp-0.1.0](https://github.com/jellydn/slot-tsp/releases/tag/knulli-tsp-0.1.0) (4 November 2026) is **Knulli-only**, GB/GBC/GBA. Extract its ZIP into `/userdata/roms/ports/Slot`, then refresh and launch Ports in EmulationStation. Follow the [Knulli instructions](knulli/README.md) and [issue #1](https://github.com/jellydn/slot-tsp/issues/1); this is not a stock-Tina package.
