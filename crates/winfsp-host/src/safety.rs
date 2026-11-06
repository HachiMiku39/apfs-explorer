//! Shipping v0.1 has no APFS write authorization path.
//! The upstream writer is retained for future disposable-image validation.
pub const WRITE_DISABLED: &str =
    "OpenAPFS v0.1 is read-only; APFS writes are disabled, including physical disks";

pub fn require_read_only(writable: bool) -> Result<(), &'static str> {
    if writable {
        Err(WRITE_DISABLED)
    } else {
        Ok(())
    }
}

/// Windows can request a writable handle without writing immediately. Refuse
/// those rights before creating any handle or consulting filesystem metadata.
pub fn requests_mutating_open(granted_access: u32, create_options: u32) -> bool {
    // FILE_WRITE_DATA, FILE_APPEND_DATA, FILE_WRITE_EA, FILE_DELETE_CHILD,
    // FILE_WRITE_ATTRIBUTES, DELETE, WRITE_DAC, WRITE_OWNER, GENERIC_WRITE,
    // GENERIC_ALL. Read/list/execute, READ_CONTROL and SYNCHRONIZE remain valid.
    const WRITE_RIGHTS: u32 = 0x0002
        | 0x0004
        | 0x0010
        | 0x0040
        | 0x0100
        | 0x0001_0000
        | 0x0004_0000
        | 0x0008_0000
        | 0x4000_0000
        | 0x1000_0000;
    const FILE_DELETE_ON_CLOSE: u32 = 0x1000;
    granted_access & WRITE_RIGHTS != 0 || create_options & FILE_DELETE_ON_CLOSE != 0
}

#[cfg(test)]
mod tests {
    #[test]
    fn writing_has_no_runtime_override() {
        assert!(super::require_read_only(false).is_ok());
        assert_eq!(super::require_read_only(true), Err(super::WRITE_DISABLED));
    }

    #[test]
    fn read_handles_allow_inspection_but_never_mutation_or_delete_on_close() {
        // FILE_GENERIC_READ / FILE_GENERIC_EXECUTE and their generic forms.
        for rights in [0, 0x0012_0089, 0x0012_00a0, 0x8000_0000, 0x2000_0000] {
            assert!(!super::requests_mutating_open(rights, 0));
            assert!(super::requests_mutating_open(rights, 0x1000));
        }
        for rights in [
            0x2,
            0x4,
            0x10,
            0x40,
            0x100,
            0x1_0000,
            0x4_0000,
            0x8_0000,
            0x4000_0000,
            0x1000_0000,
        ] {
            assert!(super::requests_mutating_open(rights, 0));
            assert!(super::requests_mutating_open(rights | 0x0012_0089, 0));
        }
    }
}
