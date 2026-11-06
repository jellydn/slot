#!/usr/bin/env python3
"""Assemble a local test ZIP. Never open a device, SD volume or remote release."""

import argparse
import gzip
import hashlib
import io
import json
import shutil
import subprocess
import tarfile
import zipfile
from pathlib import Path

VERSION = "stock-tsp-0.3.0-test.9"
BASE = "7d14cc6c9b4017a5dc2739c5ea54279a70f7a14b"
IMAGE = "rust@sha256:4730e387a220a08a365c77da3096544dde214f9d796c16284d4be45438cad4a9"
SWAN_COMMIT = "631f0fb31b809e0f889d7c551fe695788e39c1d3"
SWAN_RUNTIME_SHA256 = "3b5d66f649aa1a1d53feaf102a14d6787ae337f758199d89cfd42ba0f9da94a8"
SWAN_GIT_TAR_SHA256 = "f5ead949860a003e928e581c4a2b154574d91672350f2887d152985cb122bb68"
PINS = {
    "fake-08": (
        "814991a2571ad3970e386cef48f3b148aa1c27b9",
        "libs/z8lua",
        "e6928578d46b61fd5ea30cfcf547e855a30a0553",
    ),
    "pcsx_rearmed": (
        "7b0e8f8fadfbce3d170d195b9d9eecaabfa08c8e",
        "frontend/libpicofe",
        "a2697389e76744cfa0c56c13a172845c45ede719",
    ),
}


