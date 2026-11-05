# slot

**Goal.** Run [Slot](https://github.com/BrandonKowalski/slot) on TrimUI Stock OS.

Stock OS is the factory system on TrimUI handhelds. Under that UI, the system is Tina Linux. This fork is [jellydn/slot-tsp](https://github.com/jellydn/slot-tsp). Slot is the cartridge shelf. Upstream Slot targets Anbernic H700 devices on BaseOS. The Game Boy guide stays at [slot-cfw.fyi](https://slot-cfw.fyi).

## Host

| Item | Value |
| --- | --- |
| Devices | TrimUI handhelds that ship Stock OS |
| System | Tina Linux |
| UI | TrimUI Stock |
| App folder | `/SDCARD/Apps/Slot/` |
| Launch files | `launch.sh`, `config.json` |
| Firmware | [trimui](https://github.com/trimui) firmware repositories |

On a 1280 × 720 panel, the shelf fills the screen. A game stays 3:2, with black bars on the left and right.

This build does not ship the Stock OS app folder yet.

## Knulli Port

Slot also runs as a Port on Knulli. The TrimUI Smart Pro build is [knulli-tsp-0.1.0](https://github.com/jellydn/slot-tsp/releases/tag/knulli-tsp-0.1.0).

1. Download [slot-knulli-tsp-0.1.0.zip](https://github.com/jellydn/slot-tsp/releases/download/knulli-tsp-0.1.0/slot-knulli-tsp-0.1.0.zip).
2. Unzip it into `/userdata/roms/ports/Slot`.
3. Confirm `Slot.sh` is at `/userdata/roms/ports/Slot/Slot.sh`.
4. Refresh Ports in EmulationStation.
5. Launch Slot from Ports.

`Slot.sh` stops EmulationStation, sets `SLOT_ROOT` to the port folder, and runs `System/slot`. On exit it starts EmulationStation again. The launcher source is [knulli/Slot.sh](knulli/Slot.sh). Port work continues in [issue 1](https://github.com/jellydn/slot-tsp/issues/1).

## Games in this build

ROMs must be unzipped. Slot does not read `.zip` or `.7z` files.

| System | Folder | Cores |
| --- | --- | --- |
| Game Boy | `Games/GB` | Gambatte, mGBA |
| Game Boy Color | `Games/GBC` | Gambatte, mGBA |
| Game Boy Advance | `Games/GBA` | mGBA, gpSP |

## Planned systems

These systems are the next goal after Slot runs on Stock OS. Track them in [issue 2](https://github.com/jellydn/slot-tsp/issues/2). This build does not run them.

| System | Reference fork |
| --- | --- |
| PICO-8 | [unslot](https://github.com/cloudwithax/unslot) (Retro8) |
| Neo Geo | unslot (FinalBurn Neo) |
| Mega Drive | unslot (Genesis Plus GX) |
| Arcade, FBNeo, MAME | unslot (one arcade path) |
| PlayStation | unslot, [Fragile](https://github.com/fragilesilver/slot-fragile), [slot.ps1](https://github.com/gmaster1998/slot.ps1) (PCSX ReARMed) |
| Nintendo 64 | unslot (ParaLLEl N64) |
| Nintendo DS | [slotPlus](https://github.com/nelynes/slotPlus) Ultra and Fragile (DSperate) |
| Dreamcast | unslot (Nodosumi, then Flycast) |
