# Testing and release gates

Release compilation is performed in GitHub Actions. An explicitly authorized local WOA build may be used for development diagnostics; record it separately from cloud release evidence. This document describes required coverage, not a claim that all scenarios passed. Pin every report to the tested commit, OS, WinFsp version, image SHA-256, test command and result. Skipped tests are not passes.

## Current verification record — 2026-11-06

This run was an explicitly authorized local WOA build, not a successful GitHub
Actions run. Environment: Windows 11 ARM64 build 26300.9711, Rust 1.99.1,
MSVC 14.44, LLVM 21.1.8, official WinFsp 2.1. Large tools were downloaded to
macOS, verified against official release hashes, then transferred to WOA.
Registry dependencies were verified against Cargo.lock before offline use.

- Rustfmt passed. Configuration, frontend syntax, PE verifier unit tests and
  `git diff --check` passed. `cargo deny --locked --offline check licenses`
  passed; exact locked WinFsp packages need scoped allowances for the deprecated
  SPDX GPL-3.0 identifier (GPL-3.0-only). Inherited source provenance remains a
  separate review gate.
- ARM64: apfs 11, apfs-core 228, apfs-write 178, winfsp-host 50 library tests
  passed (467 total), plus one shipping CLI write-denial regression. Raw-device
  tests ran with administrative rights; an ordinary-user attempt was rejected
  by Windows permissions and was not counted as a pass.
- x64/x86: the same 467 library tests passed under WOA emulation. A separate
  native-SIZE_T overflow regression passed as an x86 process. These do not
  qualify physical x86/x64 Windows machines or 32-bit Windows 10.
- ARM64, x64 and x86 helper, Tauri GUI and MSI builds succeeded. The production
  custom-protocol feature is explicit. ARM64 MSI installation succeeded and
  registered read-only WinFsp Launcher classes. Package PE verification and
  installed-file hashes are recorded with the local artifacts. All three MSI
  payloads match their builds byte-for-byte after the pinned Tauri CLI's expected
  `UNK` to `MSI` bundle-marker substitution in the GUI executable. The helper and
  DLL match without modification. ARM64 install/uninstall succeeded; GUI startup
  remains unverified because the UI control tool could capture the VM display
  but returned `noWindowsAvailable` for input.
- Owner-authorized empty external 128GB APFS SSD: 2,058 files, including 2,048
  small files, Unicode/NFD, a 384-character relative path, and a sparse
  5GiB+257-byte file, all passed source and copied SHA-256 comparisons and the
  case lookup probe. Explorer enumerated the volume and small-file directory.
- Hardware probing found an existing-file write-open was granted even though
  write callbacks were protected. The open callback now rejects mutating rights
  and delete-on-close before creating a handle. Rebuilt hardware tests returned
  ERROR_WRITE_PROTECT for write-open and create; all 2,058 read/copy checks passed
  again. No content write was issued by the denial probe.
- Normal unmount succeeded. After return to macOS, every file hash, native hard
  link identity, symlink target and xattr matched. `diskutil verifyVolume` ran
  `fsck_apfs -n -x`, returned 0, and reported the volume/container OK (checkpoint
  89). No repair was requested. This is not a byte-for-byte whole-device hash.

Local detailed logs and manifests are kept outside the repository. No personal
file contents or device identifiers are uploaded. This remains a development
candidate; untested rows below remain release gates.

## Automatic v0.1 checks

- Linux/macOS/Windows: parser and filesystem unit tests; host library tests; CLI subprocess regression asserting every mutation fails and a source image remains byte-identical; inherited writer unit tests only.
- macOS: generate tiny and populated 256MiB APFS fixtures with hdiutil, then read them through FsView and the GPT-offset integration stack. Fixture content must be present; do not allow a size-only fixture to count as successful population.
- Windows ARM64/x64/x86: compile the matching mount helper + Tauri GUI, verify PE machine IDs, create separately named ZIP/MSI packages, hash outputs and include the exact source archive. ARM64 tests run natively; x86 tests run under WoW64 on an x64 runner.
- cargo-deny checks SPDX license policy against locked dependencies. Format checks gate source style. No physical disk test runs automatically.

