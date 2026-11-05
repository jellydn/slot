# Stock Tina TG5040 candidate

## Native TG5040 direction — owner-reported Wi-Fi, Tools and reboot pass

The owner reports that Wi-Fi is working and Tools are usable from Slot. Stop
speculative Wi-Fi changes. This does not establish a new association test,
credential persistence or Internet access. Keep the working backend unchanged.

### Selected SD startup logo (host candidate; hardware test pending)

Stock startup deliberately refuses the internal boot-logo installer. It also
did not draw Slot's selected image. The SD renderer now draws that selection
once through its existing EGL compositor, after rotation setup and before
frontend loading: `Slot/Config/bootlogo.bmp` takes precedence over
`Slot/System/bootlogo.bmp`. Use an uncompressed 24-bit BMP; an invalid override
is skipped, not silently replaced. Art is centered, aspect-fitted down if
needed, without forced enlargement or an added boot delay.

The update must supply the existing default `card/System/bootlogo.bmp` if the
card lacks it, and must preserve a user override. The last deployed receipt
recorded both selection files absent; this is not a fresh card inspection.
No dispatcher, input, firmware, LED, network or power behavior changes.

This is a later application splash, not a replacement for an early firmware
frame. Spruce's SmartPro installer at revision
`6861c2b56ef23013dfdb7c87a00959825a8f68d4` writes internal
`/dev/mmcblk0p1`'s `bootlogo.bmp`. That explains a possible early Spruce source,
but this device's internal image has not been inspected. No internal writes
are included or authorized. An early Spruce frame can remain before Slot.

Frozen deployed baseline ZIP SHA256:
`b452d2891466e25f3e5140b1e669cd1e915e84b763e923b4f82618242ce066bc`.
Its Slot binary SHA256:
`d0ba0b1cc1e87d4421763204613781454ecc021600bddca79034f0eab1c5c5e6`.
Keep that archive immutable. Test the new logo candidate on hardware before
publishing a release that includes it. First-release format remains a FAT32
copy-to-card content package, not a raw OS image. Exclude ROMs, BIOS, personal
saves/configuration, credentials and Grout runtime.

Slot is the primary frontend and must provide its own settings and controls.
Retain the stock Tina kernel, drivers and FAT32 content package. A stock menu
dependency is not a completed native platform. Kernel services can remain
backends, but each control must have one owner. No SD update, uninstall, device
action or live service/configuration change is authorized by this host work.

### Phases and acceptance criteria

1. **About and reboot (deployed; owner-reported reboot PASS):** identify TRIMUI SMART PRO /
   TG5040, remove the Anbernic wordmark and electrical rating, identify the
   retained stock Tina kernel and keep upstream credits. About X/Restart saves,
   settles pending writes and requests the factory `/sbin/reboot` without force
   flags, module unloads or stock menu handoff. A rejected request shows
   `Device reboot failed`, leaves audio available and permits retry in Slot.
   An accepted request is sent once; Slot remains present awaiting init rather
   than exiting to MainUI. A successful command status is not proof of reboot.
   The owner confirms About Restart works on hardware. This does not establish
   power-loss save durability, physical Power behavior or sleep/wake support.
   Physical Power/sleep guards stay intact; do not repeat a reboot test.
2. **Wi-Fi ownership research (deferred after owner success):** keep the existing bounded
   supplicant association/profile code and credential handling. Establish a
   single DHCP/radio backend owner before enabling native connection/toggle
   controls. Verify whether an existing wlan0 lease client is owned by stock;
   do not kill or adopt an unknown client. If Slot needs its own client, give
   only that child an explicit lifecycle and PID and bound failures. Do not
   invoke the factory all-client-kill helper or reuse the factory lease hook
   blindly: its route cleanup can delete other interfaces' default routes,
   it rewrites global DNS and it can source `/etc/udhcpc.user`. A native lease
   hook must constrain changes to its owned interface/routes and explicitly
   manage shared resolver ownership and conditional restoration.
   Distinguish association, fresh address acquisition, route/DNS readiness and
   saved profile state. Test rollback, stale IP, lease failure and radio
   restoration with mocks before any separately approved association test.
