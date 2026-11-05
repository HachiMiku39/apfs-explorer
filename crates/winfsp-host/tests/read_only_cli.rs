//! Execute shipping entry points; rejection must not open or modify a source.
use std::process::Command;

#[test]
fn every_mutating_entry_point_refuses_and_preserves_the_image() {
    let dir = tempfile::tempdir().unwrap();
    let image = dir.path().join("source.img");
    let original = vec![0x5au8; 8192];
    std::fs::write(&image, &original).unwrap();
    let path = image.to_str().unwrap();
    for args in [
        vec!["--image", path, "--writable"],
        vec!["--drive", "999", "--writable"],
        vec!["snapshot", "create", "--image", path, "--name", "no"],
        vec!["snapshot", "delete", "--image", path, "--xid", "1"],
        vec!["snapshot", "revert", "--image", path, "--xid", "1"],
        vec!["snapshot", "create", "--drive", "999", "--name", "no"],
        vec!["clone", "--image", path, "--src", "/a", "--dst", "/b"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_apfs-mount")).args(&args).output().unwrap();
        assert!(!out.status.success(), "write request succeeded: {args:?}");
        assert!(String::from_utf8_lossy(&out.stderr).contains("v0.1 is read-only"), "wrong rejection: {out:?}");
        assert_eq!(std::fs::read(&image).unwrap(), original);
    }
}
