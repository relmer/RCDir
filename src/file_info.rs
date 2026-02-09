// file_info.rs — File information structures, RAII handles, and file attribute map
//
// Port of: DirectoryInfo.h (FileInfo, SStreamInfo), FileAttributeMap.h, UniqueFindHandle.h

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use windows::Win32::Foundation::HANDLE;
use windows::Win32::Storage::FileSystem::{
    FindClose, WIN32_FIND_DATAW,
};

// ── File attribute constants (Win32 values) ───────────────────────────────────

pub const FILE_ATTRIBUTE_READONLY:      u32 = 0x0000_0001;
pub const FILE_ATTRIBUTE_HIDDEN:        u32 = 0x0000_0002;
pub const FILE_ATTRIBUTE_SYSTEM:        u32 = 0x0000_0004;
pub const FILE_ATTRIBUTE_DIRECTORY:     u32 = 0x0000_0010;
pub const FILE_ATTRIBUTE_ARCHIVE:       u32 = 0x0000_0020;
pub const FILE_ATTRIBUTE_TEMPORARY:     u32 = 0x0000_0040;
pub const FILE_ATTRIBUTE_ENCRYPTED:     u32 = 0x0000_4000;
pub const FILE_ATTRIBUTE_COMPRESSED:    u32 = 0x0000_0800;
pub const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
pub const FILE_ATTRIBUTE_SPARSE_FILE:   u32 = 0x0000_0200;

// ── File attribute map ────────────────────────────────────────────────────────

/// Maps file attribute flags to their single-char display keys.
/// Port of: FileAttributeMap.h → k_rgFileAttributeMap[]
///
/// Used by Config (env var attr: overrides) and ResultsDisplayerNormal (attribute column).
pub const FILE_ATTRIBUTE_MAP: [(u32, char); 9] = [
    (FILE_ATTRIBUTE_READONLY,      'R'),
    (FILE_ATTRIBUTE_HIDDEN,        'H'),
    (FILE_ATTRIBUTE_SYSTEM,        'S'),
    (FILE_ATTRIBUTE_ARCHIVE,       'A'),
    (FILE_ATTRIBUTE_TEMPORARY,     'T'),
    (FILE_ATTRIBUTE_ENCRYPTED,     'E'),
    (FILE_ATTRIBUTE_COMPRESSED,    'C'),
    (FILE_ATTRIBUTE_REPARSE_POINT, 'P'),
    (FILE_ATTRIBUTE_SPARSE_FILE,   '0'),
];

// ── RAII handles ──────────────────────────────────────────────────────────────

/// RAII wrapper for Win32 find handles (FindFirstFile/FindNextFile).
/// Drop calls FindClose. NOT interchangeable with SafeHandle (per research R-02).
///
/// Port of: UniqueFindHandle.h → UniqueFindHandle
pub struct FindHandle(pub HANDLE);

impl Drop for FindHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe { let _ = FindClose(self.0); }
        }
    }
}

/// RAII wrapper for generic Win32 handles.
/// Drop calls CloseHandle. NOT interchangeable with FindHandle.
pub struct SafeHandle(pub HANDLE);

impl Drop for SafeHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe { let _ = windows::Win32::Foundation::CloseHandle(self.0); }
        }
    }
}

// ── Stream information ────────────────────────────────────────────────────────

/// Port of: SStreamInfo
#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub name: String,       // Stream name (e.g., ":hidden", stripped of :$DATA suffix)
    pub size: i64,          // Stream size in bytes
}

// ── File information ──────────────────────────────────────────────────────────

/// Port of: FileInfo (extends WIN32_FIND_DATA)
///
/// Holds all information about a single file entry from FindFirstFile/FindNextFile.
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub file_name:       OsString,
    pub file_attributes: u32,
    pub file_size:       u64,
    pub creation_time:   u64,    // FILETIME as u64
    pub last_write_time: u64,    // FILETIME as u64
    pub last_access_time: u64,   // FILETIME as u64
    pub reparse_tag:     u32,    // dwReserved0 — reparse tag for cloud/symlink detection
    pub streams:         Vec<StreamInfo>,
}

impl FileInfo {
    /// Construct a FileInfo from WIN32_FIND_DATAW fields.
    ///
    /// Port of: FileInfo(const WIN32_FIND_DATA & wfd) constructor
    pub fn from_find_data(wfd: &WIN32_FIND_DATAW) -> Self {
        // Extract the filename from the wide char array (null-terminated)
        let name_len = wfd.cFileName.iter().position(|&c| c == 0).unwrap_or(wfd.cFileName.len());
        let file_name = OsString::from_wide(&wfd.cFileName[..name_len]);

        // Combine high/low parts into u64 for sizes and times
        let file_size = ((wfd.nFileSizeHigh as u64) << 32) | (wfd.nFileSizeLow as u64);

        let creation_time   = ((wfd.ftCreationTime.dwHighDateTime as u64) << 32)
                            | (wfd.ftCreationTime.dwLowDateTime as u64);
        let last_write_time = ((wfd.ftLastWriteTime.dwHighDateTime as u64) << 32)
                            | (wfd.ftLastWriteTime.dwLowDateTime as u64);
        let last_access_time = ((wfd.ftLastAccessTime.dwHighDateTime as u64) << 32)
                             | (wfd.ftLastAccessTime.dwLowDateTime as u64);

        FileInfo {
            file_name,
            file_attributes: wfd.dwFileAttributes,
            file_size,
            creation_time,
            last_write_time,
            last_access_time,
            reparse_tag: wfd.dwReserved0,
            streams: Vec::new(),
        }
    }

