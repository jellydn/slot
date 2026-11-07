# Combined handheld test — stock-tsp-0.3.0-test.9

**Not hardware accepted. Do not overwrite an unknown card baseline.** This
document is a test plan, not permission for an agent to install or operate the
device. No SD writes were required to build this candidate. Stable 0.2.0 stays
unchanged. Record the candidate ZIP and executable SHA256 from `BUILD-INFO.json`
and `SHA256SUMS`, firmware, game format/region/BIOS and result for every test.

The owner reports PICO-8 and brightness good on test.4. The cases below remain
separate checks, not implied passes. Test.7 retains test.6 and adds the empty
native bezel collection fix, provider-coverage guidance and opt-in diagnostics.
Bloody Roar II remains slow on TSP; this is not a performance-fixed build.
Offline card installation is
separate from device operation; a mounted card alone is not approval.

## Before installation

- [ ] Verify the ZIP SHA256 against the external checksum file; extract it on
  the Mac and verify its internal manifest. Stop on a mismatch.
- [ ] Power the handheld off normally. Back up all files that would be replaced
  plus saves/settings. Identify the intended removable FAT32 card afresh; do
  not assume a disk number. Stop if the startup scripts, updater or current
  executable have an unexplained baseline mismatch.
- [ ] A later approved copy uses the four root directories `Slot`, `Apps`,
  `System` and `trimui`, not an enclosing folder. Preserve all user games,
  BIOS, saves, states, settings, pairing data and splash overrides. The ZIP
  contains no user content. Never hot-replace the running executable.
- [ ] After a separately approved copy and safe eject, check the displayed build
  label is `stock-tsp-0.3.0-test.9`. Keep the previous payload for rollback.

## Test.9 comparison-only SwanStation (owner-controlled, not executed)

- [ ] PCSX must remain the PS1 default. Select Swan explicitly for one game in
  the core picker. Record game/core/BIOS hashes and renderer/options privately.
  Software 1x/PGXP off is intentional; no handheld speed claim is established.
- [ ] Use disposable games/cards. Swan cards are separate from PCSX; no import
  or reconciliation is provided. Do not move cards or states between cores.
  Confirm Quick Save, Quick Load, Rewind and automatic resume are unavailable
  for Swan. Preserve existing explicit rewind preferences when switching cores.
- [ ] Reach the same scene independently on each core. Compare motion, controls,
  picture and audio in PCSX/Swan/Swan/PCSX order under equal conditions. Do not
  change clocks, BIOS or quality options during the comparison. No agent device
  profiling is authorised by this checklist.
- [ ] Save through the game's memory-card menu, exit normally, restart Slot and
  reopen. Check actual progress in each core, plus unchanged original PCSX files.
  For multidisc games, use the Disc row when asked and confirm the inserted disc.
- [ ] Fetch Missing Labels should extract local PICO-8 labels without an account.
  Check both `.p8` and `.p8.png`, missing-label reporting and custom-label
  preservation. PS1 art still needs manual PNG upload; Nintendo is unchanged.
- [ ] Fresh/unset PS1 rewind is off for both core choices. Existing explicit
  global/per-PS1 choices remain effective where supported; this is a preference
  change, not proof that Bloody Roar II performance is fixed.

## Combined follow-up owner check

- [ ] Before uploading a native bezel, open both GameBit bezel pages: an empty
  collection must not show listFiles 404. Upload a PNG, download it, and select
  it on the handheld; preserve the existing selection on the other platform.
- [ ] Open handheld Fetch Missing Labels. Read the provider coverage notice.
  PS1 automatic covers remain unsupported; PICO-8 uses embedded local labels
  when available. Missing labels must be explicit. Upload a PNG through GameBit and
  verify the native label appears. No provider promises every title.
- [ ] Enable Settings → Gameplay → Performance HUD in the same Bloody Roar II
  combat scene, then a confirmed Metal Slug X gameplay scene. Keep settings
  unchanged. Record 10 seconds of Emu/fresh/swaps, stage ms and audio samples/s
  after warmup. Host screenshots are not target measurements. Open/close the
  menu, disable the HUD, and confirm a restart leaves it off.
