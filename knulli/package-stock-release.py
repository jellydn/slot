#!/usr/bin/env python3
"""Package frozen host-verified 0.2.0 runtime. Never opens a device or SD volume."""

import argparse
import gzip
import hashlib
import json
import shutil
import subprocess
import tarfile
import zipfile
from pathlib import Path

VERSION = "stock-tsp-0.2.0"
TESTED_ZIP = "953cc4271f22061e6957f8ab3e1efedfa890f4599ee5183d4848f4ca0f11fba8"
TESTED_BINARY = "56539682478c77d33351890b6efcb7a9fb0660ea253a7f8880122591cbfbb7fa"
HOST_ZIP = "e9d65399eb809aca33aefda87c1e5a31b831bc3db9c587ad431302703df1d7a1"
HOST_BINARY = "5bde5434a78c6923a61842753c8f20b7682dcd8c24a645dffdbd710fc59cc4ac"


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def source_archive(source, target):
    omitted = []
    with (
        tarfile.open(source) as original,
        target.open("wb") as raw,
        gzip.GzipFile(fileobj=raw, mode="wb", filename="", mtime=0) as gz,
        tarfile.open(fileobj=gz, mode="w|") as public,
    ):
        for member in original:
            if Path(member.name).suffix.lower() in {
                ".gba",
                ".gb",
                ".gbc",
                ".nds",
                ".rom",
            }:
                omitted.append(member.name)
                continue
            public.addfile(
                member, original.extractfile(member) if member.isfile() else None
            )
    return omitted


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--runtime-candidate", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    candidate, out = args.candidate.resolve(), args.out.resolve()
    runtime = args.runtime_candidate.resolve()
    repo = Path(__file__).resolve().parents[1]
    assert not out.exists(), "refuse overwriting release output"
    base = candidate / "proposed/payload"
    host = runtime / "proposed/payload"
    assert sha(base / "Slot/System/slot") == TESTED_BINARY
    assert sha(host / "Slot/System/slot") == HOST_BINARY
    for frozen, expected in [(candidate, TESTED_ZIP), (runtime, HOST_ZIP)]:
        assert sha(frozen.with_suffix(".zip")) == expected
        with zipfile.ZipFile(frozen.with_suffix(".zip")) as z:
            assert z.testzip() is None
            for member in z.infolist():
                assert not member.is_dir() and ".." not in Path(member.filename).parts
                assert z.read(member) == (frozen / member.filename).read_bytes()
    commit = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=repo, text=True
    ).strip()
    runtime_changes = subprocess.check_output(
        [
            "git",
            "diff",
            "--name-only",
            "0f486802b4283a0ca831a053445ae1e7cffa88f8",
            "HEAD",
        ],
        cwd=repo,
        text=True,
    ).splitlines()
    out.mkdir()
    tree = out / "fat32"
    tree.mkdir()
    # Explicit allowlist: never copy private verification logs, Config/Games,
    # old candidate warnings, Grout or internal firmware payloads.
    shutil.copytree(base / "Slot/System", tree / "Slot/System")
    shutil.copyfile(host / "Slot/System/slot", tree / "Slot/System/slot")
    shutil.copytree(base / "Apps/SlotStockTest", tree / "Apps/SlotStockTest")
    shutil.copytree(candidate / "proposed/trimui", tree / "trimui")
    app_config = tree / "Apps/SlotStockTest/config.json"
    metadata = json.loads(app_config.read_text())
    metadata["description"] = (
        "Slot TG5040 stock-OS frontend; see Slot release notes for verified scope and limitations."
    )
    write(app_config, metadata)
    frontend_provenance = tree / "trimui/PROVENANCE.json"
    metadata = json.loads(frontend_provenance.read_text())
    metadata["historical_preparation_status"] = metadata["status"]
    metadata["status"] = "STOCK_TSP_0_2_0_ADAPTER_PINS_UNCHANGED_HARDWARE_PENDING"
    write(frontend_provenance, metadata)
    (tree / "System/starts").mkdir(parents=True)
    shutil.copyfile(repo / "knulli/stock-start.sh", tree / "System/starts/slot.sh")
    sources = tree / "Slot/Sources"
    sources.mkdir()
    omitted = {}
    for name in ["gambatte-source.tar.gz", "gpsp-source.tar.gz", "mgba-source.tar.gz"]:
        omitted[name] = source_archive(base / "Slot/Sources" / name, sources / name)
    shutil.copytree(base / "Slot/Sources/cores", sources / "cores")
    shutil.copyfile(repo / "README.md", tree / "Slot/README.md")
    shutil.copyfile(
        repo / "knulli/RELEASE-STOCK-0.2.0.md", tree / "Slot/RELEASE-NOTES.md"
    )
    shutil.copyfile(repo / "LICENSE", tree / "Slot/LICENSE")
    tested_source = host / "Slot/Sources/stock-tg5040-source.tar.gz"
    source_asset = out / "slot-tsp-stock-0.2.0-source.tar.gz"
    omitted[source_asset.name] = source_archive(tested_source, source_asset)
    changed = []
    with tarfile.open(tested_source) as t:
        for m in t:
            if m.isdir():
                continue
            assert (
                m.isfile()
                and m.name.startswith("source/")
                and ".." not in Path(m.name).parts
            )
            path = repo / m.name.removeprefix("source/")
            if not path.is_file() or path.read_bytes() != t.extractfile(m).read():
                changed.append(m.name.removeprefix("source/"))
    # Integration must not silently alter anything used to build tested runtime.
    assert set(changed) <= {
        "README.md",
        "knulli/package-stock-release.py",
    }, changed
    for p in sorted(tree.rglob("*")):
        assert not p.is_symlink(), str(p)
        if p.is_file():
            relative = p.relative_to(tree).as_posix()
            assert not any(
                part
                in {
                    "Games",
                    "Roms",
                    "BIOS",
                    "Config",
                    "Saves",
                    "States",
                    "Logs",
                    "Verification",
                    "Grout",
                }
                for part in p.relative_to(tree).parts
            ), relative
            assert p.suffix.lower() not in {
                ".gba",
                ".gb",
                ".gbc",
                ".nds",
                ".rom",
                ".img",
                ".pem",
                ".key",
            }, relative
    for p in (tree / "Slot/System").rglob("*"):
        if p.is_file():
            reference = host if p.name == "slot" else base
            assert (
                p.read_bytes()
                == (
                    reference / "Slot/System" / p.relative_to(tree / "Slot/System")
                ).read_bytes()
            )
    for p in (tree / "trimui").rglob("*"):
        if p.is_file() and p.name != "PROVENANCE.json":
            assert (
                p.read_bytes()
                == (
                    candidate / "proposed/trimui" / p.relative_to(tree / "trimui")
                ).read_bytes()
            )
    with tarfile.open(tested_source) as t:
        assert (
            t.extractfile("source/knulli/stock-start.sh").read()
            == (tree / "System/starts/slot.sh").read_bytes()
        )
    provenance = {
        "release": VERSION,
        "source_commit": commit,
        "runtime_source_commit": "0f486802b4283a0ca831a053445ae1e7cffa88f8",
        "baseline_candidate_zip_sha256": TESTED_ZIP,
        "baseline_owner_tested_executable_sha256": TESTED_BINARY,
        "host_candidate_zip_sha256": HOST_ZIP,
        "host_verified_executable_sha256": HOST_BINARY,
        "host_source_archive_sha256": sha(tested_source),
        "corresponding_source_asset": source_asset.name,
        "corresponding_source_sha256": sha(source_asset),
        "omitted_compiled_test_fixtures": omitted,
        "source_download": f"https://github.com/jellydn/slot-tsp/releases/download/{VERSION}/{source_asset.name}",
        "runtime_rebuilt_from_0_1_0": True,
        "runtime_rebuilt_for_publication": False,
        "runtime_bytes_unchanged_from_host_candidate": True,
        "internal_build_label": "0.2.0-rc.1-stock-wake-candidate",
        "hardware_acceptance": "PENDING: not the owner-tested 0.1.0 executable",
        "release_metadata_changes": [
            "Apps/SlotStockTest/config.json description",
            "trimui/PROVENANCE.json release status",
            "Slot documentation and manifests",
        ],
        "integration_differences_from_tested_archive": changed,
        "release_commit_changed_paths_from_runtime_source": runtime_changes,
        "owner_evidence": "Historical 0.1.0 Wi-Fi/power/boot/splash reports do not establish hardware acceptance of this rebuilt combined binary.",
        "limits": "Combined RomM and Wi-Fi wake fix host-verified only. Internet/persistence/repeated reconnect/wake reliability unverified. Real sleep/wake unresolved; LED controls unavailable, Fn unwired; physical LEDs/internal artwork unchanged. SD deployment blocked by baseline conflict.",
        "source_fixture_notice": "Original exact tested archives retained locally. Public corresponding sources omit compiled test ROMs but retain their sources. No ROM or BIOS is installed by the FAT32 ZIP.",
    }
    write(tree / "Slot/BUILD-INFO.json", provenance)
    records = [
        {
            "path": p.relative_to(tree).as_posix(),
            "bytes": p.stat().st_size,
            "sha256": sha(p),
        }
        for p in sorted(tree.rglob("*"))
        if p.is_file()
    ]
    write(tree / "MANIFEST.json", {"version": VERSION, "files": records})
    (tree / "SHA256SUMS").write_text(
        "".join(f"{r['sha256']}  {r['path']}\n" for r in records)
        + sha(tree / "MANIFEST.json")
        + "  MANIFEST.json\n"
    )
    archive = out / "slot-tsp-stock-0.2.0-fat32.zip"
    with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as z:
        for p in sorted(tree.rglob("*")):
            if p.is_file():
                info = zipfile.ZipInfo(
                    p.relative_to(tree).as_posix(), (2026, 11, 5, 0, 0, 0)
                )
                info.compress_type = zipfile.ZIP_DEFLATED
                mode = (
                    0o755
                    if p.name == "slot"
                    or p.suffix == ".sh"
                    or p.parent.name in {"app", "osd"}
                    else 0o644
                )
                info.external_attr = (0o100000 | mode) << 16
                z.writestr(info, p.read_bytes())
    with zipfile.ZipFile(archive) as z:
        assert z.testzip() is None
        for m in z.infolist():
            assert z.read(m) == (tree / m.filename).read_bytes()
    shutil.copyfile(tree / "Slot/RELEASE-NOTES.md", out / "RELEASE-NOTES.md")
    write(out / "PROVENANCE.json", provenance)
    assets = [archive, source_asset, out / "PROVENANCE.json"]
    (out / "SHA256SUMS").write_text("".join(f"{sha(p)}  {p.name}\n" for p in assets))
    print(
        json.dumps(
            {
                "commit": commit,
                "install_zip_sha256": sha(archive),
                "payload_files": len(records),
                "source_differences": changed,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
