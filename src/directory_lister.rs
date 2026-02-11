// directory_lister.rs — Single-threaded directory enumeration
//
// Port of: DirectoryLister.h, DirectoryLister.cpp → CDirectoryLister
//
// Core enumeration loop: FindFirstFileW/FindNextFileW, attribute filtering,
// match collection, size/count tracking, stream collection delegation.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::Arc;

use windows::Win32::Storage::FileSystem::{
    FindFirstFileW, FindNextFileW, WIN32_FIND_DATAW,
};

use crate::command_line::CommandLine;
use crate::config::Config;
use crate::directory_info::DirectoryInfo;
use crate::file_info::{FileInfo, FindHandle, FILE_ATTRIBUTE_DIRECTORY};
use crate::listing_totals::ListingTotals;
use crate::streams;





////////////////////////////////////////////////////////////////////////////////
//
//  collect_matching_files
//
//  Collect matching files and directories for a single directory + file spec.
//  Enumerates files matching dir_path/file_spec, applies attribute filters,
//  builds FileInfo entries, and populates the DirectoryInfo with matches and
//  counters.
//
//  Port of: CDirectoryLister::CollectMatchingFilesAndDirectories
//
////////////////////////////////////////////////////////////////////////////////

pub fn collect_matching_files(
    dir_path: &Path,
    file_spec: &OsStr,
    di: &mut DirectoryInfo,
    cmd: &CommandLine,
    totals: &mut ListingTotals,
    _config: &Arc<Config>,
) {
    // Build the search path: dir_path/file_spec
    let mut search_path = dir_path.to_path_buf();
    search_path.push(file_spec);

    // Convert to wide string for Win32 API
    let search_wide: Vec<u16> = search_path.as_os_str().encode_wide().chain(Some(0)).collect();

    let mut wfd = WIN32_FIND_DATAW::default();

    // FindFirstFileW
    let handle = unsafe { FindFirstFileW(windows::core::PCWSTR(search_wide.as_ptr()), &mut wfd) };

    let handle = match handle {
        Ok(h) if !h.is_invalid() => h,
        _ => return, // No matches found
    };

    let _find_handle = FindHandle(handle);

    loop {
        // Skip "." and ".." entries
        if !is_dots(&wfd.cFileName) {
            // Apply attribute filters: required attrs must all be present,
            // excluded attrs must all be absent
            let attrs = wfd.dwFileAttributes;
            let required_ok = (attrs & cmd.attrs_required) == cmd.attrs_required;
            let excluded_ok = (attrs & cmd.attrs_excluded) == 0;

            if required_ok && excluded_ok {
                add_match_to_list(&wfd, di, totals, cmd);
            }
        }

        // FindNextFileW
        let success = unsafe { FindNextFileW(handle, &mut wfd) };
        if success.is_err() {
            break;
        }
    }

    // Enumerate alternate data streams if --streams enabled
    if cmd.show_streams {
        streams::enumerate_streams(di, totals);
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  add_match_to_list
//
//  Add a matched file entry to the DirectoryInfo.  Tracks directory/file
//  counts, sizes, and widest filename.
//
//  Port of: CDirectoryLister::AddMatchToList
//
////////////////////////////////////////////////////////////////////////////////

fn add_match_to_list(
    wfd: &WIN32_FIND_DATAW,
    di: &mut DirectoryInfo,
    totals: &mut ListingTotals,
    cmd: &CommandLine,
) {
    let file_entry = FileInfo::from_find_data(wfd);

    // Track filename length for wide listing
    let file_name_len = if cmd.wide_listing {
        let name_len = wfd.cFileName.iter().position(|&c| c == 0).unwrap_or(0);
        if (wfd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0 {
            // Directories in wide listing get brackets: [dirname]
            name_len + 2
        } else {
            name_len
        }
    } else {
        0
    };

    if (wfd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0 {
        // Directory match
        di.subdirectory_count += 1;
    } else {
        // File match — track sizes
        let file_size = file_entry.file_size;

        if file_size > di.largest_file_size {
            di.largest_file_size = file_size;
        }

        di.bytes_used += file_size;
        di.file_count += 1;

        totals.file_bytes += file_size;
        totals.file_count += 1;
    }

    if cmd.wide_listing && file_name_len > di.largest_file_name {
        di.largest_file_name = file_name_len;
    }

    di.matches.push(file_entry);
}





////////////////////////////////////////////////////////////////////////////////
//
//  is_dots
//
//  Check if a filename is "." or "..".
//
//  Port of: CDirectoryLister::IsDots
//
////////////////////////////////////////////////////////////////////////////////

fn is_dots(filename: &[u16]) -> bool {
    if filename[0] == b'.' as u16 {
        if filename[1] == 0 {
            return true; // "."
        }
        if filename[1] == b'.' as u16 && filename[2] == 0 {
            return true; // ".."
        }
    }
    false
}





#[cfg(test)]
mod tests {
    use super::*;

    ////////////////////////////////////////////////////////////////////////////
    //
    //  is_dots_single_dot
    //
    //  Verify "." is detected as a dot entry.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn is_dots_single_dot() {
        let name = [b'.' as u16, 0, 0, 0];
        assert!(is_dots(&name));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  is_dots_double_dot
    //
    //  Verify ".." is detected as a dot entry.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn is_dots_double_dot() {
        let name = [b'.' as u16, b'.' as u16, 0, 0];
        assert!(is_dots(&name));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  is_dots_regular_name
    //
    //  Verify a regular filename is not detected as a dot entry.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn is_dots_regular_name() {
        let name = [b'f' as u16, b'o' as u16, b'o' as u16, 0];
        assert!(!is_dots(&name));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  is_dots_dotfile
    //
    //  Verify a dotfile (e.g. ".git") is not detected as a dot entry.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn is_dots_dotfile() {
        let name = [b'.' as u16, b'g' as u16, b'i' as u16, b't' as u16, 0];
        assert!(!is_dots(&name));
    }
}
