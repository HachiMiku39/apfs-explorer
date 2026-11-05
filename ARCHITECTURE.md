# Architecture

OpenAPFS v0.1 forks upstream commit `7d6cd8006e9234f9d4aabe9d132cb89227ecd6d3` (2026-11-05). It reuses the implementation rather than adding another APFS engine. All changes remain GPL-3.0-only.

```text
Tauri GUI (device and volume selection; core English/Chinese UI)
    -> WinFsp Launcher -> apfs-mount (one mount per process)
Windows Explorer / file APIs -> WinFsp user-mode callbacks
    -> FsView -> catalog / object maps / B-trees / checksum / checkpoint
    -> bounded cache -> checked 64-bit partition offset -> read-only handle
```

`apfs-core` owns on-disk parsing and the BlockDevice boundary; `apfs` resolves paths and reads file ranges. `winfsp-host` adapts Windows semantics and raw-device discovery. `apfs-explorer-gui` does not link the engine, and the MSI registers only read-only launcher classes. WinFsp supplies its signed driver; OpenAPFS adds no kernel driver.

The source image is opened with File::open and the same handle is retained through validation and mounting. ReadOnlyBlockDevice privately wraps it and rejects write_at even though the existing host generic bound requires WritableBlockDevice. PhysicalDrive refuses writable access before CreateFile. CLI, one-shot mutation commands, GUI backend and mount APIs reject writes independently. Read-only volume flags and callback rejection remain additional defenses.

Unknown container and volume incompatible feature bits are refused. The bootstrap and selected checkpoint are both checked. Encrypted data is refused before catalog use. Non-contiguous checkpoint descriptor trees must not silently fall back to the bootstrap. Malformed metadata checksums remain errors; no force option is provided.

The inherited `apfs-write` engine and image tests are development sources, not a supported v0.1 mutation path. Future image-only experiments need a separate reviewed capability, regular-file handle validation, fixture provenance, fault injection and independent macOS verification before any consideration of hardware writes.

64-bit addressing avoids a 2TB MBR-style limit; this is not evidence of a complete 4TB hardware test. Cache bounds and range reads avoid loading a file into memory for normal Explorer reads. Path/Unicode and large-directory behavior are verified separately in TESTING.md.

The GUI prevents normal window close while a mount is pending or a tracked
drive remains. Unmount only removes tracking after the drive letter disappears;
a failed stop keeps the mount visible for retry. Forced process termination and
OS shutdown still need separate Windows testing.
