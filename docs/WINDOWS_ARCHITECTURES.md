# Windows architecture support

The implementation/build targets are ARM64, x64 and x86 (32-bit application).
They are development targets until the corresponding build and runtime gates
pass. No success is inferred from a workflow matrix entry.

| Target | Rust MSVC triple | Cloud runner | WinFsp DLL | Launcher |
|---|---|---|---|---|
| ARM64 | aarch64-pc-windows-msvc | windows-11-arm | winfsp-a64.dll | launchctl-a64.exe |
| x64 | x86_64-pc-windows-msvc | windows-latest | winfsp-x64.dll | launchctl-x64.exe |
| x86 | i686-pc-windows-msvc | windows-latest (WoW64 tests) | winfsp-x86.dll | launchctl-x86.exe |

ARM32 is not a target. x86 native OS acceptance is Windows 10 32-bit; Windows 11
uses a 64-bit OS and runs 32-bit applications through its compatibility layer.
The kernel driver must match the OS, while the WinFsp user-mode DLL matches the
OpenAPFS process. ARM64 builds are native and do not rely on an x64 APFS helper.

## Build and packaging

GitHub Actions always passes an explicit `--target` to Cargo and Tauri. Outputs
live under `target/<triple>/release`; the helper, GUI and DLL are staged from the
same target directory. A generated Tauri config bundles that DLL in the MSI.
The portable ZIP includes it too. The official WinFsp driver is installed
separately; shipping a DLL alone does not make WinFsp available.

Before upload, `scripts/verify_windows_arch.py` validates all three executables
and the DLL against IMAGE_FILE_MACHINE_I386 (0x014c), AMD64 (0x8664) or ARM64
(0xaa64). Packages and artifact names carry the architecture. Every package
includes a matching source archive and checksums. This does not prove that the
MSI installs or that Explorer mounts work; those require TESTING.md acceptance.
Install one OpenAPFS architecture at a time; uninstall before changing package
architecture while the product version is unchanged.

The GUI launcher path follows the current process architecture and the Program
Files environment paths (or explicit WINFSP_BIN), not a hard-coded C: drive or
x64 executable. MSI components use the 32-bit registry view for the WinFsp 2.1
launcher service registrations; no literal WOW6432Node is embedded in the key.

## i686 bindgen compatibility

The locked `winfsp-sys 0.12.1` build script passes `x86-pc-windows-msvc` to Clang,
which Clang rejects. `.cargo/config.toml` supplies the correct i686 triple through
`BINDGEN_EXTRA_CLANG_ARGS_i686_pc_windows_msvc`. Bindgen 0.72 appends these arguments
after builder arguments, so the target override applies only to the i686 build.
Dependencies and Cargo.lock are otherwise unchanged. Remove the workaround only
after upgrading to a verified winfsp-sys version that fixes the target.

Ranged file/stream sizes and positions remain u64 on x86. Small reads beyond
4GiB must work; the API must never return EOF merely because a file offset cannot
fit usize. Whole-file buffers and resource forks too large for the process are
refused explicitly. Compressed logical file ranges also retain 64-bit positions.

## Primary references (checked 2026-11-05)

- [Rust Windows MSVC targets](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html)
- [Tauri Windows architectures](https://v2.tauri.app/distribute/windows-installer/#building-for-32-bit-or-arm)
- [Tauri MSI architecture implementation](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-bundler/src/bundle/windows/msi/mod.rs)
- [GitHub ARM64 Windows runner](https://github.blog/changelog/2025-08-07-arm64-hosted-runners-for-public-repositories-are-now-generally-available/)
- [WinFsp DLL/launcher distribution](https://github.com/winfsp/winfsp/blob/master/build/VStudio/installer/Product.wxs)
- [Locked winfsp-sys build script](https://github.com/SnowflakePowered/winfsp-rs/blob/0.13.0/winfsp-sys/build.rs)
- [Bindgen extra argument order](https://github.com/rust-lang/rust-bindgen/blob/v0.72.1/bindgen/lib.rs)