3. **Fn deferred; physical RGB off requested:** keep Slot Fn-triggered LED,
   Wi-Fi and mute actions disabled. No Fn action/settings are currently wired;
   do not add or repurpose the switch while fixing Wi-Fi. This does not disable
   the retained system scene daemon or its own Fn response.
   The owner explicitly means physical lights off, not hiding a setting.
   MinUI's TG5040 Smart Pro path uses `0` at
   `/sys/class/led_anim/max_scale` for decorative LED-off; its extra `_lr` and
   `_f1f2` nodes are Brick-only and were absent on this target. The live custom
   scene script refers to Spruce LED policy and warns that disabling the
   animation engine alone can freeze the last lit colour. Do not use
   `effect_enable=0` as an off command, run the Spruce helper, stop scened or
   edit internal scene scripts. First review a bounded operation: recheck the
   node and writer ownership, record its current numeric value, write `0` once
   and read back. It needs specific live-write approval; no write occurred.
   Do not implement a competing retry loop or claim persistence across scene
   changes. Charging/status indicators are not established as controlled by
   this decorative RGB interface. Brightness/mixer work is deferred too.
4. **Sleep/wake/shutdown cooperation (blocked on contract):** identify stock
   notification/acknowledgment and save deadlines, then integrate checkpoint,
   audio and renderer lifecycle. Do not infer a protocol from marker names or
   equate local black rendering with real suspend. Verify EGL/context and
   scanout behavior in separately approved tests. Keep truthful save/exit on
   swap failure until a recovery contract exists. No timed wake guesses.
5. **Grout removal and next package:** exclude Grout from future native
   payloads and retire the stock-menu-only launch route. The separate
   `stock-grout/UNINSTALL-PROPOSAL.json` permits only the eight pinned installed
   package files, with fresh old-hash checks and verified Mac backups before
   any approved removal. It is a review manifest, not an executable uninstaller.
   Preserve runtime config, tokens, cache, downloaded ROMs/saves, unknown or
   changed files, AppleDouble companions and all directories. No SD removal
   has occurred. Old archives remain historical evidence, not current advice.

### Offline interface evidence and ownership gaps

- Matched factory rootfs SHA256:
  `7e54f673899c824d3043f2eb9ebc81da622ecff0812c13a3fc010d27b573cb11`.
  Its `/sbin/reboot` links to `../bin/busybox`; factory rc.button uses normal
  `/sbin/poweroff`. Reboot implementation uses only the normal init request.
- Factory `wpa_supplicant` init uses wlan0/nl80211, control directory
  `/etc/wifi/sockets`, main config plus overlay and procd ownership. The
  factory udhcpc restart helper kills all udhcpc clients: it is unsuitable.
  The default lease script changes interface, routes and `/etc/resolv.conf`;
  that is a separate operational scope, not a harmless association detail.
- Matched `runtrimui.sh` explicitly exports GPIO243 as input for the DIP
  switch. Matched `trimui_scened` strings identify its GPIO243 value path,
  change detection and `/usr/trimui/bin/scene.sh 1` / `0`. The script runs
  configured `/usr/trimui/scene/*.sh` actions in the background and kills
  matching script names. Do not invoke it as a generic Slot toggle helper.
  GPIO electrical polarity, enabled scenes and a safe handoff/restore contract
  are not established by these strings. Do not edit internal scene config or
  stop scened without a separately reviewed operational scope.
- Factory startup configures `class/led_anim` effect controls. MinUI TG5040
  reference uses `class/led_anim/max_scale` for ambient LEDs, `/dev/disp`
  ioctl `0x102` for brightness and the stock ALSA digital/DAC controls for
  volume (digital-volume direction is reversed). These are target-specific
  candidate interfaces, not permission to copy MinUI service/power takeover.
  LED role/range, Fn interaction, brightness restoration, mixer ownership and
  OSD/scene conflicts remain unverified. Slot PCM gain already exists and
  can implement Slot-only mute once the Fn policy is integrated.

### Latest bounded read-only Wi-Fi diagnosis

One authorized USB target initially showed Slot running, `wlan0` up with
carrier, an IPv4 address and a default route. A supplicant and `udhcpc -i wlan0`
were present with parent PID1; daemonization does not identify their launcher
or authorize adopting/stopping them. The existing supplicant socket was present.
The current Slot stdout log was located through its fd metadata and only
allowlisted failure-phrase counts were reported; those phrases did not occur
in the bounded log tail. No raw log, SSID, password or profile was reported.
These observations do not prove Internet access or reproduce a failing Slot
attempt. The device disconnected before a final read-only supplicant STATUS
query could run. Fresh ADB inventory then had zero targets; `/Volumes/TSP`
was absent on the Mac. No runtime implementation or new SD update resulted.

