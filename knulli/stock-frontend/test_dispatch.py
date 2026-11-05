"""Linux disposable chroot checks. Only mock services/frontends ever execute."""

import hashlib
import os
import shutil
import stat
import subprocess
import tempfile
from pathlib import Path

SOURCE = Path(__file__).with_name("dispatch.sh")
SERVICES = [
    "keymon",
    "trimui_inputd",
    "trimui_scened",
    "trimui_btmanager",
    "hardwareservice",
    "musicserver",
]


def write(base, path, text):
    target = base / path.lstrip("/")
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text)
    target.chmod(0o755)


def fixture(base):
    for directory in [
        "bin",
        "tmp",
        "lib/aarch64-linux-gnu",
        "usr/trimui/lib",
        "usr/trimui/osd",
        "mnt/SDCARD/trimui/lib",
        "mnt/SDCARD/trimui/app",
        "mnt/SDCARD/trimui/osd",
        "mnt/SDCARD/Slot/Config",
    ]:
        (base / directory).mkdir(parents=True, exist_ok=True)
    dependencies = set()
    for original, target in [
        (shutil.which("dash"), "bin/sh"),
        (shutil.which("sha256sum"), "bin/sha256sum"),
        (shutil.which("mkdir"), "bin/mkdir"),
        (shutil.which("rm"), "bin/rm"),
        (shutil.which("readlink"), "bin/readlink"),
    ]:
        shutil.copyfile(original, base / target)
        (base / target).chmod(0o755)
        linked = subprocess.run(
            ["ldd", original], capture_output=True, text=True, check=True
        )
        dependencies.update(
            word for word in linked.stdout.split() if word.startswith("/")
        )
    for original in dependencies:
        target = base / original.lstrip("/")
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(original, target)
        target.chmod(0o755)
    shutil.copyfile(SOURCE, base / "mnt/SDCARD/trimui/dispatch.sh")
    write(base, "/bin/sleep", "#!/bin/sh\nprintf 'wait\\n' >>/tmp/waits\n")
    runtime = base / "usr/trimui"
    pins = []
    for name in SERVICES + ["MainUI", "preload.sh", "trimui_osdd"]:
        path = ("osd/" if name == "trimui_osdd" else "bin/") + name
        text = '#!/bin/sh\n[ "$LD_LIBRARY_PATH" = /usr/trimui/lib ] || exit 90\n[ -z "${LD_PRELOAD-}${LD_AUDIT-}" ] || exit 91\n'
        text += (
            '[ "$PWD" = /usr/trimui/osd ] || exit 92\n'
            if name == "trimui_osdd"
            else '[ "$PWD" = /usr/trimui/bin ] || exit 92\n'
        )
        if name == "MainUI":
            text += "[ ! -e /tmp/mock-slot-active ] || exit 93\nprintf 'stock\\n' >>/tmp/trace\n"
        elif name == "preload.sh":
            text += "printf 'preload\\n' >>/tmp/trace\nexit 1\n"
        else:
            text += f"printf '{name}\\n' >>/tmp/trace\n"
        write(base, "/usr/trimui/" + path, text)
        pins.append(
            hashlib.sha256((runtime / path).read_bytes()).hexdigest()
            + "  /usr/trimui/"
            + path
            + "\n"
        )
    write(base, "/mnt/SDCARD/trimui/runtime.sha256", "".join(pins))
    (base / "usr/trimui/lib/test.so").symlink_to("optional-missing.so")
    write(
        base,
        "/mnt/SDCARD/trimui/runtime.links",
        "optional-missing.so /usr/trimui/lib/test.so\n",
    )
    write(
        base,
        "/mnt/SDCARD/Apps/SlotStockTest/launch.sh",
        "#!/bin/sh\nprintf 'slot start\\n' >>/tmp/trace\n: >/tmp/mock-slot-active\nrm /tmp/mock-slot-active\nprintf 'slot end\\n' >>/tmp/trace\nexit 7\n",
    )
    write(base, "/proc/213/comm", "trimui_inputd\n")
    for pid, name in enumerate(
        ["keymon", "trimui_scened", "hardwareservice", "musicserver", "trimui_osdd"],
        214,
    ):
        write(base, f"/proc/{pid}/comm", name + "\n")
    write(base, "/sys/class/input/event7/device/name", "TRIMUI Player1\n")
    write(base, "/sys/class/input/event7/device/capabilities/ev", "2b\n")
    (base / "dev/input").mkdir(parents=True)
    os.mknod(base / "dev/input/event7", stat.S_IFCHR | 0o600, os.makedev(1, 3))


def run(base, mode):
    return subprocess.run(
        ["chroot", str(base), "/bin/sh", "/mnt/SDCARD/trimui/dispatch.sh", mode],
        capture_output=True,
        timeout=5,
        check=False,
    )


for case in (
    [
        "foreground",
        "offline-bypass",
        "once-per-boot",
        "missing-input",
        "missing-keymon",
        "missing-hardware",
        "wrong-controller",
        "bad-hash",
        "bad-link",
        "linked-runtime",
        "preload",
    ]
    + SERVICES
    + ["trimui_osdd"]
):
    with tempfile.TemporaryDirectory() as directory:
        base = Path(directory)
        fixture(base)
        if case == "offline-bypass":
            (base / "mnt/SDCARD/Slot/Config/stock-menu-only").touch()
        if case == "once-per-boot":
            (base / "tmp/slot-frontend-return-to-stock").mkdir()
        if case == "missing-input":
            (base / "proc/213/comm").unlink()
        if case == "missing-keymon":
            (base / "proc/214/comm").unlink()
        if case == "missing-hardware":
            (base / "proc/216/comm").unlink()
        if case == "wrong-controller":
            (base / "sys/class/input/event7/device/name").write_text("Power Key\n")
        if case == "bad-hash":
            (base / "usr/trimui/bin/keymon").write_text("altered\n")
        if case == "bad-link":
            (base / "usr/trimui/lib/test.so").unlink()
            (base / "usr/trimui/lib/test.so").symlink_to("different.so")
        if case == "linked-runtime":
            (base / "mnt/SDCARD/trimui/lib").rmdir()
            (base / "mnt/SDCARD/trimui/lib").symlink_to("/usr/trimui/lib")
        result = run(
            base, case if case in SERVICES + ["trimui_osdd", "preload"] else "MainUI"
        )
        trace = (
            (base / "tmp/trace").read_text() if (base / "tmp/trace").exists() else ""
        )
        if case in ["bad-hash", "bad-link", "linked-runtime"]:
            assert result.returncode == 78 and trace == "", (case, result.stderr, trace)
        elif case == "foreground":
            assert (
                result.returncode == 0 and trace == "slot start\nslot end\nstock\n"
            ), (result.stderr, trace)
            assert run(base, "MainUI").returncode == 0
            assert (base / "tmp/trace").read_text() == trace + "stock\n"
        elif case in SERVICES + ["trimui_osdd", "preload"]:
            assert result.returncode == 0 and trace == case + "\n", (
                case,
                result.stderr,
                trace,
            )
        else:
            assert result.returncode == 0 and trace == "stock\n", (
                case,
                result.stderr,
                trace,
            )
            if case in [
                "missing-input",
                "missing-keymon",
                "missing-hardware",
                "wrong-controller",
            ]:
                assert (base / "tmp/waits").read_text().count("wait\n") == 10
        print("PASS frontend", case)
