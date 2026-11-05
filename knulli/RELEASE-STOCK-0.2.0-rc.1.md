# Stock TSP 0.2.0-rc.1 — RomM preview

Copy-to-FAT32 preview for TrimUI Smart Pro TG5040 on internal stock Tina
firmware 1.1.1. This is not firmware, a Knulli package, or a stable replacement
for stock-tsp-0.1.0. The rebuilt executable has not been tested on a handheld.

## Changes

- Network → RomM → Library lists GB, GBC and GBA games from a paired server.
- Left/Right switches systems; Up/Down selects; A downloads. Pages contain
  up to 20 games. Previous/Next page changes pages; A on the heading retries.
- B cancels or exits. Installed games appear on the shelf after exit.
- Single-ROM ZIPs are unpacked before installation. 7z and multi-file entries
  are unsupported. Existing games are not replaced.
- Pairing now requests platforms.read. Disconnect and pair again if an older
  pairing lacks that permission. Use a reachable LAN address, not localhost.

The same client is enabled in Knulli source builds, but this release ZIP is
stock-only. No Grout, box art, save sync, collections, ROM or BIOS is included.

## Verification and limits

Host checks cover browsing, paging, permissions, cancellation, installation,
stale replies and shelf refresh. Live RomM 5.4.1 checks covered libraries and
downloads for GB/GBC/GBA in a disposable host folder. Rendered UI states were
checked at 640×480 and 1280×720. Linux stock/device checks and the stock ARM64
build are recorded in the release verification asset.

Only Slot is rebuilt. Cores, web assets and startup adapters retain the stable
0.1.0 package bytes. See PROVENANCE.json for source, binary and toolchain hashes.
This is not a new hardware pass for startup, power, gameplay, saves or Wi-Fi.
The separate Wi-Fi handoff fix is not included. A working IP connection is
required; Network In Use after a power/wake sequence remains a known issue.
Real sleep/wake and physical LED-off remain unresolved. Use normal save/exit
and power controls, not suspend recovery.

## Install and recover

Follow the root README's stock install and offline recovery instructions.
Verify SHA256SUMS. Power off, back up the card, then copy Slot/Apps/System/trimui
to its root. Preserve Games, BIOS, Saves, States, Config and Logs. Review any
existing trimui startup tree before replacing files. Do not update a running
Slot executable or remove the card during use. No internal firmware write is
needed. Keep the stable 0.1.0 ZIP for rollback.

Keep Slot/Config/romm.conf private: it contains the pairing token. Complete
on-device browsing, download, archive, cancel/retry and launch checks before
treating this preview as suitable for daily use.
