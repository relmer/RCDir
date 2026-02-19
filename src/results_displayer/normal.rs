// normal.rs — Standard format displayer: date, time, attributes, size, filename
//
// Port of: CResultsDisplayerNormal + CResultsDisplayerWithHeaderAndFooter

use std::sync::Arc;

use crate::cloud_status;
use crate::command_line::{CommandLine, TimeField};
use crate::config::{Attribute, Config};
use crate::console::Console;
use crate::directory_info::DirectoryInfo;
use crate::drive_info::DriveInfo;
use crate::file_info::{FileInfo, FILE_ATTRIBUTE_MAP};
use crate::listing_totals::ListingTotals;
use crate::owner;

use super::common::{
    display_cloud_status_symbol,
    display_directory_summary,
    display_drive_header,
    display_empty_directory_message,
    display_listing_summary,
    display_path_header,
    display_volume_footer,
    format_number_with_separators,
    get_string_length_of_max_file_size,
};
use super::{DirectoryLevel, ResultsDisplayer};





////////////////////////////////////////////////////////////////////////////////

/// Standard format displayer — date, time, attributes, size, filename.
///
/// Port of: CResultsDisplayerNormal + CResultsDisplayerWithHeaderAndFooter
pub struct NormalDisplayer {
    console:      Console,
    cmd:          Arc<CommandLine>,
    config:       Arc<Config>,
    icons_active: bool,
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl NormalDisplayer
//
//  Normal displayer construction and console access.
//
////////////////////////////////////////////////////////////////////////////////

impl NormalDisplayer {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  new
    //
    //  Create a new NormalDisplayer.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn new(console: Console, cmd: Arc<CommandLine>, config: Arc<Config>, icons_active: bool) -> Self {
        NormalDisplayer { console, cmd, config, icons_active }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  into_console
    //
    //  Consume the displayer and return the Console for further use.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn into_console(self) -> Console {
        self.console
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  console_mut
    //
    //  Get a mutable reference to the console (for flushing from lib::run).
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn console_mut(&mut self) -> &mut Console {
        &mut self.console
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl ResultsDisplayer for NormalDisplayer
//
//  Normal-format directory listing and recursive summary.
//
////////////////////////////////////////////////////////////////////////////////

impl ResultsDisplayer for NormalDisplayer {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_results
    //
    //  Display results for a single directory.
    //  Port of: CResultsDisplayerWithHeaderAndFooter::DisplayResults
    //
    ////////////////////////////////////////////////////////////////////////////

    fn display_results(&mut self, drive_info: &DriveInfo, dir_info: &DirectoryInfo, level: DirectoryLevel) {
        // Skip empty subdirectories
        if level == DirectoryLevel::Subdirectory && dir_info.matches.is_empty() {
            return;
        }

        if level == DirectoryLevel::Initial {
            // Show drive header only for initial directory
            display_drive_header(&mut self.console, drive_info);
        }

        display_path_header(&mut self.console, dir_info);

        if dir_info.matches.is_empty() {
            display_empty_directory_message(&mut self.console, dir_info);
        } else {
            display_file_results(&mut self.console, &self.cmd, &self.config, dir_info, self.icons_active);
            display_directory_summary(&mut self.console, dir_info);

            // Only show volume footer if we're not doing recursive listing
            if !self.cmd.recurse {
                display_volume_footer(&mut self.console, dir_info);
            }
        }

        // Trailing blank line + separator
        self.console.puts(Attribute::Default, "");
        self.console.puts(Attribute::Default, "");

        let _ = self.console.flush();
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_recursive_summary
    //
    //  Display recursive summary after all directories.
    //  Port of: CResultsDisplayerWithHeaderAndFooter::DisplayRecursiveSummary
    //
    ////////////////////////////////////////////////////////////////////////////

    fn display_recursive_summary(&mut self, dir_info: &DirectoryInfo, totals: &ListingTotals) {
        display_listing_summary(&mut self.console, dir_info, totals);
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_file_results
//
//  Display all file entries in a directory.
//  Port of: CResultsDisplayerNormal::DisplayFileResults
//
////////////////////////////////////////////////////////////////////////////////

fn display_file_results(
    console: &mut Console,
    cmd: &CommandLine,
    config: &Config,
    dir_info: &DirectoryInfo,
    icons_active: bool,
) {
    let max_size_width = get_string_length_of_max_file_size(dir_info.largest_file_size);
    let in_sync_root = cloud_status::is_under_sync_root(dir_info.dir_path.as_os_str());

    // Collect file owners if --owner is enabled (two-pass: first collect, then display)
    let (owners, max_owner_len) = if cmd.show_owner {
        owner::get_file_owners(dir_info)
    } else {
        (Vec::new(), 0)
    };

    for (idx, file_info) in dir_info.matches.iter().enumerate() {
        let style = config.get_display_style_for_file (file_info);
        let text_attr = style.text_attr;

        // Date and time
        let time_value = get_time_field_for_display(file_info, cmd.time_field);
        display_date_and_time(console, time_value);

        // Attributes
        display_attributes(console, config, file_info.file_attributes);

        // File size or <DIR>
        display_file_size(console, file_info, max_size_width);

        // Cloud status symbol
        let cloud = cloud_status::get_cloud_status(file_info.file_attributes, in_sync_root);
        display_cloud_status_symbol(console, config, cloud, icons_active);

        // Debug attribute display (debug builds only, gated by --debug)
        #[cfg(debug_assertions)]
        if cmd.debug {
            display_raw_attributes(console, config, file_info);
        }

        // Owner column (if --owner)
        if let (true, Some(owner_str)) = (cmd.show_owner, owners.get(idx)) {
            display_file_owner(console, config, owner_str, max_owner_len);
        }

        // Icon glyph (when icons are active and not suppressed)
        if icons_active {
            if let Some(icon) = style.icon_code_point {
                if !style.icon_suppressed {
                    console.writef (text_attr, format_args! ("{} ", icon));
                } else {
                    console.printf (text_attr, "  ");
                }
            } else {
                console.printf (text_attr, "  ");
            }
        }

        // Filename
        let name_str = file_info.file_name.to_string_lossy();
        console.writef_line (text_attr, format_args! ("{}", name_str));

        // Streams (if --streams and this is a file, not a directory)
        if cmd.show_streams && !file_info.streams.is_empty() {
            let owner_width = if cmd.show_owner { max_owner_len } else { 0 };
            display_file_streams(console, config, file_info, max_size_width, owner_width);
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  get_time_field_for_display
//
//  Get the appropriate time field for display based on /T: switch.
//  Port of: CResultsDisplayerNormal::GetTimeFieldForDisplay
//
////////////////////////////////////////////////////////////////////////////////

fn get_time_field_for_display(fi: &FileInfo, time_field: TimeField) -> u64 {
    match time_field {
        TimeField::Creation => fi.creation_time,
        TimeField::Access   => fi.last_access_time,
        TimeField::Written  => fi.last_write_time,
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_date_and_time
//
//  Display date and time from a FILETIME (as u64).
//  Uses Win32 APIs for locale-aware formatting.
//  Port of: CResultsDisplayerNormal::DisplayResultsNormalDateAndTime
//
////////////////////////////////////////////////////////////////////////////////

fn display_date_and_time(console: &mut Console, filetime_u64: u64) {
    let ft = windows::Win32::Foundation::FILETIME {
        dwLowDateTime:  (filetime_u64 & 0xFFFF_FFFF) as u32,
        dwHighDateTime: ((filetime_u64 >> 32) & 0xFFFF_FFFF) as u32,
    };

    let mut st = windows::Win32::Foundation::SYSTEMTIME::default();
    let mut st_local = windows::Win32::Foundation::SYSTEMTIME::default();

    let ok1 = unsafe {
        windows::Win32::System::Time::FileTimeToSystemTime(&ft, &mut st)
    };
    if ok1.is_err() {
        console.color_printf("{Date}??/??/????  {Time}??:?? ??{Default} ");
        return;
    }

    let ok2 = unsafe {
        windows::Win32::System::Time::SystemTimeToTzSpecificLocalTime(
            None, &st, &mut st_local,
        )
    };
    if ok2.is_err() {
        console.color_printf("{Date}??/??/????  {Time}??:?? ??{Default} ");
        return;
    }

    // Format date using GetDateFormatEx
    let date_format: Vec<u16> = "MM/dd/yyyy\0".encode_utf16().collect();
    let mut date_buf = [0u16; 11]; // "12/34/5678" + null

    let date_len = unsafe {
        windows::Win32::Globalization::GetDateFormatEx(
            windows::core::PCWSTR(std::ptr::null()),
            windows::Win32::Globalization::ENUM_DATE_FORMATS_FLAGS(0),
            Some(&st_local),
            windows::core::PCWSTR(date_format.as_ptr()),
            Some(&mut date_buf),
            None,
        )
    };

    // Format time using GetTimeFormatEx
    let time_format: Vec<u16> = "hh:mm tt\0".encode_utf16().collect();
    let mut time_buf = [0u16; 9]; // "12:34 PM" + null

    let time_len = unsafe {
        windows::Win32::Globalization::GetTimeFormatEx(
            windows::core::PCWSTR(std::ptr::null()),
            windows::Win32::Globalization::TIME_FORMAT_FLAGS(0),
            Some(&st_local),
            windows::core::PCWSTR(time_format.as_ptr()),
            Some(&mut time_buf),
        )
    };

    if date_len > 0 && time_len > 0 {
        let date_str = String::from_utf16_lossy(&date_buf[..(date_len as usize - 1)]);
        let time_str = String::from_utf16_lossy(&time_buf[..(time_len as usize - 1)]);
        console.color_printf(&format!(
            "{{Date}}{}  {{Time}}{}{{Default}} ",
            date_str, time_str,
        ));
    } else {
        console.color_printf("{Date}??/??/????  {Time}??:?? ??{Default} ");
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_attributes
//
//  Display the 9-char attribute column with colorization.
//  Port of: CResultsDisplayerNormal::DisplayResultsNormalAttributes
//
////////////////////////////////////////////////////////////////////////////////

fn display_attributes(console: &mut Console, config: &Config, file_attributes: u32) {
    let present_attr = config.attributes[Attribute::FileAttributePresent as usize];
    let absent_attr  = config.attributes[Attribute::FileAttributeNotPresent as usize];

    for &(flag, ch) in &FILE_ATTRIBUTE_MAP {
        if (file_attributes & flag) != 0 {
            console.putchar(present_attr, ch);
        } else {
            console.putchar(absent_attr, '-');
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_file_size
//
//  Display file size (right-aligned with separators) or centered <DIR>.
//  Port of: CResultsDisplayerNormal::DisplayResultsNormalFileSize
//
////////////////////////////////////////////////////////////////////////////////

fn display_file_size(console: &mut Console, fi: &FileInfo, max_size_width: usize) {
    let dir_label = "<DIR>";
    let col_width = max_size_width.max(dir_label.len());

    if !fi.is_directory() {
        let formatted = format_number_with_separators(fi.file_size);
        console.writef_attr (Attribute::Size, format_args! (" {:>width$} ", formatted, width = col_width));
    } else {
        // Center <DIR> within the column
        let left_pad = (col_width - dir_label.len()) / 2;
        let right_pad = col_width - dir_label.len() - left_pad;
        console.writef_attr (Attribute::Directory, format_args! (
            " {:>left$}{}{:>right$} ",
            "", dir_label, "",
            left = left_pad,
            right = right_pad,
        ));
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_raw_attributes
//
//  Display raw file attributes and cloud placeholder state in hex.
//  Format: [XXXXXXXX:YY] — 8 hex digits for file attributes, 2 hex
//  digits for CF state.  Only compiled in debug builds.
//  Port of: CResultsDisplayerNormal::DisplayRawAttributes
//
////////////////////////////////////////////////////////////////////////////////

#[cfg(debug_assertions)]
fn display_raw_attributes(console: &mut Console, config: &Config, file_info: &FileInfo) {
    use windows::Win32::Storage::CloudFilters::CfGetPlaceholderStateFromAttributeTag;

    let cf_state = unsafe {
        CfGetPlaceholderStateFromAttributeTag(file_info.file_attributes, file_info.reparse_tag)
    };

    let info_color = config.attributes[Attribute::Information as usize];
    console.writef (info_color, format_args! ("[{:08X}:{:02X}] ", file_info.file_attributes, cf_state.0 as u8));
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_file_owner
//
//  Display a file owner string, padded to max_width.
//  Port of: CResultsDisplayerNormal::DisplayFileOwner
//
////////////////////////////////////////////////////////////////////////////////

fn display_file_owner(console: &mut Console, config: &Config, owner: &str, max_width: usize) {
    let color = config.attributes[Attribute::Owner as usize];
    let padding = if max_width > owner.len() { max_width - owner.len() } else { 0 };
    console.writef (color, format_args! ("{}{:width$} ", owner, "", width = padding));
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_file_streams
//
//  Display alternate data streams below a file entry.
//  Port of: CResultsDisplayerNormal::DisplayFileStreams
//
////////////////////////////////////////////////////////////////////////////////

fn display_file_streams(
    console: &mut Console,
    config: &Config,
    file_info: &FileInfo,
    max_size_width: usize,
    owner_width: usize,
) {
    let size_field_width = max_size_width.max(5);
    let file_name = file_info.file_name.to_string_lossy();
    let stream_color = config.attributes[Attribute::Stream as usize];
    let size_color = config.attributes[Attribute::Size as usize];
    let owner_padding = if owner_width > 0 { owner_width + 1 } else { 0 };

    let default_color = config.attributes[Attribute::Default as usize];

    for si in &file_info.streams {
        let formatted_size = format_number_with_separators(si.size as u64);

        // 30 chars indentation (date/time 21 + attributes 9)
        // Then size field with padding, 2 spaces cloud placeholder, owner padding, then filename:stream
        console.writef (default_color, format_args! ("{:30}", ""));
        console.writef (size_color, format_args! (" {:>width$} ", formatted_size, width = size_field_width));
        console.writef (default_color, format_args! ("  {:width$}", "", width = owner_padding));
        console.writef_line (stream_color, format_args! ("{}{}", file_name, si.name));
    }
}
