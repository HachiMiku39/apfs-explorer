# Testing and release gates

All compilation is performed in GitHub Actions. This document describes required coverage, not a claim that all scenarios passed. Pin every report to the tested commit, OS, WinFsp version, image SHA-256, test command and result. Skipped tests are not passes.

## Current verification record — 2026-11-05

Local JSON configuration and WiX XML parsing passed. Frontend JavaScript syntax,
English/Chinese language selection and unknown-message fallback passed in Node.
`git diff --check` passed. Rust compilation/unit/integration tests were **not run**:
the machine has no Rust toolchain and the attempted official toolchain download
timed out. GitHub Actions workflows and draft PR #1 exist, but no successful cloud
run or Windows artifact is recorded. This is a source development candidate,
not an accepted or production-ready v0.1 release.

## Automatic v0.1 checks

- Linux/macOS/Windows: parser and filesystem unit tests; host library tests; CLI subprocess regression asserting every mutation fails and a source image remains byte-identical; inherited writer unit tests only.
- macOS: generate tiny and populated 256MiB APFS fixtures with hdiutil, then read them through FsView and the GPT-offset integration stack. Fixture content must be present; do not allow a size-only fixture to count as successful population.
- Windows x64: compile mount helper + Tauri GUI, create ZIP/MSI, hash outputs and include the exact source archive.
- cargo-deny checks SPDX license policy against locked dependencies. Format checks gate source style; the current candidate still needs a successful rustfmt run. No physical disk test runs automatically.

## Acceptance matrix

| Scenario | v0.1 evidence / required gate |
|---|---|
| macOS writes -> OpenAPFS reads | macOS generated fixtures + byte comparisons; expand corpus with metadata/link cases |
| Windows Explorer mount/copy/unmount | Required on disposable image using WinFsp; desktop Windows 10 and 11 separately |
| Read-only enforcement | CLI/adapter regression, Windows writes must fail and source SHA-256 remain identical |
| 4TB address space | Synthetic 4TiB address device beyond 2TiB (512/4Kn sectors), GPT header/array CRC corruption regressions; real 4TB GPT SSD read-only still pending |
| Large files and many small files | Range/overflow unit tests and populated fixtures; >4GiB copy and 100k-entry Windows runs pending |
| Unicode/NFD/case-insensitive + case-sensitive | Existing normalization tests; Windows forbidden/reserved names and >260-character paths pending |
| Symlink/hardlink/xattr | Existing library tests; Windows reparse semantics and metadata exposure pending |
| Full device / allocation recovery | Future image-only writer stage; not enabled in shipping v0.1 |
| Crash / interrupted write / unplug | Future fault injection on images, checkpoint inspection and independent fsck; source reads must surface I/O errors |
| Windows writes -> macOS reads/edits | Future image-only stage, independent content hashes + fsck_apfs -n; never run against a personal physical disk |
| Unknown features/checksum corruption | Must refuse instead of guessing or falling back to unrelated metadata |

## Manual read-only image acceptance

1. Produce a disposable GPT APFS image on macOS with Unicode names, long paths, links, sparse/large files and a SHA-256 manifest. Unmount normally and run `fsck_apfs -n` on its virtual device. Store OS/image/tool provenance.
2. On Windows with WinFsp 2.1, hash the image, mount via `apfs-mount --image ... --mount M:`, enumerate every file, copy into a separate Windows directory and compare hashes with the macOS manifest. Verify denied creates/deletes/renames/writes and visible unsupported-feature errors.
3. Unmount normally, check the source image hash unchanged, reopen on macOS read-only, compare manifest and run fsck again. Repeat with >4GiB files, >2TiB sparse address space, low memory and many small files.
4. Exercise process failure and simulated removal on disposable media. A copied target can be incomplete; the source must remain unchanged. Distinguish virtual unmount from physical eject.

## Future write qualification (not v0.1)

Use copied regular images only. Cover create/delete/move/rename for files and directories, random overwrite/append, truncate/extend, exhaustion, allocation/free queues, snapshots/clones, hard links/xattrs, COW checkpoint commit and both durability barriers. Inject short writes, I/O errors, dropped/reordered flushes and termination at each commit stage. Independently mount/read/write on macOS and run non-repair fsck. Enumerate permitted old/new checkpoint outcomes and verify every referenced extent and free-space invariant. No passing test authorizes hardware writes or proves production reliability.
