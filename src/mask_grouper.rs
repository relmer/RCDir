// mask_grouper.rs — Group file masks by target directory
//
// Port of: MaskGrouper.h, MaskGrouper.cpp
//
// Groups command-line file masks by their directory component. Pure masks
// (no path separator) are combined under CWD. Directory-qualified masks
// are grouped by their normalized directory path (case-insensitive).

use std::ffi::OsString;
use std::path::{Path, PathBuf};





/// Trait for filesystem queries needed by mask grouping.
/// Enables unit testing without touching the real filesystem.
pub trait FileSystemQuery {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  is_dir
    //
    //  Returns true if the given path is an existing directory.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn is_dir(&self, path: &Path) -> bool;

    ////////////////////////////////////////////////////////////////////////////
    //
    //  canonicalize
    //
    //  Returns the canonical, absolute form of a path.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf>;
}





/// Default implementation that queries the real filesystem.
pub struct DefaultFileSystemQuery;





////////////////////////////////////////////////////////////////////////////////
//
//  impl FileSystemQuery for DefaultFileSystemQuery
//
//  Delegates to std::path::Path methods.
//
////////////////////////////////////////////////////////////////////////////////

impl FileSystemQuery for DefaultFileSystemQuery {
    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf> {
        path.canonicalize()
    }
}





/// A group of file specs for a single directory.
/// Port of: MaskGroup (pair<path, vector<path>>)
pub type MaskGroup = (PathBuf, Vec<OsString>);





////////////////////////////////////////////////////////////////////////////////
//
//  is_pure_mask
//
//  Check if a mask is "pure" — has no directory component.
//  Port of: CMaskGrouper::IsPureMask
//
////////////////////////////////////////////////////////////////////////////////

pub fn is_pure_mask(mask: &str) -> bool {
    // Check for path separators
    if mask.contains('\\') || mask.contains('/') {
        return false;
    }

    // Check for drive letter prefix (e.g., "C:file.txt")
    let bytes = mask.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return false;
    }

    true
}





////////////////////////////////////////////////////////////////////////////////
//
//  strip_extended_length_prefix
//
//  Remove the \\?\ extended-length path prefix that Windows
//  std::fs::canonicalize() adds.  Keeps paths human-readable.
//
////////////////////////////////////////////////////////////////////////////////

