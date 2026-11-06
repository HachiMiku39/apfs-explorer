#!/usr/bin/env python3
"""Verify extracted MSI bytes, including Tauri's documented GUI bundle marker."""
import argparse
import hashlib
from pathlib import Path

from verify_windows_arch import DLLS, MACHINES, pe_machine

# Pinned Tauri CLI 2.12.4 replaces this marker for an MSI, then restores the
# unpatched portable EXE. Every other byte must match the build output.
# https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.12.4/crates/tauri-bundler/src/bundle.rs
PORTABLE_MARKER = b"__TAURI_BUNDLE_TYPE_VAR_UNK"
MSI_MARKER = b"__TAURI_BUNDLE_TYPE_VAR_MSI"


def verify(target, build, payload):
    hashes = {}
    for name in ["apfs-explorer.exe", "apfs-mount.exe", DLLS[target]]:
        source = build / name
        installed = payload / name
        if pe_machine(source) != MACHINES[target] or pe_machine(installed) != MACHINES[target]:
            raise ValueError(f"Wrong PE architecture: {name}")
        expected = source.read_bytes()
        if name == "apfs-explorer.exe":
            if expected.count(PORTABLE_MARKER) != 1:
                raise ValueError("Expected exactly one Tauri portable bundle marker")
            expected = expected.replace(PORTABLE_MARKER, MSI_MARKER, 1)
        actual = installed.read_bytes()
        if actual != expected:
            raise ValueError(f"Unexpected MSI payload bytes: {name}")
        hashes[name] = hashlib.sha256(actual).hexdigest()
        print(f"{name}: exact MSI payload match ({hashes[name]})")
    return hashes


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=MACHINES, required=True)
    parser.add_argument("--build-dir", type=Path, required=True)
    parser.add_argument("--payload-dir", type=Path, required=True)
    args = parser.parse_args()
    verify(args.target, args.build_dir, args.payload_dir)


if __name__ == "__main__":
    main()
