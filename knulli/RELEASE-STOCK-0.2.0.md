# Stock TSP 0.2.0 — 5 November 2026

Stable-channel **copy-to-FAT32** package for TrimUI Smart Pro TG5040 on
internal stock Tina 1.1.1 (UI 20251128, kernel 4.9.191). Not an OS image,
firmware updater, Knulli package or support for another model.

**Combined hardware acceptance is pending.** This is a rebuilt executable,
not the owner's previously tested 0.1.0 binary. Stable identifies the release
channel; it does not mean the handheld has passed every function or wake cycle.

## Included changes

- Built-in paired RomM client: **Network → RomM → Library**, GB/GBC/GBA
  paging and downloads through Slot's existing installer. A downloads; B
  cancels/returns; Left/Right selects platform. Supports individual ROMs and
  ZIPs containing exactly one playable ROM. No 7z, multi-file downloads,
  box-art sync, save sync or collections. See README for pairing permissions.
- Stock Wi-Fi wake stale-busy correction: after stock takes over wlan0, Slot
  stops masking later stock addresses. A fresh completed association with
  valid IPv4 can clear only that network's stale Network In Use indicator.
  It does not adopt, kill or restart stock DHCP clients, bypass competitor
  guards or start another client during passive recovery. Missing addresses
  and unrelated failures remain visible. This is not a general suspend fix.
- Existing Device A-select Restart/Shutdown, B Back, save/flush/settle and
  failure handling remain. Startup adapters, cores, art and web runtime retain
  their baseline bytes. No new physical LED or internal-logo writer.

## Verification and limits

- Combined source: upstream RomM `eec7687` plus Wi-Fi correction `0f48680`.
- **616 pinned Linux Rust test executions**, stock/nonstock Wi-Fi, power,
  RomM/UI, launcher/dispatcher mocks and ARM64 build/ABI passed. Maximum
  required GLIBC 2.30; target provides 2.33.
- **229 clean macOS integration tests**, scoped strict Clippy, rustfmt and
  diff checks passed after integration. These are host results, not hardware.
- The full workspace run before the later clean upstream rebase reported
  **3,121 passed, four failed, three ignored**. All four failures also
  reproduce on unchanged latest upstream `eec7687`: emulation display-stall
  timing (intermittent), link screen opening, Tools alignment (71.5 px), and
  header-index dirty-state assertion. No full-suite green claim. Per the
  delivery instructions, the full suite was not repeated after the clean
  rebase; combined checks above cover the integration.
- Exact-head GitHub CI for the Wi-Fi commit reported no checks, not CI PASS.
  Release packaging is independently checked for bytes, hashes and exclusions.

Prior 0.1.0 owner reports of boot, A/B, scan, Tools, real power actions,
first-connect Wi-Fi and selected SD splash are historical. They are not
acceptance of this combined build, RomM on the handheld or wake recovery.
Internet, profile persistence, repeated reconnect/cold-boot/wake reliability,
game/core/save/link compatibility and real sleep/wake are not fully verified.
Sleep/wake has produced white/gray output; prefer normal save/exit and Device
power controls. Physical LED-off remains unavailable, Fn is disabled in Slot,
and early internal artwork/LED flash can remain before the SD splash.

## Frozen executable and sources

Executable SHA256:
`5bde5434a78c6923a61842753c8f20b7682dcd8c24a645dffdbd710fc59cc4ac`.

This release reuses that exact host-verified combined candidate, with no
additional executable rebuild for publication. Its internal About/build label
remains **0.2.0-rc.1-stock-wake-candidate** to preserve those bytes; the GitHub
release/tag and install package are stock-tsp-0.2.0. This label is disclosed,
not evidence that the RomM-only RC contains the Wi-Fi fix. Runtime source is
commit `0f48680`; tag integration adds only release docs and packaging changes.
Provenance records full commits, input hashes and corresponding sources.
Download the source, PROVENANCE.json, VERIFICATION.json and SHA256SUMS assets.
Compiled homebrew test-ROM artifacts are omitted from corresponding archives;
their source remains available. No game ROM or BIOS is installed.

## Installation and recovery

Follow README: power off normally, back up the intended FAT32 card, verify
SHA256SUMS, then copy Slot/Apps/System/trimui to the card root. Preserve all
games, BIOS, saves, states, settings, selected splash and private RomM pairing.
No ROMs, BIOS, credentials, personal config/logs, Grout runtime, raw image,
internal boot-logo writer or firmware payload is included. Review existing
startup trees/updaters; stop on conflicts. Never hot-replace a running binary.

Offline recovery with the device off and card on a host: create empty
`Slot/Config/stock-menu-only`, or back up and rename the whole root `trimui`
directory to restore the internal startup branch. Restore verified old files
after a failed update. These are not running-hang recovery commands.

The owner's current card update is **blocked**: two expected baseline checks
failed and the executable then became unavailable. No backup or SD write was
performed in that attempt. Publication does not authorize blind overwrite;
resolve the stable mount/baseline and repeat exact checks/backups first.
