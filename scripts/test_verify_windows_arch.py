"""Packaging regressions: valid architectures, mislabeled builds and bad headers."""
import contextlib
import io
from pathlib import Path
import struct
import tempfile
import unittest

from verify_windows_arch import DLLS, MACHINES, pe_machine, verify


def image(machine):
    data = bytearray(256)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3C, 128)
    data[128:132] = b"PE\0\0"
    struct.pack_into("<H", data, 132, machine)
    return data


class PackagingArchitecture(unittest.TestCase):
    def test_all_three_targets_and_wrong_machine_rejection(self):
        for target, machine in MACHINES.items():
            with self.subTest(target=target), tempfile.TemporaryDirectory() as temp:
                directory = Path(temp)
                for name in ["apfs-mount.exe", "apfs-dump.exe", "apfs-explorer.exe", DLLS[target]]:
                    (directory / name).write_bytes(image(machine))
                with contextlib.redirect_stdout(io.StringIO()):
                    verify(target, directory)
                (directory / "apfs-mount.exe").write_bytes(image(machine ^ 1))
                with self.assertRaisesRegex(ValueError, "expected machine"):
                    verify(target, directory)

    def test_malformed_and_missing_inputs_fail(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "bad.exe"
            for data in [b"", b"MZ", bytearray(256), image(0x8664)[:132]]:
                with self.subTest(size=len(data)):
                    path.write_bytes(data)
                    with self.assertRaises(ValueError):
                        pe_machine(path)
            with self.assertRaises(FileNotFoundError):
                pe_machine(path.with_name("missing.exe"))


if __name__ == "__main__":
    unittest.main()
