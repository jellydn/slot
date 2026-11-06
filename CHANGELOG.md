# Changelog

Notable jellydn/slot-tsp fork changes are listed first. The upstream Slot
release history is retained below. Fork entries follow the monthly sync and
fixes convention used by [jellydn/VoiceInk](https://github.com/jellydn/VoiceInk).

## [2026-11] - Sync & Fixes

### Draft milestone — stock-tsp-0.3.0-test.9, not hardware accepted

- Combine the local label/rewind work with optional SwanStation software
  comparison. PCSX remains default; Swan cards are isolated and states,
  quick states, automatic resume and rewind are disabled.
- Authored offline boot, gameplay/input/audio, disc reads and card persistence
  pass, including full Slot exit/restart. This is not a target-speed claim.
- Test.9 was installed with separate owner approval, verified backup/readback
  and safe eject. No additional SD/device change or stable release is included
  in this draft. Frozen package/source receipts remain unchanged.
- Latest owner feedback: PICO still fails at an unspecified step; Swan is
  subjectively better but reported at 30–40 fps with choppy sound. Game, scene
  and HUD metric are unconfirmed. Startup volume/LED and PS1 artwork sourcing
  remain unresolved. See `cores/ps1-pico8/STATUS.md` for provenance and checks.

### Local follow-up — account-free PICO-8 labels

- Extract embedded `.p8` text labels and standard `.p8.png` visible labels
  without accounts, network access, Lua execution or external includes.
- Use the existing Fetch Missing Labels, Cart Studio and shared fetch-label
  paths; keep existing/custom labels and Nintendo catalogue fetching.
- Bound input size and PNG decoding; report missing, malformed and unsupported
  labels without guessing. PS1 artwork remains manual; no provider mirror,
  account settings, SD update or publication is included.
- Default fresh/unset PS1 rewind to off, separately from Nintendo preferences.
  Preserve explicit legacy global choices and existing capability gates; this
  is an owner preference, not a proven performance repair.

### Local diagnostic candidate — stock-tsp-0.3.0-test.7

- Fix GameBit PS1/PICO-8 bezel collection 404 before the first upload, retaining
  path security and individual-file 404 behavior. Exercise upload/download and
  real handheld menu selection through public endpoint regressions.
- Add opt-in, runtime-only Performance HUD with separate emulated/fresh/swap
  rates, per-stage wall time and audio missing/dropped sample rates. No saved
  settings change, quality reduction or claim that BRII performance is fixed.
- Explain handheld art-server GB/GBC/GBA coverage and native manual PNG upload.
  Current metadata has no PS1/PICO-8 paths; no new provider or art is bundled.
- Preserve test.6 provenance, core binaries, PICO-8/brightness and user data.
  Local candidate only; no SD update or publication is authorised by bug reports.

### Local combined follow-up — stock-tsp-0.3.0-test.6

- Include the GameBit platform-art correction and preserve working test.4
  PICO-8/brightness fixes, core binaries, settings and saved data.
- Native Default bezels use original grey disc/vent and teal/coral pixel
  side panels, without imported art or marks. Native custom textures now use
  the actual panel dimensions rather than a 720×480 intermediate crop.
- Supply PCSX's documented GPU-thread `auto` default, previously absent from
  the host preset. Dynarec/NEON were already present. No quality/timing hacks,
  rewind change, clock/voltage adjustment or core rebuild. Host validation is
  not proof of TSP speed or a Bloody Roar II audio-starvation fix.

### Local follow-up — stock-tsp-0.3.0-test.5

- GameBit PS1 uses a neutral square sleeve; PICO-8 uses a portrait virtual
  card. Supplied labels retain their aspect and edges rather than entering a
  GBA mould. Defaults use original geometry and text, without borrowed logos
  or game art. Native shelf images and other platforms remain unchanged.
- Owner reports PICO-8 and brightness good after test.4; this does not accept
  all save-state paths, OSD reliability or sleep/wake. Keep its working fixes.
- Optimised ARM64 host comparisons include snapshot/compression work and a
  simulated 48 kHz consumer. They do not reproduce the severe handheld audio
  starvation. Bloody Roar II remains unresolved; no speculative settings or
  performance correction, imported bezel replacement or SD write is included.

### Local follow-up — stock-tsp-0.3.0-test.4

- Reproduce the PICO-8 exit/resume heap corruption with the affected cart and
  an authored real-core regression. Pause staged Lua collection until Eris
  finishes linking restored prototypes; retain allocation bounds and rollback.
- Extend the Slot contract through manual save/load, fresh resume and repeated
  Exit Game/shelf/reinsert. Old core crashes; corrected Mac and ARM64 checks pass.
- Stock brightness uses TG5040 LCD ioctl readback/adjustment instead of an
  unconditional Unavailable row. No startup writes, saved-level replay or
  software dimming; volume stays firmware-owned and Knulli is unchanged.
- Installed with owner approval, verified backup/readback and safe eject.
  Owner later reports PICO-8 and brightness good, without broad reliability
  acceptance. Core fingerprints differ; retain prior snapshots and separate
  cartdata. Bloody Roar II remains unresolved. No bezel assets replaced,
  publication or changes to stable/test.3 artifacts.

### Combined follow-up — stock-tsp-0.3.0-test.3

- PS1/PICO-8 art uses existing label upload and cartridge rendering. GameBit
  exposes both platform libraries and custom bezel management.
- Independent native Bezel choices: None, procedural Default and custom PNG.
  Borders draw behind native video without cropping it or changing its aspect.
- Pinned fake-08 serialization repair: full Lua gameplay, VM/audio/input/timing,
  versioned build-specific state container, bounded payloads and staged restore
  with propagated errors. Quick save and resume pass real-core and fresh Slot
  reopen on macOS/ARM64; cartdata remains independent. Rewind stays disabled.
  Legacy/imported states, oversized Lua and custom persistence/finalizers are
  unsupported. Noise audio uses a restorable PRNG instead of libc randomness.
- Stock TSP hardware volume/brightness ownership is explicit. Slot no longer
  adds its own volume/mute gain or reports saved brightness as hardware state.
  Its brightness row is Unavailable; nonstock controls/preferences are retained.
  No mixer/display write or hardware acceptance is claimed. Colour effects
  remain software shaders, not hardware calibration.
- Frozen test.2 and stable 0.2 assets remain unchanged. No new SD update or
  publication until the combined packaging and verification gates complete.

### Unreleased two-core test candidate — stock-tsp-0.3.0-test.2

- RomM download progress: received bytes, HTTP total and percentage when known,
  explicit unknown-total state otherwise, separate installation and terminal
  states, cancellation and retry. Coalesced snapshots keep the UI responsive.
- Detect PICO-8 cart encoding from content, not the `.p8` extension. The local
  RomM library's PNG-encoded `.p8` files no longer fail text-cart validation.
  Empty, HTML, malformed and wrong-size images still fail validation.

- PCSX ReARMed PS1 and fake-08 PICO-8 native shelves, video/timing/audio,
  full-path loading and isolated per-game saves. Existing GB paths are retained.
- PS1 Digital/DualShock, independent sticks and face/shoulder buttons, existing
  mapper, disc selection and bounded CUE/M3U paths. No physical L3/R3 or rumble
  support is claimed. PCSX state streams are bounded and CUE siblings take
  priority over unrelated working-directory tracks.
- fake-08 cart-data persistence only. Quick states, rewind and automatic state
  resume are disabled; old state files are left untouched.
- RomM PS1 single-file CHD/PBP and PICO-8 text/PNG carts route to native shelves.
  Multi-file disc downloads and arbitrary PNG artwork remain unsupported.
- Includes the 7d14cc6 fork baseline and existing 0f48680 Wi-Fi correction.
  Host real-core save/reopen and format tests use authored content, not a
  commercial ROM or BIOS. Handheld gameplay, speed, real BIOS and Power-button
  Wi-Fi acceptance remain pending. Stable 0.2.0 assets are unchanged.

### Stock TSP 0.2.0 — RomM & Wi-Fi wake correction

Released [stock-tsp-0.2.0](https://github.com/jellydn/slot-tsp/releases/tag/stock-tsp-0.2.0)
from fork runtime commit `0f48680`, with release docs and packaging at `8f26007`.
Highlights:
- Built-in paired RomM Library for GB/GBC/GBA, with paging and downloads through
  the existing installer. No credentials or games are bundled.
- A later healthy stock association and IPv4 address can clear a stale
  Network In Use error after stock takes over Wi-Fi. Competitor guards remain;
  Slot does not adopt, kill or restart stock DHCP clients.
- Device A-select Restart/Shutdown, B Back, save/flush/settle, Disconnect and
  stock Lid Power Off Unavailable remain in place.
- Copy-to-FAT32 install package, corresponding source, checksums and provenance.

Host-verified combined executable; handheld acceptance is pending. The 616
pinned Linux executions and 229 clean host integration tests passed. Four
full-workspace failures also reproduce on unchanged fork upstream; no full-suite
or Internet/repeated-wake reliability pass is claimed. See the
[release notes](knulli/RELEASE-STOCK-0.2.0.md) for the exact frozen executable,
test exceptions, installation/recovery and LED/Fn/sleep/early-logo limits.

### Upstream Sync (e66715d → f1f6ef9)

Merged BrandonKowalski/slot@f1f6ef9 (`main`), 14 commits. Highlights:
- slot.local mDNS probing, collision handling and receipt updates on rename.
- BaseOS-managed network time in place of Slot's SNTP worker. Stock TG5040
  retains its inert platform default; this does not add a Tina NTP service.
- Gamebit label refresh after upload/fetch and persistent label/guide file
  inputs for Safari. A can skip the receipt animation; B still closes it.
- Gambatte rumble enabled and a shorter no-network notice.
- Browser-only Boot Logo Studio and upstream boot-logo documentation.
  This is not an internal-logo writer for stock TG5040.

Owner-approved merge resolutions:
- `crates/slot-power/src/lib.rs`: retain fork `request_restart` and
  `request_poweroff`; adopt upstream `set_network_time` and remove the obsolete
  background clock helper. Adapt the stock inert-controls regression to the new
  API, without adding stock service commands.
- `crates/slot/src/server/net.rs`: adopt upstream mDNS claim/rename handling.
- `crates/slot/src/server/session.rs`: retain upstream `follow_rename`.

Stock power, DHCP ownership/wake correction, A/B controls and RomM integration
are preserved. This is a source sync only. Published stable 0.2.0 tag/assets
remain frozen at their earlier source and executable; the sync is not part of
that binary or its hardware evidence. No SD installation or new device-action
workflow is included.

## Upstream Slot release history

## [1.14.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.14.0)

- Gambatte is a new core for Game Boy and Game Boy Color, and the default on new cards. Existing cards stay on mGBA. Press START on a Game Boy cart to switch cores. Saves and the clock carry over between them. ([#184](https://github.com/BrandonKowalski/slot/issues/184))
- Game Boy and Game Boy Color audio no longer pops going into and out of silence, on both Gambatte and mGBA. Potentially fixes ([#184](https://github.com/BrandonKowalski/slot/issues/184))
- Brightness has six new software steps below the lowest backlight level. ([#189](https://github.com/BrandonKowalski/slot/issues/189))
- Press X on the shelf to take a game off Recents. It can be rebound under Settings → Controls → Shelf as Remove Recent. ([#185](https://github.com/BrandonKowalski/slot/issues/185))
- Large shelves scroll more smoothly. slot keeps an index of ROM headers and a faster label cache, so moving along the shelf no longer reads ROMs or source labels. Existing label caches are converted, not redrawn.
- slot keeps a log of the last five link attempts in `Logs/slotlink.log`.

## [1.13.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.13.0)

- OTA Updater: Open the menu on the shelf and pick Network → Check for Updates. It checks if there is a new release of slot or BaseOS.
- Network is a new menu for Wi-Fi. It lists your saved networks, with Scan for Networks at the bottom to join a new one. A network that rejects your password offers to try another, and isn't saved.
- Gamebit is a web UI for managing the card over Wi-Fi. Start it from Network, then go to `slot.local` and enter the pin or scan the QR code. You can add and remove games, saves, labels, guides, BIOS files and the boot logo, and browse the card's files.
- On device Cart Studio where art can be fetched over Wi-Fi. Fetch Missing Art on the Network page gets labels for every game without one.
- Set Automatically is a new row in Date & Time when Wi-Fi is set up. It keeps the clock right from network time.
- `Shelf` is a new Wallpaper option that follows the shelf you're on.
- slot now loads RetroArch's compressed saves. ([#165](https://github.com/BrandonKowalski/slot/issues/165))
- Guides can wrap to the screen. Press X to turn it on, and each guide remembers its place, size and wrap setting. ([#176](https://github.com/BrandonKowalski/slot/issues/176))
- Shimmerless shader is hidden for GBA on 3:2 screens as it has no effect.

## [1.12.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.12.0)

- Boktai's solar sensor works on mGBA. The sun follows the clock: none at night, brightest at noon. Sunlight Up and Sunlight Down are new shortcuts to set it by hand for the current session. ([#42](https://github.com/BrandonKowalski/slot/issues/42))
- Shimmerless is a new shader on every system. It keeps pixels sharp and gives each one the same width, so the picture doesn't shimmer as it scrolls when the screen isn't a whole multiple of the game. ([#99](https://github.com/BrandonKowalski/slot/issues/99))
- Saturation and Contrast are new rows in Settings → Screen → General and on the in-game Screen page. Both go from Off to 9 and apply to the whole screen. ([#79](https://github.com/BrandonKowalski/slot/issues/79))
- A bezel can be made for one Scale by ending its name with `_spscreen`, `_integer` or `_fit`. It only shows under Bezel while that Scale is picked, and changing Scale switches to the version with the same name.
- A bezel that's solid from just under the game to the bottom edge, like AGS-102, now puts the game at the top of the screen on 640×480 devices, for GBA, Game Boy and Game Boy Color. ([#159](https://github.com/BrandonKowalski/slot/issues/159))
- The Default Game Boy Color bezel in Fit on 640×480 devices shows just the power light, without the cut-off POWER label beside it.
- Reset Game is a new shortcut that starts the game over from power on. ([#168](https://github.com/BrandonKowalski/slot/issues/168))
- Game guides: put a `.txt` with the game's name in `Guides/GBA`, `Guides/GB` or `Guides/GBC`, and Guide shows up in the in-game menu. ([#167](https://github.com/BrandonKowalski/slot/issues/167))
- Game Boy and Game Boy Color games on mGBA no longer flash a frame of stray tiles and sprites when the screen comes back on, as in Pokémon Pinball when the ball moves between the top and bottom of the table. ([#169](https://github.com/BrandonKowalski/slot/issues/169))
- Frame Blending's LCD Ghosting is now called Ghosting, and the Fast Forward Toggle shortcut is now FF Toggle, so both fit on 640×480 screens. A long button combo is drawn smaller instead of running into its row's name.

## [1.11.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.11.0)

- Headphones no longer buzz a few seconds after the sound stops. slot keeps the audio device open while headphones are in. ([#163](https://github.com/BrandonKowalski/slot/issues/163))
- slot leaves the CPU at its full fixed speed again, as it did in 1.9.0. `schedutil` will have to be revisited. ([#162](https://github.com/BrandonKowalski/slot/issues/162))
- Fast Forward Toggle is a new shortcut. One press turns fast-forward on and the next turns it off. It's unbound by default. ([#164](https://github.com/BrandonKowalski/slot/issues/164))
- Fast Forward has a 1.5x speed, for games where 2x is too fast. ([#161](https://github.com/BrandonKowalski/slot/issues/161))

## [1.10.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.10.0)

- Screenshots: Press SELECT + X while you play. Once you take one, you can pick Screenshots in the in-game menu to view them. They're saved as `.png` in `Screenshots/`. ([#94](https://github.com/BrandonKowalski/slot/issues/94))
- Scale, a new row on each system's Screen page, sets how big the picture is. ([#121](https://github.com/BrandonKowalski/slot/issues/121))
- Stretch, on the Game Boy and Game Boy Color Screen pages, sets how far L1 stretches: Classic, matching GBA SP behavior of only widening, or Full Screen.
- The Default Game Boy and Game Boy Color bezels have additional versions that fit the Integer and Fit modes.
- Frame Blending can be toggled on each system's Screen page. This smooths games that flicker sprites every other frame. Mix blends each frame with the last; LCD Ghosting, on mGBA only, leaves a fading trail like the original screens. ([#66](https://github.com/BrandonKowalski/slot/issues/66))
- Date & Time takes a city instead of an offset from UTC, and moves the clock for daylight saving on its own. A new Clock row allows you to pick 24-hour or 12-hour time.
- Game Boy Advance Video carts get their own shelf when placed in `Games/GBAV`. Game Boy Player support is disabled for this shelf. ([#140](https://github.com/BrandonKowalski/slot/issues/140))
- Removed the last brightness step that turned the backlight completely off.
- Powering off or restarting closes the audio device first. ([#142](https://github.com/BrandonKowalski/slot/issues/142))
- The jumped-to letter on the shelf is drawn at the title size. ([#151](https://github.com/BrandonKowalski/slot/issues/151))

## [1.9.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.9.0)

- mGBA no longer crashes on an empty resume state, which a card repair can leave behind. Booting with that cart in the slot crashed slot every time.
- Saves, states, settings and slot's log reach the card as soon as they are written, not up to 30 seconds later. That shrinks the window in which pressing reset or the battery dying can leave the card read-only. ([#72](https://github.com/BrandonKowalski/slot/issues/72), [#144](https://github.com/BrandonKowalski/slot/issues/144))
- A game's own save is written to the card every minute while it plays. The resume state is written every 5 minutes instead of every minute.
- If a game takes slot down while it loads, slot starts on the shelf next time instead of loading it again.
- A core that stops answering no longer freezes slot. Saving waits at most 2 seconds for it, and ejecting at most 10 before slot restarts.
- Caching New Labels no longer stays up forever on a label it cannot read. After 20 seconds without progress the shelf comes up, and a label that crashes its decoder is skipped.
- GBA can use your own bezels on 640x480 devices. Add .png files to `Bezels/GBA` and pick one in Settings > Screen > Game Boy Advance > Bezel. On 720x480 devices a GBA game fills the screen, so there is no bezel.
- A card repair no longer hangs on a card too damaged to check. Each step is given up on after a time limit, the repair reports that it failed, and the device restarts.
- For 30 seconds after a save or a load, the in-game menu now offers Undo Save or Undo Load under Resume. This option is also accessible with X on the state switcher screen.

## [1.8.1](https://github.com/BrandonKowalski/slot/releases/tag/v1.8.1)

- Game Boy games on an MBC2 cartridge, such as Kirby's Pinball Land, no longer freeze after the boot screen. mGBA crashed on the first read of their save RAM; slot now builds it with upstream's fix. ([#137](https://github.com/BrandonKowalski/slot/issues/137))
- Load State on Insert, a new row on the Gameplay page, decides whether putting a cart in, or booting with one in the slot, loads its state. Auto Save on Eject is renamed Save State on Eject and only decides whether ejecting saves one.
- Holding Up or Down to jump through the shelf a letter at a time no longer empties the shelf.
- The BaseOS startup sound plays again. slot no longer takes the speaker while it boots, only once it has something to play. ([#135](https://github.com/BrandonKowalski/slot/issues/135))
- Save states and rewind are smaller: slot no longer keeps the extra space mGBA sets aside before a game finds its save chip. Metroid Fusion's states drop from 528 KB to 430 KB. ([#126](https://github.com/BrandonKowalski/slot/issues/126))
- Fast Forward can be set to Off, so holding it does nothing. ([#123](https://github.com/BrandonKowalski/slot/issues/123))

## [1.8.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.8.0)

- Controls are remappable. Split between three categories: In-Game buttons, Shortcuts, Shelf keys. All three are global. ([#23](https://github.com/BrandonKowalski/slot/issues/23), [#92](https://github.com/BrandonKowalski/slot/issues/92))
- The in-game menu and the shelf menu share the carousel footer, with button art legends that follow the light and dark themes. ([#102](https://github.com/BrandonKowalski/slot/issues/102), @MarkusMaribu)
- Game audio plays at 48 kHz through a filtered resampler, so high frequencies no longer fold back as grit. mGBA benefits most.
- When the SD card turns read-only, SELECT + START repairs it: slot steps aside, checks FAT32 or exFAT, and restarts. ([#72](https://github.com/BrandonKowalski/slot/issues/72), [#111](https://github.com/BrandonKowalski/slot/issues/111))
- Power off, including the shutdown after the lid has been closed for 3 minutes, no longer hangs on a black screen: the Wi-Fi driver is unloaded first, as Restart already does.
- If slot stops responding with the lid closed, a separate watcher powers off a minute after the Lid Off time.
- Run-Ahead is removed. It scrambled saves in games with a 128 KB flash chip, such as Super Mario Advance 4. May revisit in the future. ([#104](https://github.com/BrandonKowalski/slot/issues/104))
- Turbo A and Turbo B have their own rows on the In-Game controls page and can be moved off X and Y. ([#127](https://github.com/BrandonKowalski/slot/issues/127))
- Shortcuts and shelf keys take any two buttons held together, such as L1+R1. In a game the first button still reaches the game at once; the second fires the shortcut. SELECT, START and MENU only work as the first button.
- Setting a control that something else already uses shows a warning naming it, such as "Same as Fast Forward", and marks both rows with a warning triangle.
- Loading a state while a game is saving to a 128 KB flash chip no longer writes the save into the wrong half of the chip.
- The cart insertion animation no longer stutters, and a large ROM still loading after the cart seats powers the screen on to a blank panel that brightens until the game appears. ([#119](https://github.com/BrandonKowalski/slot/issues/119))
- On Anbernics with a left stick, the stick works as the D-pad. Left Stick as D-Pad on the Gameplay page turns it off. ([#106](https://github.com/BrandonKowalski/slot/issues/106), [#107](https://github.com/BrandonKowalski/slot/issues/107))
- `Labels/`, `Wallpapers/` and `Bezels/` each hold a `README.txt` with the sizes and formats they take, and the site's Shelf questions cover them too. ([#122](https://github.com/BrandonKowalski/slot/issues/122))
- On 640x480 devices a 640x480 wallpaper and a 640x480 bezel now show pixel for pixel. Both used to be scaled to 720x480 first. Bezels, including the default Game Boy and Game Boy Color ones, now reach the top and bottom of the screen. ([#122](https://github.com/BrandonKowalski/slot/issues/122))
- Brightness and Blue Light have their own rows: Settings → Screen → General on the shelf, and at the end of the in-game Screen page.
- One Palette row on the Game Boy screen page replaces the Palettes switch: Auto, then mGBA's 48 palettes. Every choice changes a running game at once, including Grayscale for plain monochrome, and SELECT + L2 / R2 step through the same list. ([#124](https://github.com/BrandonKowalski/slot/issues/124))
- slot no longer writes a full memory dump to the SD card when it crashes. The last 5 crash logs are kept.
- Power off and Restart write everything to the card before unloading the Wi-Fi driver, and give the driver 5 seconds, so a driver that hangs can no longer leave unsaved data behind.
- The resume state is not rewritten when nothing changed, and files a power cut left half-written are cleaned up at boot.
- The boot partition is only opened for writing when the boot logo changes.
- The card repair waits for a charge below 20% battery, and runs fsck until the card comes back clean instead of reporting a card it fixed as failed.
- The red card alarm no longer clears when an unrelated write succeeds, and a failed save state raises it too.
- Deleting a save state asks first: Y opens the question, X deletes and B keeps it.

## [1.7.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.7.0)

- The slot, carts and shelf now have a skeuomorphic effect, with light and dark themes and bundled wallpapers. Huge thanks to Markus for this amazing work! ([#74](https://github.com/BrandonKowalski/slot/issues/74), @MarkusMaribu)
- An in-game menu: tap MENU to pause the game behind it. A double tap still goes straight to States.
- Favorites: Y stars a cart onto its own shelf. ([#89](https://github.com/BrandonKowalski/slot/issues/89), @diluno)
- A Recents shelf of the last 10 games, switched on from Settings -> General.
- slot remembers the shelf and cart at power off and boots back onto them. ([#53](https://github.com/BrandonKowalski/slot/issues/53))
- Per-platform bezels. Markus again! ([#77](https://github.com/BrandonKowalski/slot/issues/77), @MarkusMaribu)
- On 4:3 panels the shelf fills the screen. ([#81](https://github.com/BrandonKowalski/slot/issues/81), @diluno)
- Game Titles on the General page fades the shelf name into the selected game's name. ([#84](https://github.com/BrandonKowalski/slot/issues/84))
- A 1 frame run-ahead setting for GBA games on mGBA. ([#56](https://github.com/BrandonKowalski/slot/issues/56), @454135431515)
- Wallpaper Tint darkens your own wallpaper on the shelf.
- GBA, GB and GBC each get their own Shader and Color Correction rows. ([#83](https://github.com/BrandonKowalski/slot/issues/83), @paulocoe-creator)
- LCD3x is offered for GB and GBC games. ([#88](https://github.com/BrandonKowalski/slot/issues/88), @paulocoe-creator)
- Game Boy stretch now properly only widens the picture.
- The backlight steps follow a curve, so the low-end goes dimmer. ([#54](https://github.com/BrandonKowalski/slot/issues/54), @flo333)
- Holding a direction repeats it in menus, the clock and the shelf letter jump, but never in play.
- RG34XX uses its own chords for brightness and the menu display. ([#85](https://github.com/BrandonKowalski/slot/issues/85), @paulocoe-creator)
- The RG28XX picture is turned the way BaseOS says.
- Restart no longer hangs on a black screen: the WiFi driver is unloaded first.
- Muted play keeps its frame rate. ([#51](https://github.com/BrandonKowalski/slot/issues/51), @flo333)
- Fast-forward no longer stutters on frames that take longer than average. ([#70](https://github.com/BrandonKowalski/slot/issues/70))
- gpSP no longer crashes when a crowded boot left its JIT cache unmapped.
- Core options follow the platform that is playing. ([#82](https://github.com/BrandonKowalski/slot/issues/82))
- Card logs moved into a `Logs` folder, and the session log rotates. Card alarms are logged too.

## [1.6.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.6.0)

- Runs on the RG28XX: its panel is mounted portrait, so slot turns the picture onto it. ([#35](https://github.com/BrandonKowalski/slot/issues/35), @GamerAnim)
- Pokémon Pinball: Ruby & Sapphire rumble. mGBA now sees the game's Game Boy Player check. You still need to turn rumble on in game settings. ([#25](https://github.com/BrandonKowalski/slot/issues/25))
- A red alarm shows when the SD card turns read-only or a save fails to write.
- A new shortcut for mute: SELECT + VOL+ / VOL-. The 34XX has a fake volume wheel so VOL+ + VOL- isn't possible. ([#39](https://github.com/BrandonKowalski/slot/issues/39))
- Game Boy cartridge clocks are kept in the .sav. ([#22](https://github.com/BrandonKowalski/slot/issues/22))
- The GBA cartridge clock keeps running on gpSP while the game is off.
- Large libraries now should boot faster, and the first frame shows sooner.
- Games start faster: resting on a cart reads its ROM ahead, so big ROMs no longer wait on the SD card when you insert them.
- Wallpapers are dealt from a shuffled deck, so none repeats until all have shown.
- PNG art is turned the way its EXIF orientation tag says.
- When slot crashes it logs where, and keeps that run's log in `crash/` to attach to a bug report.
- The BaseOS startup sound no longer leaves slot silent. ([#20](https://github.com/BrandonKowalski/slot/issues/20))
- A user defined `Config/bootlogo.bmp` will take precedence over the shipped one. ([#30](https://github.com/BrandonKowalski/slot/issues/30))
- Cart shell, core and video mode files are now `.txt` instead of `.ini`, so Chrome on Windows opens them. slot renames them on first boot. ([#15](https://github.com/BrandonKowalski/slot/issues/15))
- The grid shader's lines stay even on the RG34XX and at fractional scales, and the dot shader no longer darkens a white screen as much. ([#26](https://github.com/BrandonKowalski/slot/issues/26), [#36](https://github.com/BrandonKowalski/slot/issues/36))

## [1.5.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.5.0)

- Rumble regression fixed.
- The boot logo is built to the size of the one BaseOS installed, so it fits every panel instead of only 720x480.
- Runs on the RG35XXSP: the picture fills the 4:3 screen's width and a GBA SP bezel sits underneath while a game plays.
- Screen shaders chosen per platform: Off, LCD3x, Grid or Dot for GBA, and Off, Grid or Simpletex for Game Boy and Game Boy Color.
- SELECT + B / A in game steps back and forth through the screen shaders. A game's own soft reset that holds SELECT with A or B no longer reaches it; hold A on the carousel to start a game fresh instead.
- New settings: auto save on eject, turbo buttons and rewind can each be switched off.
- Settings are grouped into Screen and Gameplay pages, and every value wraps from the last choice to the first.
- Label art is cached on the card, so the shelf no longer stutters building cart faces. New labels are cached once at boot behind a Caching New Labels screen.
- Named palettes for Game Boy games: turn on GB Palettes in the Screen settings, then SELECT + L2 / R2 in game steps back and forth through mGBA's 48 palettes.
- The speaker no longer buzzes while nothing is playing. slot lets go of the audio device after 3 seconds of silence and takes it back when sound starts. Thanks to flo333 for finding it.

## [1.4.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.4.0)

- The volume bar shows a headphones icon while they are plugged in.
- Speaker and headphones keep their own volume and mute, and switch the moment you plug / unplug.
- Settings moved from `System/` to a new `Config/` folder, so replacing `System/` on update no longer resets them. slot moves them on first boot.
- The quick menu and power menu wrap from the last item to the first, and back.

## [1.3.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.3.0)

- The SP boots to a slot logo instead of BaseOS's. slot installs `System/bootlogo.bmp` on first run and keeps the original as `bootlogo.baseos.bmp`.

## [1.2.2](https://github.com/BrandonKowalski/slot/releases/tag/v1.2.2)

- Fix for hitch in games caused by autosave. The autosave now writes to the card in the background.

## [1.2.1](https://github.com/BrandonKowalski/slot/releases/tag/v1.2.1)

- Shell colors from Cart Studio now apply to games with accented names.
- Clear GBA carts no longer show their circuit board above the label.

## [1.2.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.2.0)

- Hold X for turbo A and Y for turbo B.
- Carts on the shelf are larger, and shrink into the slot as they go in.
- Carts are drawn at their real proportions, so full label scans fit without cropping.
- Jumping a letter with Up or Down shows the letter in the slot.
- Saves now stick for games whose save is smaller than the core first reports, such as Golden Sun on mGBA.
- The seated cart no longer shows through the slot as a game starts.
- SELECT reaches the game as soon as it can no longer be the start of a shortcut.

## [1.1.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.1.0)

- Fast-forward runs several game frames per refresh, so it is properly fast, and rewind no longer records while fast forwarding.
- The About label credits BaseOS.

## [1.0.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.0.0)

- Game Boy and Game Boy Color, each on its own shelf. L1 and R1 switch shelves, and carts are drawn at their own size in their own plastics.
- Two cores, mGBA and gpSP. Press START on a cart to open it and swap the chip.
- Link play between two RG SPs over WiFi: the link cable and the Wireless Adapter for Pokémon trades and Advance Wars, mGBA's link mode for Mario Kart, and Game Boy link for Tetris.
- A quick menu on the shelf for fast forward speed and sound, rumble, color correction, the date and time, and About.
- Game Boy games stretch to fill the screen with L1 and go back with R1.
- The real BIOS boot animation plays when the card has one.
- Per-cart shell colors, set in `cart_shell.ini`.
- Up and Down jump through the shelf a letter at a time.
- Your own folder layout on the card is kept, and slot runs on upstream BaseOS.

## [0.1.1](https://github.com/BrandonKowalski/slot/releases/tag/v0.1.1)

- The game holds still behind the power menu.
- Rewind records at every speed and replays exactly what happened.

## [0.1.0](https://github.com/BrandonKowalski/slot/releases/tag/v0.1.0)

- First release: a GBA-only shelf of cartridges with an insert scrape and clunk.
- The game in the slot resumes on the next boot. Closing the lid sleeps and saves.
- Save states with a switcher, rewind, fast-forward, brightness, blue light and volume.
- A power menu on a held POWER, and an About label.
