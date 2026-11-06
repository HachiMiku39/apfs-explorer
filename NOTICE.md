# Third-Party Notices

APFS Explorer for Windows builds upon prior work in open-source APFS
implementations and Windows user-mode filesystem infrastructure:

## Reference implementations

- linux-apfs-rw (Ernesto Fernández) - GPL-2.0+ - github.com/eafer/linux-apfs-rw
- libfsapfs (libyal) - LGPL-3.0+ - github.com/libyal/libfsapfs
- apfs-fuse (sgan81) - GPL-2.0 - github.com/sgan81/apfs-fuse
- apfsprogs (Ernesto Fernández) - GPL-2.0+ - github.com/eafer/apfsprogs
- go-apfs (blacktop) - MIT - github.com/blacktop/go-apfs

## Specification

- APFS specification - Apple's published file system reference (PDF, public)

## Runtime infrastructure

- WinFsp (Bill Zissimopoulos) - GPL-3.0+ with commercial exception - winfsp.dev

The Windows installer bundles and silently installs the **unmodified** official
WinFsp MSI when WinFsp is not already present. WinFsp is licensed under GPL-3.0
(© Bill Zissimopoulos); its complete source is available at
<https://github.com/winfsp/winfsp>.

## OpenAPFS fork clarification (2026-11-05)

The list above is retained as upstream historical attribution, not a license
compatibility determination. linux-apfs/linux-apfs-rw transaction.c explicitly
states SPDX-License-Identifier: GPL-2.0-only; the broad GPL-2.0+ description above
must not be relied on. Do not import GPL-2.0-only implementation code into this
GPL-3.0-only fork. No code from that project is added by the OpenAPFS changes.
WinFsp describes its exception as a FLOSS exception and separately offers a
commercial license. OpenAPFS v0.1's MSI does not bundle/install the WinFsp MSI;
install the unmodified official runtime separately. Preserve all applicable
notices and audit per-file provenance before future code reuse.

Architecture packages include the unmodified matching WinFsp user-mode DLL
from the locked winfsp-sys distribution (winfsp-x86/x64/a64.dll), not its kernel
driver or MSI. WinFsp GPLv3/FLOSS terms and source remain available from
https://github.com/winfsp/winfsp. The official driver is a separate prerequisite.

## Local Rust WinFsp compatibility patch (2026-11-06)

The locked winfsp 0.13.0+winfsp-2.1 wrapper is preserved under vendor/winfsp
(GPL version 3; upstream commit 17d756ae434957738b5c72cfe20091a3120fdaf3).
OpenAPFS corrects the native SIZE_T ABI for x86 and rejects narrowing overflow.
Original metadata/source/README and GPL text are retained; vendor/README.md
records the original crate checksum and patch scope. winfsp-sys remains locked.
