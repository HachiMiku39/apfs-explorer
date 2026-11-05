//! Minimal GPT parser: find the APFS container partition's byte range.
//!
//! Only the fields needed to locate the APFS container are read. Offsets
//! are validated against the project plan's empirically-locked GPT layout.

use crate::block_device::{BlockDevice, BlockError};

/// APFS container GPT partition type GUID, canonical string form.
pub const APFS_TYPE_GUID: &str = "7C3457EF-0000-11AA-AA11-00306543ECAC";

/// Bounds on the partition entry array. The UEFI specification requires
/// `SizeOfPartitionEntry` to be a power of two of at least 128 bytes, and
/// real tables use exactly 128 with 128 entries; both fields come from the
/// disk, so a crafted header must not size a buffer or a loop on its own.
const MIN_PARTITION_ENTRY_SIZE: u64 = 128;
const MAX_PARTITION_ENTRY_SIZE: u64 = 64 * 1024;
const MAX_PARTITION_ENTRIES: u64 = 65_536;
const MAX_PARTITION_TABLE_BYTES: u64 = 16 * 1024 * 1024;
/// The part of an entry this parser reads: the type GUID (bytes 0..16) and
/// the first/last LBA pair (bytes 32..48). A longer entry only carries
/// vendor extensions past that, so the rest is never read.
const PARTITION_ENTRY_READ_LEN: usize = 128;

#[derive(Debug)]
pub enum GptError {
    Block(BlockError),
    NoProtectiveMbr,
    NoGptHeader,
    NoApfsPartition,
    Malformed(&'static str),
}

impl From<BlockError> for GptError {
    fn from(e: BlockError) -> Self {
        GptError::Block(e)
    }
}

/// Byte range of the APFS container within the whole disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerRange {
    pub offset: u64,
    pub length: u64,
}

fn u32_le(b: &[u8], at: usize) -> Result<u32, GptError> {
    let s: [u8; 4] = b
        .get(at..at + 4)
        .ok_or(GptError::Malformed("u32"))?
        .try_into()
        .map_err(|_| GptError::Malformed("u32"))?;
    Ok(u32::from_le_bytes(s))
}

fn u64_le(b: &[u8], at: usize) -> Result<u64, GptError> {
    let s: [u8; 8] = b
        .get(at..at + 8)
        .ok_or(GptError::Malformed("u64"))?
        .try_into()
        .map_err(|_| GptError::Malformed("u64"))?;
    Ok(u64::from_le_bytes(s))
}

/// Format the 16-byte GPT partition type field as the canonical GUID
/// string. GPT stores Data1/2/3 little-endian and Data4 big-endian.
#[allow(clippy::indexing_slicing)] // fixed [u8;16] - all indices < 16, always in bounds
fn guid_string(g: &[u8]) -> Result<String, GptError> {
    let g: [u8; 16] = g
        .get(..16)
        .ok_or(GptError::Malformed("guid"))?
        .try_into()
        .map_err(|_| GptError::Malformed("guid"))?;
    let d1 = u32::from_le_bytes([g[0], g[1], g[2], g[3]]);
    let d2 = u16::from_le_bytes([g[4], g[5]]);
    let d3 = u16::from_le_bytes([g[6], g[7]]);
    Ok(format!(
        "{d1:08X}-{d2:04X}-{d3:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        g[8], g[9], g[10], g[11], g[12], g[13], g[14], g[15]
    ))
}

/// IEEE CRC-32 used by GPT (UEFI header and partition-entry array).
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

