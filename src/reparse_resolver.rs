// reparse_resolver.rs — Symlink, junction, and AppExecLink target resolution
//
// Reads reparse data via Win32 DeviceIoControl(FSCTL_GET_REPARSE_POINT)
// and parses the three supported buffer formats.  Pure parsing functions
// are separated from I/O for testability.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;

use crate::file_info::FileInfo;





////////////////////////////////////////////////////////////////////////////////
//
//  Constants
//
////////////////////////////////////////////////////////////////////////////////

/// Junction / mount point reparse tag.
pub const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;

/// Symbolic link reparse tag (file or directory).
pub const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000_000C;

/// Windows Store app execution alias reparse tag.
pub const IO_REPARSE_TAG_APPEXECLINK: u32 = 0x8000_001B;

/// Flag in the symlink reparse buffer indicating a relative symlink.
const SYMLINK_FLAG_RELATIVE: u32 = 0x0000_0001;

/// Maximum reparse data buffer size (16 KB, per NTFS specification).
const MAXIMUM_REPARSE_DATA_BUFFER_SIZE: usize = 16_384;

/// File attribute flag for reparse points.
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;

/// FSCTL code to read reparse point data.
const FSCTL_GET_REPARSE_POINT: u32 = 0x000900A8;

/// Device prefix that junctions store internally.
const DEVICE_PREFIX: &str = "\\??\\";

/// Size of the reparse data buffer header (tag + data_length + reserved).
const REPARSE_HEADER_SIZE: usize = 8;

/// Size of the mount-point sub-header (4 u16 fields = 8 bytes).
const MOUNT_POINT_HEADER_SIZE: usize = 8;

/// Size of the symlink sub-header (4 u16 fields + 1 u32 flags = 12 bytes).
const SYMLINK_HEADER_SIZE: usize = 12;





////////////////////////////////////////////////////////////////////////////////
//
//  strip_device_prefix
//
//  Remove the \??\ NT device prefix from a path string.
//  Returns the input unchanged if the prefix is not present.
//
////////////////////////////////////////////////////////////////////////////////

