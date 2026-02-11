// drive_info.rs — Volume and drive information
//
// Port of: DriveInfo.h, DriveInfo.cpp → CDriveInfo
//
// Retrieves volume label, filesystem name, drive type, and UNC info
// using Win32 APIs: GetVolumeInformationW, GetDriveTypeW, WNetGetConnectionW.

use std::path::{Path, PathBuf};

use widestring::U16CString;

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





////////////////////////////////////////////////////////////////////////////////
//
//  impl DriveInfo
//
//  Drive and volume information queries.
//
////////////////////////////////////////////////////////////////////////////////

impl DriveInfo {
    ////////////////////////////////////////////////////////////////////////////
    //
    //  new
    //
    //  Create a DriveInfo for the given directory path.  Queries volume
    //  information, drive type, and UNC mapping.
    //
    //  Port of: CDriveInfo::CDriveInfo
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn new(dir_path: &Path) -> Result<Self, AppError> {
        let mut info = DriveInfo {
            unc_path:        PathBuf::new(),
            root_path:       PathBuf::new(),
            volume_name:     String::new(),
            file_system_name: String::new(),
            volume_type:     DRIVE_UNKNOWN,
            is_unc_path:     false,
            remote_name:     String::new(),
        };

        info.initialize_volume_info(dir_path);
        info.initialize_unc_info();

        Ok(info)
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  volume_description
    //
    //  Return a human-readable description of the volume type.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn volume_description(&self) -> &str {
        VOLUME_DESCRIPTIONS.get(self.volume_type as usize).unwrap_or(&"an unknown type")
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  is_ntfs
    //
    //  Return true if the volume is NTFS.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn is_ntfs(&self) -> bool {
        self.file_system_name.eq_ignore_ascii_case("NTFS")
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  is_refs
    //
    //  Return true if the volume is ReFS.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn is_refs(&self) -> bool {
        self.file_system_name.eq_ignore_ascii_case("ReFS")
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  initialize_volume_info
    //
    //  Initialize volume information from the directory path.
    //
    //  Port of: CDriveInfo::InitializeVolumeInfo
    //
    ////////////////////////////////////////////////////////////////////////////

    fn initialize_volume_info(&mut self, dir_path: &Path) {
        // Get the root path (e.g., "C:\")
        // Check if it has a drive letter
        let dir_str = dir_path.to_string_lossy();
        let has_drive_letter = dir_str.len() >= 2
            && dir_str.as_bytes()[1] == b':'
            && dir_str.as_bytes()[0].is_ascii_alphabetic();

        if has_drive_letter {
            // Local drive — extract root path "X:\"
            let drive_letter = dir_str.as_bytes()[0] as char;
            self.root_path = PathBuf::from(format!("{}:\\", drive_letter));

            // Get drive type
            if let Ok(root_wide) = U16CString::from_str(self.root_path.to_string_lossy()) {
                self.volume_type = unsafe {
                    windows::Win32::Storage::FileSystem::GetDriveTypeW(
                        windows::core::PCWSTR(root_wide.as_ptr()),
                    )
                };
            }
        } else {
            // No drive letter → UNC path
            self.is_unc_path = true;
            self.unc_path = dir_path.to_path_buf();
            self.root_path = if let Some(root) = dir_path.ancestors().last() {
                root.to_path_buf()
            } else {
                dir_path.to_path_buf()
            };
            self.volume_type = DRIVE_REMOTE;
        }

        // Get volume information
        let root_str = self.root_path.to_string_lossy();
        if let Ok(root_wide) = U16CString::from_str(&*root_str) {
            let mut volume_name_buf = [0u16; 261];
            let mut fs_name_buf = [0u16; 261];

            let success = unsafe {
                windows::Win32::Storage::FileSystem::GetVolumeInformationW(
                    windows::core::PCWSTR(root_wide.as_ptr()),
                    Some(&mut volume_name_buf),
                    None,
                    None,
                    None,
                    Some(&mut fs_name_buf),
                )
            };

            if success.is_ok() {
                // Extract volume name
                let vn_len = volume_name_buf.iter().position(|&c| c == 0).unwrap_or(volume_name_buf.len());
                self.volume_name = String::from_utf16_lossy(&volume_name_buf[..vn_len]);

                // Extract filesystem name
                let fs_len = fs_name_buf.iter().position(|&c| c == 0).unwrap_or(fs_name_buf.len());
                self.file_system_name = String::from_utf16_lossy(&fs_name_buf[..fs_len]);
            }
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  initialize_unc_info
    //
    //  Initialize UNC/mapped drive info.  If this is a remote drive, try
    //  WNetGetConnectionW to get the remote name.
    //
    //  Port of: CDriveInfo::InitializeUncInfo
    //
    ////////////////////////////////////////////////////////////////////////////

    fn initialize_unc_info(&mut self) {
        if !self.is_unc_path || self.volume_type != DRIVE_REMOTE {
            return;
        }

        // Get the root name (drive letter part) for WNetGetConnection
        let root_str = self.root_path.to_string_lossy();
        let local_name = if root_str.len() >= 2 && root_str.as_bytes()[1] == b':' {
            format!("{}:", root_str.as_bytes()[0] as char)
        } else {
            return; // UNC paths like \\server\share don't have a mapped drive letter
        };

        if let Ok(local_wide) = U16CString::from_str(&local_name) {
            let mut remote_buf = vec![0u16; 261];
            let mut buf_len = remote_buf.len() as u32;

            let result = unsafe {
                windows::Win32::NetworkManagement::WNet::WNetGetConnectionW(
                    windows::core::PCWSTR(local_wide.as_ptr()),
                    Some(windows::core::PWSTR(remote_buf.as_mut_ptr())),
                    &mut buf_len,
                )
            };

            if result == windows::Win32::Foundation::WIN32_ERROR(0) {
                let rn_len = remote_buf.iter().position(|&c| c == 0).unwrap_or(remote_buf.len());
                self.remote_name = String::from_utf16_lossy(&remote_buf[..rn_len]);
            } else if result == windows::Win32::Foundation::WIN32_ERROR(234) {
                // ERROR_MORE_DATA — retry with larger buffer
                remote_buf.resize(buf_len as usize, 0);
                let result2 = unsafe {
                    windows::Win32::NetworkManagement::WNet::WNetGetConnectionW(
                        windows::core::PCWSTR(local_wide.as_ptr()),
                        Some(windows::core::PWSTR(remote_buf.as_mut_ptr())),
                        &mut buf_len,
                    )
                };
                if result2 == windows::Win32::Foundation::WIN32_ERROR(0) {
                    let rn_len = remote_buf.iter().position(|&c| c == 0).unwrap_or(remote_buf.len());
                    self.remote_name = String::from_utf16_lossy(&remote_buf[..rn_len]);
                }
            }
        }
    }
}
