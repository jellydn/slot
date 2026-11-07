#!/usr/bin/env python3
"""Build original offline PS1 test discs with LLVM; never access a device."""

import argparse
import hashlib
import json
import shutil
import struct
import subprocess
import tempfile
from pathlib import Path


def record(name, sector, size, directory=False):
    data = bytearray((33 + len(name) + 1) & ~1)
    data[0] = len(data)
    data[2:10] = struct.pack("<I", sector) + struct.pack(">I", sector)
    data[10:18] = struct.pack("<I", size) + struct.pack(">I", size)
    data[25] = 2 if directory else 0
    data[28:32] = b"\x01\x00\x00\x01"
    data[32] = len(name)
    data[33 : 33 + len(name)] = name
    return data


def disc(exe, identity):
    iso = bytearray(64 * 2048)
    pvd = 16 * 2048
    iso[pvd : pvd + 7] = b"\x01CD001\x01"
    iso[pvd + 8 : pvd + 40] = b"PLAYSTATION".ljust(32)
    iso[pvd + 40 : pvd + 72] = f"SLOT_AUTHORED_{identity}".encode().ljust(32)
    iso[pvd + 80 : pvd + 88] = struct.pack("<I", 64) + struct.pack(">I", 64)
    iso[pvd + 120 : pvd + 128] = b"\x01\x00\x00\x01" * 2
    iso[pvd + 128 : pvd + 132] = b"\x00\x08\x08\x00"
    iso[pvd + 132 : pvd + 140] = struct.pack("<I", 10) + struct.pack(">I", 10)
    iso[pvd + 140 : pvd + 144] = struct.pack("<I", 18)
    iso[pvd + 148 : pvd + 152] = struct.pack(">I", 19)
    iso[18 * 2048 : 18 * 2048 + 10] = bytes([1, 0, 20, 0, 0, 0, 1, 0, 0, 0])
    iso[19 * 2048 : 19 * 2048 + 10] = bytes([1, 0, 0, 0, 0, 20, 0, 1, 0, 0])
    root = record(b"\0", 20, 2048, True)
    iso[pvd + 156 : pvd + 156 + len(root)] = root
    iso[17 * 2048 : 17 * 2048 + 7] = b"\xffCD001\x01"
    config = b"BOOT = cdrom:\\AUTHORED.EXE;1\r\nTCB = 4\r\nEVENT = 10\r\nSTACK = 801fff00\r\n"
    offset = 20 * 2048
    for entry in [
        root,
        record(b"\1", 20, 2048, True),
        record(b"SYSTEM.CNF;1", 21, len(config)),
        record(b"AUTHORED.EXE;1", 22, len(exe)),
        record(b"DISC.BIN;1", 30, 2048),
    ]:
        iso[offset : offset + len(entry)] = entry
        offset += len(entry)
    iso[21 * 2048 : 21 * 2048 + len(config)] = config
    iso[22 * 2048 : 22 * 2048 + len(exe)] = exe
    iso[30 * 2048] = identity
    raw = bytearray(64 * 2352)
    for n in range(64):
        base = n * 2352
        raw[base + 1 : base + 11] = b"\xff" * 10
        msf = [(n + 150) // 4500, (n + 150) // 75 % 60, n % 75]
        raw[base + 12 : base + 16] = bytes([(v // 10) * 16 + v % 10 for v in msf] + [2])
        raw[base + 18] = raw[base + 22] = 8
        raw[base + 24 : base + 2072] = iso[n * 2048 : (n + 1) * 2048]
    return raw


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--llvm-bin", type=Path)
    args = parser.parse_args()
    source = Path(__file__).with_name("acceptance.S")
    llvm = args.llvm_bin
    mc = str(llvm / "llvm-mc") if llvm else shutil.which("llvm-mc")
    objcopy = str(llvm / "llvm-objcopy") if llvm else shutil.which("llvm-objcopy")
    if not mc or not objcopy:
        parser.error("LLVM llvm-mc and llvm-objcopy are required")
    args.output.mkdir(parents=True, exist_ok=False)
    with tempfile.TemporaryDirectory(prefix="slot-authored-") as tmp:
        obj, binary = Path(tmp) / "program.o", Path(tmp) / "program.bin"
        subprocess.run(
            [
                mc,
                "-triple=mipsel",
                "-mcpu=mips1",
                "-filetype=obj",
                str(source),
                "-o",
                str(obj),
            ],
            check=True,
        )
        subprocess.run(
            [objcopy, "-O", "binary", "--only-section=.text", str(obj), str(binary)],
            check=True,
        )
        program = binary.read_bytes()
    size = (len(program) + 2047) & ~2047
    if size + 2048 > 8 * 2048:
        raise ValueError("authored program overlaps DISC.BIN")
    exe = bytearray(2048 + size)
    exe[:8] = b"PS-X EXE"
    for offset, value in [
        (0x10, 0x80010000),
        (0x18, 0x80010000),
        (0x1C, size),
        (0x30, 0x801FFF00),
    ]:
        struct.pack_into("<I", exe, offset, value)
    exe[2048 : 2048 + len(program)] = program
    for number, name in [(1, "first"), (2, "second")]:
        (args.output / f"{name}.bin").write_bytes(disc(exe, number))
        (args.output / f"{name}.cue").write_text(
            f'FILE "{name}.bin" BINARY\n TRACK 01 MODE2/2352\n INDEX 01 00:00:00\n'
        )
    (args.output / "authored.m3u").write_text("first.cue\nsecond.cue\n")
    hashes = {
        p.name: hashlib.sha256(p.read_bytes()).hexdigest()
        for p in sorted(args.output.iterdir())
    }
    hashes["source"] = hashlib.sha256(source.read_bytes()).hexdigest()
    (args.output / "SHA256.json").write_text(json.dumps(hashes, indent=2) + "\n")
    print(json.dumps(hashes, indent=2))


if __name__ == "__main__":
    main()
