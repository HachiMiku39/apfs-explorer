# Local WinFsp Rust binding patch

`winfsp/` preserves the crates.io `winfsp 0.13.0+winfsp-2.1` source,
upstream commit `17d756ae434957738b5c72cfe20091a3120fdaf3`, crate SHA-256
`86829821fff6ed3bc1ace8a0c9a67a5a63a121a9f0a025929ffd5428bad1fa62`.
Upstream: https://github.com/SnowflakePowered/winfsp-rs . Its declared license
is GPL-3.0; the complete GPL version 3 text is included as LICENSE-GPL-3.0.
Original README and source are preserved.

OpenAPFS changes only the native SIZE_T callback/notification boundary for
32-bit Windows. Public Rust u64 lengths stay unchanged. Conversion to a native
SIZE_T is checked; overflow returns STATUS_INTEGER_OVERFLOW. File offsets and
logical file sizes remain u64. No APFS parser or write logic is changed here.
The package is excluded from workspace membership but selected reproducibly
through [patch.crates-io]. The registry winfsp-sys dependency remains locked.
