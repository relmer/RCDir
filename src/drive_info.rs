// drive_info.rs — Volume and drive information
//
// Port of: DriveInfo.h → CDriveInfo
//
// Stub — full implementation in US-1.

use std::path::PathBuf;

use crate::ehm::AppError;

/// Volume type constants (matching GetDriveType return values).
pub const DRIVE_UNKNOWN:     u32 = 0;
pub const DRIVE_NO_ROOT_DIR: u32 = 1;
pub const DRIVE_REMOVABLE:   u32 = 2;
pub const DRIVE_FIXED:       u32 = 3;
pub const DRIVE_REMOTE:      u32 = 4;
pub const DRIVE_CDROM:       u32 = 5;
pub const DRIVE_RAMDISK:     u32 = 6;

/// Volume type descriptions (indexed by GetDriveType value).
pub const VOLUME_DESCRIPTIONS: [&str; 7] = [
    "an unknown type",
    "an unknown type",
    "a removable disk",
    "a hard drive",
    "a network drive",
    "a CD/DVD",
    "a RAM disk",
];

/// Port of: CDriveInfo
pub struct DriveInfo {
    pub unc_path:        PathBuf,
    pub root_path:       PathBuf,
    pub volume_name:     String,
    pub file_system_name: String,
    pub volume_type:     u32,
    pub is_unc_path:     bool,
    pub remote_name:     String,
}

impl DriveInfo {
    /// Create a DriveInfo for the given directory path.
    /// Stub — full implementation in US-1.
    pub fn new(_dir_path: &std::path::Path) -> Result<Self, AppError> {
        Ok(DriveInfo {
            unc_path:        PathBuf::new(),
            root_path:       PathBuf::new(),
            volume_name:     String::new(),
            file_system_name: String::new(),
            volume_type:     DRIVE_UNKNOWN,
            is_unc_path:     false,
            remote_name:     String::new(),
        })
    }

    pub fn volume_description(&self) -> &str {
        VOLUME_DESCRIPTIONS.get(self.volume_type as usize).unwrap_or(&"an unknown type")
    }

    pub fn is_ntfs(&self) -> bool {
        self.file_system_name.eq_ignore_ascii_case("NTFS")
    }

    pub fn is_refs(&self) -> bool {
        self.file_system_name.eq_ignore_ascii_case("ReFS")
    }
}
