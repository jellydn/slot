# Slot for TrimUI Smart Pro

**Native frontend for TG5040 on internal stock Tina Linux.** [jellydn/slot-tsp](https://github.com/jellydn/slot-tsp) is an independent [Slot](https://github.com/BrandonKowalski/slot) fork. Other TrimUI models are unverified.

Install stock packages by **copying content to an existing FAT32 card**, not flashing an OS image. Stock kernel, drivers and services stay in place.

## Verified status — 5 November 2026

First stable stock package: **[stock-tsp-0.1.0](https://github.com/jellydn/slot-tsp/releases/tag/stock-tsp-0.1.0)** for stock firmware **1.1.1, UI 20251128, kernel 4.9.191**. Download `slot-tsp-stock-0.1.0-fat32.zip` and `SHA256SUMS`. Read the [release notes](knulli/RELEASE-STOCK-0.1.0.md) before installing. “Stable” identifies this release channel, not acceptance of every device function.

| Evidence level | Status |
| --- | --- |
| Owner-confirmed | Direct boot, corrected A/B, scan, Tools, real Restart/Shutdown and selected SD splash. Latest first-connect correction: owner reports “Wifi good now.” Not a pass for every function, game or core. |
| Host-checked baseline | Mount/write boundaries, input/panel mapping, EGL cleanup, dispatcher/fallback and native power paths. Not proof of hardware gameplay, save durability or suspend/resume. |
| Included controls | **Device → Restart / Shutdown: select a row, A activates, B returns.** X/Y do not activate power actions. Save/flush/settle and visible failure handling remain in place. Real power actions passed on the earlier build; the current A-select mapping is host-tested, not a separate new hardware pass. |
| Wi-Fi scope | Explicit Connect owns one bounded wlan0 DHCP client after association; Disconnect preserves profiles. Competing clients/unmanaged state fail closed with Network In Use. No Internet, repeated cold-boot/reconnect reliability or persistence claim. |
| Unresolved | Physical LED-off, Fn and real sleep/wake (white/gray failure). Early internal artwork/LED flash can remain before the SD splash. |

LED controls and stock **Lid Power Off** show **Unavailable**. Slot Fn actions are disabled; physical lights and stock Fn policy are not changed. The TG5040 has no lid; no inactivity timer is added.

Grout is not desired or required; historical proposals are not install advice, and removal of an existing installation is unconfirmed.

Details: [stock notes](knulli/STOCK-TG5040.md) and [frontend design](knulli/stock-frontend/README.md). These retain older candidate-stage records; they are not blanket hardware passes.

## Stock install and card layout

Use the stock FAT32 ZIP above. It retains the exact owner-tested executable, cores and startup adapters; release packaging/documentation and source integration are disclosed separately. **Do not use the Knulli ZIP or a BaseOS/unslot image for stock installation.** No firmware, internal boot-logo writer, ROM, BIOS, personal settings/logs or Grout runtime is bundled.

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

**GB/GBC/GBA only.** ROMs must be unzipped; `.zip`/`.7z` are unsupported. Gameplay uses a 240 × 160 canvas, with GB/GBC centered inside it.

| System | Folder below `Slot/` | Core choices in source |
| --- | --- | --- |
| Game Boy | `Games/GB` | mGBA, Gambatte |
| Game Boy Color | `Games/GBC` | mGBA, Gambatte |
| Game Boy Advance | `Games/GBA` | mGBA, gpSP |

`Games/GBAV` is a GBA Video category, not another system. Core availability depends on the package; game/save/link compatibility is not fully hardware-verified.

## Built-in RomM client

Stock preview: **[stock-tsp-0.2.0-rc.1](https://github.com/jellydn/slot-tsp/releases/tag/stock-tsp-0.2.0-rc.1)**.
Download `slot-tsp-stock-0.2.0-rc.1-fat32.zip` and verify `SHA256SUMS`.
Read the [preview notes](knulli/RELEASE-STOCK-0.2.0-rc.1.md). This rebuilt
executable is host-checked, not handheld-tested; 0.1.0 remains the stable release.

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
4. Open **Library**. Left/Right selects GB, GBC or GBA. Up/Down selects a
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

These are source capabilities, not a hardware verification claim. Stock
Wi-Fi must have a working IP connection before browsing.

## Planned systems — not supported yet

[Issue #2](https://github.com/jellydn/slot-tsp/issues/2): PICO-8, Neo Geo, Mega Drive, arcade/FBNeo/MAME, PS1, N64, NDS and Dreamcast. The [implementation plan](knulli/ISSUE-2-PLAN.md) covers dependencies, ABI/GPU feasibility and acceptance gates. **None is supported yet.**

## Knulli port

Experimental [knulli-tsp-0.1.0](https://github.com/jellydn/slot-tsp/releases/tag/knulli-tsp-0.1.0) (4 November 2026) is **Knulli-only**, GB/GBC/GBA. Extract its ZIP into `/userdata/roms/ports/Slot`, then refresh and launch Ports in EmulationStation. Follow the [Knulli instructions](knulli/README.md) and [issue #1](https://github.com/jellydn/slot-tsp/issues/1); this is not a stock-Tina package.
