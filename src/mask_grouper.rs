// mask_grouper.rs — Group file masks by target directory
//
// Port of: MaskGrouper.h, MaskGrouper.cpp
//
// Groups command-line file masks by their directory component. Pure masks
// (no path separator) are combined under CWD. Directory-qualified masks
// are grouped by their normalized directory path (case-insensitive).

use std::ffi::OsString;
use std::path::PathBuf;





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
//  split_mask_into_dir_and_filespec
//
//  Split a mask into a directory path and file specification.
//  Port of: CMaskGrouper::SplitMaskIntoDirAndFileSpec
//
////////////////////////////////////////////////////////////////////////////////

fn split_mask_into_dir_and_filespec(mask: &str, cwd: &std::path::Path) -> (PathBuf, OsString) {
    if is_pure_mask(mask) {
        // Pure mask — use CWD
        return (cwd.to_path_buf(), OsString::from(mask));
    }

    // Directory-qualified mask — make absolute
    let mask_path = PathBuf::from(mask);
    let absolute_path = if mask_path.is_absolute() {
        mask_path
    } else {
        // Make relative paths absolute against CWD
        let mut abs = cwd.to_path_buf();
        abs.push(&mask_path);

        // Normalize the path by canonicalizing where possible
        match abs.canonicalize() {
            Ok(canonical) => canonical,
            Err(_) => abs,
        }
    };

    // Check if the mask is a directory (ends with separator or is existing dir)
    let is_dir = if !mask.is_empty() && (mask.ends_with('\\') || mask.ends_with('/')) {
        true
    } else {
        absolute_path.is_dir()
    };

    if is_dir {
        // Directory only — use "*" as filespec
        (absolute_path, OsString::from("*"))
    } else {
        // Has file component — split into parent dir and filename
        let dir_path = absolute_path.parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();
        let file_spec = absolute_path.file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("*"))
            .to_os_string();
        (dir_path, file_spec)
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
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let mut groups: Vec<MaskGroup> = Vec::new();
    let mut dir_to_index: Vec<(String, usize)> = Vec::new();

    if masks.is_empty() {
        // No masks — return CWD with "*"
        groups.push((cwd, vec![OsString::from("*")]));
    } else {
        for mask_os in masks {
            let mask = mask_os.to_string_lossy();
            let (dir_path, file_spec) = split_mask_into_dir_and_filespec(&mask, &cwd);
            add_mask_to_groups(dir_path, file_spec, &mut groups, &mut dir_to_index);
        }
    }

    groups
}





#[cfg(test)]
mod tests {
    use super::*;

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
}