def sha(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git(repo, *args):
    return subprocess.check_output(["git", "-C", str(repo), *args])


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def archive(target, members):
    with (
        target.open("wb") as raw,
        gzip.GzipFile(fileobj=raw, filename="", mode="wb", mtime=0) as gz,
        tarfile.open(fileobj=gz, mode="w|") as tar,
    ):
        for name, data, mode in members:
            info = tarfile.TarInfo(name)
            info.size, info.mode = len(data), mode
            tar.addfile(info, io.BytesIO(data))


def swan_inputs(source, runtime):
    """Read only pinned upstream bytes, never source-checkout extras or user BIOS."""
    if git(source, "rev-parse", "HEAD").decode().strip() != SWAN_COMMIT:
        raise ValueError("Swan source revision mismatch")
    if sha(runtime) != SWAN_RUNTIME_SHA256:
        raise ValueError(
            "Swan runtime hash mismatch; review a new build before packaging"
        )
    header = runtime.read_bytes()[:20]
    if (
        header[:6] != b"\x7fELF\x02\x01"
        or int.from_bytes(header[18:20], "little") != 183
    ):
        raise ValueError("Swan runtime must be AArch64 ELF64 little-endian")
    raw = git(source, "archive", "--format=tar", SWAN_COMMIT)
    if hashlib.sha256(raw).hexdigest() != SWAN_GIT_TAR_SHA256:
        raise ValueError("Swan pinned source archive hash mismatch")
    members = []
    with tarfile.open(fileobj=io.BytesIO(raw)) as tar:
        for member in tar:
            if member.isdir():
                continue
            path = Path(member.name)
            if not member.isfile() or path.is_absolute() or ".." in path.parts:
                raise ValueError("unsafe Swan source member")
            members.append(
                (
                    "swanstation/" + member.name,
                    tar.extractfile(member).read(),
                    member.mode,
                )
            )
    info = {
        "commit": SWAN_COMMIT,
        "runtime_sha256": SWAN_RUNTIME_SHA256,
        "upstream_git_tar_sha256": SWAN_GIT_TAR_SHA256,
        "source_archive": "Sources/swanstation-source.tar.gz",
        "source_manifest": "Sources/SWANSTATION-MANIFEST.json",
        "embedded_firmware": "MIT PCSX-Redux OpenBIOS; binary and generated header retained",
        "renderer": "Software, 1x, PGXP off",
        "states_resume": "disabled: snapshot/card reconciliation is not verified",
        "default_ps1_core": "pcsx_rearmed",
        "build_command": 'CFLAGS="-mcpu=cortex-a53" CXXFLAGS="-mcpu=cortex-a53" make -f Makefile.libretro platform=unix CPU_ARCH=aarch64 -j4',
        "container": IMAGE,
    }
    return members, info


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in [
        "baseline",
        "build",
        "web",
        "clones",
        "cc-license",
        "verification",
        "out",
    ]:
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument(
        "--swan-source", type=Path, help="optional pinned Swan Git checkout"
    )
    parser.add_argument(
        "--swan-core", type=Path, help="optional reviewed AArch64 Swan runtime"
    )
    args = parser.parse_args()
    if bool(args.swan_source) != bool(args.swan_core):
        parser.error("--swan-source and --swan-core must be supplied together")
    swan_members, swan_info = (
        swan_inputs(args.swan_source, args.swan_core) if args.swan_core else ([], None)
    )
    repo = Path(__file__).resolve().parents[2]
    out = args.out.resolve()
    # This tool writes only a new disposable host output, never a mounted card.
    assert out.is_relative_to(repo / ".amp/in"), "output must be under repo .amp/in"
    assert not out.exists(), "refuse to overwrite an existing candidate"
    subprocess.run(
        ["git", "-C", str(repo), "merge-base", "--is-ancestor", BASE, "HEAD"],
        check=True,
    )
    frozen = json.loads((args.baseline / "MANIFEST.json").read_text())
    for item in frozen["files"]:
        path = args.baseline / item["path"]
        assert path.stat().st_size == item["bytes"] and sha(path) == item["sha256"], (
            path
        )
    for path in [
        args.build / "target/device/slot",
        args.build / "fake08_libretro.so",
        args.build / "pcsx_bounded_libretro.so",
    ]:
        assert path.read_bytes()[:6] == b"\x7fELF\x02\x01", path
        assert int.from_bytes(path.read_bytes()[18:20], "little") == 183, path
    assert (args.web / "index.html").is_file()
    verification = json.loads(args.verification.read_text())
    assert verification["version"] == VERSION, "verification belongs to another build"
    assert (
        verification["ready_for_owner_test"] and not verification["hardware_accepted"]
    )
    if swan_info:
        assert verification.get("swanstation_authored_acceptance") is True, (
            "Swan requires real authored gameplay/input/audio/disc/card acceptance"
        )
    out.mkdir()
    tree = out / "fat32"
    shutil.copytree(args.baseline, tree)
    # All old stable metadata remains in the frozen baseline, not this candidate.
    for name in ["MANIFEST.json", "SHA256SUMS"]:
        (tree / name).unlink()
    system, sources = tree / "Slot/System", tree / "Slot/Sources"
    shutil.copyfile(args.build / "target/device/slot", system / "slot")
    for built, installed in [
        ("fake08_libretro.so", "fake08_libretro.so"),
        ("pcsx_bounded_libretro.so", "pcsx_rearmed_libretro.so"),
    ]:
        metadata = (args.build / (built + ".meta")).read_text()
        assert sha(args.build / built) in metadata, "core metadata hash mismatch"
        if built == "fake08_libretro.so":
            assert "fake08-slot-state.patch state_abi=" in metadata
        shutil.copyfile(args.build / built, system / installed)
        shutil.copyfile(args.build / (built + ".meta"), system / (installed + ".meta"))
    if swan_info:
        shutil.copyfile(args.swan_core, system / "swanstation_libretro.so")
        archive(sources / "swanstation-source.tar.gz", swan_members)
        swan_info["source_archive_sha256"] = sha(sources / "swanstation-source.tar.gz")
        write(
            sources / "SWANSTATION-MANIFEST.json",
            [
                {
                    "path": name,
                    "bytes": len(data),
                    "sha256": hashlib.sha256(data).hexdigest(),
                }
                for name, data, _ in swan_members
            ],
        )
        for name, data, _ in swan_members:
            if Path(name).name.lower().startswith(("license", "copying", "copyright")):
                dest = system / "licenses/ps1-pico8" / name
                dest.parent.mkdir(parents=True, exist_ok=True)
                dest.write_bytes(data)
        write(system / "swanstation_libretro.so.meta", swan_info)
    shutil.rmtree(system / "web")
    shutil.copytree(args.web, system / "web")
    here = Path(__file__).resolve().parent
    for name in ["NOTICE.md", "TEST-CHECKLIST.md", "SWANSTATION.md"]:
        shutil.copyfile(here / name, tree / "Slot" / name)
    shutil.copyfile(here / "README.md", tree / "Slot/RELEASE-NOTES.md")
    (tree / "Slot/README.md").write_text(
        f"# {VERSION}\n\nLocal combined PS1/fake-08/RomM test candidate; NOT hardware accepted.\n"
        "Read [the test checklist](TEST-CHECKLIST.md), [notes](RELEASE-NOTES.md) and [licences](NOTICE.md).\n"
        "Do not overwrite an unknown SD baseline. Preserve games, BIOS, saves, settings and splash.\n"
        "No user/proprietary game ROM, Sony BIOS, credential or user configuration is included. Stable 0.2.0 is unchanged.\n"
        + (
            "Optional SwanStation embeds MIT OpenBIOS; see SWANSTATION.md. PCSX is default; Swan states/resume are disabled.\n"
            if swan_info
            else ""
        )
        + "PICO-8: patched exact-build quick states/resume and cartdata; rewind disabled.\n"
        "Legacy/cross-build states and unsupported Lua persistence are not supported.\n"
        "Stock owns volume; Slot brightness uses live LCD readback without startup replay.\n"
        "PICO-8 test.3 states are incompatible; preserve refused snapshots and cartdata.\n"
        "PS1: Digital/DualShock and both sticks; no physical L3/R3 or rumble claim.\n"
        "Wi-Fi correction is included, but physical Power-button acceptance is still pending.\n"
        "See BUILD-INFO.json for hashes and source identity; Sources/ contains build materials.\n"
    )
    config = tree / "Apps/SlotStockTest/config.json"
    value = json.loads(config.read_text())
    value["description"] = (
        "PS1/fake-08/RomM test candidate; hardware acceptance pending."
    )
    write(config, value)
    frontend_files = (
        git(repo, "ls-files", "--cached", "--others", "--exclude-standard", "-z")
        .decode()
        .split("\0")
    )
    frontend, omitted_frontend = [], []
    for name in sorted(set(filter(None, frontend_files))):
        if name.startswith(".amp/") or "__pycache__" in Path(name).parts:
            continue  # Local previews and generated Python caches are not source.
        path = repo / name
        assert not path.is_symlink(), name
        if not path.is_file():
            continue
        assert not name.startswith((".amp/", "vendor/", "Games/", "Config/")), name
        if path.suffix.lower() in {
            ".gb",
            ".gbc",
            ".gba",
            ".nds",
            ".rom",
            ".chd",
            ".pbp",
        }:
            omitted_frontend.append(name)
            continue
        frontend.append(
            ("source/" + name, path.read_bytes(), path.stat().st_mode & 0o777)
        )
    archive(sources / "combined-frontend-source.tar.gz", frontend)
    source_manifest = [
        {"path": name, "sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data)}
        for name, data, _ in frontend
    ]
    write(sources / "FRONTEND-MANIFEST.json", source_manifest)
    (sources / "WORKTREE.patch").write_bytes(git(repo, "diff", "--binary", BASE))
    omissions = {}
    for core, (commit, sub, dependency) in PINS.items():
        checkout = args.clones / core
        assert git(checkout, "rev-parse", "HEAD").decode().strip() == commit
        assert git(checkout / sub, "rev-parse", "HEAD").decode().strip() == dependency
        members, omitted = [], []
        for root, revision, prefix in [
            (checkout, commit, ""),
            (checkout / sub, dependency, sub + "/"),
        ]:
            with tarfile.open(
                fileobj=io.BytesIO(git(root, "archive", revision))
            ) as tar:
                for member in tar:
                    if member.isdir():
                        continue
                    assert member.isfile() and ".." not in Path(member.name).parts
                    name = prefix + member.name
                    if name.startswith(("carts/", "test/carts/")):
                        omitted.append(name)
                        continue
                    data = tar.extractfile(member).read()
                    members.append((core + "/" + name, data, member.mode))
                    leaf = Path(name).name.lower()
                    if leaf.startswith(("license", "copying", "copyright")) or name in {
                        "frontend/libpicofe/README",
                        "libs/z8lua/README.md",
                    }:
                        dest = system / "licenses/ps1-pico8" / core / name
                        dest.parent.mkdir(parents=True, exist_ok=True)
                        dest.write_bytes(data)
        archive(sources / (core + "-source.tar.gz"), members)
        omissions[core] = omitted
    recipes = sources / "cores/ps1-pico8"
    if recipes.exists():
        shutil.rmtree(recipes)
    shutil.copytree(here, recipes, ignore=shutil.ignore_patterns("__pycache__"))
    shutil.copyfile(args.cc_license, system / "licenses/ps1-pico8/CC-BY-SA-3.0.txt")
    shutil.copyfile(here / "NOTICE.md", system / "licenses/ps1-pico8/NOTICE.md")
    write(tree / "Slot/VERIFICATION.json", verification)
    build = {
        "version": VERSION,
        "hardware_accepted": False,
        "source_checkpoint": git(repo, "rev-parse", "HEAD").decode().strip(),
        "source_is_uncommitted_snapshot": True,
        "fork_baseline": BASE,
        "wifi_handoff_fix_included": "0f486802b4283a0ca831a053445ae1e7cffa88f8",
        "frontend_source": "Sources/combined-frontend-source.tar.gz",
        "frontend_source_sha256": sha(sources / "combined-frontend-source.tar.gz"),
        "frontend_build": f"SLOT_VERSION={VERSION} cargo build --offline --locked -p slot --no-default-features --features stock-tg5040 --profile device",
        "container": IMAGE,
        "rust": "1.98.0",
        "core_pins": PINS,
        "optional_swanstation": swan_info,
        "core_build": "sh cores/ps1-pico8/build.sh CORE tg5040 PINNED_SOURCE NEW_WORKDIR OUTPUT",
        "omitted_compiled_frontend_test_roms": omitted_frontend,
        "omitted_uncompiled_vendor_carts": omissions,
        "runtime_sha256": {
            p.name: sha(p)
            for p in [
                system / "slot",
                system / "fake08_libretro.so",
                system / "pcsx_rearmed_libretro.so",
            ]
            + ([system / "swanstation_libretro.so"] if swan_info else [])
        },
        "limits": [
            "Hardware and sustained speed unverified",
            "No physical L3/R3 or rumble",
            "fake-08 states require this exact patched build; rewind disabled",
            "Unsupported/oversized Lua states fail explicitly; arbitrary carts unverified",
            "Stock owns volume; LCD brightness readback/adjustment needs handheld acceptance",
            "Wi-Fi Power-button acceptance pending",
            "No user ROM/BIOS or credentials",
        ],
    }
    write(tree / "Slot/BUILD-INFO.json", build)
    files = [
        {"path": str(p.relative_to(tree)), "bytes": p.stat().st_size, "sha256": sha(p)}
        for p in sorted(tree.rglob("*"))
        if p.is_file()
    ]
    write(tree / "MANIFEST.json", {"version": VERSION, "files": files})
    (tree / "SHA256SUMS").write_text(
        "".join(f"{f['sha256']}  {f['path']}\n" for f in files)
    )
    package = out / f"slot-tsp-{VERSION.removeprefix('stock-tsp-')}-fat32.zip"
    with zipfile.ZipFile(
        package, "x", compression=zipfile.ZIP_DEFLATED, compresslevel=9
    ) as z:
        for path in sorted(tree.rglob("*")):
            if path.is_file():
                assert path.stat().st_size < 2**32 - 1
                z.write(path, str(path.relative_to(tree)))
    with zipfile.ZipFile(package) as z:
        assert z.testzip() is None
        assert len(z.namelist()) == len(set(z.namelist()))
        for member in z.infolist():
            assert z.read(member) == (tree / member.filename).read_bytes()
    shutil.copyfile(tree / "Slot/BUILD-INFO.json", out / "PROVENANCE.json")
    shutil.copyfile(tree / "Slot/VERIFICATION.json", out / "VERIFICATION.json")
    shutil.copyfile(tree / "Slot/TEST-CHECKLIST.md", out / "TEST-CHECKLIST.md")
    (out / "SHA256SUMS").write_text(f"{sha(package)}  {package.name}\n")
    print(
        json.dumps(
            {
                "zip": str(package),
                "sha256": sha(package),
                "payload_files": len(files),
                "build": build,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
