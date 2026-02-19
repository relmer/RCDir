// file_comparator.rs — File sorting comparisons
//
// Port of: FileComparator.h, FileComparator.cpp
//
// Sorts FileInfo entries using a tiebreaker chain. Directories always sort
// before files. Only the primary sort attribute respects reverse direction;
// tiebreakers always use ascending order.
//
// Performance: sort keys (wide strings for name/extension) are pre-computed
// once per file before sorting, avoiding O(n log n) repeated allocations.

use std::cmp::Ordering;
use std::os::windows::ffi::OsStrExt;

use crate::command_line::{CommandLine, SortOrder, SortDirection, TimeField};
use crate::file_info::{FileInfo, FILE_ATTRIBUTE_DIRECTORY};





////////////////////////////////////////////////////////////////////////////////
//
//  Pre-computed sort key — avoids per-comparison allocations.
//
////////////////////////////////////////////////////////////////////////////////

struct SortKey {
    name_wide:        Vec<u16>,
    ext_wide:         Vec<u16>,
    is_dir:           bool,
    file_size:        u64,
    creation_time:    u64,
    last_write_time:  u64,
    last_access_time: u64,
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl SortKey
//
//  Sort key construction from FileInfo.
//
////////////////////////////////////////////////////////////////////////////////

impl SortKey {
    ////////////////////////////////////////////////////////////////////////////
    //
    //  from_file_info
    //
    //  Pre-compute sort key fields from a FileInfo entry.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn from_file_info(f: &FileInfo) -> Self {
        let name_wide: Vec<u16> = f.file_name.encode_wide().chain (Some (0)).collect();

        // Extract extension for pre-computation
        let name_str = f.file_name.to_string_lossy();
        let ext_str  = name_str.rfind ('.').map (|i| &name_str[i..]).unwrap_or ("");
        let ext_wide: Vec<u16> = std::ffi::OsStr::new (ext_str).encode_wide().chain (Some (0)).collect();

