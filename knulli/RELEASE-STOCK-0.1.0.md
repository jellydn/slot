# Stock TSP 0.1.0 — 5 November 2026

First stable **copy-to-FAT32** package for TrimUI Smart Pro TG5040 running
internal stock Tina firmware 1.1.1 (UI 20251128, kernel 4.9.191).
Not a raw OS image, firmware updater, Knulli package or support for other models.

## What is included

- Native Slot frontend, corrected A/B input, selected SD splash and matched
  SD-only direct-start adapters. Stock kernel, GPU drivers and services remain.
- Device group: select Restart or Shutdown and press A; B returns. X/Y are
  inactive. Existing save/flush/settle, normal reboot/poweroff, duplicate-request
  guard and visible failure/retry handling are retained.
- Explicit Connect/Disconnect through the stock supplicant. One bounded owned
  wlan0 DHCP client acquires and retains renewal after matching association.
  Stock competitors/unmanaged network state fail closed with Network In Use.
  No killall, default stock DHCP hook or stock-service takeover.
- First-connect correction: defer the initial stock profile read until Hold;
  offer Connect for idle associated/no-IP state without a Disconnect round trip.
- Stock Lid Power Off and LED controls are Unavailable; Slot Fn actions remain
  disabled. No physical LED writer or substitute inactivity timer is added.
- Existing GB/GBC/GBA cores, licences and corresponding source offers.

## Evidence and limits

Owner reports direct/cold startup, corrected A/B, scan, Tools, real Restart and
Shutdown, selected SD splash, and now **“Wifi good now”** on the latest candidate.
The real power actions passed before the current A-select UI mapping; the latter
is covered by host regression tests, not a separately reported hardware pass.
Wi-Fi feedback is not Internet, persistence, repeated cold-boot/reconnect or
all-device acceptance. Gameplay/core/save/link compatibility is not exhaustive.

Real sleep/wake remains unresolved and has produced white/gray output. Use
normal save/exit and Device power controls; do not rely on suspend recovery.
Physical LED-off is not implemented. Stock lights/Fn policy can still operate.
The earliest internal pixel-handheld artwork and early LED flash remain before
the selected Slot SD splash. Removing those is not an SD-only change and is not
part of this release. No internal boot partition or firmware is modified.

## Exact tested executable and source provenance

The release does **not** rebuild or relabel the executable as hardware-tested.
Its SHA256 is:

`56539682478c77d33351890b6efcb7a9fb0660ea253a7f8880122591cbfbb7fa`

It is byte-identical to the owner's first-connect SD candidate, whose original
candidate ZIP SHA256 is
`953cc4271f22061e6957f8ab3e1efedfa890f4599ee5183d4848f4ca0f11fba8`.
The public FAT32 install ZIP is a new clean archive, not that historical delta
ZIP: it removes old candidate warnings, private host logs and packaging helpers,
and updates release documentation, provenance and manifests. Runtime executable,
cores, default art, web assets and startup adapters retain their tested bytes.
The exact tested source archive is retained locally and its hash is recorded.
The downloadable corresponding source omits the compiled homebrew RTC test ROM
but retains its C source; no game ROM is installed. Integrated release source
is available at the tag. Remote documentation through 21ec103 is preserved;
integration changes are disclosed, not described as a rebuilt hardware pass.
See the shipped provenance and verification records for exact hashes/results.

## Install, update and recover

Follow the root README. Verify the downloaded SHA256SUMS, back up a powered-off
device's FAT32 card, and copy Slot/Apps/System/trimui to its root. No formatting,
flashing, firmware updater or internal write is needed. Review existing startup
trees and conflicting SD updaters before overwrite. Preserve all user content.
The package has no ROMs, BIOS, credentials, personal config/logs or Grout runtime.

Offline recovery, with the device off and card on a host: create empty
`Slot/Config/stock-menu-only` to bypass Slot; or back up and rename the whole
card-root `trimui` directory to restore the internal startup branch. Restore
verified old files after a failed update. These are not running-hang recovery
commands. Never remove the card or replace its executable during a live session.

For Wi-Fi, select saved network → Connect → A once. This explicitly runs the
owned DHCP/scoped address/default-route/temporary-DNS lifecycle. Opening the
page alone does not acquire DHCP. Stop on Network In Use or immediate failure
and preserve the state for diagnosis; do not kill/restart stock clients blindly.
