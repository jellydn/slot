#!/usr/bin/env python3
"""Encode only the authored raw-disc fixture exported by the software test."""

import struct
import sys
import zlib
from pathlib import Path


def main():
    directory = Path(sys.argv[1])
    raw = (directory / "authored.bin").read_bytes()
    assert len(raw) == 64 * 2352, "expected the authored 64-sector test disc"
    assert b"AUTHORED.EXE;1" in raw and b"bu00:SLOT_TEST" in raw
    psar = 40
    image = bytearray(psar + 0x100000)
    struct.pack_into("<10I", image, 0, 0x50425000, 0x10000, *([40] * 8))
    image[psar : psar + 11] = b"PSISOIMG00\0"
    toc = psar + 0x800
    image[toc + 10 + 7] = 1
    image[toc + 20 + 7 : toc + 20 + 10] = bytes([0, 0, 0x64])
    image[toc + 30] = 2
    image[toc + 30 + 2] = 1
    image[toc + 30 + 7 : toc + 30 + 10] = bytes([0, 2, 0])
    offset = 0
    for index, start in enumerate(range(0, len(raw), 16 * 2352)):
        encoder = zlib.compressobj(wbits=-15)
        packed = encoder.compress(raw[start : start + 16 * 2352]) + encoder.flush()
        struct.pack_into("<II", image, psar + 0x4000 + index * 32, offset, len(packed))
        image.extend(packed)
        offset += len(packed)
    struct.pack_into("<I", image, psar + 0x4000 + 4 * 32, offset)
    output = directory / "authored.pbp"
    assert not output.exists(), "refuse replacing a fixture"
    output.write_bytes(image)


if __name__ == "__main__":
    main()