        SortKey {
            name_wide,
            ext_wide,
            is_dir:           (f.file_attributes & FILE_ATTRIBUTE_DIRECTORY) != 0,
            file_size:        f.file_size,
            creation_time:    f.creation_time,
            last_write_time:  f.last_write_time,
            last_access_time: f.last_access_time,
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  sort_files
//
//  Sort a slice of FileInfo entries according to the CommandLine sort
//  preferences.  Pre-computes sort keys to avoid per-comparison allocations.
//
//  Port of: std::sort with FileComparator.
//
////////////////////////////////////////////////////////////////////////////////

pub fn sort_files(matches: &mut [FileInfo], cmd: &CommandLine) {
    if matches.len() <= 1 {
        return;
    }

    // Pre-compute sort keys (name/extension wide strings, sizes, times)
    let keys: Vec<SortKey> = matches.iter()
        .map (SortKey::from_file_info)
        .collect();

    // Sort indices using pre-computed keys
    let mut indices: Vec<usize> = (0..matches.len()).collect();
    indices.sort_by (|&a, &b| compare_keyed (&keys[a], &keys[b], cmd));

    // Apply the permutation in-place
    apply_permutation (matches, indices);
}





////////////////////////////////////////////////////////////////////////////////
//
//  apply_permutation
//
//  Apply an index permutation to a mutable slice in-place.
//  perm[i] = the original index of the element that should end up at
//  position i.  Computes the inverse permutation, then uses the
//  cycle-following algorithm: O(n) time, O(n) extra space for the inverse.
//
////////////////////////////////////////////////////////////////////////////////

fn apply_permutation<T>(slice: &mut [T], perm: Vec<usize>) {
    let n = slice.len();

    // Compute inverse permutation: inv[perm[i]] = i
    // This maps "element currently at position j goes to position inv[j]"
    let mut inv = vec![0usize; n];
    for i in 0..n {
        inv[perm[i]] = i;
    }

    // Cycle-following on the inverse permutation
    for i in 0..n {
        while inv[i] != i {
            let j = inv[i];
            slice.swap (i, j);
            inv.swap (i, j);
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  compare_keyed
//
//  Compare two pre-computed SortKeys.  Directories always sort before
//  files, then walks the sort_preference tiebreaker chain.
//
//  Port of: FileComparator::operator()
//
////////////////////////////////////////////////////////////////////////////////

fn compare_keyed(lhs: &SortKey, rhs: &SortKey, cmd: &CommandLine) -> Ordering {
    // Directories always sort before files
    if lhs.is_dir != rhs.is_dir {
        return if lhs.is_dir { Ordering::Less } else { Ordering::Greater };
    }

    // Walk the sort_preference chain
    for (idx, sort_attr) in cmd.sort_preference.iter().enumerate() {
        let cmp = match sort_attr {
            SortOrder::Default | SortOrder::Name => compare_name_wide (&lhs.name_wide, &rhs.name_wide),
            SortOrder::Date      => compare_date_keyed (lhs, rhs, cmd.time_field),
            SortOrder::Extension => compare_name_wide (&lhs.ext_wide, &rhs.ext_wide),
            SortOrder::Size      => lhs.file_size.cmp (&rhs.file_size),
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





////////////////////////////////////////////////////////////////////////////////
//
//  compare_name_wide
//
//  Locale-aware case-insensitive comparison of pre-computed wide strings.
//  Port of: FileComparator::CompareName (uses lstrcmpiW)
//
////////////////////////////////////////////////////////////////////////////////

fn compare_name_wide(lhs_wide: &[u16], rhs_wide: &[u16]) -> Ordering {
    let result = unsafe {
        windows::Win32::Globalization::lstrcmpiW (
            windows::core::PCWSTR (lhs_wide.as_ptr()),
            windows::core::PCWSTR (rhs_wide.as_ptr()),
        )
    };

    result.cmp (&0)
}





////////////////////////////////////////////////////////////////////////////////
//
//  compare_date_keyed
//
//  Compare by date based on the selected time field.
//  Port of: FileComparator::CompareDate
//
////////////////////////////////////////////////////////////////////////////////

fn compare_date_keyed(lhs: &SortKey, rhs: &SortKey, time_field: TimeField) -> Ordering {
    let (lhs_time, rhs_time) = match time_field {
        TimeField::Creation => (lhs.creation_time, rhs.creation_time),
        TimeField::Access   => (lhs.last_access_time, rhs.last_access_time),
        TimeField::Written  => (lhs.last_write_time, rhs.last_write_time),
    };

    lhs_time.cmp (&rhs_time)
}





#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    ////////////////////////////////////////////////////////////////////////////
    //
    //  make_file
    //
    //  Creates a FileInfo with the given name, attributes, and size.
    //
    ////////////////////////////////////////////////////////////////////////////

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





    ////////////////////////////////////////////////////////////////////////////
    //
    //  directories_before_files
    //
    //  Verifies that directories sort before files.
    //
    ////////////////////////////////////////////////////////////////////////////

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





    ////////////////////////////////////////////////////////////////////////////
    //
    //  sort_by_name_default
    //
    //  Verifies default sort order is ascending by name.
    //
    ////////////////////////////////////////////////////////////////////////////

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





    ////////////////////////////////////////////////////////////////////////////
    //
    //  sort_by_size
    //
    //  Verifies ascending sort by size.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    #[allow(clippy::field_reassign_with_default)]
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





    ////////////////////////////////////////////////////////////////////////////
    //
    //  sort_by_size_descending
    //
    //  Verifies descending sort by size.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    #[allow(clippy::field_reassign_with_default)]
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





    ////////////////////////////////////////////////////////////////////////////
    //
    //  permutation_empty
    //
    //  Verify apply_permutation handles an empty slice.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn permutation_empty() {
        let mut data: Vec<i32> = vec![];
        apply_permutation (&mut data, vec![]);
        assert!(data.is_empty());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  permutation_single
    //
    //  Verify apply_permutation handles a single-element slice.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn permutation_single() {
        let mut data = vec![42];
        apply_permutation (&mut data, vec![0]);
        assert_eq!(data, [42]);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  permutation_identity
    //
    //  Verify apply_permutation is a no-op for an identity permutation.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn permutation_identity() {
        let mut data = vec![10, 20, 30, 40];
        apply_permutation (&mut data, vec![0, 1, 2, 3]);
        assert_eq!(data, [10, 20, 30, 40]);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  permutation_reverse
    //
    //  Verify apply_permutation reverses elements with a reverse permutation.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn permutation_reverse() {
        let mut data = vec![10, 20, 30, 40];
        // perm[i] = original index that goes to position i
        // Position 0 gets element from index 3, position 1 from 2, etc.
        apply_permutation (&mut data, vec![3, 2, 1, 0]);
        assert_eq!(data, [40, 30, 20, 10]);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  permutation_cycle
    //
    //  Verify apply_permutation handles a non-trivial cycle: (0→1→2→0).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn permutation_cycle() {
        let mut data = vec!['a', 'b', 'c', 'd'];
        // Rotate first 3: position 0 gets from 1, 1 from 2, 2 from 0, 3 stays
        apply_permutation (&mut data, vec![1, 2, 0, 3]);
        assert_eq!(data, ['b', 'c', 'a', 'd']);
    }
}
