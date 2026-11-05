#!/usr/bin/env python3
"""Reject incorrectly labelled Windows EXE/DLL artifacts without running them."""
import argparse
from pathlib import Path
import struct

MACHINES = {
    "i686-pc-windows-msvc": 0x014C,
    "x86_64-pc-windows-msvc": 0x8664,
    "aarch64-pc-windows-msvc": 0xAA64,
}
DLLS = {
    "i686-pc-windows-msvc": "winfsp-x86.dll",
    "x86_64-pc-windows-msvc": "winfsp-x64.dll",
    "aarch64-pc-windows-msvc": "winfsp-a64.dll",
}


def pe_machine(path: Path) -> int:
    with path.open("rb") as source:
        header = source.read(64)
        if len(header) != 64 or header[:2] != b"MZ":
            raise ValueError(f"{path}: invalid DOS header")
        offset = struct.unpack_from("<I", header, 0x3C)[0]
        if offset < 64 or offset > path.stat().st_size - 6:
            raise ValueError(f"{path}: invalid PE header offset")
        source.seek(offset)
        header = source.read(6)
        if len(header) != 6 or header[:4] != b"PE\0\0":
            raise ValueError(f"{path}: invalid PE signature")
        return struct.unpack_from("<H", header, 4)[0]


def verify(target: str, directory: Path) -> None:
    expected = MACHINES[target]
    files = ["apfs-mount.exe", "apfs-dump.exe", "apfs-explorer.exe", DLLS[target]]
    for name in files:
        path = directory / name
        actual = pe_machine(path)
        if actual != expected:
            raise ValueError(f"{path}: expected machine {expected:#06x}, got {actual:#06x}")
        print(f"{name}: {target} ({actual:#06x})")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=MACHINES, required=True)
    parser.add_argument("--directory", type=Path, required=True)
    args = parser.parse_args()
    verify(args.target, args.directory)


if __name__ == "__main__":
    main()
