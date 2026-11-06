# Security and data safety

OpenAPFS v0.1 is an experimental read-only tool. No command, UI action, environment variable or runtime confirmation enables source APFS writes. Physical writable handles and one-shot mutation commands are refused. The upstream writer remains available as development source and must not be treated as a validated production interface.

Use disposable disk images first. Never run inherited real_usb or physical-device write harnesses against valuable data. Real device writes require a separate explicit authorization identifying the device and operation, after image-only validation; v0.1 intentionally cannot carry that out.

Untrusted media can contain malformed lengths, trees, checkpoints, feature flags and names. Preserve checksum and bounds failures and reject unsupported structures. Never disable validation or patch metadata to make a failing image mount. Diagnostic reports should include the commit/build, error and a minimal synthetic reproducer; remove user names, file paths, UUIDs and file content before sharing logs publicly.

Mount helpers may run elevated or under the existing WinFsp Launcher. Keep Launcher access limited to Administrators/SYSTEM. Install WinFsp from its official signed distribution. This fork's MSI does not install WinFsp. A read-only mount does not protect against other software concurrently modifying the source: use a stable detached image or avoid concurrent mounts.

Report suspected vulnerabilities privately through the fork owner's GitHub security reporting channel when available; otherwise request a private channel without posting sensitive media. Do not submit a whole personal disk image in an issue.

Do not advertise CI success as production reliability. Document remaining Windows desktop, hardware, interoperability and fault-injection gaps in TESTING.md. GPL license and third-party notices accompany artifacts; matching source is included.