    pub fn is_directory(&self) -> bool {
        (self.file_attributes & FILE_ATTRIBUTE_DIRECTORY) != 0
    }

    pub fn is_hidden(&self) -> bool {
        (self.file_attributes & FILE_ATTRIBUTE_HIDDEN) != 0
    }

    pub fn is_dot_dir(&self) -> bool {
        let name = self.file_name.to_string_lossy();
        name == "." || name == ".."
    }
}

// ── Attribute display string ──────────────────────────────────────────────────

/// Build a 9-char attribute display string from file attributes.
/// Present attributes show their letter; absent show '-'.
///
/// Port of: FileAttributeMap.h → attribute display logic in ResultsDisplayerNormal
///
/// Example: file with RHSA set → "RHSA-----"
pub fn build_attribute_display_string(attributes: u32) -> String {
    let mut result = String::with_capacity(FILE_ATTRIBUTE_MAP.len());

    for &(flag, ch) in &FILE_ATTRIBUTE_MAP {
        if (attributes & flag) != 0 {
            result.push(ch);
        } else {
            result.push('-');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_attribute_map_count() {
        assert_eq!(FILE_ATTRIBUTE_MAP.len(), 9);
    }

    #[test]
    fn file_attribute_map_keys_unique() {
        let chars: Vec<char> = FILE_ATTRIBUTE_MAP.iter().map(|&(_, c)| c).collect();
        let mut deduped = chars.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(chars.len(), deduped.len());
    }

    #[test]
    fn is_directory_flag() {
        let fi = FileInfo {
            file_name:       OsString::from("test"),
            file_attributes: FILE_ATTRIBUTE_DIRECTORY,
            file_size:       0,
            creation_time:   0,
            last_write_time: 0,
            last_access_time: 0,
            reparse_tag:     0,
            streams:         Vec::new(),
        };
        assert!(fi.is_directory());
    }

    #[test]
    fn is_dot_dir() {
        let fi = FileInfo {
            file_name:       OsString::from(".."),
            file_attributes: FILE_ATTRIBUTE_DIRECTORY,
            file_size:       0,
            creation_time:   0,
            last_write_time: 0,
            last_access_time: 0,
            reparse_tag:     0,
            streams:         Vec::new(),
        };
        assert!(fi.is_dot_dir());
    }

    #[test]
    fn build_attribute_string_no_attrs() {
        assert_eq!(build_attribute_display_string(0), "---------");
    }

    #[test]
    fn build_attribute_string_all_attrs() {
        let all = FILE_ATTRIBUTE_READONLY | FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM
                | FILE_ATTRIBUTE_ARCHIVE | FILE_ATTRIBUTE_TEMPORARY | FILE_ATTRIBUTE_ENCRYPTED
                | FILE_ATTRIBUTE_COMPRESSED | FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_SPARSE_FILE;
        assert_eq!(build_attribute_display_string(all), "RHSATECP0");
    }

    #[test]
    fn build_attribute_string_partial() {
        let attrs = FILE_ATTRIBUTE_READONLY | FILE_ATTRIBUTE_ARCHIVE;
        assert_eq!(build_attribute_display_string(attrs), "R--A-----");
    }

    #[test]
    fn from_find_data_basic() {
        let mut wfd = WIN32_FIND_DATAW::default();
        wfd.dwFileAttributes = FILE_ATTRIBUTE_ARCHIVE | FILE_ATTRIBUTE_READONLY;
        wfd.nFileSizeHigh = 0;
        wfd.nFileSizeLow = 12345;
        wfd.ftCreationTime.dwHighDateTime = 0x01D0;
        wfd.ftCreationTime.dwLowDateTime = 0xABCD;

        // Set filename "test.rs\0"
        let name: &[u16] = &[b't' as u16, b'e' as u16, b's' as u16, b't' as u16,
                              b'.' as u16, b'r' as u16, b's' as u16, 0];
        wfd.cFileName[..name.len()].copy_from_slice(name);

        let fi = FileInfo::from_find_data(&wfd);
        assert_eq!(fi.file_name, OsString::from("test.rs"));
        assert_eq!(fi.file_attributes, FILE_ATTRIBUTE_ARCHIVE | FILE_ATTRIBUTE_READONLY);
        assert_eq!(fi.file_size, 12345);
        assert_eq!(fi.creation_time, ((0x01D0u64) << 32) | 0xABCD);
        assert!(fi.streams.is_empty());
    }
}
