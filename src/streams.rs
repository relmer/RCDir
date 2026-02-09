// streams.rs — Alternate data stream enumeration
//
// Port of: DirectoryLister.cpp → HandleFileMatchStreams()
//
// Uses FindFirstStreamW/FindNextStreamW to enumerate alternate data streams.
// Populates FileInfo.streams with StreamInfo entries (name stripped of :$DATA suffix).

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows::Win32::Storage::FileSystem::{
    FindFirstStreamW, FindNextStreamW, FindStreamInfoStandard,
    WIN32_FIND_STREAM_DATA,
};

use crate::directory_info::DirectoryInfo;
use crate::file_info::{FindHandle, StreamInfo};
use crate::listing_totals::ListingTotals;

/// Enumerate alternate data streams for all non-directory files in a DirectoryInfo.
///
/// For each file that has streams (beyond the default ::$DATA), populates
/// `file_info.streams` with StreamInfo entries, updates `di.stream_count`,
/// `di.stream_bytes_used`, and `di.largest_file_size`. Also updates the
/// global `totals` for recursive summaries.
///
/// Port of: CDirectoryLister::HandleFileMatchStreams
pub fn enumerate_streams(di: &mut DirectoryInfo, totals: &mut ListingTotals) {
    use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY;

    for file_info in &mut di.matches {
        // Only enumerate streams for files, not directories
        if (file_info.file_attributes & FILE_ATTRIBUTE_DIRECTORY.0) != 0 {
            continue;
        }

        let full_path = di.dir_path.join(&file_info.file_name);
        let streams = enumerate_file_streams(full_path.as_os_str());

        for si in &streams {
            // Update largest file size if a stream is bigger
            if si.size as u64 > di.largest_file_size {
                di.largest_file_size = si.size as u64;
            }

            di.stream_count += 1;
            di.stream_bytes_used += si.size as u64;

            totals.stream_count += 1;
            totals.stream_bytes += si.size as u64;
        }

        file_info.streams = streams;
    }
}

/// Enumerate alternate data streams for a single file.
///
/// Returns a Vec of StreamInfo for each non-default stream found.
/// The default unnamed data stream (::$DATA) is skipped.
/// Stream names have the ":$DATA" suffix stripped.
///
/// Returns an empty Vec if the file has no alternate streams or if
/// the call fails (e.g., non-NTFS volume).
pub fn enumerate_file_streams(file_path: &OsStr) -> Vec<StreamInfo> {
    let path_wide: Vec<u16> = file_path.encode_wide().chain(Some(0)).collect();
    let mut stream_data = WIN32_FIND_STREAM_DATA::default();
    let mut results = Vec::new();

    // FindFirstStreamW — returns a find handle or error
    let handle = unsafe {
        FindFirstStreamW(
            windows::core::PCWSTR(path_wide.as_ptr()),
            FindStreamInfoStandard,
            &mut stream_data as *mut _ as *mut _,
            None,
        )
    };

    let handle = match handle {
        Ok(h) => FindHandle(h),
        Err(_) => return results,
    };

    loop {
        // Process current stream
        if let Some(si) = process_stream_data(&stream_data) {
            results.push(si);
        }

        // Try to get next stream
        let next = unsafe {
            FindNextStreamW(
                handle.0,
                &mut stream_data as *mut _ as *mut _,
            )
        };

        if next.is_err() {
            break;
        }
    }

    results
}

/// Process a WIN32_FIND_STREAM_DATA entry into a StreamInfo.
///
/// Skips the default unnamed data stream (::$DATA).
/// Strips the ":$DATA" suffix from the stream name.
fn process_stream_data(data: &WIN32_FIND_STREAM_DATA) -> Option<StreamInfo> {
    // Convert stream name from wide string
    let name_len = data.cStreamName.iter().position(|&c| c == 0).unwrap_or(data.cStreamName.len());
    let name = String::from_utf16_lossy(&data.cStreamName[..name_len]);

    // Skip the default unnamed data stream (::$DATA)
    if name.eq_ignore_ascii_case("::$DATA") {
        return None;
    }

    // Strip ":$DATA" suffix if present
    let stripped = if name.len() > 6 && name[name.len() - 6..].eq_ignore_ascii_case(":$DATA") {
        name[..name.len() - 6].to_string()
    } else {
        name
    };

    Some(StreamInfo {
        name: stripped,
        size: data.StreamSize,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn process_stream_data_skips_default() {
        let mut data = WIN32_FIND_STREAM_DATA::default();
        let name = "::$DATA";
        for (i, ch) in name.encode_utf16().enumerate() {
            data.cStreamName[i] = ch;
        }
        data.StreamSize = 1024;
        assert!(process_stream_data(&data).is_none());
    }

    #[test]
    fn process_stream_data_strips_suffix() {
        let mut data = WIN32_FIND_STREAM_DATA::default();
        let name = ":hidden:$DATA";
        for (i, ch) in name.encode_utf16().enumerate() {
            data.cStreamName[i] = ch;
        }
        data.StreamSize = 512;
        let si = process_stream_data(&data).unwrap();
        assert_eq!(si.name, ":hidden");
        assert_eq!(si.size, 512);
    }

    #[test]
    fn process_stream_data_no_suffix() {
        let mut data = WIN32_FIND_STREAM_DATA::default();
        let name = ":metadata";
        for (i, ch) in name.encode_utf16().enumerate() {
            data.cStreamName[i] = ch;
        }
        data.StreamSize = 256;
        let si = process_stream_data(&data).unwrap();
        assert_eq!(si.name, ":metadata");
        assert_eq!(si.size, 256);
    }

    #[test]
    fn enumerate_nonexistent_file_returns_empty() {
        let path = OsString::from("C:\\__definitely_nonexistent_file_12345__.txt");
        let streams = enumerate_file_streams(&path);
        assert!(streams.is_empty());
    }

    #[test]
    fn enumerate_current_exe_no_panic() {
        // Should not panic — may or may not have streams
        let exe = std::env::current_exe().unwrap();
        let streams = enumerate_file_streams(exe.as_os_str());
        // Just verify it doesn't panic; streams may be empty
        let _ = streams.len();
    }
}