Next evidence gate: reconnect and finish bounded read-only status/ownership
checks, plus obtain the exact Slot error/phase if it is not recorded. Reproduce
that phase in host tests before changing the backend. Then implement explicit
foreground supplicant/DHCP ownership without MainUI: reuse an established OS
backend only under a verified contract; refuse unknown lease clients rather
than kill/adopt them. Test missing daemon, foreign/recycled PID, stale IP,
fresh lease/route/DNS readiness, cancellation, renewal and profile rollback.
Any intentional association/SAVE_CONFIG, daemon start/handoff, address/route
or resolver change needs a concrete active-test scope approved first. No
credentials in the thread. Review the separate one-node RGB-off operation
before a live write; no power/Fn/service test is implied.
Keep working A/B, renderer/controller startup, core bytes, offline bypass,
games, settings and saves unchanged in the next exact package.

## Historical connection and Grout host preparation — superseded

The previously deployed full frontend and live scan have owner-reported passes.
The connection/Grout revision below is **host-only, not deployed**. Earlier
preparation-time deployment warnings in this document are historical.

Stock mode now uses `wifi::stock_station::Station` over the existing stock
`/etc/wifi/sockets/wlan0` supplicant endpoint. It supports selecting saved
profiles and adding open or WPA/WPA2 personal profiles (ASCII passwords 8–63
bytes, SSIDs up to 32 bytes, including hidden names). Observed EAP/WEP/OWE and
SAE-only networks are refused. No helper, shell credential argument, config
file writer, radio toggle or DHCP client is introduced. Stock retains the
overlay `/etc/wifi/wpa_supplicant.conf` and its `update_config=1` policy.
Only stock `SAVE_CONFIG` persists a new profile, after matching profile ID,
SSID, completed association and a usable reported IPv4 address. This is not
proof of a fresh DHCP lease, routing or Internet access. Credentials stay in
memory until passed to the stock control socket; they are not logged by Slot.
Stock storage permissions/encryption remain stock's responsibility.

Failed/cancelled joins remove only the newly allocated Slot profile and restore
the previous selected/enabled profiles. Existing duplicate SSIDs are not
deleted. An uncertain add/cleanup/save blocks further mutations for that
backend session to avoid accidentally persisting unknown state. A failed save
shows `Connected; Save Unconfirmed` without tearing down association. Radio,
Forget, Disconnect and Auto-Join controls remain stock-owned and unavailable.
Leaving Slot leaves completed connections running; unfinished joins receive
bounded rollback. Control requests use three-second deadlines; the existing
worker's 20-second association/address timeout is retained.

Factory MainUI starts udhcpc, but a separate DHCP owner during direct Slot boot
is not established. Slot therefore shows `Waiting for Stock DHCP`, then
`No IP Address` if none arrives. It does not invoke the stock restart helper,
kill DHCP or take over network policy. Usable addresses reported by the
supplicant can be stale; no live association or DHCP test is claimed.

RomM's public Mac heartbeat reported server **5.4.1**. Grout **v5.4.1.0** is the
matching official TrimUI release; see [`stock-grout/README.md`](stock-grout/README.md)
for pins, MIT notice, packaging and updater restrictions. Supported handoff is
Slot Shelf → Menu → About → X save/exit → stock Apps → Grout → stock return,
not concurrent display ownership or automatic Slot resume. The handheld needs
a reachable LAN server URL; `localhost:5678` refers to the handheld itself.
No server credentials or runtime config are prefilled.

Separate approval is required for exact SD deployment, live association and
stock config persistence, and Grout launch/authentication/downloads. Existing
A/B mapping, startup dispatch, offline bypass, cores and user data are not
changed. White/gray wake, real suspend and shutdown cooperation remain unresolved.

**HOST CANDIDATE ONLY. Separate SD update and active full-Slot test approvals
are required. The existing SD package is unchanged.**

The later full-frontend host candidate is documented in
[`stock-frontend/README.md`](stock-frontend/README.md). It prepares every entry
of the official SD service/frontend branch using the installed stock runtime,
and removes duplicate Slot Power handling. It is not staged or boot-verified;
the white/gray suspend/wake fault remains unresolved. Earlier direct-boot
decision notes below describe the preceding scan-only package, not this new
host preparation. No independent OS image is proposed.

Build with `--no-default-features --features stock-tg5040`. Do not combine the
stock feature with the default host backend. Stock mode uses
`/mnt/SDCARD/Slot` and the TG5040 input mapping, without relying on Knulli
board files or environment overrides. Stock EGL uses DEFAULT_DISPLAY and a
NULL native window only, with the installed matched EGL/GLES libraries.

