# Stock frontend candidate — HOST ONLY

This is preparation for INTERNAL stock Tina 1.1.1, UI 20251128, on TG5040.
It is not a new OS, raw image, installer or firmware replacement. It has not
been staged or boot-tested. The original white → brief Slot UI → gray fault
is NOT fixed or reproduced by host tests. Do not deploy this directory yet.

## Phased plan and evidence

1. Preserve the working A/B normalization and scan-only Wi-Fi adapter. Keep
   `System/starts/slot.sh` and the normal Apps launcher byte-identical.
2. Use the complete official SD frontend branch, not the early starts hook.
   Official startup requests seven services before calling SD `app/MainUI`
   foreground, then SD `app/preload.sh`. Each SD entry restores installed
   `/usr/trimui` working/library paths and execs its matched internal binary.
   There is no copied vendor runtime, partial shared-library replacement,
   service killer/restart or background Slot child.
3. At MainUI dispatch, wait at most ten half-second intervals for keymon,
   inputd, scened, hardwareservice, musicserver and osdd process metadata, and
   a readable TRIMUI Player1 EV_KEY/EV_ABS character node. Do not read events.
   Bluetooth manager may exit with Bluetooth disabled; do not enable it.
   Run the existing Apps launcher foreground. MainUI is not launched until
   that child and its normal cleanup have returned. On refusal/timeout, use
   stock MainUI. A tmpfs marker prevents repeated automatic Slot attempts
   during the same boot after either failure or normal exit.
4. Give stock services exclusive physical Power policy. Slot checkpoints on
   press but does not interpret tap/hold/release as local doze or power-off.
   Lid pause/resume remains local; its timer cannot shut down under stock
   ownership. Critical battery checkpoints and requests clean Slot exit,
   not a machine power-off or the misleading Powering Down screen. The
   platform and device-loop refusal safeguards remain in place.
5. Keep real suspend/resume support gated on the stock power/display contract.
   Do not add keep-awake flags, disable sleep, guess kernel/register controls,
   or recreate EGL on a guessed wake event. A new GL context would require
   recreating compositor resources and frontend textures, not just rebinding.

The official startup SHA-256 is
`15cd707abc58016d564f32ec8501cbc01d2fc7fa80111ed8d29d78a7569d85fb`.
The previously captured installed wrapper SHA-256 is
`40efe40f3b30fc99326763c6120bb9d62cd15faf96314b9d44686d71bda0245d`.
Its updater override is not used or modified. A present updater can bypass
this design and must be checked before staging. The captured startup is
historical evidence, not proof of current hardware state.

## Explicit ownership

| Resource | Proposed owner and boundary |
| --- | --- |
| Initial GPIO, mounts, syslog, Bluetooth | Existing stock startup; no replacement init script |
| Controller translation | Installed stock trimui_inputd, launched/deduplicated by stock supervisor |
| Power button, idle, LCD/system suspend | Installed keymon; MainUI cooperation still unproved |
| Scene/function switch, hardware volume, OSD | Existing scened/hardwareservice/osdd, unchanged; overlay interactions require hardware validation |
| Main display frontend | Foreground Slot, then foreground stock MainUI only after Slot returns |
| EGL/GLES driver and scanout | Installed matched vendor stack; context survival across stock suspend unknown |
| Audio device | Slot ALSA child stream; existing stock audio services remain untouched; mixing/exclusivity unproved |
| Wi-Fi | Existing stock supplicant/config/DHCP/radio; Slot scan-only, no join/forget/autojoin/power ownership |
| Saves/settings/logs | Existing guarded Slot SD root; no new firmware writes |

These service entrypoints are adapters, not alternative service managers. Stock
still decides whether each daemon needs starting. A script failure returns 78
so the original supervisor can use its stock fallback branch. Runtime hashes
and private symlink targets must match before any adapter exec. Dependencies
outside `/usr/trimui/lib` still use the installed system runtime; ELF dependency
inspection is static evidence, not proof of service behavior or power safety.

The preload adapter runs the ORIGINAL installed preload script after MainUI
returns. That script changes CPU policy and removes `/tmp/stay_alive` and
`/tmp/stay_awake` as part of normal stock behavior. Its trailing missing-file
`rm` may return 1. The adapter logs that status and returns 0 so stock does not
start a second MainUI fallback after this dispatcher already ran MainUI.
Those normal stock operations must be included in the later boot-test approval.
No such operation has been executed during host preparation.

## Power evidence and unresolved cause

The owner reported idle white/inactive output, Power briefly showing Slot,
then gray; later Powering Down and return to stock. A previous bounded log
captured stock action refusal, save/return and child exit 0. That establishes
the final app exit, not a machine shutdown or the earlier graphics cause.

Slot's ordinary idle path does not automatically doze. Its doze draws black.
The existing 180-second timeout starts only AFTER local doze. Stock backlight
writes are no-ops. Thus removing the duplicate local Power handler resolves
a code-confirmed conflict, not proof of the white/gray incident's root cause.

Pinned official keymon strings contain LCD-off, `echo mem > /sys/power/state`,
`/tmp/system_suspend` and `/tmp/wakeup_from_suspend`; matched MainUI references
both markers. This is evidence that stock owns power and MainUI participates,
NOT a specified notification/acknowledgment API. No marker is created, removed
or consumed as a guessed protocol here. A pre-suspend/pre-poweroff save window,
OSD/scanout behavior, missed Power release and EGL context survival are open.
Current EGL swap error handling saves/stops and returns; it does not retry or
claim display recovery. Normal save/audio cleanup logs precede graphics Drop;
they do not prove EGL cleanup succeeded or that the stock menu is visible.

## Recovery and next approval

Offline `Slot/Config/stock-menu-only` bypasses Slot in both the existing early
guard and new foreground dispatcher. Alternatively rename the entire SD
`trimui` directory while powered off to restore the original internal branch.
This is an offline recovery switch, not a recovery command during a hung app.

Before any SD copy: reverify intended removable FAT32 media, current hashes,
absence of conflicting SD trimui/updater trees, and verified Mac backups.
Review the exact delta first. No format, internal startup edit or firmware
payload is required. No approved Wi-Fi deployment is implied.

Next recommended gate is a separately approved bounded READ-ONLY live survey
of relevant installed runtime hashes/link targets and power-marker/service
metadata, with no service action or live scan. Then review one cold-boot test
with offline rollback prepared. A separate observed sleep/wake test and its
bounded diagnostics need explicit approval; do not combine them with staging.
Primary boot and sleep/wake are not verified until those gates pass.

## Host checks

`prepare.py` accepts a new host output directory and the extracted, pinned
official runtime. It emits all nine SD entrypoints, dispatcher and runtime
pin manifests. It copies no proprietary binaries. Repository adapters use
GPL-3.0-or-later; no claim is made about redistribution rights to vendor code.
`test_dispatch.py` runs disposable Linux chroots containing only mock services
and frontends. It verifies sequential foreground ownership, once-per-boot
behavior, recovery, readiness timeout, hash/link refusal, all service paths and
preload status handling. Power tests use the public App/Session interfaces;
the generic Knulli/Anbernic power and input paths remain covered unchanged.