## Acceptance matrix

| Scenario | v0.1 evidence / required gate |
|---|---|
| macOS writes -> OpenAPFS reads | macOS generated fixtures + byte comparisons; expand corpus with metadata/link cases |
| Windows Explorer mount/copy/unmount | Required on disposable image using WinFsp; desktop Windows 10 and 11 separately |
| Read-only enforcement | CLI/adapter regression, Windows writes must fail and source SHA-256 remain identical |
| 4TB address space | Synthetic 4TiB address device beyond 2TiB (512/4Kn sectors), GPT header/array CRC corruption regressions; real 4TB GPT SSD read-only still pending |
| Large files and many small files | 5GiB+257-byte sparse file and 2,048 small files copied and hashed on WOA; 100k-entry runs pending |
| Unicode/NFD/case-insensitive + case-sensitive | Unicode/NFD, case lookup and 384-character relative path passed on WOA; reserved names, conflicts and case-sensitive corpus pending |
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

## Architecture-specific release gates

Run `python -m unittest discover -s scripts -p test_verify_windows_arch.py`.
On each built output run `python scripts/verify_windows_arch.py --target <triple>
--directory target/<triple>/release` to reject mixed-architecture EXEs/DLLs.
The verifier does not replace actual installation or WinFsp mount tests.

Run the Rust unit regressions on each target, including the file extent read
straddling 4GiB, compressed reads above 4GiB, 4TiB GPT addressing, architecture
mapping and source write-denial tests. Explicitly test the x86 package on
32-bit Windows 10 in addition to WoW64; test the ARM64 package on ARM64 Windows
with the ARM64 WinFsp driver. Install/uninstall the MSI, detect the launcher,
mount/copy/hash/unmount a disposable image, and compare the source hash.
The local WOA results above cover compilation and a subset of runtime gates; all other architecture/desktop/install-uninstall scenarios remain pending.

## Reproducible physical-volume read fixture

Only run `scripts/make_macos_read_fixture.py` on an external APFS volume after
its owner explicitly authorizes creating test files. It verifies the supplied
volume UUID and creates a new timestamped directory without replacing existing
files. It does not format, repair or modify raw metadata. Keep the generated
manifest and device identifiers in a local evidence directory, not in Git.
The corpus contains 2,048 small files, Unicode/NFD names, a path over 260
characters, a case lookup probe, links, an xattr and a sparse 5GiB+257-byte file
with data around the 4GiB boundary. A sparse file tests logical size; it does not
prove a 4TB physical disk works.

Cleanly unmount the volume on macOS before passing its USB device to Windows.
Select the identified physical disk explicitly; never assume disk numbers are
stable. Mount it using the project's read-only helper. For example, with the
fixture at `M:\OpenAPFS-ReadTest-<timestamp>`, run:

```powershell
.\scripts\verify_windows_read_fixture.ps1 `
  -MountedRoot 'M:\OpenAPFS-ReadTest-<timestamp>' `
  -ManifestPath 'C:\OpenAPFSTestHarness\manifest.json' `
  -CopyRoot 'C:\OpenAPFSTestHarness\copies-new' `
  -ReportPath 'C:\OpenAPFSTestHarness\read-report.json'
```

The verifier uses read-only source handles, streams data, verifies every source
and copied SHA-256, and refuses output on the APFS drive or an existing copy
directory. It reports link identity, symlink metadata and xattr semantics as
untested. Successful file contents alone cannot validate those features. It
also does not replace Explorer, denied-write, unmount or macOS recheck gates.

Use `scripts/test_windows_read_only_mount.ps1` with the same MountedRoot and
ManifestPath, saving ReportPath to C:, to test denied write-open/create without
issuing a content write. It requires error 5 or 19; unrelated I/O failures do not
count as enforcement. After a clean unmount and USB handoff back to macOS, run
`scripts/verify_macos_read_fixture.py` with the original UUID and manifest to
recheck content and native metadata. Run the non-repair APFS consistency check
separately from file hashing because diskutil can temporarily unmount the volume.
