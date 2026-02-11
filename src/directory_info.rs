// directory_info.rs — Directory enumeration results container
//
// Port of: DirectoryInfo.h → CDirectoryInfo

use std::path::PathBuf;
use std::sync::{Arc, Mutex, Condvar};

use crate::file_info::FileInfo;





/// Multithreading status for a directory enumeration job.
/// Port of: CDirectoryInfo::Status
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DirectoryStatus {
    Waiting,
    InProgress,
    Done,
    Error,
}





/// Port of: CDirectoryInfo
///
/// Contains the results of a directory enumeration: matched files, counts, sizes.
/// In multithreaded mode, each node is wrapped in Arc<Mutex<DirectoryInfo>>
/// and the condvar is used to signal completion.
pub struct DirectoryInfo {
    pub matches:             Vec<FileInfo>,
    pub dir_path:            PathBuf,
    pub file_specs:          Vec<String>,
    pub largest_file_size:   u64,
    pub largest_file_name:   usize,
    pub file_count:          u32,
    pub subdirectory_count:  u32,
    pub stream_count:        u32,
    pub bytes_used:          u64,
    pub stream_bytes_used:   u64,

    // Multithreading support
    pub status:              DirectoryStatus,
    pub error:               Option<String>,
    pub children:            Vec<Arc<(Mutex<DirectoryInfo>, Condvar)>>,
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl DirectoryInfo
//
//  Construction of directory info entries for listing.
//
////////////////////////////////////////////////////////////////////////////////

impl DirectoryInfo {
    ////////////////////////////////////////////////////////////////////////////
    //
    //  new
    //
    //  Create a DirectoryInfo for a single file spec.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn new(dir_path: PathBuf, file_spec: String) -> Self {
        DirectoryInfo {
            matches:            Vec::new(),
            dir_path,
            file_specs:         vec![file_spec],
            largest_file_size:  0,
            largest_file_name:  0,
            file_count:         0,
            subdirectory_count: 0,
            stream_count:       0,
            bytes_used:         0,
            stream_bytes_used:  0,
            status:             DirectoryStatus::Waiting,
            error:              None,
            children:           Vec::new(),
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  new_multi
    //
    //  Create a DirectoryInfo for multiple file specs.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn new_multi(dir_path: PathBuf, file_specs: Vec<String>) -> Self {
        DirectoryInfo {
            matches:            Vec::new(),
            dir_path,
            file_specs,
            largest_file_size:  0,
            largest_file_name:  0,
            file_count:         0,
            subdirectory_count: 0,
            stream_count:       0,
            bytes_used:         0,
            stream_bytes_used:  0,
            status:             DirectoryStatus::Waiting,
            error:              None,
            children:           Vec::new(),
        }
    }
}