- [ ] With the same BIOS/controller/rewind settings as test.4, compare Bloody
  Roar II in the same combat scene, then Metal Slug X as a control. Record
  audio breaks, speed and any graphics differences; no full-speed promise.
- [ ] Verify Default, None and an existing custom bezel on PS1 and PICO-8.
  Picture edges/aspect must stay intact; custom image edges must not be lost.
- [ ] Open GameBit PS1/PICO-8 shelves and details on desktop/mobile. Confirm
  square PS sleeves and portrait virtual cards, not GBA moulds; check existing
  uploaded labels remain contained and readable.
- [ ] Retest the working PICO-8 cart, normal exit/reopen and brightness keys.
  This is not acceptance of all snapshots, carts, stock OSD or sleep/wake.

## Existing systems and startup

- [ ] Direct boot and alternate **Slot Stock Test** launch reach the shelf.
  A/B, D-pad and current left-stick menu navigation still work.
- [ ] Open one existing GB/GBC/GBA game; check picture, sound, input and the
  previous save. Make new progress, exit normally and reopen. Do not use an
  only copy of valuable saves for initial testing.
- [ ] Confirm Settings → Controls still accepts the current mapping. Do not
  reset a good mapping merely to test the new systems.

## PS1 — PCSX ReARMed

- [ ] Supply only lawfully obtained content under `Slot/Games/PS/`. Local CUE
  files need their sibling tracks; M3U entries must stay within the content
  root. CHD/PBP are single-file choices. FAT32 files must be below 4 GiB.
- [ ] Supply a lawfully obtained BIOS under `Slot/BIOS/` (for example
  `scph5501.bin` for US content). HLE fixture passes do not verify real BIOS.
  Record BIOS filename, region and hash privately; do not upload the BIOS.
- [ ] Open each available CUE/CHD/PBP/M3U format. Check one logical shelf entry
  per playlist, no BIN-track clutter, loading errors, aspect, resolution
  changes, audio and return to the shelf. Never count a mock image as a pass.
- [ ] In game settings select **Digital**, then **DualShock** with a title that
  supports it. Check the selection persists after exit/reopen. Some older
  titles require Digital; failure there is not a reason to fake analog support.
- [ ] Check four independent face buttons, Start/Select, D-pad, L1/R1/L2/R2.
  Check both sticks independently in all directions, simultaneous motion and
  neutral return. The PS1 left stick must not also press D-pad directions.
- [ ] **No physical L3/R3, no substitute binding and no rumble claim.** Record
  games blocked by missing click buttons instead of inventing a control.
- [ ] Make an in-game memory-card save, exit normally, restart Slot and reopen
  the same content. Confirm progress, not only a file's existence. Card two,
  external save interoperability and every game are not acceptance claims.
- [ ] For a multidisc title test the Disc row at the game's requested swap,
  then verify the correct disc after state restore. Try a quick state and
  reload on this exact core build only; do not import unknown legacy states.
- [ ] Run a representative title for at least 15 minutes; record slow scenes,
  audio underruns, load time, frame pacing and heat. Target feasibility is not
  a full-speed guarantee.

## PICO-8 — fake-08

- [ ] Put lawful `.p8` or `.p8.png` carts under `Slot/Games/PICO8/`. An ordinary
  PNG is not a cart. Check 128×128 picture/aspect, music, six-button controls,
  menu return and relaunch using one text and one PNG cart.
- [ ] In a cart that uses `cartdata`/`dset`, save progress, exit normally, restart
  Slot and reopen. Confirm progress. A cart that never saves cannot test this.
- [ ] On this exact patched build, make recognizable gameplay progress, quick
  save, advance again and load. Confirm Lua gameplay rolls back, not just the
  cart's own persistent data. Exit normally, reopen Slot and confirm resume.
  Test both a text and PNG cart; keep independent copies of valuable saves.
- [ ] Rewind remains unavailable. Do not import legacy/cross-build states or
  count cartdata alone as a quick-save pass. Unsupported/oversized Lua state
  must report failure without stopping gameplay. A refused resume is retained
  separately for recovery, not silently loaded or overwritten.
