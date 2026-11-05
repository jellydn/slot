#!/bin/sh
# Legacy refusal diagnostic. The prepared stock Apps entry is separate.
# Keep diagnostics on stderr so the supervisor can capture them on the host.
printf '%s\n' \
  'Slot stock TG5040: NOT READY TO LAUNCH' \
  'Required root: /mnt/SDCARD/Slot; board: tg5040.' \
  'BLOCKED: this legacy diagnostic is not the normal stock Apps launcher.' \
  'Full Slot presentation and gameplay remain unverified.' \
  'No Slot process, hardware probe, service command or log file was started.' \
  'Stock return: no takeover occurred; all existing stock processes remain untouched.' >&2
# Do not add PID guessing, killall, a reboot/shutdown fallback, or a runtime
# override. Use only the separately reviewed normal Apps candidate.
exit 78
