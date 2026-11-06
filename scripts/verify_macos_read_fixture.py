#!/usr/bin/env python3
"""Recheck fixture content and native metadata after a read-only Windows test."""
import argparse
import json
from pathlib import Path
import plistlib
import subprocess

from make_macos_read_fixture import digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--volume', type=Path, required=True)
    parser.add_argument('--expected-uuid', required=True)
    parser.add_argument('--manifest', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    info = plistlib.loads(subprocess.check_output(['diskutil', 'info', '-plist', str(args.volume)]))
    if info.get('VolumeUUID', '').lower() != args.expected_uuid.lower():
        raise SystemExit('Volume UUID mismatch')
    if info.get('FilesystemType') != 'apfs' or info.get('Internal') is not False:
        raise SystemExit('Expected the identified external APFS volume')
    manifest = json.loads(args.manifest.read_text(encoding='utf-8'))
    fixture = manifest['fixture']
    if manifest.get('schema') != 1 or Path(fixture).name != fixture:
        raise SystemExit('Invalid fixture manifest')
    root = args.volume / fixture
    failures = []
    for entry in manifest['files']:
        relative = Path(entry['path'])
        if relative.is_absolute() or '..' in relative.parts:
            raise SystemExit('Unsafe manifest path')
        path = root / relative
        if path.is_symlink() or not path.is_file() or path.stat().st_size != entry['size'] or digest(path) != entry['sha256']:
            failures.append(entry['path'])
    for entry in manifest['symlinks']:
        path = root / entry['path']
        if not path.is_symlink() or path.readlink().as_posix() != entry['target']:
            failures.append(entry['path'] + ':symlink')
    for entry in manifest['xattrs']:
        value = subprocess.check_output(['xattr', '-px', entry['name'], str(root / entry['path'])])
        if ''.join(value.decode().split()).lower() != entry['value_hex']:
            failures.append(entry['path'] + ':xattr')
    hardlink_ok = (root / 'plain.txt').stat().st_ino == (root / 'links/hardlink.txt').stat().st_ino
    if not hardlink_ok:
        failures.append('hardlink identity')
    report = {'schema': 1, 'regular_files': len(manifest['files']), 'failures': failures,
              'native_hardlink_identity': hardlink_ok, 'native_symlinks_checked': len(manifest['symlinks']),
              'native_xattrs_checked': len(manifest['xattrs'])}
    args.report.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(report))
    if failures:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
