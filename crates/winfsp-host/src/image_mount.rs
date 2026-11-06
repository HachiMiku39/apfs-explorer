//! `--image <path>` mount path: open a raw `.img`/`.dmg` file as a block
//! device, run GPT autodetect to locate the APFS container, then hand it
//! off to the WinFsp mount stack (Windows-only).
//!
//! The open + GPT-detect layer (`probe_image`) is always compiled so it can
//! be unit-tested on macOS.  The WinFsp mount entry point is
//! `#[cfg(windows)]`-gated, mirroring the `mount` module convention.

use std::path::Path;

use apfs_core::block_device::{BlockDevice, BlockError, FileBlockDevice, ReadOnlyBlockDevice};

use crate::gpt::{self, ContainerRange, GptError};
#[cfg(windows)]
use crate::offset_dev::OffsetBlockDevice;

/// Minimum image size accepted: one APFS block (4 KiB).
const APFS_MIN_SIZE: u64 = 4096;

/// Error type for the image-open + GPT-detect path.
#[derive(Debug)]
pub enum ImageMountError {
    /// The image file could not be opened or stat'd.
    Io(std::io::Error),
    /// The block layer returned an error during the GPT scan.
    Block(BlockError),
    /// GPT parsing failed (no valid APFS partition found).
    Gpt(GptError),
    /// File is too small to be a valid APFS container.
    TooSmall(u64),
    InvalidContainer(apfs_core::container::ContainerError),
}

impl std::fmt::Display for ImageMountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidContainer(e) => write!(f, "invalid APFS container: {e:?}"),
            Self::Io(e) => write!(f, "image I/O error: {e}"),
            Self::Block(e) => write!(f, "block device error: {e}"),
            Self::Gpt(e) => write!(f, "GPT parse error: {e:?}"),
            Self::TooSmall(n) => write!(
                f,
                "image too small ({n} bytes); minimum is {APFS_MIN_SIZE} bytes"
            ),
        }
    }
}

impl From<std::io::Error> for ImageMountError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<BlockError> for ImageMountError {
    fn from(e: BlockError) -> Self {
        Self::Block(e)
    }
}

impl From<GptError> for ImageMountError {
    fn from(e: GptError) -> Self {
        Self::Gpt(e)
    }
}

/// Validate image size and locate the APFS container range via GPT.
///
/// Opens a read-only handle. Bare images must contain a valid APFS
/// superblock and checkpoint; arbitrary bytes are never guessed as APFS.
/// Returns `(file_size, ContainerRange)`.
pub fn probe_image(path: &Path) -> Result<(u64, ContainerRange), ImageMountError> {
    let (dev, range) = open_image(path)?;
    Ok((dev.size(), range))
}

fn open_image(
    path: &Path,
) -> Result<(ReadOnlyBlockDevice<FileBlockDevice>, ContainerRange), ImageMountError> {
    let mut dev = ReadOnlyBlockDevice::new(FileBlockDevice::open(path)?);
    let size = dev.size();
    if size < APFS_MIN_SIZE {
        return Err(ImageMountError::TooSmall(size));
    }
    // Disk images created by hdiutil/dd use 512-byte logical blocks.
    let range = match gpt::find_apfs_container(&mut dev, 512) {
        Ok(r) => r,
        // No GPT protective MBR - treat the whole file as a bare container.
        Err(GptError::NoProtectiveMbr) => ContainerRange {
            offset: 0,
            length: size,
        },
        Err(e) => return Err(ImageMountError::Gpt(e)),
    };
    let mut bounded = crate::offset_dev::OffsetBlockDevice::new(dev, range.offset, range.length);
    apfs_core::container::Container::open(&mut bounded)
        .map_err(ImageMountError::InvalidContainer)?;
    // Return the same opened handle: no path reopen between validation and mount.
    Ok((bounded.into_inner(), range))
}

// ---------------------------------------------------------------------------
// Windows-only WinFsp mount entry point
// ---------------------------------------------------------------------------

#[cfg(windows)]
type ImageDev =
    crate::cache::CachingBlockDevice<OffsetBlockDevice<ReadOnlyBlockDevice<FileBlockDevice>>>;

