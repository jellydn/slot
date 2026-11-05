# Stock Grout host preparation

**Retired from the native Slot package.** The owner requested a fully Slot-owned
frontend and removal of Grout. The following describes the historical package,
not a current launch recommendation. Native About Restart requests an actual
device reboot; it is no longer a route to stock Apps. Do not run this old
preparer for a native update. Review `UNINSTALL-PROPOSAL.json` before any
separately approved exact removal; preserve runtime config and unknown data.

Target: internal TrimUI Smart Pro TG5040 Tina 1.1.1 / UI20251128,
AArch64 glibc 2.33, FAT32 content card, RomM server **5.4.1**.
This directory does not change Slot, Wi-Fi, startup, a card, or internal firmware.

## Supported foreground flow

In the current full Slot frontend: **Shelf → Menu → About → X (save/exit)**.
Then use **stock MainUI → Apps → Grout**. Grout runs synchronously and returns
to the stock supervisor when it exits. There is no direct launch from Slot,
simultaneous frontend use, or automatic Slot resume. The wrapper does not stop
Slot or any stock service; complete the existing save/exit flow first.

## Prepare on the host

```sh
python3 prepare.py /host/path/Grout-Trimui-v5.4.1.0.zip /host/path/new-candidate
```

The parent must exist and must have no symlink components. Output must be a
new directory, not an existing tree or anything under `/Volumes` (including
linked paths). Output is `Apps/Grout/` plus `PROVENANCE.json`. This command does
not download, deploy, authenticate, or execute release code.

The generator checks the exact ZIP hash, paths, complete member set, types,
sizes, modes and CRC on read. It retains the original launcher, executable,
gfx library, MIT notice and other original files byte-for-byte, except for the
outer Apps descriptor's `launch` field. It adds `slot-launch.sh`. It creates
no inner runtime config, credentials, token, platform mappings or cache.

## Launcher guards

- Use the stock absolute entry `/mnt/SDCARD/Apps/Grout/slot-launch.sh` with no
  arguments. Missing, linked, or other roots are refused with status 78.
- The original launcher, binary and gfx library have embedded SHA256 pins.
  Changed or linked release files are refused before the original launcher runs.
- Loader overrides (`LD_PRELOAD`/`LD_AUDIT`) are refused. The exact absolute
  inner `grout/lib` and stock `/usr/trimui/lib` precede inherited library paths.
  This fixes upstream's invocation-dependent relative library path hazard.
  The upstream launcher's nonexistent outer `Grout/lib` must remain absent.
- Any `Apps/.update`, including a dangling symlink, blocks launch. The wrapper
  never copies, deletes, applies or clears pending updates.
- An exclusive `/tmp/slot-grout.lock` directory prevents two wrapper launches.
  It is removed on normal return. A stale lock causes safe refusal; do not clear
  it until an operator confirms that no Grout instance is active.
- The wrapper runs only the official launcher in the foreground, retains its
  child exit status, and performs no service, process-control, mount, CPU,
  power, driver, or runtime-config changes. It creates only its temporary lock.

### Updates are not disabled

The official in-app updater remains present. If explicitly used, it can download
and stage files under **Apps/.update** and immediately replace official
`Grout/launch.sh`. The next wrapper launch refuses the pending update and/or
changed hash. Review and host-repackage a server-matched release before any
future update. Do not claim this wrapper disables updater writes. The pending
check is not a defence against an unrelated writer racing after the check.

Back up inner `Apps/Grout/grout/config.json` before future manual replacement;
it can contain a bearer token. It is not the outer MainUI descriptor. Runtime
cache is inner `.cache/`; mappings/saves use stock `Roms`, `Imgs`, and
`RetroArch/.retroarch` paths. FAT32 limits each downloaded file to 4 GiB − 1.

## Release evidence

- Release: https://github.com/rommapp/grout/releases/tag/v5.4.1.0
- Revision: `1484116313cee19d12b92590c2057fa02bc3cba2`
- Release ID `427306752`; asset ID `686708099`; size `24836861` bytes.
- Asset: https://github.com/rommapp/grout/releases/download/v5.4.1.0/Grout-Trimui.zip
- SHA256: `4650d6dcee4b16a15929e7b77b3ea5268f3b6e48be875c7bc13c21dcc40acc0f`
- Official MIT notice is retained in `Apps/Grout/LICENSE`. New repository adapter
  code follows the repository licence; this does not replace the upstream notice.

## Checks

```sh
GROUT_OFFICIAL_ZIP=/host/path/Grout-Trimui-v5.4.1.0.zip python3 test_grout.py
```

Packaging checks run on the host. Launcher checks need Linux, root chroot access,
dash and coreutils; they use disposable mock roots and replace only the three
hash literals in an in-memory wrapper fixture. No downloaded script or binary
is executed. Run in a pinned cached Docker image with network disabled,
read-only source/archive mounts and a disposable executable `/tmp` tmpfs.
Tests cover output/ZIP refusal, byte/mode preservation, CWD/library/CFW values,
pending updates, linked roots/files, hash changes, child status, sequential
handoff and concurrent wrapper refusal. These are not on-device graphics,
audio, input, network, power or return-to-MainUI tests; those need separate approval.