## Runtime policy

The stock platform reads only the verified `axp2202-battery` capacity/status
interface. Stock services retain backlight, LED, rumble, RTC, USB and radio
ownership. The stock device loop returns to its supervisor after saving when
power/restart/repair is requested; it does not reboot, power off, repair a card,
or unload kernel modules. Platform-level system actions also refuse if called
outside that loop. Updates, automatic Wi-Fi setup, link-radio scripts, input
poll-rate writes, automatic bootlogo refresh, `/run` first-frame markers and
filesystem-wide stderr syncing are disabled for this build.

The launcher and binary require the exact writable FAT/exFAT `/mnt/SDCARD`
device mount and `/mnt/SDCARD/Slot` root. They reject nested Slot mounts,
symlinks and special files. The binary also rejects multiply-linked files,
pins the root device/inode, and checks managed mutations against this boundary.
Saves, settings, caches and logs use guarded SD paths. HOME/TMPDIR/XDG paths
and loading markers stay under Slot. Core files come only from Slot/System.
Numeric ALSA card fallbacks, core dumps and inherited library overrides are
disabled. The frame loop returns if the root identity changes, without a
final save to the changed root.

This is not a kernel sandbox: native cores and vendor drivers are not mediated,
and pathname checks are not race-proof against concurrent privileged mutation.
Keep the card inserted, its mount unchanged and the Slot tree unmodified during
a run. An SD I/O failure may still lose a pending save. Unknown mount types,
including an unverified FUSE mount, refuse rather than weakening checks.

## Foreground normal Apps entrypoint

`stock-app/config.json` and `stock-app/launch.sh` are prepared for
`Apps/SlotStockTest`. Select **Slot Stock Test** through the normal stock Apps
menu only, after separate staging and hardware-test approval. The shell waits
for its foreground child, records its status, and exits to its stock caller.
It does not kill, stop, signal, restart or duplicate any stock process. There is
no autostart, shutdown/reboot fallback or deadline killer.

Before hardware initialization, the binary requires the fixed executable path,
working directory, root and board, launcher marker, regular SD log descriptors,
valid mount and safe existing tree. Failure exits 78. All extra CLI arguments,
including `--init-root` and `--install-bootlogo`, remain refused. The launcher
marker is a contract check, not proof of display ownership or authentication.

Logs use `Slot/Logs/stock-app-PID.log`; noclobber refuses a reused filename.
Failures before opening that log go to the stock caller's stderr. The legacy
`stock-tg5040-check.sh` remains an always-refusing diagnostic, not a launcher.

## Optional stock SD startup hook — prepared only

`stock-start.sh` is a separate optional candidate for the card path
`System/starts/slot.sh`. It uses the same foreground Apps launcher, once, then
returns 0 so the existing official startup can continue. It does not run the
official startup itself, start MainUI, change services or use an updater.
Create `Slot/Config/stock-menu-only` while the card is offline to bypass the
hook. Removing the hook is another offline fallback. Neither prevents a
driver/kernel hang during an already running Slot process.

The input-readiness revision requires a running `trimui_inputd` and a readable
character node named `TRIMUI Player1` advertising EV_KEY and EV_ABS. It checks
metadata only, accepts any event number, and immediately returns 0 if either is
missing. It never waits, backgrounds Slot or starts a service. The first stock
boot demonstrated that this hook blocks the later input initialization: Slot
reached the clock page with no controller available. On that startup path the
guard intentionally skips Slot so the stock menu can initialize. Use the normal
Apps entry only in a separately approved later test. This is safe fallback,
not a verified post-service direct-start mechanism or proof that input works.

The retained live MinUI wrapper calls the saved official original when
`.tmp_update/updater` is absent. That original runs `System/starts/*.sh`
synchronously before its later services/menu startup. No `boot.sh` dispatch
was verified. An existing updater can bypass this route; do not replace or
delete it automatically. Other start scripts are not suppressed. The early
hook is not equivalent to the tested Apps lifecycle: Slot display/input/audio
dependencies at that point remain untested. No internal wrapper edit is needed.

The host content package separates this hook from the normal copy payload.
Copying it would deploy autostart and requires explicit separate approval.
This is a FAT32-compatible content package using INTERNAL stock Tina, not an
independent Linux image, kernel/bootloader replacement or firmware update.

## Evidence and remaining hardware gate

