"""Host packaging and disposable Linux chroot tests. Release code never runs."""

import hashlib
import io
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import time
import unittest
import zipfile
from pathlib import Path

import prepare

SOURCE = Path(__file__).with_name("slot-launch.sh")
ROOT = "/mnt/SDCARD/Apps/Grout"
ENTRY = ROOT + "/slot-launch.sh"
PINS = {
    "launch.sh": "5b51e2149a192823662a07a6200d86b1f43343bee81e9ea742aa2e7433951e12",
    "grout/grout": "9d5c6a51683462aa81ce83baef65940c65105603e6a7b3ef588ef815b6ed6958",
    "grout/lib/libSDL2_gfx-1.0.so.0": "8f124e2666901f68913fcce997816c167ab28a5723c709803eed4d91ad1f24ad",
}


class Packaging(unittest.TestCase):
    def test_wrong_zip(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory).resolve()
            archive = base / "wrong.zip"
            archive.write_bytes(b"not the release")
            with self.assertRaisesRegex(ValueError, "SHA256"):
                prepare.prepare(archive, base / "out")
            self.assertFalse((base / "out").exists())

    def test_unsafe_members(self):
        for name, mode in [
            ("../escape", 0o100644),
            ("/absolute", 0o100644),
            ("Grout/../escape", 0o100644),
            ("Grout/link", 0o120777),
            ("Grout/fifo", 0o010644),
            ("Grout//file", 0o100644),
        ]:
            with self.subTest(name=name):
                data = io.BytesIO()
                with zipfile.ZipFile(data, "w") as archive:
                    entry = zipfile.ZipInfo(name)
                    entry.external_attr = mode << 16
                    archive.writestr(entry, b"")
                data.seek(0)
                with zipfile.ZipFile(data) as archive, self.assertRaises(ValueError):
                    prepare.validate_members(archive)

    def test_member_set(self):
        data = io.BytesIO()
        with zipfile.ZipFile(data, "w"):
            pass
        data.seek(0)
        with (
            zipfile.ZipFile(data) as archive,
            self.assertRaisesRegex(ValueError, "member set"),
        ):
            prepare.validate_members(archive)

    def test_output_guards(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory).resolve()
            (base / "linked").symlink_to(base, target_is_directory=True)
            (base / "Volumes").mkdir()
            for output in [base, base / "linked/new", base / "Volumes/new"]:
                with self.subTest(output=output), self.assertRaises(ValueError):
                    prepare.prepare(base / "missing.zip", output)

    def test_official_preparation(self):
        archive = os.environ.get("GROUT_OFFICIAL_ZIP")
        if not archive:
            self.skipTest("set GROUT_OFFICIAL_ZIP for pinned asset checks")
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory).resolve() / "candidate"
            prepare.prepare(archive, output)
            with zipfile.ZipFile(archive) as original:
                for name, (_, mode) in prepare.MEMBERS.items():
                    path = output / "Apps" / name
                    self.assertEqual(
                        stat.S_IMODE(path.stat().st_mode), stat.S_IMODE(mode)
                    )
                    if not name.endswith("/") and name != "Grout/config.json":
                        self.assertEqual(path.read_bytes(), original.read(name))
            app = output / "Apps/Grout"
            self.assertEqual(
                json.loads((app / "config.json").read_text())["launch"],
                "slot-launch.sh",
            )
            self.assertFalse((app / "grout/config.json").exists())
            for name, pin in PINS.items():
                self.assertEqual(prepare.sha((app / name).read_bytes()), pin)
            self.assertIn("MIT License", (app / "LICENSE").read_text())
            self.assertEqual((app / "slot-launch.sh").stat().st_mode & 0o777, 0o755)
            with self.assertRaises(ValueError):
                prepare.prepare(archive, output)


def write(base, path, data):
    target = base / path.lstrip("/")
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(data)
    target.chmod(0o755)
    return target


def fixture(base):
    for directory in ["bin", "dev", "tmp", "usr/bin", "usr/trimui/lib"]:
        (base / directory).mkdir(parents=True, exist_ok=True)
    (base / "dev/null").touch()
    dependencies = set()
    for name in ["dash", "sha256sum", "mkdir", "rmdir", "sleep"]:
        original = shutil.which(name)
        target = base / "bin" / ("sh" if name == "dash" else name)
        shutil.copyfile(original, target)
        target.chmod(0o755)
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
    # These are authored fixtures, not downloaded scripts or executable content.
    write(
        base,
        ROOT + "/launch.sh",
        '#!/bin/sh\ncd "${0%/*}/grout" || exit 90\nexport CFW=TRIMUI\nexport LD_LIBRARY_PATH="${0%/*}/lib:$LD_LIBRARY_PATH"\n./grout\n',
    )
    write(
        base,
        ROOT + "/grout/grout",
        '#!/bin/sh\n[ ! -e /tmp/mock-slot-active ] || exit 91\nprintf "child-start\\n%s\\n%s\\n%s\\n" "$PWD" "$LD_LIBRARY_PATH" "$CFW" >>/tmp/trace\n[ ! -e /tmp/slow ] || sleep 1\nprintf "child-end\\n" >>/tmp/trace\nexit "${MOCK_EXIT:-0}"\n',
    )
    write(
        base,
        ROOT + "/grout/lib/libSDL2_gfx-1.0.so.0",
        "fixture, not a shared library\n",
    )
    wrapper = SOURCE.read_text()
    for name, pin in PINS.items():
        assert wrapper.count(pin) == 1
        mock_pin = hashlib.sha256(
            (base / ROOT.lstrip("/") / name).read_bytes()
        ).hexdigest()
        wrapper = wrapper.replace(pin, mock_pin)
    write(base, ENTRY, wrapper)


