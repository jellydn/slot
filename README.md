# Slot for TrimUI Smart Pro

**Experimental native frontend for TG5040 on internal stock Tina Linux.** [jellydn/slot-tsp](https://github.com/jellydn/slot-tsp) is an independent [Slot](https://github.com/BrandonKowalski/slot) fork. Other TrimUI models are unverified.

Install stock packages by **copying content to an existing FAT32 card**, not flashing an OS image. Stock kernel, drivers and services stay in place.

## Verified status — 5 November 2026

Committed native baseline: [796750a](https://github.com/jellydn/slot-tsp/commit/796750a), plus SD splash fix [8581da0](https://github.com/jellydn/slot-tsp/commit/8581da0). **No stock-Tina release is published**; the available release is [Knulli-only](#knulli-port).

| Evidence level | Status |
| --- | --- |
| Owner-confirmed | Direct boot, corrected A/B, scan, earlier Wi-Fi/Tools states, real Restart/Shutdown and selected SD splash. Not a pass for every function, game or core. |
| Host-checked baseline | Mount/write boundaries, input/panel mapping, EGL cleanup, dispatcher/fallback and native power paths. Not proof of hardware gameplay, save durability or suspend/resume. |
| Reconnect broken | Supplicant reached `COMPLETED`, but no IPv4, default route or DHCP process followed. Internet access and profile persistence are unverified. |
| Ongoing, not shipped | [Native DHCP, Device settings → Device, A-select power actions and Lid Power Off: Unavailable](https://ampcode.com/threads/T-01a1a72d-2b69-7096-b538-4ead69cd2f6e): uncommitted, host-checked and copied to the owner's card; hardware pass pending. Not in this baseline/release. |
| Unresolved | Physical LED-off, Fn and real sleep/wake (white/gray failure). Early internal artwork/LED flash can remain before the SD splash. |

Grout is not desired or required; historical proposals are not install advice, and removal of an existing installation is unconfirmed.

Details: [stock notes](knulli/STOCK-TG5040.md) and [frontend design](knulli/stock-frontend/README.md). These retain older candidate-stage records; they are not blanket hardware passes.

## Stock install and card layout

Use only a reviewed, matched stock package; the owner-tested delivery is separate from releases. **Do not use the Knulli ZIP or a BaseOS/unslot image for stock installation.**

1. Power off normally. Back up the intended FAT32 card; no format is needed.
2. Verify package manifests, checksums and Tina requirements. Copy reviewed content to the card root. Preserve games, BIOS, saves, states, settings and splash; review exact changes before replacing an existing `trimui` or updater tree.
3. Safely eject. On-device, the card is `/mnt/SDCARD`. Launch **Slot Stock Test** from Apps, or use the package's complete matched `trimui` direct-boot adapters. The early `System/starts/slot.sh` hook alone is insufficient.

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

**GB/GBC/GBA only.** ROMs must be unzipped; `.zip`/`.7z` are unsupported. Gameplay uses a 240 × 160 canvas, with GB/GBC centered inside it.

| System | Folder below `Slot/` | Core choices in source |
| --- | --- | --- |
| Game Boy | `Games/GB` | mGBA, Gambatte |
| Game Boy Color | `Games/GBC` | mGBA, Gambatte |
| Game Boy Advance | `Games/GBA` | mGBA, gpSP |

`Games/GBAV` is a GBA Video category, not another system. Core availability depends on the package; game/save/link compatibility is not fully hardware-verified.

## Planned systems — not supported yet

[Issue #2](https://github.com/jellydn/slot-tsp/issues/2): PICO-8, Neo Geo, Mega Drive, arcade/FBNeo/MAME, PS1, N64, NDS and Dreamcast. The [implementation plan](knulli/ISSUE-2-PLAN.md) covers dependencies, ABI/GPU feasibility and acceptance gates. **None is supported yet.**

## Knulli port

Experimental [knulli-tsp-0.1.0](https://github.com/jellydn/slot-tsp/releases/tag/knulli-tsp-0.1.0) (4 November 2026) is **Knulli-only**, GB/GBC/GBA. Extract its ZIP into `/userdata/roms/ports/Slot`, then refresh and launch Ports in EmulationStation. Follow the [Knulli instructions](knulli/README.md) and [issue #1](https://github.com/jellydn/slot-tsp/issues/1); this is not a stock-Tina package.
