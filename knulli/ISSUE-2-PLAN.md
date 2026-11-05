# Issue #2: cores beyond Game Boy

**Plan only — 5 November 2026.** No new core or system is implemented by this document. Track acceptance in [issue #2](https://github.com/jellydn/slot-tsp/issues/2); do not tick a system merely because it has a folder or builds.

This file is beside the existing stock port notes; the plan targets stock Tina, not just the Knulli launcher.

## Scope and verified starting point

Plan against `jellydn/slot-tsp` default branch (`knulli-tsp`) at [802de9a](https://github.com/jellydn/slot-tsp/commit/802de9af9d8f3ddf0f303711a6b6c1845b687adc). The native baseline and SD splash are committed; only the Knulli port has a published release. Target TG5040 on internal stock Tina, with a copy-to-FAT32 package, not a new OS image. GB/GBC/GBA remain the only implemented systems; GBAV is a GBA category.

The [ongoing implementation thread](https://ampcode.com/threads/T-01a1a72d-2b69-7096-b538-4ead69cd2f6e) reports owner passes for direct boot, A/B, scan, earlier Wi-Fi/Tools states, Restart/Shutdown and SD splash. Reconnect failed with completed association but no IPv4, default route or DHCP process. Later owned-DHCP, Device menu and lid-Unavailable work is uncommitted, host-tested and owner-card-deployed, with hardware confirmation pending. Do not import that dirty work or modify its network/power ownership while implementing this plan. Recheck the eventual merged baseline first.

Physical LED-off, Fn, sleep/wake and early internal artwork remain outside this core plan. Do not add Grout, stock-menu dependencies, firmware writes, daemon takeover, automatic core downloads or an updater. Future device tests, card deployment and releases require separate approval. Host development must remain useful without Wi-Fi or hardware access.

### Code constraints checked in this tree

| Owner | Constraint and required change |
| --- | --- |
| [`platform.rs`](../crates/slot-store/src/platform.rs), [`core.rs`](../crates/slot-store/src/core.rs) | Game Boy platform variants, GB extensions and exactly three cores. `Core::pair` and defaults assume two choices and mGBA. Introduce only the capability data needed by the approved systems. |
| [`core.rs`](../crates/slot-retro/src/core.rs), [`libretro.rs`](../crates/slot-retro/src/libretro.rs) | `RetroCore` exposes only a byte slice for video; `AvInfo` has timing, not geometry. The video callback centers smaller frames and clips larger frames to 240 × 160. |
| [`pipeline.rs`](../crates/slot-gfx/src/pipeline.rs), [`fbo.rs`](../crates/slot-gfx/src/fbo.rs), [`emu.rs`](../crates/slot/src/emu.rs) | Game texture and publication path assume the Game Boy canvas. Changing only the callback would leave downstream uploads, effects and captures wrong. |
| [`libretro.rs`](../crates/slot-retro/src/libretro.rs), [`ffi.rs`](../crates/slot-retro/src/ffi.rs) | Loader reads the whole file into RAM, passes both path and bytes, and does not honor a core's full-path/archive contract. Environment callback has no hardware-render support. |
| [`mask.rs`](../crates/slot/src/input/mask.rs), [`evdev.rs`](../crates/slot/src/input/evdev.rs), [`core.rs`](../crates/slot-retro/src/core.rs) | Gameplay X/Y mean turbo A/B. Stock stick events can become digital directions, not continuous core axes. `input_state` accepts joypad only. |
| [`core_picker.rs`](../crates/slot/src/core_picker.rs), [`core.rs`](../crates/slot/src/core.rs) | Picker holds `[Core; 2]`. Missing core falls back to a mock test pattern; that must never count as successful emulation. Stock cores load only from `Slot/System/`. |
| [`root.rs`](../crates/slot/src/root.rs), [`scan.rs`](../crates/slot-store/src/scan.rs) | Fixed folder list and Game Boy shelf/header assumptions. Existing BIOS, saves, states, labels, thumbnails, art and imports must be audited, not just file extensions. |

## Reference evidence and reuse boundary

These are source references, not TG5040 compatibility evidence. Pin the exact core revisions and audit licenses again when implementation starts.

- **Primary: [cloudwithax/unslot at 56aa09b](https://github.com/cloudwithax/unslot/tree/56aa09bf111b283d40efbd667e8146281a58d0f3).** Inspected `crates/slot-store/src/{platform,core,scan}.rs` and `crates/slot-retro/src/{libretro,hw}.rs`. Platform specs hold variable core lists and native-frame/input/save capabilities. Loader honors `need_fullpath`; input supports analog axes; disk callbacks and geometry/timing commands exist. Hardware cores use their own GLES context and a framebuffer/readback path. The GL factory is a useful ownership boundary, but its H700/BaseOS driver, ION/shared-buffer and service assumptions must not cross into stock Tina unchanged. `Cores/` downloads there do not justify changing this fork's `System/` trust boundary.
- **[slotPlus Ultra at a61ccfe](https://github.com/nelynes/slotPlus/tree/a61ccfeae8e4e78100afcda53044ec603e898ad3).** Inspected `slotPlusUltra` platform/core tables: DSperate, `.nds`, stacked 256 × 384 output and a separate layout canvas. Use its DS screen/layout approach as a reference. Verify screen-swap, stylus and hotkey behavior against the chosen revision before porting; no physical touch screen is assumed on TG5040.
- **[Fragile at 20703ed](https://github.com/fragilesilver/slot-fragile/tree/20703edd4f3b8337549e243896bc681d1b1ec62a).** Platform table confirms PlayStation and NDS with a separate spec layer. Issue #2 identifies PCSX ReARMed and DSperate. Inspect the actual spec, input and build recipes before choosing code to reuse.
- **[slot.ps1 at b662665](https://github.com/gmaster1998/slot.ps1/tree/b662665eea23dd1968cd69b067377d6ce214ae0b).** Inspected its libretro loader/video callback: a native video branch and path-based disc-image handling. Prefer libretro `need_fullpath` negotiation over its extension-only disc heuristic.
- **[pvaibhav/slot at c19ccbc](https://github.com/pvaibhav/slot/tree/c19ccbc54ba08ffff4cdcc825ee83f406b6441ee).** Useful upstream-adjacent panel/shader reference listed by the issue, not evidence of these new systems. This plan does not port its platform services.

unslot's checked platform rows name Retro8; Genesis Plus GX/PicoDrive/clownmdemu; FBNeo and legacy MAME variants; PCSX ReARMed/SwanStation; ParaLLEl N64; and Nodosumi/Flycast. It has no NDS platform row at the checked revision. A named candidate is not a selected, buildable or supported core on TG5040.

## Dependency order

Implement each phase as a small reviewable change. Preserve the three existing cores and stock boundaries throughout. Complete phases 0–5 before enabling the first new system; phase 6 is an extra gate for hardware-rendered cores. Work on software cores need not wait for phase 6, Wi-Fi reconnect or a suspend fix.

### 0. Establish TG5040 ABI and GPU feasibility before promising cores

**Deliverable:** a pinned target/core compatibility matrix with pass, fail and unknown entries, not a support claim.

- TG5040 is A133P/Cortex-A53 with PowerVR GE8300, not the H700/Mali target of unslot. [Published specifications](https://trimui.net/products/trimui-smart-pro) identify the hardware; they do not prove a usable API on the installed stock driver.
- Source [`fbdev.rs`](../crates/slot-gfx/src/fbdev.rs) opens installed `/usr/lib/libEGL.so*` and `libGLESv2.so*`, using DEFAULT_DISPLAY, NULL native window and a GLES2 context for stock. Earlier target evidence established ES2 context creation; it did not establish every extension, an offscreen core context, GLES3 or Vulkan. Do not infer desktop OpenGL, Mesa/GBM, Vulkan, or surfaceless/pbuffer support from the GPU name.
- Inspect a matched, lawfully obtained Tina rootfs/toolchain offline. Record AArch64 ELF class/machine, dynamic loader, libc and libstdc++/C++ ABI symbol versions, dependencies, pthread/atomic requirements and CPU instruction flags. Run `file`/`readelf` over the frontend and every candidate `.so`; AArch64 alone is insufficient. A 32-bit core cannot load into this process. Use the established stock-compatible build recipe, not an arbitrary modern Linux release binary.
- For dynarecs (gpSP precedent, PCSX ReARMed, N64, Dreamcast), verify the pinned core has an AArch64 backend and compatible executable-memory/cache-flush behavior. Document interpreter fallback and measured cost; do not assume every ARM build enables the same JIT.
- Prepare a separately approved minimal target probe for EGL/GL version strings, extensions, max texture/renderbuffer sizes, depth/stencil, proc lookup and offscreen context creation alongside Slot's display context. Verify draw, readback and cleanup, not just symbol presence. Keep stock services and vendor libraries unchanged. No probe/device action is part of this documentation task.
- For each hardware core, compare its exact renderer build and API/version/extension requirements with probe results. A Vulkan-only ParaLLEl renderer is blocked without verified Vulkan; evaluate a GLES-capable renderer within the candidate instead. Nodosumi's requirements are unknown until its source/build is audited. Failure blocks that core, not the software milestones. A different core/renderer needs a recorded choice, not an undocumented substitution.

**Exit gate:** reproducible software-core ABI recipe; explicit HW feasibility decision or continued block for each renderer. Report loadability separately from performance.

### 1. Model platforms and variable core choices

**Depends on:** phase 0's software ABI contract. **Owners:** `slot-store/{platform,core,scan}`, Slot `core`, `root`, `core_picker`, and their UI callers.

- Add a small platform spec containing stable folder/id, extensions, header strategy, ordered compatible cores/default, rendering and control capabilities. Keep target/build availability per core distinct from platform compatibility. Avoid a broad plugin framework or wholesale unslot import.
- Replace `Core::pair` and two-seat assumptions with a list: zero installed cores gives a clear unavailable state; one needs no fake second choice; multiple allow selection. Persist stable string IDs, not enum positions. Reject an incompatible saved choice and use the platform's available default, not unconditional mGBA. Preserve existing GB configuration.
- Retain reviewed cores under `Slot/System/`. No network downloader in the first implementation. Identify missing/ABI-incompatible core errors clearly; a mock pattern is a developer diagnostic, never a game or acceptance pass.
- Audit root creation, shelf ordering/art, header/CRC readers, label import, RomM/import paths, screen menus, link/rewind options and game identity. Apply GB-specific parsing only to GB content. Folder/relative-path identity must distinguish same-stem games and multidisc entries without overwriting existing saves.

**Host gate:** existing GB/GBC/GBA selection still works; zero/one/three-choice cases, unavailable files, stale selections and same-stem content have observable correct results. Do not enable a new platform until its remaining contracts pass.

### 2. Carry arbitrary frame geometry through the whole software path

**Depends on:** phase 1 capability boundary. **Owners:** `slot-retro/{core,ffi,libretro}`, Slot `emu`, screenshots/thumbnails/rewind, `slot-gfx/{pipeline,fbo}` and shaders.

- Define one frame contract with width, height, byte stride/pitch, pixel format, display aspect and valid buffer length. Support arbitrary core-reported dimensions within checked memory and device texture limits, not a larger fixed canvas. Keep the current centered GB canvas as a compatibility mode if needed for existing effects.
- Read initial `retro_get_system_av_info` geometry and handle `SET_GEOMETRY` and `SET_SYSTEM_AV_INFO`; propagate timing changes to audio resampling/pacing too. Validate dimensions, overflow and pitch before copy/allocation. Preserve null-data duplicate frames; distinguish the later hardware-frame sentinel from a CPU pointer.
- Preserve row stride and conversion for XRGB8888/RGB565; negotiate other formats only when implemented. Resize textures, UVs, aspect fitting, rotation and shader source-size uniforms together. Do not force square-pixel output when a core reports a different aspect. Interlacing/rotation must not silently crop or stretch.
- Carry the same geometry to frame publication, capture, thumbnails, state previews and screen layouts. Bound buffers and resize only when required; do not read disc-sized content into frame memory.

**Host gate:** asymmetric pixel fixtures at 128 × 128, 160 × 144, 320 × 224, 320 × 448, 256 × 384 and changing PS1-style resolutions; padded pitch, both formats, duplicate frames and invalid sizes. Verify visible corner markers survive, correct aspect and no stale border after shrink. Existing GB screenshots/effects remain correct. Actual PS1/N64 sizes remain core-dependent, not these fixtures' promise.

### 3. Honor full-path content, romsets and multidisc ownership

**Depends on:** phase 1 identity/storage boundary; phase 2 for visible acceptance. **Owners:** `slot-retro` FFI/loader, `slot-store/scan`, Slot import/storage paths.

- Query `retro_get_system_info`. Honor `valid_extensions`, `need_fullpath` and `block_extract`: path-only with null data/zero length when required; keep bytes and path alive for memory-loaded content. Archive-as-content is not a request to unpack it.
- Leave Neo Geo/FBNeo/MAME zip or 7z romsets intact where the selected core supports that format. Never unzip and treat one member as a cartridge. Match each core's pinned DAT/romset revision, parent/clone layout and BIOS search rules; list missing dependencies rather than marking every zip playable. Hide BIOS archives such as `neogeo.zip` from the shelf. Some sets need CHDs and directories as well as the zip.
- Preserve CUE/BIN and GDI track relationships and relative paths. Validate missing siblings; do not move one track on import. Treat M3U as one game with ordered discs, deduplicate its members on the shelf, and implement libretro disk-control callbacks for eject/select/insert and resume. CHD being one file does not prove the chosen core/build supports it.
- Bound playlist/descriptor parsing and reject absolute paths, traversal, symlink escape and cycles outside the permitted SD content root. Respect guarded writes; native cores can still perform their own I/O, so this is not a security sandbox. Add a VFS interface only if a selected core requires it, with an explicit read/write scope.
- Account for FAT32's single-file limit (less than 4 GiB). Report oversize/missing content clearly; no filesystem conversion, silent splitting or disc recompression in the runtime.

**Host gate:** untouched archive bytes; valid and missing clone/BIOS dependencies; a CUE with multiple tracks; a GDI set; M3U dedup/disc switching; missing tracks; escaped paths; same names in different folders. Use synthetic fixtures or lawfully redistributable content, not bundled copyrighted ROMs.

### 4. Separate gameplay controls from turbo and menu shortcuts

**Depends on:** phase 1 capabilities. **Owners:** `slot-input`, Slot `input/{evdev,mask,bind}`, `slot-store` controls, `slot-retro` input, quick-menu hints.

- Keep four real face buttons and L/R/L2/R2 plus supported stick-click inputs distinct from turbo and rewind/fast-forward actions. Preserve the owner-tested stock A/B normalization. Apply optional turbo only to platforms/bindings that request it; never strip genuine X/Y for Neo Geo or six-button Mega Drive.
- Add per-platform/controller mappings: Mega Drive three/six-button, Neo Geo A/B/C/D, arcade coin/start, PS1 digital/DualShock, N64 stick/Z/C buttons, DS screen swap/stylus, Dreamcast stick/triggers. TG5040 cannot provide every original control directly; document remaps and conflicts. Menu escape must remain accessible without stealing normal gameplay inputs.
- Carry both sticks as continuous signed libretro analog values, with verified evdev axis codes/ranges, dead zones, inversion and saturation. Do not infer right-stick or trigger axes from the existing digital left-stick conversion. Respect port, device, index and controller subclass; answer joypad bitmasks and individual buttons consistently.
- DS needs a stick-controlled pointer/stylus and press mapping plus a layout-aware transform. Test both-screen, single-screen and swapped layouts; no touch panel is assumed. Arcade pointer/mouse/lightgun games remain excluded until their device interfaces and usable mappings exist.

**Host gate:** simultaneous X/Y+A/B, shoulders/triggers without hotkey collisions; asymmetric left/right-stick inputs including center and both endpoints; device/port selection; stylus corners after swap/scale. **Hardware gate:** verify each physical input and neutral return on TG5040, not an H700 mapping copied by name.

### 5. Make BIOS, build, options and save acceptance explicit

**Depends on:** phases 1–4. **Owners:** `cores/`, `taskfile.yml`, packaging/`licenses/`, Slot core options and save/state lifecycle.

- Pin source commit/archive hash, patches, build flags/toolchain/sysroot and resulting `.so` hash for each selected core. Include required notices and corresponding source/build materials according to each license. Audit each core and dependency separately; do not assume every emulator has Slot's GPL terms or permits the same redistribution. Do not ship vendor firmware, user ROMs, BIOS or credentials without rights.
- Keep BIOS in `Slot/BIOS/`, with per-core required/optional filenames, hashes where published, region requirements and search subdirectories. Neo Geo BIOS placement must match the chosen FBNeo/MAME build. PS1 HLE or Dreamcast HLE, if offered, is not equivalent to a verified real-BIOS path. Determine DSperate's BIOS/firmware needs from the pinned build. Official PICO-8 is a separate licensed runtime; Retro8 is not a license to bundle it.
- Audit the environment commands used by each core (options v1/v2, directories, logging, controller info, geometry/timing, disk, rumble, performance interfaces). Implement only required contracts, with lifetime-safe FFI data. Return honest unsupported results and actionable errors; do not pretend a callback is implemented. Check Retro8/fake-08 behavior rather than relying on cart dimensions alone.
- Namespace battery saves, memory cards, RTC and core-written auxiliary files by game/platform where needed; namespace save states by core identity/version. Never feed another core's state to a selected alternate automatically. Preserve existing GB paths through a tested migration if identity changes. Reject incompatible/corrupt states without losing the prior save.
- Some cores have no usable serialize support or prohibit rewind. Gate auto-resume, rewind, runahead/link and state UI by actual capability. Flush SRAM/memory cards on normal exit using the existing writer lifecycle. A serialize call returning success is not evidence that reload restored progress.

**Host gate:** clean build/ELF dependency audit; missing/wrong BIOS messages; options and errors; save progress then close/reopen and inspect restored progress; same-stem isolation; incompatible state/core version refusal; no-serialize core with working in-game save and truthful state UI. Use public behavior tests, not callback-count assertions.

### 6. Add hardware rendering only after a target feasibility pass

**Depends on:** phase 0 GPU decision and phases 1–5. **Owners:** `slot-retro` environment/core lifecycle, `slot-gfx` EGL/context backend, Slot emulation/render thread boundary.

- Implement `SET_HW_RENDER` negotiation for only verified GLES context types/versions. Supply real framebuffer and procedure callbacks, depth/stencil and origin handling. Handle `RETRO_HW_FRAME_BUFFER_VALID` without dereferencing it. Implement `context_reset`/`context_destroy` with correct FFI layouts and ownership on load, unload and error.
- Follow unslot's separate core-context boundary, not its driver implementation. Keep the core context current only on its owning emulation thread. Start with a bounded FBO readback into the phase-2 frame path if the stock driver supports it; measure its cost before attempting shared textures/buffers. Explicitly manage synchronization, GL state and context lifetime. No copied H700 ION path or assumed context sharing.
- Test unsupported API/version, FBO creation failure, resolution changes, missing GL functions, origin inversion, unload/reload and frontend rendering after a core exits. A GPU error must not leave a stale context or a fake successful game. Retain the current honest save/exit behavior on display failure; do not invent suspend recovery or take over stock Power services.

**Host gate:** negotiated callbacks and lifecycle using synthetic software/HW test cores plus rendered corner markers. **Hardware gate:** actual TG5040 offscreen draw/readback, audio/input under load, repeated launch/return without corrupting the shelf, memory/temperature/frame-time measurements. GLES3/Vulkan remains blocked unless independently verified. If readback is too slow, record the result and assess a smaller render resolution or separately designed shared path before claiming support.

### 7. Integrate cores in low-risk waves

**Depends on:** phases 0–5 for every row; phase 6 when the chosen build requires hardware rendering. Each alternate gets its own acceptance evidence, not its default's pass.

| Order / system | Initial candidate and content | Additional gate / later choices |
| --- | --- | --- |
| 1. PICO-8 | Retro8; `PICO8`, `.p8`/cart `.png` | Small software-rendered pilot after environment audit. Reject ordinary PNG art as a cart. Check compatibility with representative carts; do not call it the official runtime. fake-08 is a separate evaluated alternative, not unslot's listed default. |
| 2. Mega Drive | Genesis Plus GX; `MD`, `.md/.gen/.bin/.smd` | Full frame, aspect/timing changes and three/six-button pad. Compare PicoDrive and clownmdemu only after build/license/performance review. |
| 3. Neo Geo + Arcade | FBNeo shared implementation; `NEOGEO` / `ARCADE`, core-supported `.zip/.7z` | Matching romsets/BIOS, four/six-button inputs, coin/start and core-native saves. Keep separate acceptance rows for Neo Geo, Arcade and FBNeo; no claim for every arcade board or input device. |
| 4. MAME alternatives | MAME 2003-Plus first; same shared content path | Its own DAT/BIOS/save validation. MAME 2003, 2000, 2010 and FB Alpha 2012 are separate optional builds, not current MAME or interchangeable romsets. Neo Geo's FB Alpha 2012 variant also needs its own pass. |
| 5. PlayStation | PCSX ReARMed; `PS`, `.cue/.chd/.pbp/.m3u` as the build supports | Software renderer first if feasible, AArch64 dynarec review, variable resolutions, memory cards, disc change and analog modes. SwanStation is a separate ABI/GPU/performance decision. |
| 6. Nintendo DS | DSperate; `NDS`, `.nds` | Inspect pinned source/build first. Both 256 × 192 screens, layout/swap, pointer and save persistence; no untested melonDS substitution. Software/HW dependency comes from that build, not the system name. |
| 7. Nintendo 64 | ParaLLEl N64; `N64`, `.n64/.v64/.z64` | Choose a renderer that passes phase 0; GPU path requires phase 6. Analog/Z/C controls, save types and native-resolution performance. A Vulkan renderer is not presumed available. |
| 8. Dreamcast | Audit Nodosumi, then Flycast; `DC`, `.chd/.cdi/.gdi/.cue/.m3u` as supported | Nodosumi feasibility is open. Flycast needs a compatible hardware-render backend. Validate BIOS/HLE policy, VMU save files, disc content, triggers and sustained performance. Do not promise either core will meet the target. |

Neo Geo CD is not Neo Geo and is not added by the shared arcade work. PSP, NES/SNES and other systems seen in reference forks are outside issue #2 unless separately scoped.

## Acceptance and rollout gates

For **each selected core/build**, retain an evidence row with source/hash, target firmware, game/content format (lawfully obtained), BIOS mode, renderer, controls, save method, measurements and known failures. Keep **host pass**, **owner hardware pass**, **packaged**, and **published** as distinct columns. A build pass or mock pattern cannot tick issue #2.

1. **Regression gate:** run relevant store/retro/input/gfx/Slot tests, stock feature tests and nonstock checks; formatting and scoped lint. Retest GB/GBC/GBA selection, video/audio/input, save reopen, labels, resume and direct-boot boundaries. Confirm no DHCP/power/Fn/service changes enter the core diff.
2. **Host integration gate:** shelf lists one logical game, real core loads content, rendered frames/audio are correct, controls work through public input, and an in-game save or state survives a new session. Exercise missing core/BIOS/track, invalid content and failed state load without overwriting good data. Inspect representative rendered states; publish no copyrighted test assets.
3. **Hardware approval gate:** before deployment, review the exact package delta, removable FAT32 target, hashes, backups and offline rollback. No automatic device actions follow from this plan. Do not reuse the ongoing DHCP candidate as the core development baseline without its owner's merged source.
4. **Hardware core gate:** on TG5040, verify shelf → game → menu → exit, usable video and audio, every mapped control, save then clean restart/reopen and restored progress. Record average and high-percentile frame time, underruns, RAM, load time and sustained thermal behavior over an agreed representative session (proposed minimum 15 minutes), with core-reported timing as the budget. Report intentional frame skip and slow titles; no universal full-speed claim. Keep suspend tests separate while its platform contract is unresolved.
5. **Release gate:** only after owner acceptance, prepare a FAT32 content update with source, manifests, notices and rollback. Preserve games/BIOS/saves/settings/splash; exclude credentials and Grout. Document formats, exact supported core versions, BIOS placement and limitations. A future release requires explicit approval; no issue comments/checklist updates are made by this documentation work.

## Decisions to settle during implementation

- Exact stock sysroot/toolchain and reproducible per-core recipe; permissible redistribution for every selected build.
- Measured installed GPU APIs/extensions and renderer choice for N64, Dreamcast and any hardware PS1 core. If no compatible path exists, leave that row blocked rather than swapping drivers or OS.
- Stable content/save identity and migration, per-platform hotkeys, TG5040 analog calibration, DS stylus/layout policy and disc-control UX.
- Representative lawful games and performance targets for each core, with explicit exclusions where required controls or performance are unavailable.

These decisions are checkpoints for future implementation, not claims that the unanswered items already work.
