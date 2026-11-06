#!/usr/bin/env python3
"""Create isolated read-test samples on an explicitly identified external APFS volume."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import plistlib
import subprocess
import unicodedata


def digest(path):
    value = hashlib.sha256()
    with path.open('rb') as source:
        for block in iter(lambda: source.read(8 * 1024 * 1024), b''):
            value.update(block)
    return value.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--volume', type=Path, required=True)
    parser.add_argument('--expected-uuid', required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    volume = args.volume.resolve(strict=True)
    info = plistlib.loads(subprocess.check_output(['diskutil', 'info', '-plist', str(volume)]))
    if info.get('VolumeUUID', '').lower() != args.expected_uuid.lower():
        raise SystemExit('Volume UUID mismatch: refusing to create anything')
    if info.get('FilesystemType') != 'apfs' or info.get('Internal') is not False:
        raise SystemExit('Expected an external APFS volume')
    if info.get('MountPoint') != str(volume) or info.get('Encryption') or info.get('FileVault'):
        raise SystemExit('Expected the identified, unencrypted mounted volume')
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%SZ')
    root = volume / ('OpenAPFS-ReadTest-' + stamp)
    root.mkdir(exist_ok=False)
    for name in ['unicode', 'small', 'links', 'longpath']:
        (root / name).mkdir()
    (root / 'plain.txt').write_bytes(b'OpenAPFS read-only interoperability fixture\n')
    (root / 'empty.bin').write_bytes(b'')
    (root / 'CaseProbe.TXT').write_bytes(b'Case-insensitive lookup must preserve this content.\n')
    for name in ['中文 文件.txt', 'emoji-😀.txt', unicodedata.normalize('NFD', 'café.txt'), 'Straße.txt']:
        (root / 'unicode' / name).write_text('OpenAPFS Unicode sample: ' + name + '\n', encoding='utf-8')
    for index in range(2048):
        data = hashlib.sha256(('OpenAPFS-small-' + str(index)).encode()).digest() * 128
        (root / 'small' / ('file-%04d.bin' % index)).write_bytes(data)
    deep = root / 'longpath'
    for index in range(6):
        deep = deep / ('segment-%02d-' % index + 'a' * 48)
        deep.mkdir()
    (deep / 'unicode-长路径.txt').write_bytes(b'Path length exceeds the legacy MAX_PATH boundary.\n')
    os.link(root / 'plain.txt', root / 'links' / 'hardlink.txt')
    os.symlink('../plain.txt', root / 'links' / 'relative-symlink.txt')
    subprocess.run(['xattr', '-wx', 'user.openapfs.readtest', b'OpenAPFS-xattr-v1'.hex(), str(root / 'plain.txt')], check=True)
    large = root / 'large-5GiB-plus-257.bin'
    large_size = 5 * 1024**3 + 257
    block = bytes(range(256)) * 4096
    with large.open('xb') as target:
        target.truncate(large_size)
        for offset in [0, 2**32 - 512, 2**32 + 2 * 1024**2, large_size - len(block)]:
            target.seek(offset)
            target.write(block)
        target.flush()
        os.fsync(target.fileno())
    files, links = [], []
    for path in sorted(root.rglob('*')):
        relative = path.relative_to(root).as_posix()
        if path.is_symlink():
            links.append({'path': relative, 'target': os.readlink(path)})
        elif path.is_file():
            files.append({'path': relative, 'size': path.stat().st_size, 'sha256': digest(path)})
    manifest = {'schema': 1, 'fixture': root.name, 'files': files, 'symlinks': links,
                'case_probe': {'stored': 'CaseProbe.TXT', 'lookup': 'caseprobe.txt'},
                'xattrs': [{'path': 'plain.txt', 'name': 'user.openapfs.readtest', 'value_hex': b'OpenAPFS-xattr-v1'.hex()}]}
    text = json.dumps(manifest, ensure_ascii=False, indent=2) + '\n'
    (root / 'manifest.json').write_text(text, encoding='utf-8')
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(text, encoding='utf-8')
    print(json.dumps({'fixture': str(root), 'regular_files': len(files), 'symlinks': len(links),
                      'logical_bytes': sum(f['size'] for f in files), 'manifest': str(args.report)}, ensure_ascii=False))


if __name__ == '__main__':
    main()