/// Mount a raw disk-image file as a Windows drive letter via WinFsp.
///
/// Parallels [`crate::mount::mount_apfs`] for physical drives.
///
/// * `path`       - `.img` or `.dmg` file path.
/// * `writable`   - rejected in OpenAPFS v0.1; retained for API compatibility.
/// * `mountpoint` - e.g. `"Z:"`.
#[cfg(windows)]
pub fn mount_apfs_image(
    path: &Path,
    writable: bool,
    mountpoint: &str,
    selector: apfs::VolumeSelector,
    snapshot: Option<&apfs::SnapshotRef>,
) -> winfsp::Result<(
    winfsp::host::FileSystemHost<crate::mount::ApfsFs<ImageDev>, winfsp::host::FineGuard>,
    std::sync::Arc<std::sync::atomic::AtomicBool>,
)> {
    use apfs::FsView;
    use winfsp::host::{FileSystemHost, FineGuard, VolumeParams};

    use crate::cache::CachingBlockDevice;
    use crate::mount::{ensure_winfsp_dll, io_err, ApfsFs};

    crate::safety::require_read_only(writable).map_err(io_err("write policy"))?;
    ensure_winfsp_dll();
    let init = winfsp::winfsp_init().map_err(io_err("winfsp_init"))?;
    let _ = init;

    let (file_dev, range) = open_image(path).map_err(io_err("open_image"))?;

    let dev = CachingBlockDevice::new(OffsetBlockDevice::new(file_dev, range.offset, range.length));
    // A snapshot is never written.
    let writable = writable && snapshot.is_none();
    let mut fsview = match snapshot {
        Some(s) => FsView::open_snapshot(crate::group::GroupDev::Plain(dev), selector, s),
        None => FsView::open_selected(crate::group::GroupDev::Plain(dev), selector),
    }
    .map_err(io_err("FsView::open"))?;
    let writable = crate::mount::effective_writable(&mut fsview, writable);
    // P14: stable Windows volume serial from the APFS volume UUID (OR 1).
    let serial = {
        let u = fsview.volume_uuid().unwrap_or([0u8; 16]);
        u32::from_le_bytes([u[0], u[1], u[2], u[3]]) | 1
    };

    let mut vp = VolumeParams::new();
    vp.filesystem_name("APFS")
        .sector_size(4096)
        .sectors_per_allocation_unit(1)
        .case_sensitive_search(false)
        .case_preserved_names(true)
        .unicode_on_disk(true)
        .read_only_volume(!writable)
        .post_cleanup_when_modified_only(true)
        // Same as the drive mount; see PassQueryDirectoryFileName in mount.rs.
        .pass_query_directory_filename(true)
        .persistent_acls(true)
        .flush_and_purge_on_cleanup(true)
        // P14 + P7 (parity with the physical-drive mount path).
        .volume_serial_number(serial)
        .max_component_length(255)
        .reparse_points(true);
    crate::mount::apply_info_timeouts(&mut vp);
    // NOTE: WinFsp 2.1 SDK (winfsp-sys 0.12 / winfsp 0.13) does not expose a
    // RemovableMedia bitfield in FSP_FSCTL_VOLUME_PARAMS - the field was present
    // in earlier WinFsp versions but is absent from the 2.1 bindgen output.
    // WinFsp disk volumes appear as DriveType=3 (Fixed) by default.
    // Removable-media reporting (DriveType=2, "Safely Remove Hardware" tray)
    // is a deferred enhancement tracked in the M13 roadmap.

    let fs = ApfsFs::new(fsview).read_only(!writable);
    let dirty_arc = fs.dirty_arc();
    let mut host = FileSystemHost::<ApfsFs<ImageDev>, FineGuard>::new_with_options(
        crate::mount::host_params(vp),
        fs,
    )
    .map_err(io_err("FileSystemHost::new"))?;
    // Plain drive-letter mount; cross-elevation visibility via the install-time
    // EnableLinkedConnections policy (see mount_apfs for the rationale).
    host.mount(mountpoint).map_err(io_err("host.mount"))?;
    host.start().map_err(io_err("host.start"))?;
    Ok((host, dirty_arc))
}

// ---------------------------------------------------------------------------
// Unit tests (cross-platform)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp_zeros(size: usize) -> (tempfile::NamedTempFile, std::path::PathBuf) {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(&vec![0u8; size]).unwrap();
        let p = f.path().to_path_buf();
        (f, p)
    }

    #[test]
    fn too_small_image_rejected() {
        let (_f, p) = write_tmp_zeros(512);
        let err = probe_image(&p).unwrap_err();
        assert!(matches!(err, ImageMountError::TooSmall(_)));
    }

    #[test]
    fn arbitrary_bytes_are_not_guessed_as_a_container() {
        let (_f, p) = write_tmp_zeros(64 * 1024);
        assert!(matches!(
            probe_image(&p),
            Err(ImageMountError::InvalidContainer(_))
        ));
    }
}