fn strip_extended_length_prefix(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some (stripped) = s.strip_prefix (r"\\?\") {
        PathBuf::from (stripped)
    } else {
        path
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  split_pure_mask
//
//  Handles a pure mask (no path separator).  If the mask has no wildcards
//  and matches an existing directory name under cwd, it is treated as a
//  directory to list.  Otherwise it is treated as a file pattern for cwd.
//  Port of: CMaskGrouper::SplitPureMask
//
////////////////////////////////////////////////////////////////////////////////

fn split_pure_mask(mask: &str, cwd: &Path, fs: &dyn FileSystemQuery) -> (PathBuf, OsString) {
    if !mask.contains ('*') && !mask.contains ('?') {
        let candidate = cwd.join (mask);
        if fs.is_dir (&candidate) {
            return (candidate, OsString::from ("*"));
        }
    }

    (cwd.to_path_buf(), OsString::from (mask))
}





////////////////////////////////////////////////////////////////////////////////
//
//  split_qualified_mask
//
//  Handles a directory-qualified mask (contains path separators or a drive
//  letter).  The mask is made absolute, then checked to see if it refers to
//  an existing directory.  If so, "*" is used as the file spec.  Otherwise
//  the mask is split into parent directory and filename.
//  Port of: CMaskGrouper::SplitQualifiedMask
//
////////////////////////////////////////////////////////////////////////////////

fn split_qualified_mask(mask: &str, cwd: &Path, fs: &dyn FileSystemQuery) -> (PathBuf, OsString) {
    let mask_path = PathBuf::from (mask);
    let absolute_path = if mask_path.is_absolute() {
        mask_path
    } else {
        // Make relative paths absolute against CWD
        let mut abs = cwd.to_path_buf();
        abs.push(&mask_path);

        // Normalize the path by canonicalizing where possible.
        // On Windows, canonicalize() returns paths with \\?\ prefix —
        // strip it so display paths are clean.
        match fs.canonicalize (&abs) {
            Ok(canonical) => strip_extended_length_prefix (canonical),
            Err(_) => abs,
        }
    };

    // Check if the mask is a directory (ends with separator or is existing dir)
    let is_dir = if !mask.is_empty() && (mask.ends_with('\\') || mask.ends_with('/')) {
        true
    } else {
        fs.is_dir (&absolute_path)
    };

    if is_dir {
        // Directory only — use "*" as filespec
        (absolute_path, OsString::from("*"))
    } else {
        // Has file component — split into parent dir and filename
        let dir_path = absolute_path.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let file_spec = absolute_path.file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("*"))
            .to_os_string();
        (dir_path, file_spec)
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  split_mask_into_dir_and_filespec
//
//  Split a mask into a directory path and file specification.
//  Dispatches to split_pure_mask or split_qualified_mask.
//  Port of: CMaskGrouper::SplitMaskIntoDirAndFileSpec
//
////////////////////////////////////////////////////////////////////////////////

fn split_mask_into_dir_and_filespec(mask: &str, cwd: &Path, fs: &dyn FileSystemQuery) -> (PathBuf, OsString) {
    if is_pure_mask (mask) {
        split_pure_mask (mask, cwd, fs)
    } else {
        split_qualified_mask (mask, cwd, fs)
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  add_mask_to_groups
//
//  Add a directory/filespec pair to the groups collection.
//  Port of: CMaskGrouper::AddMaskToGroups
//
////////////////////////////////////////////////////////////////////////////////

fn add_mask_to_groups(
    dir_path: PathBuf,
    file_spec: OsString,
    groups: &mut Vec<MaskGroup>,
    dir_to_index: &mut Vec<(String, usize)>,
) {
    // Normalize directory path for case-insensitive comparison
    let mut normalized = dir_path.to_string_lossy().to_ascii_lowercase();
    if !normalized.ends_with('\\') {
        normalized.push('\\');
    }

    // Find existing group (case-insensitive)
    for (existing_dir, idx) in dir_to_index.iter() {
        if *existing_dir == normalized {
            groups[*idx].1.push(file_spec);
            return;
        }
    }

    // Create new group
    let new_index = groups.len();
    dir_to_index.push((normalized, new_index));
    groups.push((dir_path, vec![file_spec]));
}





////////////////////////////////////////////////////////////////////////////////
//
//  group_masks_by_directory
//
//  Group command-line masks by their target directory.
//  Port of: CMaskGrouper::GroupMasksByDirectory
//
////////////////////////////////////////////////////////////////////////////////

pub fn group_masks_by_directory(masks: &[OsString]) -> Vec<MaskGroup> {
    group_masks_by_directory_with_fs (masks, &DefaultFileSystemQuery)
}





////////////////////////////////////////////////////////////////////////////////
//
//  group_masks_by_directory_with_fs
//
//  Internal entry point that accepts a FileSystemQuery for testability.
//
////////////////////////////////////////////////////////////////////////////////

fn group_masks_by_directory_with_fs(masks: &[OsString], fs: &dyn FileSystemQuery) -> Vec<MaskGroup> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let mut groups: Vec<MaskGroup> = Vec::new();
    let mut dir_to_index: Vec<(String, usize)> = Vec::new();

    if masks.is_empty() {
        // No masks — return CWD with "*"
        groups.push((cwd, vec![OsString::from("*")]));
    } else {
        for mask_os in masks {
            let mask = mask_os.to_string_lossy();
            let (dir_path, file_spec) = split_mask_into_dir_and_filespec(&mask, &cwd, fs);
            add_mask_to_groups(dir_path, file_spec, &mut groups, &mut dir_to_index);
        }
    }

    groups
}





#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;





    /// Mock implementation for unit tests.
    /// Returns preset is_dir results based on stored paths.
    struct MockFileSystemQuery {
        directories: HashSet<PathBuf>,
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  impl MockFileSystemQuery
    //
    //  Mock filesystem setup for unit tests.
    //
    ////////////////////////////////////////////////////////////////////////////

    impl MockFileSystemQuery {

        ////////////////////////////////////////////////////////////////////////
        //
        //  new
        //
        //  Creates a new empty MockFileSystemQuery.
        //
        ////////////////////////////////////////////////////////////////////////

        fn new() -> Self {
            MockFileSystemQuery {
                directories: HashSet::new(),
            }
        }

        ////////////////////////////////////////////////////////////////////////
        //
        //  with_dir
        //
        //  Registers a path as an existing directory.
        //
        ////////////////////////////////////////////////////////////////////////

        fn with_dir(mut self, path: &Path) -> Self {
            self.directories.insert (path.to_path_buf());
            self
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  impl FileSystemQuery for MockFileSystemQuery
    //
    //  Returns mock results based on registered directories.
    //
    ////////////////////////////////////////////////////////////////////////////

    impl FileSystemQuery for MockFileSystemQuery {
        fn is_dir(&self, path: &Path) -> bool {
            self.directories.contains (path)
        }

        fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf> {
            Ok (path.to_path_buf())
        }
    }

    ////////////////////////////////////////////////////////////////////////////
    //
    //  pure_mask_simple_wildcard
    //
    //  Verifies pure masks with simple wildcards.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn pure_mask_simple_wildcard() {
        assert!(is_pure_mask("*.rs"));
        assert!(is_pure_mask("*.toml"));
        assert!(is_pure_mask("hello.txt"));
        assert!(is_pure_mask("*"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  pure_mask_with_path_separator
    //
    //  Verifies masks with path separators are not pure.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn pure_mask_with_path_separator() {
        assert!(!is_pure_mask("foo\\*.rs"));
        assert!(!is_pure_mask("foo/bar.txt"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  pure_mask_with_drive_letter
    //
    //  Verifies masks with drive letter prefix are not pure.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn pure_mask_with_drive_letter() {
        assert!(!is_pure_mask("C:file.txt"));
        assert!(!is_pure_mask("D:*.rs"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  group_empty_masks_returns_cwd_star
    //
    //  Verifies empty mask list returns CWD with "*".
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn group_empty_masks_returns_cwd_star() {
        let groups = group_masks_by_directory(&[]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].1.len(), 1);
        assert_eq!(groups[0].1[0], OsString::from("*"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  group_pure_masks_same_dir
    //
    //  Verifies pure masks are grouped under the same directory.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn group_pure_masks_same_dir() {
        let masks = vec![OsString::from("*.rs"), OsString::from("*.toml")];
        let groups = group_masks_by_directory(&masks);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].1.len(), 2);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  group_single_mask
    //
    //  Verifies a single mask is grouped correctly.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn group_single_mask() {
        let masks = vec![OsString::from("*.txt")];
        let groups = group_masks_by_directory(&masks);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].1.len(), 1);
        assert_eq!(groups[0].1[0], OsString::from("*.txt"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  pure_mask_existing_directory_lists_contents
    //
    //  Verifies that a pure mask matching an existing directory is treated
    //  as a directory to list, not a file pattern.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn pure_mask_existing_directory_lists_contents() {
        let cwd = PathBuf::from (r"C:\Projects");
        let subdir = cwd.join ("subdir");
        let fs = MockFileSystemQuery::new()
            .with_dir (&subdir);

        let (dir, spec) = split_pure_mask ("subdir", &cwd, &fs);
        assert_eq! (dir, subdir);
        assert_eq! (spec, OsString::from ("*"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  pure_mask_nonexistent_treated_as_pattern
    //
    //  Verifies that a pure mask not matching a directory stays as a pattern.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn pure_mask_nonexistent_treated_as_pattern() {
        let cwd = PathBuf::from (r"C:\Projects");
        let fs = MockFileSystemQuery::new();

        let (dir, spec) = split_pure_mask ("nonexistent", &cwd, &fs);
        assert_eq! (dir, cwd);
        assert_eq! (spec, OsString::from ("nonexistent"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  wildcard_mask_never_treated_as_directory
    //
    //  Verifies that wildcard masks skip the directory check even when a
    //  matching directory exists.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn wildcard_mask_never_treated_as_directory() {
        let cwd = PathBuf::from (r"C:\Projects");
        let star_path = cwd.join ("*");
        let fs = MockFileSystemQuery::new()
            .with_dir (&star_path);

        let (dir, spec) = split_pure_mask ("*", &cwd, &fs);
        assert_eq! (dir, cwd);
        assert_eq! (spec, OsString::from ("*"));
    }
}