The owner-selected exit-only Apps test returned to the stock menu. A later
independent probe verified NULL EGL surface creation at 1280x720, ES2 context,
make-current and explicit cleanup. It did not draw or swap. Slot's own draw,
swap, input, audio, gameplay/save and normal exit remain untested. This host
implementation does not prove full resource ownership or guarantee menu return
after a driver fault. MinUI takeover/shutdown logic is not used.

EGL resources are owned immediately; all applicable error and success paths
unbind, destroy context/surface, terminate and release the thread. Cleanup
errors are logged without skipping later releases. Forced process death and
driver/kernel hangs can bypass cleanup; no recovery action is authorized here.
No hardware library or device was probed during this host implementation.

## Verification

Run stock refusal/mount tests, SD write-boundary tests, mocked EGL cleanup
tests, TG5040 input/panel tests, stock-platform fixtures and non-stock checks.
Static ELF architecture, interpreter and version requirements must still fit
official 1.1.1. Host refusal tests execute no graphics/input/audio initialization.

## Stock scan-only revision — host preparation, not deployed

The owner confirmed the stock-only A/B correction on hardware. This revision
keeps that mapping, the Apps launcher, input guard and all cores unchanged.
It adds network discovery only. A Linux abstract UNIX-datagram client uses the
existing stock supplicant endpoint `/etc/wifi/sockets/wlan0`; no socket file,
helper process, configuration or DHCP client is created. ATTACH, SCAN,
SCAN_RESULTS and DETACH are the only protocol commands. One eight-second
deadline covers the request, completion event, results and best-effort detach.
No completion event means timeout, not a fabricated empty scan. Abstract-client
compatibility and actual scans on the vendor supplicant remain hardware gates.

The network page says scan-only and directs connection management to stock
Settings. Join, Disconnect, Forget, Auto-Join and radio power are unavailable.
The worker rejects mutation requests too, and all stock backend mutation methods
are no-ops. Slot's saved Wi-Fi-off preference is preserved but does not pretend
to control stock radio power. Missing endpoint, failure, timeout and a completed
empty scan have separate messages. No scan occurs merely by starting Slot:
scans are requested while its Scan page is open, using the existing worker's
refresh cycle. Back stops future refreshes; an in-flight operation can run until
its deadline. Existing status/connection display is not implemented by this
scan-only backend. Nonstock setup, configuration and DHCP behavior are unchanged.

No active endpoint command, SD staging or device launch is part of host checks.
The white/gray idle/wake fault is unresolved and is not changed by this revision.

## Direct stock boot — exact boundary, no new dispatcher prepared

Captured installed wrapper SHA-256:
`40efe40f3b30fc99326763c6120bb9d62cd15faf96314b9d44686d71bda0245d`.
Captured saved official 1.1.1 startup SHA-256:
`15cd707abc58016d564f32ec8501cbc01d2fc7fa80111ed8d29d78a7569d85fb`.
These are retained in the host startup-provenance evidence, not bundled as an
updater or internal firmware replacement.

The installed wrapper execs `.tmp_update/updater` if present, otherwise it calls
the saved official startup. That original runs `System/starts/*.sh` synchronously
before inputd. Waiting there deadlocks input readiness; backgrounding Slot races
the later MainUI. The present readiness guard remains unchanged and skips Slot.

In the official internal frontend branch, stock launches inputd and other
services, then runs MainUI foreground. There is no intervening SD callback.
`/tmp/cmd_to_run.sh` is consumed only AFTER MainUI returns; writing it early is
not evidence of an automatic handoff. No guessed dispatch file is supplied.

The exact official alternative is selected by an SD `trimui` directory. It
changes LD_LIBRARY_PATH to `trimui/lib`, launches named SD services and runs
`trimui/app/MainUI`, then `trimui/app/preload.sh`. This is NOT a safe one-file
shim: a partial tree can omit services, and the fallback status follows preload,
not Slot. The smallest next decision is whether to prepare this official SD
frontend branch with a complete matched runtime and a reviewed foreground Slot
dispatcher/fallback. That design needs verification of service readiness,
single display ownership, exit policy and complete runtime provenance before
staging. No runtime tree or MainUI replacement is included now.

The wrapper's updater override would bypass official initialization and require
a larger SD-supervisor design; MinUI's launcher additionally changes CPU/USB,
starts its own input monitor, kills network processes and ends in shutdown.
Those takeover behaviors are not adopted. No internal startup modification,
service kill/restart, background Slot or new OS image is proposed by this package.
If SD frontend ownership is not approved, normal Apps launch with the guarded
stock-menu boot is the supported route. Direct Slot boot is NOT solved.
