"""Prepare inactive SD adapters from the pinned official runtime. No device I/O."""

import hashlib
import json
import shutil
from pathlib import Path

SERVICES = [
    "keymon",
    "trimui_inputd",
    "trimui_scened",
    "trimui_btmanager",
    "hardwareservice",
    "musicserver",
]
STARTUP_SHA256 = "15cd707abc58016d564f32ec8501cbc01d2fc7fa80111ed8d29d78a7569d85fb"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def prepare(official, output):
    """Only a NEW host directory is accepted; never an SD mount or existing tree."""
    official = Path(official)
    output = Path(output)
    if (
        output.exists()
        or output.is_symlink()
        or str(output.resolve()).startswith("/Volumes/")
    ):
        raise ValueError("new host output required, not a mounted volume")
    if sha(official / "bin/runtrimui.sh") != STARTUP_SHA256:
        raise ValueError("official startup mismatch")
    paths = [official / "bin" / name for name in SERVICES + ["MainUI", "preload.sh"]]
    paths += [official / "osd/trimui_osdd"]
    # Do not redistribute vendor binaries. Pin every installed private library,
    # including each symlink name, against the official extraction instead.
    paths += sorted((official / "lib").iterdir())
    pins = [
        (STARTUP_SHA256, "/usr/trimui/bin/runtrimui-original.sh"),
        (
            "40efe40f3b30fc99326763c6120bb9d62cd15faf96314b9d44686d71bda0245d",
            "/usr/trimui/bin/runtrimui.sh",
        ),
    ]
    links = []
    for path in paths:
        if not path.resolve().is_relative_to(official.resolve()):
            raise ValueError(f"missing or external official runtime entry: {path.name}")
        installed = "/usr/trimui/" + path.relative_to(official).as_posix()
        if path.is_symlink():
            links.append((str(path.readlink()), installed))
        if path.is_file():
            pins.append((sha(path), installed))
        elif not path.is_symlink():
            raise ValueError(f"missing official runtime entry: {path.name}")
    output.mkdir(parents=True)
    for directory in ["app", "osd", "lib"]:
        (output / directory).mkdir()
    shutil.copyfile(Path(__file__).with_name("dispatch.sh"), output / "dispatch.sh")
    for name in SERVICES + ["MainUI", "preload.sh", "trimui_osdd"]:
        relative = ("osd/" if name == "trimui_osdd" else "app/") + name
        mode = "preload" if name == "preload.sh" else name
        (output / relative).write_text(
            "#!/bin/sh\nexec /bin/sh /mnt/SDCARD/trimui/dispatch.sh " + mode + "\n"
        )
        (output / relative).chmod(0o755)
    (output / "runtime.sha256").write_text(
        "".join(f"{digest}  {path}\n" for digest, path in pins)
    )
    (output / "runtime.links").write_text(
        "".join(f"{target} {path}\n" for target, path in links)
    )
    (output / "lib/README.txt").write_text(
        "Intentionally no copied libraries. All entrypoints restore /usr/trimui/lib.\n"
    )
    (output / "PROVENANCE.json").write_text(
        json.dumps(
            {
                "status": "HOST_CANDIDATE_NOT_APPROVED_FOR_STAGING_OR_BOOT",
                "official_firmware_zip_sha256": "43950ed504b83cca2f553168bf8dfcc17e6d689c02dc621f0f0f26954d182d18",
                "official_rootfs_sha256": "7e54f673899c824d3043f2eb9ebc81da622ecff0812c13a3fc010d27b573cb11",
                "official_startup_sha256": STARTUP_SHA256,
                "dispatch_sha256": sha(output / "dispatch.sh"),
                "runtime_manifest_sha256": sha(output / "runtime.sha256"),
                "runtime_links_sha256": sha(output / "runtime.links"),
                "unresolved_official_private_links": [
                    str(p.relative_to(official))
                    for p in paths
                    if p.is_symlink() and not p.exists()
                ],
                "vendor_binaries_distributed": False,
                "adapter_license": "GPL-3.0-or-later; repository license",
                "runtime": "Existing internal stock 1.1.1 only; no firmware installer or internal startup edit",
                "power_gate": "MainUI/keymon suspend handshake and EGL resume remain unverified",
            },
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("official_runtime", type=Path)
    parser.add_argument("new_host_output", type=Path)
    args = parser.parse_args()
    prepare(args.official_runtime, args.new_host_output)
