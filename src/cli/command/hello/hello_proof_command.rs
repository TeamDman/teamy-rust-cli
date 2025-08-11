use crate::cli::to_args::ToArgs;
use crate::windows::win_elevation::ensure_elevated;
use crate::windows::win_handles::get_drive_handle;
use arbitrary::Arbitrary;
use clap::Args;
use eyre::Context;
use eyre::eyre;
use std::ffi::OsString;
use std::mem::size_of;
use tracing::info;
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Ioctl::FSCTL_GET_NTFS_VOLUME_DATA;
use windows::Win32::System::Ioctl::NTFS_VOLUME_DATA_BUFFER;

#[derive(Args, Clone, Arbitrary, PartialEq, Debug)]
pub struct HelloProofArgs {}

impl HelloProofArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        ensure_elevated()?;
        validate_ntfs_filesystem('C')?;
        Ok(())
    }
}

impl ToArgs for HelloProofArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![]
    }
}

/// Validates that the specified drive is using NTFS filesystem
fn validate_ntfs_filesystem(drive_letter: char) -> eyre::Result<()> {
    // For now, we'll validate by attempting to get NTFS volume data
    // If this succeeds, we know it's an NTFS volume
    let drive_handle = get_drive_handle(drive_letter)
        .with_context(|| format!("Failed to open handle to drive {drive_letter}"))?;

    let mut volume_data = NTFS_VOLUME_DATA_BUFFER::default();
    let mut bytes_returned = 0u32;

    let result = unsafe {
        DeviceIoControl(
            *drive_handle,
            FSCTL_GET_NTFS_VOLUME_DATA,
            None,
            0,
            Some(&mut volume_data as *mut _ as *mut _),
            size_of::<NTFS_VOLUME_DATA_BUFFER>() as u32,
            Some(&mut bytes_returned),
            None,
        )
    };

    match result {
        Ok(_) => {
            info!(
                "✓ Filesystem validation passed: Drive {} is using NTFS",
                drive_letter
            );
            info!("NTFS Volume Info:");
            // info!("  VolumeSerialNumber: 0x{:X}", volume_data.VolumeSerialNumber);
            info!("  NumberSectors: {}", volume_data.NumberSectors);
            info!("  TotalClusters: {}", volume_data.TotalClusters);
            info!("  FreeClusters: {}", volume_data.FreeClusters);
            info!("  BytesPerSector: {}", volume_data.BytesPerSector);
            info!("  BytesPerCluster: {}", volume_data.BytesPerCluster);
            Ok(())
        }
        Err(e) => Err(eyre!(
            "Drive {} does not appear to be using NTFS filesystem. FSCTL_GET_NTFS_VOLUME_DATA failed: {}. MFT dumping is only supported on NTFS volumes.",
            drive_letter,
            e
        )),
    }
}
