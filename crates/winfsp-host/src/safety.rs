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

#[cfg(test)]
mod tests {
    #[test]
    fn writing_has_no_runtime_override() {
        assert!(super::require_read_only(false).is_ok());
        assert_eq!(super::require_read_only(true), Err(super::WRITE_DISABLED));
    }
}