pub fn strip_device_prefix (path: &str) -> String {
    if let Some (stripped) = path.strip_prefix (DEVICE_PREFIX) {
        stripped.to_string()
    } else {
        path.to_string()
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parse_junction_buffer
//
//  Parse an IO_REPARSE_TAG_MOUNT_POINT reparse data buffer.
//  Prefers PrintName; falls back to SubstituteName with \??\ stripping.
//  Returns empty string on any parse failure.
//
////////////////////////////////////////////////////////////////////////////////

pub fn parse_junction_buffer (buffer: &[u8]) -> String {
    // Validate minimum header size
    if buffer.len() < REPARSE_HEADER_SIZE + MOUNT_POINT_HEADER_SIZE {
        return String::new();
    }

    // Verify reparse tag
    let tag = u32::from_le_bytes ([buffer[0], buffer[1], buffer[2], buffer[3]]);
    if tag != IO_REPARSE_TAG_MOUNT_POINT {
        return String::new();
    }

    // Parse mount-point sub-header (starts at offset 8, after the main header)
    let hdr = &buffer[REPARSE_HEADER_SIZE..];
    let substitute_name_offset = u16::from_le_bytes ([hdr[0], hdr[1]]) as usize;
    let substitute_name_length = u16::from_le_bytes ([hdr[2], hdr[3]]) as usize;
    let print_name_offset      = u16::from_le_bytes ([hdr[4], hdr[5]]) as usize;
    let print_name_length      = u16::from_le_bytes ([hdr[6], hdr[7]]) as usize;

    // PathBuffer starts after header + sub-header
    let path_buffer_offset = REPARSE_HEADER_SIZE + MOUNT_POINT_HEADER_SIZE;

    // Prefer PrintName (user-friendly, no device prefix)
    if print_name_length > 0
        && let Some (name) = extract_utf16_string (buffer, path_buffer_offset + print_name_offset, print_name_length)
    {
        return name;
    }

    // Fall back to SubstituteName with prefix stripping
    if substitute_name_length > 0
        && let Some (name) = extract_utf16_string (buffer, path_buffer_offset + substitute_name_offset, substitute_name_length)
    {
        return strip_device_prefix (&name);
    }

    String::new()
}





////////////////////////////////////////////////////////////////////////////////
//
//  parse_symlink_buffer
//
//  Parse an IO_REPARSE_TAG_SYMLINK reparse data buffer.
//  Prefers PrintName; falls back to SubstituteName.
//  Strips \??\ only for absolute symlinks (not SYMLINK_FLAG_RELATIVE).
//  Returns empty string on any parse failure.
//
////////////////////////////////////////////////////////////////////////////////

pub fn parse_symlink_buffer (buffer: &[u8]) -> String {
    // Validate minimum header size
    if buffer.len() < REPARSE_HEADER_SIZE + SYMLINK_HEADER_SIZE {
        return String::new();
    }

    // Verify reparse tag
    let tag = u32::from_le_bytes ([buffer[0], buffer[1], buffer[2], buffer[3]]);
    if tag != IO_REPARSE_TAG_SYMLINK {
        return String::new();
    }

    // Parse symlink sub-header (starts at offset 8)
    let hdr = &buffer[REPARSE_HEADER_SIZE..];
    let substitute_name_offset = u16::from_le_bytes ([hdr[0], hdr[1]]) as usize;
    let substitute_name_length = u16::from_le_bytes ([hdr[2], hdr[3]]) as usize;
    let print_name_offset      = u16::from_le_bytes ([hdr[4], hdr[5]]) as usize;
    let print_name_length      = u16::from_le_bytes ([hdr[6], hdr[7]]) as usize;
    let flags                  = u32::from_le_bytes ([hdr[8], hdr[9], hdr[10], hdr[11]]);

    // PathBuffer starts after header + symlink sub-header
    let path_buffer_offset = REPARSE_HEADER_SIZE + SYMLINK_HEADER_SIZE;

    // Prefer PrintName
    if print_name_length > 0
        && let Some (name) = extract_utf16_string (buffer, path_buffer_offset + print_name_offset, print_name_length)
    {
        return name;
    }

    // Fall back to SubstituteName
    if substitute_name_length > 0
        && let Some (name) = extract_utf16_string (buffer, path_buffer_offset + substitute_name_offset, substitute_name_length)
    {
        // Strip device prefix only for absolute symlinks
        if (flags & SYMLINK_FLAG_RELATIVE) == 0 {
            return strip_device_prefix (&name);
        }
        return name;
    }

    String::new()
}





////////////////////////////////////////////////////////////////////////////////
//
//  parse_app_exec_link_buffer
//
//  Parse an IO_REPARSE_TAG_APPEXECLINK reparse data buffer.
//  The buffer contains a version u32 (must be 3) followed by three
//  NUL-terminated UTF-16 strings.  The third string is the target
//  executable path.
//  Returns empty string on any parse failure.
//
////////////////////////////////////////////////////////////////////////////////

pub fn parse_app_exec_link_buffer (buffer: &[u8]) -> String {
    // Validate minimum header size
    if buffer.len() < REPARSE_HEADER_SIZE {
        return String::new();
    }

    // Verify reparse tag
    let tag = u32::from_le_bytes ([buffer[0], buffer[1], buffer[2], buffer[3]]);
    if tag != IO_REPARSE_TAG_APPEXECLINK {
        return String::new();
    }

    // The generic data starts right after the 8-byte header
    let data = &buffer[REPARSE_HEADER_SIZE..];

    // Need at least 4 bytes for the version field
    if data.len() < 4 {
        return String::new();
    }

    let version = u32::from_le_bytes ([data[0], data[1], data[2], data[3]]);
    if version != 3 {
        return String::new();
    }

    // Walk three NUL-terminated UTF-16 strings after the version field
    let mut offset = 4; // skip version u32
    let mut string_index = 0;

    while string_index < 3 && offset + 1 < data.len() {
        // Find NUL terminator (u16 == 0)
        let start = offset;
        while offset + 1 < data.len() {
            let ch = u16::from_le_bytes ([data[offset], data[offset + 1]]);
            offset += 2;
            if ch == 0 {
                break;
            }
        }

        if string_index == 2 {
            // Third string = target exe path
            let str_data = &data[start..offset.saturating_sub (2)]; // exclude NUL
            if str_data.len() >= 2 {
                let u16_chars: Vec<u16> = str_data
                    .chunks_exact (2)
                    .map (|pair| u16::from_le_bytes ([pair[0], pair[1]]))
                    .collect();
                return String::from_utf16_lossy (&u16_chars);
            }
            return String::new();
        }

        string_index += 1;
    }

    String::new()
}





////////////////////////////////////////////////////////////////////////////////
//
//  resolve_reparse_target
//
//  Read the reparse data for a file and resolve its target path.
//  Returns empty string if:
//    - The file is not a reparse point
//    - The reparse tag is not supported (junction, symlink, AppExecLink)
//    - The file cannot be opened or the IOCTL fails
//    - The buffer cannot be parsed
//
////////////////////////////////////////////////////////////////////////////////

pub fn resolve_reparse_target (dir_path: &Path, file_info: &FileInfo) -> String {
    // Early exit: not a reparse point
    if (file_info.file_attributes & FILE_ATTRIBUTE_REPARSE_POINT) == 0 {
        return String::new();
    }

    // Early exit: unsupported reparse tag
    let tag = file_info.reparse_tag;
    if tag != IO_REPARSE_TAG_MOUNT_POINT
        && tag != IO_REPARSE_TAG_SYMLINK
        && tag != IO_REPARSE_TAG_APPEXECLINK
    {
        return String::new();
    }

    // Build full path: dir_path + filename
    let full_path = dir_path.join (&file_info.file_name);

    // Convert to wide string for CreateFileW
    let wide_path: Vec<u16> = OsStr::new (&full_path)
        .encode_wide()
        .chain (std::iter::once (0))
        .collect();

    // Open the reparse point itself (not the target)
    let handle = unsafe {
        CreateFileW (
            windows::core::PCWSTR (wide_path.as_ptr()),
            0, // No access needed — just for FSCTL
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
            None,
        )
    };

    let handle = match handle {
        Ok (h) => h,
        Err (_) => return String::new(), // Access denied or other error — graceful degradation
    };

    // Read reparse data via IOCTL
    let mut buffer = [0u8; MAXIMUM_REPARSE_DATA_BUFFER_SIZE];
    let mut bytes_returned: u32 = 0;

    let success = unsafe {
        DeviceIoControl (
            handle,
            FSCTL_GET_REPARSE_POINT,
            None,
            0,
            Some (buffer.as_mut_ptr().cast()),
            buffer.len() as u32,
            Some (&mut bytes_returned),
            None,
        )
    };

    // Close handle
    let _ = unsafe { windows::Win32::Foundation::CloseHandle (handle) };

    if success.is_err() {
        return String::new();
    }

    let data = &buffer[..bytes_returned as usize];

    // Dispatch to the appropriate parser
    match tag {
        IO_REPARSE_TAG_MOUNT_POINT => parse_junction_buffer (data),
        IO_REPARSE_TAG_SYMLINK     => parse_symlink_buffer (data),
        IO_REPARSE_TAG_APPEXECLINK => parse_app_exec_link_buffer (data),
        _                          => String::new(),
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  extract_utf16_string
//
//  Extract a UTF-16LE string from a byte buffer at the given offset and length.
//  Returns None if the range is out of bounds.
//
////////////////////////////////////////////////////////////////////////////////

fn extract_utf16_string (buffer: &[u8], offset: usize, byte_length: usize) -> Option<String> {
    let end = offset + byte_length;
    if end > buffer.len() || byte_length < 2 {
        return None;
    }

    let u16_chars: Vec<u16> = buffer[offset..end]
        .chunks_exact (2)
        .map (|pair| u16::from_le_bytes ([pair[0], pair[1]]))
        .collect();

    Some (String::from_utf16_lossy (&u16_chars))
}





////////////////////////////////////////////////////////////////////////////////
//
//  Unit tests
//
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;





    ////////////////////////////////////////////////////////////////////////////
    //
    //  Test helpers — buffer builders
    //
    ////////////////////////////////////////////////////////////////////////////

    /// Build a junction (mount-point) reparse data buffer with the given
    /// PrintName and SubstituteName.
    fn build_junction_buffer (print_name: &str, substitute_name: &str) -> Vec<u8> {
        let print_wide: Vec<u16> = print_name.encode_utf16().collect();
        let sub_wide: Vec<u16> = substitute_name.encode_utf16().collect();

        let print_bytes = print_wide.len() * 2;
        let sub_bytes = sub_wide.len() * 2;

        // PathBuffer: SubstituteName then PrintName
        let sub_offset: u16 = 0;
        let print_offset: u16 = sub_bytes as u16;

        let path_buffer_size = sub_bytes + print_bytes;
        let data_length = (MOUNT_POINT_HEADER_SIZE + path_buffer_size) as u16;

        let mut buf = Vec::new();

        // Header: tag (4) + data_length (2) + reserved (2)
        buf.extend_from_slice (&IO_REPARSE_TAG_MOUNT_POINT.to_le_bytes());
        buf.extend_from_slice (&data_length.to_le_bytes());
        buf.extend_from_slice (&0u16.to_le_bytes()); // reserved

        // Mount-point sub-header
        buf.extend_from_slice (&sub_offset.to_le_bytes());
        buf.extend_from_slice (&(sub_bytes as u16).to_le_bytes());
        buf.extend_from_slice (&print_offset.to_le_bytes());
        buf.extend_from_slice (&(print_bytes as u16).to_le_bytes());

        // PathBuffer
        for ch in &sub_wide {
            buf.extend_from_slice (&ch.to_le_bytes());
        }
        for ch in &print_wide {
            buf.extend_from_slice (&ch.to_le_bytes());
        }

        buf
    }





    /// Build a symlink reparse data buffer with the given PrintName,
    /// SubstituteName, and flags.
    fn build_symlink_buffer (print_name: &str, substitute_name: &str, flags: u32) -> Vec<u8> {
        let print_wide: Vec<u16> = print_name.encode_utf16().collect();
        let sub_wide: Vec<u16> = substitute_name.encode_utf16().collect();

        let print_bytes = print_wide.len() * 2;
        let sub_bytes = sub_wide.len() * 2;

        let sub_offset: u16 = 0;
        let print_offset: u16 = sub_bytes as u16;

        let path_buffer_size = sub_bytes + print_bytes;
        let data_length = (SYMLINK_HEADER_SIZE + path_buffer_size) as u16;

        let mut buf = Vec::new();

        // Header
        buf.extend_from_slice (&IO_REPARSE_TAG_SYMLINK.to_le_bytes());
        buf.extend_from_slice (&data_length.to_le_bytes());
        buf.extend_from_slice (&0u16.to_le_bytes());

        // Symlink sub-header
        buf.extend_from_slice (&sub_offset.to_le_bytes());
        buf.extend_from_slice (&(sub_bytes as u16).to_le_bytes());
        buf.extend_from_slice (&print_offset.to_le_bytes());
        buf.extend_from_slice (&(print_bytes as u16).to_le_bytes());
        buf.extend_from_slice (&flags.to_le_bytes());

        // PathBuffer
        for ch in &sub_wide {
            buf.extend_from_slice (&ch.to_le_bytes());
        }
        for ch in &print_wide {
            buf.extend_from_slice (&ch.to_le_bytes());
        }

        buf
    }





    /// Build an AppExecLink reparse data buffer with the given version,
    /// package ID, app user model ID, and target exe path.
    fn build_app_exec_link_buffer (version: u32, pkg_id: &str, app_id: &str, target_exe: &str) -> Vec<u8> {
        let mut buf = Vec::new();

        // Header
        buf.extend_from_slice (&IO_REPARSE_TAG_APPEXECLINK.to_le_bytes());

        // Placeholder for data_length — filled in at the end
        let data_len_pos = buf.len();
        buf.extend_from_slice (&0u16.to_le_bytes());
        buf.extend_from_slice (&0u16.to_le_bytes()); // reserved

        let data_start = buf.len();

        // Version
        buf.extend_from_slice (&version.to_le_bytes());

        // Three NUL-terminated UTF-16 strings
        for s in &[pkg_id, app_id, target_exe] {
            for ch in s.encode_utf16() {
                buf.extend_from_slice (&ch.to_le_bytes());
            }
            buf.extend_from_slice (&0u16.to_le_bytes()); // NUL terminator
        }

        // Patch data_length
        let data_length = (buf.len() - data_start) as u16;
        buf[data_len_pos..data_len_pos + 2].copy_from_slice (&data_length.to_le_bytes());

        buf
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  strip_device_prefix tests
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn strip_prefix_removes_device_prefix() {
        assert_eq! (strip_device_prefix ("\\??\\C:\\Users\\Dev"), "C:\\Users\\Dev");
    }

    #[test]
    fn strip_prefix_preserves_no_prefix_path() {
        assert_eq! (strip_device_prefix ("C:\\Users\\Dev"), "C:\\Users\\Dev");
    }

    #[test]
    fn strip_prefix_preserves_unc_path() {
        assert_eq! (strip_device_prefix ("\\\\server\\share"), "\\\\server\\share");
    }

    #[test]
    fn strip_prefix_handles_empty_string() {
        assert_eq! (strip_device_prefix (""), "");
    }

    #[test]
    fn strip_prefix_handles_prefix_only() {
        assert_eq! (strip_device_prefix ("\\??\\"), "");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_junction_buffer tests
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn junction_extracts_print_name() {
        let buf = build_junction_buffer ("C:\\Dev\\Projects", "\\??\\C:\\Dev\\Projects");
        assert_eq! (parse_junction_buffer (&buf), "C:\\Dev\\Projects");
    }

    #[test]
    fn junction_falls_back_to_substitute_name_with_prefix_stripped() {
        let buf = build_junction_buffer ("", "\\??\\C:\\Dev\\Projects");
        assert_eq! (parse_junction_buffer (&buf), "C:\\Dev\\Projects");
    }

    #[test]
    fn junction_substitute_name_without_prefix() {
        let buf = build_junction_buffer ("", "C:\\Dev\\Projects");
        assert_eq! (parse_junction_buffer (&buf), "C:\\Dev\\Projects");
    }

    #[test]
    fn junction_empty_names_returns_empty() {
        let buf = build_junction_buffer ("", "");
        assert_eq! (parse_junction_buffer (&buf), "");
    }

    #[test]
    fn junction_truncated_buffer_returns_empty() {
        assert_eq! (parse_junction_buffer (&[0u8; 4]), "");
    }

    #[test]
    fn junction_wrong_tag_returns_empty() {
        let mut buf = build_junction_buffer ("C:\\Test", "\\??\\C:\\Test");
        // Corrupt the tag
        buf[0] = 0xFF;
        assert_eq! (parse_junction_buffer (&buf), "");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_symlink_buffer tests
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn symlink_extracts_print_name() {
        let buf = build_symlink_buffer ("C:\\Target\\File.txt", "\\??\\C:\\Target\\File.txt", 0);
        assert_eq! (parse_symlink_buffer (&buf), "C:\\Target\\File.txt");
    }

    #[test]
    fn symlink_absolute_strips_prefix_from_substitute() {
        let buf = build_symlink_buffer ("", "\\??\\C:\\Target\\File.txt", 0);
        assert_eq! (parse_symlink_buffer (&buf), "C:\\Target\\File.txt");
    }

    #[test]
    fn symlink_relative_preserves_substitute_as_stored() {
        let buf = build_symlink_buffer ("", "..\\shared\\config.yml", SYMLINK_FLAG_RELATIVE);
        assert_eq! (parse_symlink_buffer (&buf), "..\\shared\\config.yml");
    }

    #[test]
    fn symlink_relative_with_print_name_uses_print_name() {
        let buf = build_symlink_buffer ("..\\shared\\config.yml", "..\\shared\\config.yml", SYMLINK_FLAG_RELATIVE);
        assert_eq! (parse_symlink_buffer (&buf), "..\\shared\\config.yml");
    }

    #[test]
    fn symlink_truncated_buffer_returns_empty() {
        assert_eq! (parse_symlink_buffer (&[0u8; 4]), "");
    }

    #[test]
    fn symlink_wrong_tag_returns_empty() {
        let mut buf = build_symlink_buffer ("C:\\Test", "", 0);
        buf[0] = 0xFF;
        assert_eq! (parse_symlink_buffer (&buf), "");
    }

    #[test]
    fn symlink_empty_names_returns_empty() {
        let buf = build_symlink_buffer ("", "", 0);
        assert_eq! (parse_symlink_buffer (&buf), "");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_app_exec_link_buffer tests
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn appexeclink_extracts_target_exe() {
        let buf = build_app_exec_link_buffer (
            3,
            "Microsoft.DesktopAppInstaller_8wekyb3d8bbwe",
            "Microsoft.DesktopAppInstaller_8wekyb3d8bbwe!winget",
            "C:\\Program Files\\WindowsApps\\winget.exe",
        );
        assert_eq! (
            parse_app_exec_link_buffer (&buf),
            "C:\\Program Files\\WindowsApps\\winget.exe"
        );
    }

    #[test]
    fn appexeclink_version_mismatch_returns_empty() {
        let buf = build_app_exec_link_buffer (2, "pkg", "app", "target.exe");
        assert_eq! (parse_app_exec_link_buffer (&buf), "");
    }

    #[test]
    fn appexeclink_truncated_buffer_returns_empty() {
        assert_eq! (parse_app_exec_link_buffer (&[0u8; 4]), "");
    }

    #[test]
    fn appexeclink_wrong_tag_returns_empty() {
        let mut buf = build_app_exec_link_buffer (3, "pkg", "app", "target.exe");
        buf[0] = 0xFF;
        assert_eq! (parse_app_exec_link_buffer (&buf), "");
    }

    #[test]
    fn appexeclink_empty_target_returns_empty() {
        let buf = build_app_exec_link_buffer (3, "pkg", "app", "");
        assert_eq! (parse_app_exec_link_buffer (&buf), "");
    }
}
