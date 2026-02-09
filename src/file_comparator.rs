// file_comparator.rs — File sorting comparisons
//
// Port of: FileComparator.h, FileComparator.cpp
//
// Sorts FileInfo entries using a tiebreaker chain. Directories always sort
// before files. Only the primary sort attribute respects reverse direction;
// tiebreakers always use ascending order.

use std::cmp::Ordering;

use crate::command_line::{CommandLine, SortOrder, SortDirection, TimeField};
use crate::file_info::{FileInfo, FILE_ATTRIBUTE_DIRECTORY};

/// Sort a slice of FileInfo entries according to the CommandLine sort preferences.
///
/// Port of: std::sort with FileComparator
pub fn sort_files(matches: &mut [FileInfo], cmd: &CommandLine) {
    if cmd.sort_order == SortOrder::Default && cmd.sort_direction == SortDirection::Ascending {
        // Default sort = name ascending (matches TCDir behavior)
        matches.sort_by(|a, b| compare_entries(a, b, cmd));
    } else {
        matches.sort_by(|a, b| compare_entries(a, b, cmd));
    }
}

/// Compare two FileInfo entries for sorting.
/// Directories always sort before files.
/// Then walks the sort_preference tiebreaker chain.
/// Only respects reverse direction for the primary sort attribute.
///
/// Port of: FileComparator::operator()
fn compare_entries(lhs: &FileInfo, rhs: &FileInfo, cmd: &CommandLine) -> Ordering {
    let lhs_is_dir = (lhs.file_attributes & FILE_ATTRIBUTE_DIRECTORY) != 0;
    let rhs_is_dir = (rhs.file_attributes & FILE_ATTRIBUTE_DIRECTORY) != 0;

    // Directories always sort before files
    if lhs_is_dir != rhs_is_dir {
        return if lhs_is_dir { Ordering::Less } else { Ordering::Greater };
    }

    // Walk the sort_preference chain
    for (idx, sort_attr) in cmd.sort_preference.iter().enumerate() {
        let cmp = match sort_attr {
            SortOrder::Default | SortOrder::Name => compare_name(lhs, rhs),
            SortOrder::Date      => compare_date(lhs, rhs, cmd.time_field),
            SortOrder::Extension => compare_extension(lhs, rhs),
            SortOrder::Size      => compare_size(lhs, rhs),
        };

        if cmp == Ordering::Equal {
            continue;
        }

        // Only reverse the primary sort attribute (idx == 0), not tiebreakers
        if idx == 0 && cmd.sort_direction == SortDirection::Descending {
            return cmp.reverse();
        }

        return cmp;
    }

    Ordering::Equal
}

/// Compare by name using case-insensitive comparison.
///
/// Port of: FileComparator::CompareName (uses lstrcmpiW)
fn compare_name(lhs: &FileInfo, rhs: &FileInfo) -> Ordering {
    // Use locale-aware comparison via Win32 lstrcmpiW for exact TCDir parity
    use std::os::windows::ffi::OsStrExt;

    let lhs_wide: Vec<u16> = lhs.file_name.encode_wide().chain(Some(0)).collect();
    let rhs_wide: Vec<u16> = rhs.file_name.encode_wide().chain(Some(0)).collect();

    let result = unsafe {
        windows::Win32::Globalization::lstrcmpiW(
            windows::core::PCWSTR(lhs_wide.as_ptr()),
            windows::core::PCWSTR(rhs_wide.as_ptr()),
        )
    };

    result.cmp(&0)
}

/// Compare by date based on the selected time field.
///
/// Port of: FileComparator::CompareDate
fn compare_date(lhs: &FileInfo, rhs: &FileInfo, time_field: TimeField) -> Ordering {
    let (lhs_time, rhs_time) = match time_field {
        TimeField::Creation => (lhs.creation_time, rhs.creation_time),
        TimeField::Access   => (lhs.last_access_time, rhs.last_access_time),
        TimeField::Written  => (lhs.last_write_time, rhs.last_write_time),
    };

    lhs_time.cmp(&rhs_time)
}