/// Locate the first APFS-type partition and return its byte range.
///
/// `lba_size` is the device's logical block size (512 on 512e/512n media,
/// 4096 on 4Kn). All LBA-to-byte arithmetic uses it, so the parser works on
/// 4K-native drives instead of assuming 512-byte sectors. The protective MBR
/// (offset 0) and the GPT header (LBA 1) are read into fixed 512-byte buffers -
/// both structures are <= 512 bytes regardless of logical block size.
pub fn find_apfs_container<D: BlockDevice>(
    dev: &mut D,
    lba_size: u64,
) -> Result<ContainerRange, GptError> {
    if !matches!(lba_size, 512 | 4096) {
        return Err(GptError::Malformed("unsupported logical sector size"));
    }
    // Protective MBR signature (always at byte offset 0).
    let mut mbr = [0u8; 512];
    dev.read_at(0, &mut mbr)?;
    if mbr.get(510..512) != Some(&[0x55, 0xAA]) {
        return Err(GptError::NoProtectiveMbr);
    }

    // GPT header at LBA 1 = 1 * lba_size bytes.
    let mut hdr = [0u8; 512];
    dev.read_at(lba_size, &mut hdr)?;
    if hdr.get(0..8) != Some(b"EFI PART") {
        return Err(GptError::NoGptHeader);
    }
    let header_len = u32_le(&hdr, 12)? as usize;
    if u32_le(&hdr, 8)? != 0x0001_0000 || !(92..=512).contains(&header_len) {
        return Err(GptError::Malformed("unsupported GPT revision or header size"));
    }
    let stored_crc = u32_le(&hdr, 16)?;
    hdr.get_mut(16..20).ok_or(GptError::Malformed("header CRC field"))?.fill(0);
    if crc32(hdr.get(..header_len).ok_or(GptError::Malformed("header length"))?) != stored_crc {
        return Err(GptError::Malformed("GPT header CRC mismatch"));
    }
    if u64_le(&hdr, 24)? != 1 {
        return Err(GptError::Malformed("GPT header is not at LBA 1"));
    }
    let first_usable = u64_le(&hdr, 40)?;
    let last_usable = u64_le(&hdr, 48)?;
    let part_lba = u64_le(&hdr, 72)?;
    let num_parts = u64::from(u32_le(&hdr, 80)?);
    let part_size = u64::from(u32_le(&hdr, 84)?);
    if !(MIN_PARTITION_ENTRY_SIZE..=MAX_PARTITION_ENTRY_SIZE).contains(&part_size)
        || !part_size.is_power_of_two()
    {
        return Err(GptError::Malformed("partition entry size"));
    }
    if num_parts == 0 || num_parts > MAX_PARTITION_ENTRIES {
        return Err(GptError::Malformed("partition entry count"));
    }
    let table_bytes = num_parts
        .checked_mul(part_size)
        .ok_or(GptError::Malformed("partition table size overflow"))?;
    if table_bytes > MAX_PARTITION_TABLE_BYTES {
        return Err(GptError::Malformed("partition table too large"));
    }

    let table_base = part_lba
        .checked_mul(lba_size)
        .ok_or(GptError::Malformed("table base overflow"))?;
    let table_end = table_base.checked_add(table_bytes)
        .ok_or(GptError::Malformed("partition table end overflow"))?;
    let usable_start = first_usable.checked_mul(lba_size)
        .ok_or(GptError::Malformed("first usable LBA overflow"))?;
    if part_lba < 2 || first_usable > last_usable || table_end > usable_start {
        return Err(GptError::Malformed("partition table overlaps usable space"));
    }
    if dev.size() != 0 && table_end > dev.size() {
        return Err(GptError::Malformed("partition table extends past the end of the device"));
    }
    let mut table = vec![0u8; table_bytes as usize];
    dev.read_at(table_base, &mut table)?;
    if crc32(&table) != u32_le(&hdr, 88)? {
        return Err(GptError::Malformed("GPT partition array CRC mismatch"));
    }
    for i in 0..num_parts {
        let start = (i * part_size) as usize;
        let ent = table.get(start..start + PARTITION_ENTRY_READ_LEN)
            .ok_or(GptError::Malformed("partition entry range"))?;
        if guid_string(&ent)?.eq_ignore_ascii_case(APFS_TYPE_GUID) {
            let first_lba = u64_le(&ent, 32)?;
            let last_lba = u64_le(&ent, 40)?;
            if first_lba < first_usable || last_lba > last_usable {
                return Err(GptError::Malformed("partition outside GPT usable range"));
            }
            if last_lba < first_lba {
                return Err(GptError::Malformed("last_lba < first_lba"));
            }
            let offset = first_lba
                .checked_mul(lba_size)
                .ok_or(GptError::Malformed("offset overflow"))?;
            let length = last_lba.checked_sub(first_lba)
                .and_then(|n| n.checked_add(1))
                .and_then(|n| n.checked_mul(lba_size))
                .ok_or(GptError::Malformed("length overflow"))?;
            // The table says where the partition is; nothing so far says the
            // device is that big. A table describing a partition past the end
            // of the medium would have every access inside it land outside.
            let end = offset
                .checked_add(length)
                .ok_or(GptError::Malformed("range end overflow"))?;
            let dev_size = dev.size();
            if dev_size != 0 && end > dev_size {
                return Err(GptError::Malformed(
                    "partition extends past the end of the device",
                ));
            }
            return Ok(ContainerRange { offset, length });
        }
    }
    Err(GptError::NoApfsPartition)
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    struct MemDisk {
        data: Vec<u8>,
    }
    impl BlockDevice for MemDisk {
        fn size(&self) -> u64 {
            self.data.len() as u64
        }
        fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), BlockError> {
            let o = offset as usize;
            let end = o + buf.len();
            let src = self.data.get(o..end).ok_or(BlockError::OutOfRange {
                offset,
                len: buf.len() as u64,
                size: self.data.len() as u64,
            })?;
            buf.copy_from_slice(src);
            Ok(())
        }
    }

    fn apfs_type_bytes() -> [u8; 16] {
        // 7C3457EF-0000-11AA-AA11-00306543ECAC in GPT mixed-endian form.
        [
            0xEF, 0x57, 0x34, 0x7C, 0x00, 0x00, 0xAA, 0x11, 0xAA, 0x11, 0x00, 0x30, 0x65, 0x43,
            0xEC, 0xAC,
        ]
    }

    fn build_disk(first_lba: u64, last_lba: u64) -> MemDisk {
        build_disk_with_table(first_lba, last_lba, 128, 128, 0)
    }

    /// A disk whose GPT header declares `num_entries` entries of
    /// `entry_size` bytes, with the APFS entry at `index` in the table.
    fn build_disk_with_table(
        first_lba: u64,
        last_lba: u64,
        entry_size: u32,
        num_entries: u32,
        index: usize,
    ) -> MemDisk {
        let total = (last_lba as usize + 16) * 512;
        let mut d = vec![0u8; total];
        d[510] = 0x55;
        d[511] = 0xAA;
        d[512..520].copy_from_slice(b"EFI PART");
        d[520..524].copy_from_slice(&0x0001_0000u32.to_le_bytes());
        d[524..528].copy_from_slice(&92u32.to_le_bytes());
        d[536..544].copy_from_slice(&1u64.to_le_bytes());
        d[552..560].copy_from_slice(&40u64.to_le_bytes());
        d[560..568].copy_from_slice(&last_lba.to_le_bytes());
        d[512 + 72..512 + 80].copy_from_slice(&2u64.to_le_bytes()); // partLBA=2
        d[512 + 80..512 + 84].copy_from_slice(&num_entries.to_le_bytes()); // numParts
        d[512 + 84..512 + 88].copy_from_slice(&entry_size.to_le_bytes()); // partSize
        let ent = 2 * 512 + index * entry_size as usize;
        d[ent..ent + 16].copy_from_slice(&apfs_type_bytes());
        d[ent + 32..ent + 40].copy_from_slice(&first_lba.to_le_bytes());
        d[ent + 40..ent + 48].copy_from_slice(&last_lba.to_le_bytes());
        let cont = first_lba as usize * 512;
        d[cont + 32..cont + 36].copy_from_slice(b"NXSB");
        let table_end = 1024usize.checked_add((num_entries as usize).saturating_mul(entry_size as usize));
        if let Some(table) = table_end.and_then(|end| d.get(1024..end)) {
            let c = crc32(table);
            d[600..604].copy_from_slice(&c.to_le_bytes());
        }
        let c = crc32(&d[512..604]);
        d[528..532].copy_from_slice(&c.to_le_bytes());
        MemDisk { data: d }
    }

    #[test]
    fn crc_matches_published_ieee_vector() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn header_and_array_corruption_are_rejected() {
        let mut disk = build_disk(40, 200);
        disk.data[512 + 56] ^= 1;
        assert!(matches!(find_apfs_container(&mut disk, 512), Err(GptError::Malformed("GPT header CRC mismatch"))));
        let mut disk = build_disk(40, 200);
        disk.data[1024 + 40] ^= 1;
        assert!(matches!(find_apfs_container(&mut disk, 512), Err(GptError::Malformed("GPT partition array CRC mismatch"))));
    }

    #[test]
    fn four_tb_disk_addresses_beyond_two_tb_with_512_and_4kn_sectors() {
        struct SparseDisk { head: Vec<u8>, size: u64 }
        impl BlockDevice for SparseDisk {
            fn size(&self) -> u64 { self.size }
            fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), BlockError> {
                let start = usize::try_from(offset).unwrap();
                let end = start.checked_add(buf.len()).unwrap();
                let src = self.head.get(start..end).expect("GPT must only read the small header/table region");
                buf.copy_from_slice(src);
                Ok(())
            }
        }
        for sector in [512u64, 4096] {
            let size = 4u64 << 40;
            let first = (3u64 << 40) / sector;
            let last = first + 1023;
            let h = sector as usize;
            let table_at = 2 * h;
            let mut head = vec![0u8; 128 * 1024];
            head[510..512].copy_from_slice(&[0x55, 0xaa]);
            head[h..h + 8].copy_from_slice(b"EFI PART");
            head[h + 8..h + 12].copy_from_slice(&0x0001_0000u32.to_le_bytes());
            head[h + 12..h + 16].copy_from_slice(&92u32.to_le_bytes());
            head[h + 24..h + 32].copy_from_slice(&1u64.to_le_bytes());
            head[h + 40..h + 48].copy_from_slice(&34u64.to_le_bytes());
            head[h + 48..h + 56].copy_from_slice(&(size / sector - 34).to_le_bytes());
            head[h + 72..h + 80].copy_from_slice(&2u64.to_le_bytes());
            head[h + 80..h + 84].copy_from_slice(&128u32.to_le_bytes());
            head[h + 84..h + 88].copy_from_slice(&128u32.to_le_bytes());
            head[table_at..table_at + 16].copy_from_slice(&apfs_type_bytes());
            head[table_at + 32..table_at + 40].copy_from_slice(&first.to_le_bytes());
            head[table_at + 40..table_at + 48].copy_from_slice(&last.to_le_bytes());
            let c = crc32(&head[table_at..table_at + 128 * 128]);
            head[h + 88..h + 92].copy_from_slice(&c.to_le_bytes());
            let c = crc32(&head[h..h + 92]);
            head[h + 16..h + 20].copy_from_slice(&c.to_le_bytes());
            let mut disk = SparseDisk { head, size };
            assert_eq!(find_apfs_container(&mut disk, sector).unwrap(), ContainerRange {
                offset: 3u64 << 40, length: 1024 * sector,
            });
        }
    }

    /// A header naming an entry size near `u32::MAX` must be refused up
    /// front; sizing a buffer from it would ask for four gigabytes per entry.
    #[test]
    fn oversized_partition_entry_size_is_rejected() {
        let mut disk = build_disk_with_table(40, 200, 0xFFFF_FF80, 1, 0);
        let err = find_apfs_container(&mut disk, 512)
            .expect_err("an entry size near u32::MAX must be refused");
        assert!(
            matches!(err, GptError::Malformed(m) if m.contains("entry size")),
            "the error must name the entry size, got: {err:?}"
        );
    }

    /// UEFI requires the entry size to be a power of two of at least 128.
    #[test]
    fn non_power_of_two_partition_entry_size_is_rejected() {
        let mut disk = build_disk_with_table(40, 200, 192, 4, 0);
        let err = find_apfs_container(&mut disk, 512)
            .expect_err("an entry size that is not a power of two must be refused");
        assert!(
            matches!(err, GptError::Malformed(m) if m.contains("entry size")),
            "got: {err:?}"
        );
    }

    /// The table as a whole is bounded too: 65 536 entries of 512 bytes is a
    /// 32 MiB table nothing writes.
    #[test]
    fn oversized_partition_table_is_rejected() {
        let mut disk = build_disk_with_table(40, 200, 512, 65_536, 0);
        let err = find_apfs_container(&mut disk, 512)
            .expect_err("a 32 MiB partition table must be refused");
        assert!(
            matches!(err, GptError::Malformed(m) if m.contains("table")),
            "got: {err:?}"
        );
    }

    /// A legitimate table with 256-byte entries is still walked at the
    /// right stride: the APFS entry at index 3 is found.
    #[test]
    fn larger_entry_size_keeps_the_entry_stride() {
        let mut disk = build_disk_with_table(40, 200, 256, 8, 3);
        let r = find_apfs_container(&mut disk, 512).expect("entry at index 3");
        assert_eq!(r.offset, 40 * 512);
        assert_eq!(r.length, (200 - 40 + 1) * 512);
    }

    /// A table that describes a partition running past the end of the medium
    /// must be rejected, not turned into a range every access falls outside.
    #[test]
    fn partition_past_the_end_of_the_device_is_rejected() {
        let mut disk = build_disk(40, 200);
        // Keep the table, cut the disk short.
        disk.data.truncate(100 * 512);
        let err = find_apfs_container(&mut disk, 512)
            .expect_err("a partition past the end must not be accepted");
        assert!(
            matches!(err, GptError::Malformed(m) if m.contains("past the end")),
            "the error must say what is wrong, got: {err:?}"
        );
    }

    #[test]
    fn finds_apfs_partition_range() {
        let mut disk = build_disk(40, 200);
        let r = find_apfs_container(&mut disk, 512).unwrap();
        assert_eq!(r.offset, 40 * 512);
        assert_eq!(r.length, (200 - 40 + 1) * 512);
        // Sanity: NXSB magic sits at container offset + 32.
        let mut m = [0u8; 4];
        disk.read_at(r.offset + 32, &mut m).unwrap();
        assert_eq!(&m, b"NXSB");
    }

    #[test]
    fn rejects_missing_protective_mbr() {
        let mut disk = build_disk(40, 200);
        disk.data[510] = 0;
        assert!(matches!(
            find_apfs_container(&mut disk, 512),
            Err(GptError::NoProtectiveMbr)
        ));
    }

    #[test]
    fn errors_when_no_apfs_partition() {
        let mut disk = build_disk(40, 200);
        let ent = 2 * 512;
        disk.data[ent] ^= 0xFF; // a valid table with a non-APFS type GUID
        let c = crc32(&disk.data[1024..1024 + 128 * 128]);
        disk.data[600..604].copy_from_slice(&c.to_le_bytes());
        disk.data[528..532].fill(0);
        let c = crc32(&disk.data[512..604]);
        disk.data[528..532].copy_from_slice(&c.to_le_bytes());
        assert!(matches!(
            find_apfs_container(&mut disk, 512),
            Err(GptError::NoApfsPartition)
        ));
    }
}
