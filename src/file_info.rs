// file_info.rs — File information structures and file attribute map
//
// Port of: DirectoryInfo.h (FileInfo, SStreamInfo), FileAttributeMap.h

use std::ffi::OsString;

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
    pub file_name:      OsString,
    pub file_attributes: u32,
    pub file_size:      u64,
    pub creation_time:  u64,    // FILETIME as u64
    pub last_write_time: u64,   // FILETIME as u64
    pub last_access_time: u64,  // FILETIME as u64
    pub streams:        Vec<StreamInfo>,
}

impl FileInfo {
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
            streams:         Vec::new(),
        };
        assert!(fi.is_dot_dir());
    }
}