- [ ] Test.3 PICO-8 snapshots are incompatible with the repaired build. Keep
  backups and confirm refusal preserves those files and independent cartdata.
  With two carts on the shelf, repeat Exit Game → shelf → reinsert three times
  on new states, including the previously affected cart. Stop on any crash.
- [ ] Record cart compatibility failures. fake-08 is not the official runtime;
  arbitrary-cart compatibility and Internet cart loading are not promised.

## Art, native bezels and GameBit

- [ ] GameBit PS1 images use a square sleeve and PICO-8 images a 160:205
  portrait virtual card, without GBA mould branding. Check card and detail
  views, no-art defaults and supplied landscape/portrait labels. All label
  edges should remain visible; defaults add no borrowed logo or game art.
- [ ] Open PS1 and PICO-8 in GameBit's library manager. Upload only art you may
  use; confirm it appears in GameBit and the correct native shelf, not another
  system with the same title. No commercial art is supplied by this package.
- [ ] In each system select Bezel None, Default and one lawful custom PNG from
  `Bezels/PS` or `Bezels/PICO8`. Check preference persistence, GameBit management
  and the full uncropped game image/aspect. Border fills unused space only.

## Stock volume, brightness and software colour effects

- [ ] Use the existing stock volume controls. Check only the stock overlay
  responds, audible level moves correctly and Slot adds no conflicting numeric
  volume/mute scale. No authoritative hardware readback is implemented in Slot.
- [ ] Record physical brightness before launch; Slot must not change it at
  startup. In Slot's Brightness row adjust left/right and check both the shown
  level and actual panel. Test the configured brightness shortcuts, minimum
  (still visible), maximum, shelf and game. Saved software dim is not applied.
- [ ] Change brightness using stock controls, then return to Slot's setting;
  it must reflect hardware readback rather than replay a stale saved level.
  Check stock OSD, exit, relaunch and normal physical sleep/wake separately.
  No background writer, stock-service restart or persistence promise is added.
  If Unavailable appears, preserve logs and stop rather than forcing sysfs.
- [ ] Check blue-light/saturation/contrast visually on shelf and game; these
  are software effects, not panel controls. Report exact setting, image and
  symptom if they differ from expectation. Hardware acceptance is pending.

## RomM — both new native shelves

- [ ] Use the existing paired server, or pair normally without sharing tokens.
  Select PlayStation or PICO-8; check page navigation and missing-platform UI.
- [ ] Download a legal PS1 **CHD or PBP** and PICO-8 **P8 or P8.PNG**. Check the
  downloaded marker, native shelf entry, launch and save/reopen. A ZIP must
  contain exactly one supported game; CUE/BIN/M3U multi-file downloads and
  ordinary PNG artwork are intentionally rejected without a false success.
- [ ] On a large download, check received bytes change while buttons still
  respond. A supplied HTTP length shows a total and percentage; without one,
  check bytes and total unknown, never a guessed percentage or ETA. Verify
  Installing then Installed, B cancellation and A retry after an error.
- [ ] Retry a local RomM PNG cart named `.p8`. Content detection must accept
  that container without renaming or altering its bytes. Empty carts must fail;
  do not count a valid image container as gameplay or cart compatibility.
- [ ] Repeat one existing GB/GBA download to check the prior path still works.
  Save sync, BIOS downloads and automatic core downloads are not included.

## Wi-Fi and Power — pending physical acceptance

- [ ] Confirm this candidate's executable hash, not the older RomM-only preview.
  This source includes 0f48680's stale-busy correction; it does not prove wake.
- [ ] Check a saved network shows Connected and an IPv4 address. Use physical
  Power off/on as before; record screen/error, IPv4 present/absent and exact
  sequence for repeated cycles. Stop on **Network In Use**; do not kill stock
  DHCP clients or bypass ownership guards to manufacture a pass.
- [ ] Check RomM still connects after wake. Keep association/IP, Internet and
  sleep/display recovery as separate observations. LED/Fn and real sleep/wake
  limitations are unchanged; repeated-wake reliability is not host-verified.

If a check fails, preserve saves and report the build/hash, steps, format,
controller mode and observed result. Review logs for credentials before sharing.
Do not update internal firmware, format the card or overwrite unrelated data.
