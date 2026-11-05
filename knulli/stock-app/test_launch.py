"""Host Linux chroot regression; only an authored mock child runs."""

import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


class Launcher(unittest.TestCase):
    def test_reused_pid_preserves_old_log_and_starts_foreground_child(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            for name in [
                "bin",
                "proc",
                "mnt/SDCARD/Slot/Logs",
                "mnt/SDCARD/Slot/System",
            ]:
                (base / name).mkdir(parents=True, exist_ok=True)
            shell = shutil.which("dash")
            shutil.copyfile(shell, base / "bin/sh")
            (base / "bin/sh").chmod(0o755)
            linked = subprocess.run(
                ["ldd", shell], capture_output=True, text=True, check=True
            )
            for name in {
                word for word in linked.stdout.split() if word.startswith("/")
            }:
                dest = base / name.lstrip("/")
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(name, dest)
                dest.chmod(0o755)
            (base / "proc/mounts").write_text("/dev/mock /mnt/SDCARD vfat rw 0 0\n")
            shutil.copyfile(Path(__file__).with_name("launch.sh"), base / "launch.sh")
            child = base / "mnt/SDCARD/Slot/System/slot"
            child.write_text("#!/bin/sh\nprintf 'mock child\\n'\nexit 7\n")
            child.chmod(0o755)
            result = subprocess.run(
                [
                    "chroot",
                    str(base),
                    "/bin/sh",
                    "-c",
                    'printf "old boot\\n" >"/mnt/SDCARD/Slot/Logs/stock-app-$$.log"; exec /bin/sh /launch.sh',
                ],
                capture_output=True,
                check=False,
                timeout=5,
            )
            self.assertEqual(result.returncode, 7, result.stderr)
            logs = sorted((base / "mnt/SDCARD/Slot/Logs").glob("*.log"))
            self.assertEqual(len(logs), 2)
            self.assertEqual(sum(p.read_text() == "old boot\n" for p in logs), 1)
            self.assertTrue(
                any("mock child\nSlot child exit=7" in p.read_text() for p in logs)
            )


if __name__ == "__main__":
    unittest.main()
