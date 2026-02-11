// cloud_status.rs — Cloud file status detection (OneDrive, iCloud, etc.)
//
// Port of: ResultsDisplayerNormal.cpp → IsUnderSyncRoot(), GetCloudStatus()
//          UnicodeSymbols.h → cloud circle symbols
//
// Detects whether a directory is under a cloud sync root and determines
// per-file cloud placeholder state from file attributes.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_OFFLINE,
    FILE_ATTRIBUTE_PINNED,
    FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
    FILE_ATTRIBUTE_RECALL_ON_OPEN,
    FILE_ATTRIBUTE_UNPINNED,
};





////////////////////////////////////////////////////////////////////////////////
// Port of: UnicodeSymbols.h

/// ○ Cloud-only (not locally available)
pub const CIRCLE_HOLLOW: char      = '\u{25CB}';
/// ◐ Locally available (can be dehydrated)
pub const CIRCLE_HALF_FILLED: char = '\u{25D0}';
/// ● Always locally available (pinned)
pub const CIRCLE_FILLED: char      = '\u{25CF}';





////////////////////////////////////////////////////////////////////////////////
// Port of: ECloudStatus in ResultsDisplayerNormal.h

/// Cloud sync status for a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudStatus {
    /// Not a cloud file (or not in a sync root)
    None,
    /// Placeholder, not locally available (○)
    CloudOnly,
    /// Available locally, can be dehydrated (◐)
    Local,
    /// Pinned, always available locally (●)
    Pinned,
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl CloudStatus
//
//  Get the display symbol for this cloud status.
//
////////////////////////////////////////////////////////////////////////////////

impl CloudStatus {
    pub fn symbol(self) -> char {
        match self {
            CloudStatus::None      => ' ',
            CloudStatus::CloudOnly => CIRCLE_HOLLOW,
            CloudStatus::Local     => CIRCLE_HALF_FILLED,
            CloudStatus::Pinned    => CIRCLE_FILLED,
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  is_under_sync_root
//
//  Check whether a directory path is under a cloud sync root (OneDrive,
//  iCloud, etc.).  Uses CfGetSyncRootInfoByPath — if it succeeds, the path
//  is under a sync root.
//
//  Port of: CResultsDisplayerNormal::IsUnderSyncRoot
//
////////////////////////////////////////////////////////////////////////////////

pub fn is_under_sync_root(path: &OsStr) -> bool {
    use windows::Win32::Storage::CloudFilters::{
        CfGetSyncRootInfoByPath, CF_SYNC_ROOT_INFO_BASIC,
    };

    let path_wide: Vec<u16> = path.encode_wide().chain(Some(0)).collect();
    let mut info = [0u8; 256]; // CF_SYNC_ROOT_BASIC_INFO is small

    let hr = unsafe {
        CfGetSyncRootInfoByPath(
            windows::core::PCWSTR(path_wide.as_ptr()),
            CF_SYNC_ROOT_INFO_BASIC,
            info.as_mut_ptr() as *mut _,
            info.len() as u32,
            None,
        )
    };

    hr.is_ok()
}





////////////////////////////////////////////////////////////////////////////////
//
//  get_cloud_status
//
//  Determine the cloud status of a file from its attributes.  Only meaningful
//  when the directory is under a sync root (call is_under_sync_root first).
//
//  Port of: CResultsDisplayerNormal::GetCloudStatus
//
////////////////////////////////////////////////////////////////////////////////

pub fn get_cloud_status(file_attributes: u32, in_sync_root: bool) -> CloudStatus {
    if !in_sync_root {
        return CloudStatus::None;
    }

    if (file_attributes & FILE_ATTRIBUTE_PINNED.0) != 0 {
        // Pinned takes priority — always available locally
        CloudStatus::Pinned
    } else if (file_attributes & (FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS.0
                                | FILE_ATTRIBUTE_RECALL_ON_OPEN.0
                                | FILE_ATTRIBUTE_OFFLINE.0)) != 0 {
        // Cloud-only: placeholder that requires download
        CloudStatus::CloudOnly
    } else if (file_attributes & FILE_ATTRIBUTE_UNPINNED.0) != 0 {
        // Unpinned means locally available but can be dehydrated
        CloudStatus::Local
    } else {
        // No cloud attributes set — fully hydrated (locally synced)
        CloudStatus::Local
    }
}





#[cfg(test)]
mod tests {
    use super::*;

    ////////////////////////////////////////////////////////////////////////////
    //
    //  cloud_status_none_when_not_in_sync_root
    //
    //  Verify None is returned when not in a sync root.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn cloud_status_none_when_not_in_sync_root() {
        assert_eq!(get_cloud_status(0, false), CloudStatus::None);
        assert_eq!(get_cloud_status(FILE_ATTRIBUTE_PINNED.0, false), CloudStatus::None);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  cloud_status_pinned_takes_priority
    //
    //  Verify Pinned takes priority over other cloud attributes.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn cloud_status_pinned_takes_priority() {
        let attrs = FILE_ATTRIBUTE_PINNED.0 | FILE_ATTRIBUTE_UNPINNED.0;
        assert_eq!(get_cloud_status(attrs, true), CloudStatus::Pinned);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  cloud_status_cloud_only
    //
    //  Verify CloudOnly is returned for offline/recall attributes.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn cloud_status_cloud_only() {
        assert_eq!(get_cloud_status(FILE_ATTRIBUTE_OFFLINE.0, true), CloudStatus::CloudOnly);
        assert_eq!(get_cloud_status(FILE_ATTRIBUTE_RECALL_ON_OPEN.0, true), CloudStatus::CloudOnly);
        assert_eq!(get_cloud_status(FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS.0, true), CloudStatus::CloudOnly);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  cloud_status_local_when_unpinned
    //
    //  Verify Local is returned for unpinned files.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn cloud_status_local_when_unpinned() {
        assert_eq!(get_cloud_status(FILE_ATTRIBUTE_UNPINNED.0, true), CloudStatus::Local);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  cloud_status_local_when_no_cloud_attrs
    //
    //  Verify Local is returned when no cloud attributes are set.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn cloud_status_local_when_no_cloud_attrs() {
        assert_eq!(get_cloud_status(0x20, true), CloudStatus::Local); // FILE_ATTRIBUTE_ARCHIVE
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  symbol_mapping
    //
    //  Verify each CloudStatus variant maps to its expected symbol.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn symbol_mapping() {
        assert_eq!(CloudStatus::None.symbol(), ' ');
        assert_eq!(CloudStatus::CloudOnly.symbol(), CIRCLE_HOLLOW);
        assert_eq!(CloudStatus::Local.symbol(), CIRCLE_HALF_FILLED);
        assert_eq!(CloudStatus::Pinned.symbol(), CIRCLE_FILLED);
    }
}
