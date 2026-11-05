"""Prepare a new host-only Apps tree from the exact official Grout release."""

import argparse
import hashlib
import io
import json
import stat
import zipfile
from pathlib import Path, PurePosixPath

ZIP_SHA256 = "4650d6dcee4b16a15929e7b77b3ea5268f3b6e48be875c7bc13c21dcc40acc0f"
MEMBERS = {
    "Grout/": (0, 0o40755),
    "Grout/grout.png": (15665, 0o100644),
    "Grout/config.json": (153, 0o100644),
    "Grout/grout/": (0, 0o40755),
    "Grout/grout/grout": (48647736, 0o100755),
    "Grout/grout/lib/": (0, 0o40755),
    "Grout/grout/lib/libSDL2_gfx-1.0.so.0": (65344, 0o100644),
    "Grout/LICENSE": (1096, 0o100644),
    "Grout/README.md": (4715, 0o100644),
    "Grout/launch.sh": (264, 0o100755),
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def validate_members(archive):
    """Validate even if called separately; never trust ZIP extraction defaults."""
    entries = archive.infolist()
    names = [entry.filename for entry in entries]
    for entry in entries:
        path = PurePosixPath(entry.filename)
        mode = entry.external_attr >> 16
        if (
            path.is_absolute()
            or ".." in path.parts
            or "\\" in entry.filename
            or entry.filename.rstrip("/") != path.as_posix()
            or not (stat.S_ISREG(mode) or stat.S_ISDIR(mode))
            or entry.flag_bits & 1
        ):
            raise ValueError("unsafe archive path or member type")
    if len(names) != len(set(names)) or set(names) != set(MEMBERS):
        raise ValueError("unexpected archive member set")
    for entry in entries:
        if (entry.file_size, entry.external_attr >> 16) != MEMBERS[entry.filename]:
            raise ValueError("unexpected archive size or mode")
    return entries


def prepare(official_zip, output):
    output = Path(output).absolute()
    if (
        output.exists()
        or output.is_symlink()
        or not output.parent.is_dir()
        or any(p.is_symlink() for p in [output, *output.parents])
        or "Volumes" in output.parts
        or "Volumes" in output.resolve().parts
    ):
        raise ValueError(
            "new host directory with existing unlinked parent required; never /Volumes"
        )
    data = Path(official_zip).read_bytes()
    if sha(data) != ZIP_SHA256:
        raise ValueError("official ZIP SHA256 mismatch")
    # Read from the verified bytes, not a path that can change after verification.
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        entries = validate_members(archive)
        payload = {entry.filename: archive.read(entry) for entry in entries}
    wrapper = Path(__file__).with_name("slot-launch.sh").read_bytes()
    output.mkdir(mode=0o755)
    (output / "Apps").mkdir(mode=0o755)
    for name, content in payload.items():
        target = output / "Apps" / name
        mode = MEMBERS[name][1]
        if stat.S_ISDIR(mode):
            target.mkdir(mode=stat.S_IMODE(mode))
            target.chmod(stat.S_IMODE(mode))
        else:
            target.write_bytes(content)
            target.chmod(stat.S_IMODE(mode))
    app = output / "Apps/Grout"
    descriptor = json.loads(payload["Grout/config.json"])
    descriptor["launch"] = "slot-launch.sh"
    (app / "config.json").write_text(json.dumps(descriptor, indent=2) + "\n")
    (app / "slot-launch.sh").write_bytes(wrapper)
    (app / "slot-launch.sh").chmod(0o755)
    files = {
        path.relative_to(output).as_posix(): sha(path.read_bytes())
        for path in sorted(app.rglob("*"))
        if path.is_file()
    }
    (output / "PROVENANCE.json").write_text(
        json.dumps(
            {
                "status": "HOST_ONLY_NOT_DEVICE_TESTED",
                "server_version": "5.4.1",
                "tag": "v5.4.1.0",
                "revision": "1484116313cee19d12b92590c2057fa02bc3cba2",
                "release_id": 427306752,
                "asset_id": 686708099,
                "url": "https://github.com/rommapp/grout/releases/download/v5.4.1.0/Grout-Trimui.zip",
                "zip_sha256": ZIP_SHA256,
                "license": "Official Grout MIT notice retained in Apps/Grout/LICENSE",
                "changes": ["outer descriptor launch", "separate slot-launch.sh"],
                "files_sha256": files,
            },
            indent=2,
        )
        + "\n"
    )
    return output


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("official_zip", type=Path)
    parser.add_argument("new_host_output", type=Path)
    args = parser.parse_args()
    prepare(args.official_zip, args.new_host_output)