@unittest.skipUnless(sys.platform == "linux", "Linux disposable chroot required")
class Launcher(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.base = Path(self.temporary.name)
        fixture(self.base)

    def tearDown(self):
        self.temporary.cleanup()

    def command(self, entry=ENTRY, args=()):
        return ["chroot", str(self.base), "/bin/sh", entry, *args]

    def run_launcher(self, entry=ENTRY, args=(), **environment):
        return subprocess.run(
            self.command(entry, args),
            env={**os.environ, **environment},
            capture_output=True,
            timeout=5,
            check=False,
        )

    def trace(self):
        path = self.base / "tmp/trace"
        return path.read_text() if path.exists() else ""

    def test_success_and_child_exit(self):
        for status in [0, 7, 78]:
            result = self.run_launcher(
                LD_LIBRARY_PATH="/stock/lib:/custom/lib", MOCK_EXIT=str(status)
            )
            self.assertEqual(result.returncode, status, result.stderr)
            self.assertFalse((self.base / "tmp/slot-grout.lock").exists())
        self.assertEqual(self.trace().count("child-end"), 3)
        self.assertIn(
            ROOT
            + "/grout\n"
            + ROOT
            + "/lib:"
            + ROOT
            + "/grout/lib:/usr/trimui/lib:/stock/lib:/custom/lib\nTRIMUI\n",
            self.trace(),
        )

    def test_pending_update(self):
        path = self.base / "mnt/SDCARD/Apps/.update"
        for kind in ["directory", "dangling-link", "file"]:
            if kind == "directory":
                path.mkdir()
            elif kind == "dangling-link":
                path.symlink_to("missing")
            else:
                path.write_text("keep")
            self.assertEqual(self.run_launcher().returncode, 78)
            self.assertEqual(self.trace(), "")
            self.assertTrue(path.exists() or path.is_symlink())
            path.rmdir() if kind == "directory" else path.unlink()

    def test_args_and_wrong_root(self):
        self.assertEqual(self.run_launcher(args=("extra",)).returncode, 78)
        write(self.base, "/tmp/slot-launch.sh", SOURCE.read_text())
        self.assertEqual(self.run_launcher(entry="/tmp/slot-launch.sh").returncode, 78)
        self.assertEqual(self.trace(), "")

    def test_stale_lock_and_outer_lib(self):
        lock = self.base / "tmp/slot-grout.lock"
        lock.mkdir()
        self.assertEqual(self.run_launcher().returncode, 78)
        self.assertTrue(lock.is_dir())
        lock.rmdir()
        outer = self.base / ROOT.lstrip("/") / "lib"
        outer.mkdir()
        self.assertEqual(self.run_launcher().returncode, 78)
        self.assertEqual(self.trace(), "")

    def test_tampered_pins(self):
        for name in PINS:
            path = self.base / ROOT.lstrip("/") / name
            content = path.read_bytes()
            path.write_bytes(content + b"tampered\n")
            self.assertEqual(self.run_launcher().returncode, 78)
            self.assertEqual(self.trace(), "")
            path.write_bytes(content)

    def test_linked_paths(self):
        for name in ["grout/lib", "grout", "launch.sh", "slot-launch.sh", "."]:
            path = self.base / ROOT.lstrip("/") / name
            if name == ".":
                path = self.base / ROOT.lstrip("/")
            target = path.with_name(path.name + "-real")
            path.rename(target)
            path.symlink_to(target.name)
            self.assertEqual(self.run_launcher().returncode, 78, name)
            self.assertEqual(self.trace(), "")
            path.unlink()
            target.rename(path)

    def test_sequential_stock_flow(self):
        # The existing Slot save/exit completes before stock Apps can launch Grout.
        write(self.base, "/tmp/mock-slot-active", "active")
        self.assertEqual(self.run_launcher().returncode, 91)
        self.assertEqual(self.trace(), "")
        (self.base / "tmp/mock-slot-active").unlink()
        self.assertEqual(self.run_launcher().returncode, 0)
        self.assertTrue(self.trace().endswith("child-end\n"))

    def test_concurrent_launch_refused(self):
        write(self.base, "/tmp/slow", "slow")
        with subprocess.Popen(
            self.command(), stdout=subprocess.PIPE, stderr=subprocess.PIPE
        ) as first:
            deadline = time.monotonic() + 3
            while "child-start" not in self.trace() and time.monotonic() < deadline:
                time.sleep(0.01)
            self.assertIn("child-start", self.trace())
            self.assertEqual(self.run_launcher().returncode, 78)
            first.communicate(timeout=5)
            self.assertEqual(first.returncode, 0)
        self.assertEqual(self.trace().count("child-start"), 1)
        self.assertTrue(self.trace().endswith("child-end\n"))


if __name__ == "__main__":
    unittest.main(verbosity=2)