/// Compare by file extension (case-insensitive).
///
/// Port of: FileComparator::CompareExtension
fn compare_extension(lhs: &FileInfo, rhs: &FileInfo) -> Ordering {
    let lhs_name = lhs.file_name.to_string_lossy();
    let rhs_name = rhs.file_name.to_string_lossy();

    let lhs_ext = lhs_name.rfind('.').map(|i| &lhs_name[i..]).unwrap_or("");
    let rhs_ext = rhs_name.rfind('.').map(|i| &rhs_name[i..]).unwrap_or("");

    // Use locale-aware comparison for extension
    use std::os::windows::ffi::OsStrExt;

    let lhs_wide: Vec<u16> = std::ffi::OsStr::new(lhs_ext).encode_wide().chain(Some(0)).collect();
    let rhs_wide: Vec<u16> = std::ffi::OsStr::new(rhs_ext).encode_wide().chain(Some(0)).collect();

    let result = unsafe {
        windows::Win32::Globalization::lstrcmpiW(
            windows::core::PCWSTR(lhs_wide.as_ptr()),
            windows::core::PCWSTR(rhs_wide.as_ptr()),
        )
    };

    result.cmp(&0)
}

/// Compare by file size.
///
/// Port of: FileComparator::CompareSize
fn compare_size(lhs: &FileInfo, rhs: &FileInfo) -> Ordering {
    lhs.file_size.cmp(&rhs.file_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn make_file(name: &str, attrs: u32, size: u64) -> FileInfo {
        FileInfo {
            file_name:       OsString::from(name),
            file_attributes: attrs,
            file_size:       size,
            creation_time:   0,
            last_write_time: 0,
            last_access_time: 0,
            reparse_tag:     0,
            streams:         Vec::new(),
        }
    }

    #[test]
    fn directories_before_files() {
        let cmd = CommandLine::default();
        let mut files = vec![
            make_file("b.txt", 0x20, 100),
            make_file("adir",  FILE_ATTRIBUTE_DIRECTORY, 0),
            make_file("a.txt", 0x20, 200),
        ];
        sort_files(&mut files, &cmd);

        assert!(files[0].is_directory());
        assert!(!files[1].is_directory());
        assert!(!files[2].is_directory());
    }

    #[test]
    fn sort_by_name_default() {
        let cmd = CommandLine::default();
        let mut files = vec![
            make_file("charlie.txt", 0x20, 100),
            make_file("alpha.txt",   0x20, 200),
            make_file("bravo.txt",   0x20, 150),
        ];
        sort_files(&mut files, &cmd);

        assert_eq!(files[0].file_name, "alpha.txt");
        assert_eq!(files[1].file_name, "bravo.txt");
        assert_eq!(files[2].file_name, "charlie.txt");
    }

    #[test]
    fn sort_by_size() {
        let mut cmd = CommandLine::default();
        cmd.sort_order = SortOrder::Size;
        cmd.sort_preference[0] = SortOrder::Size;
        let mut files = vec![
            make_file("big.txt",    0x20, 3000),
            make_file("small.txt",  0x20, 100),
            make_file("medium.txt", 0x20, 1500),
        ];
        sort_files(&mut files, &cmd);

        assert_eq!(files[0].file_name, "small.txt");
        assert_eq!(files[1].file_name, "medium.txt");
        assert_eq!(files[2].file_name, "big.txt");
    }

    #[test]
    fn sort_by_size_descending() {
        let mut cmd = CommandLine::default();
        cmd.sort_order = SortOrder::Size;
        cmd.sort_direction = SortDirection::Descending;
        cmd.sort_preference[0] = SortOrder::Size;
        let mut files = vec![
            make_file("big.txt",    0x20, 3000),
            make_file("small.txt",  0x20, 100),
            make_file("medium.txt", 0x20, 1500),
        ];
        sort_files(&mut files, &cmd);

        assert_eq!(files[0].file_name, "big.txt");
        assert_eq!(files[1].file_name, "medium.txt");
        assert_eq!(files[2].file_name, "small.txt");
    }
}
