// results_displayer.rs — Display formatting for directory listings
//
// Port of: IResultsDisplayer.h, ResultsDisplayerNormal.h/.cpp,
//          ResultsDisplayerWithHeaderAndFooter.h/.cpp,
//          ResultsDisplayerWide.h/.cpp,
//          ResultsDisplayerBare.h/.cpp
//
// Provides the ResultsDisplayer trait including NormalDisplayer, WideDisplayer,
// and BareDisplayer implementations, plus a Displayer enum wrapper.

use std::os::windows::ffi::OsStrExt;
use std::sync::Arc;

use crate::cloud_status::{self, CloudStatus};
use crate::command_line::{CommandLine, TimeField};
use crate::config::{Attribute, Config};
use crate::console::Console;
use crate::directory_info::DirectoryInfo;
use crate::drive_info::DriveInfo;
use crate::file_info::{FileInfo, FILE_ATTRIBUTE_MAP};
use crate::listing_totals::ListingTotals;
use crate::owner;

/// Directory level for display formatting.
/// Port of: IResultsDisplayer::EDirectoryLevel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectoryLevel {
    Initial,
    Subdirectory,
}

/// Trait for displaying directory listing results.
/// Port of: IResultsDisplayer
pub trait ResultsDisplayer {
    fn display_results(&mut self, drive_info: &DriveInfo, dir_info: &DirectoryInfo, level: DirectoryLevel);
    fn display_recursive_summary(&mut self, dir_info: &DirectoryInfo, totals: &ListingTotals);
}

// ── NormalDisplayer ───────────────────────────────────────────────────────────

/// Standard format displayer — date, time, attributes, size, filename.
///
/// Port of: CResultsDisplayerNormal + CResultsDisplayerWithHeaderAndFooter
pub struct NormalDisplayer {
    console: Console,
    cmd:     Arc<CommandLine>,
    config:  Arc<Config>,
}

impl NormalDisplayer {
    pub fn new(console: Console, cmd: Arc<CommandLine>, config: Arc<Config>) -> Self {
        NormalDisplayer { console, cmd, config }
    }

    /// Consume the displayer and return the Console for further use.
    pub fn into_console(self) -> Console {
        self.console
    }

    /// Get a mutable reference to the console (for flushing from lib::run).
    pub fn console_mut(&mut self) -> &mut Console {
        &mut self.console
    }
}

impl ResultsDisplayer for NormalDisplayer {
    /// Display results for a single directory.
    ///
    /// Port of: CResultsDisplayerWithHeaderAndFooter::DisplayResults
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
            display_file_results(&mut self.console, &self.cmd, &self.config, dir_info);
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

    /// Display recursive summary after all directories.
    ///
    /// Port of: CResultsDisplayerWithHeaderAndFooter::DisplayRecursiveSummary
    fn display_recursive_summary(&mut self, dir_info: &DirectoryInfo, totals: &ListingTotals) {
        display_listing_summary(&mut self.console, dir_info, totals);
    }
}

// ── Volume header ─────────────────────────────────────────────────────────────

/// Display drive/volume header.
///
/// Port of: CResultsDisplayerWithHeaderAndFooter::DisplayDriveHeader
fn display_drive_header(console: &mut Console, drive_info: &DriveInfo) {
    // First line: "Volume [path] is [description] [mapped to X] (filesystem)"
    if drive_info.is_unc_path {
        console.color_printf(&format!(
            "{{Information}} Volume {{InformationHighlight}}{}{{Information}} is {{InformationHighlight}}{}{{Information}}",
            drive_info.unc_path.display(),
            drive_info.volume_description(),
        ));
    } else {
        let drive_letter = drive_info.root_path.to_string_lossy()
            .chars().next().unwrap_or('?');
        console.color_printf(&format!(
            "{{Information}} Volume in drive {{InformationHighlight}}{}{{Information}} is {{InformationHighlight}}{}{{Information}}",
            drive_letter,
            drive_info.volume_description(),
        ));
    }

    // Mapped drive remote name
    if !drive_info.remote_name.is_empty() {
        console.color_printf(&format!(
            "{{Information}} mapped to {{InformationHighlight}}{}{{Information}}",
            drive_info.remote_name,
        ));
    }

    console.color_puts(&format!(
        "{{Information}} ({{InformationHighlight}}{}{{Information}})",
        drive_info.file_system_name,
    ));

    // Volume name (second line)
    if !drive_info.volume_name.is_empty() {
        console.color_puts(&format!(
            "{{Information}} Volume name is \"{{InformationHighlight}}{}{{Information}}\"",
            drive_info.volume_name,
        ));
    } else {
        console.color_puts("{Information} Volume has no name");
    }

    console.color_puts("");
}

// ── Path header ───────────────────────────────────────────────────────────────

/// Display "Directory of <path>" header.
///
/// Port of: CResultsDisplayerWithHeaderAndFooter::DisplayPathHeader
fn display_path_header(console: &mut Console, dir_info: &DirectoryInfo) {
    console.color_printf(&format!(
        "{{Information}} Directory of {{InformationHighlight}}{}{{Information}}\n\n",
        dir_info.dir_path.display(),
    ));
}

// ── Empty directory ───────────────────────────────────────────────────────────

/// Display message when directory has no matches.
///
/// Port of: CResultsDisplayerWithHeaderAndFooter::DisplayEmptyDirectoryMessage
fn display_empty_directory_message(console: &mut Console, dir_info: &DirectoryInfo) {
    // Check if all specs are "*"
    let all_star = dir_info.file_specs.iter().all(|s| s == "*");

    if all_star {
        console.puts(Attribute::Default, "Directory is empty.");
    } else if dir_info.file_specs.len() == 1 {
        console.printf_attr(Attribute::Default, &format!(
            "No files matching '{}' found.\n",
            dir_info.file_specs[0],
        ));
    } else {
        let specs = dir_info.file_specs.join(", ");
        console.printf_attr(Attribute::Default, &format!(
            "No files matching '{}' found.\n",
            specs,
        ));
    }
}

// ── File results ──────────────────────────────────────────────────────────────

/// Display all file entries in a directory.
///
/// Port of: CResultsDisplayerNormal::DisplayFileResults
fn display_file_results(
    console: &mut Console,
    cmd: &CommandLine,
    config: &Config,
    dir_info: &DirectoryInfo,
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
        let text_attr = config.get_text_attr_for_file(file_info.file_attributes, &file_info.file_name);

        // Date and time
        let time_value = get_time_field_for_display(file_info, cmd.time_field);
        display_date_and_time(console, time_value);

        // Attributes
        display_attributes(console, config, file_info.file_attributes);

        // File size or <DIR>
        display_file_size(console, file_info, max_size_width);

        // Cloud status symbol
        let cloud = cloud_status::get_cloud_status(file_info.file_attributes, in_sync_root);
        display_cloud_status_symbol(console, config, cloud);

        // Owner column (if --owner)
        if cmd.show_owner {
            if let Some(owner_str) = owners.get(idx) {
                display_file_owner(console, config, owner_str, max_owner_len);
            }
        }

        // Filename
        let name_str = file_info.file_name.to_string_lossy();
        console.printf(text_attr, &format!("{}\n", name_str));
    }
}

// ── Date/time formatting ──────────────────────────────────────────────────────

/// Get the appropriate time field for display based on /T: switch.
///
/// Port of: CResultsDisplayerNormal::GetTimeFieldForDisplay
fn get_time_field_for_display(fi: &FileInfo, time_field: TimeField) -> u64 {
    match time_field {
        TimeField::Creation => fi.creation_time,
        TimeField::Access   => fi.last_access_time,
        TimeField::Written  => fi.last_write_time,
    }
}

/// Display date and time from a FILETIME (as u64).
/// Uses Win32 APIs for locale-aware formatting.
///
/// Port of: CResultsDisplayerNormal::DisplayResultsNormalDateAndTime
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

// ── Attribute column ──────────────────────────────────────────────────────────

/// Display the 9-char attribute column with colorization.
///
/// Port of: CResultsDisplayerNormal::DisplayResultsNormalAttributes
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

// ── File size ─────────────────────────────────────────────────────────────────

/// Display file size (right-aligned with separators) or centered <DIR>.
///
/// Port of: CResultsDisplayerNormal::DisplayResultsNormalFileSize
fn display_file_size(console: &mut Console, fi: &FileInfo, max_size_width: usize) {
    let dir_label = "<DIR>";
    let col_width = max_size_width.max(dir_label.len());

    if !fi.is_directory() {
        let formatted = format_number_with_separators(fi.file_size);
        console.printf_attr(Attribute::Size, &format!(" {:>width$} ", formatted, width = col_width));
    } else {
        // Center <DIR> within the column
        let left_pad = (col_width - dir_label.len()) / 2;
        let right_pad = col_width - dir_label.len() - left_pad;
        console.printf_attr(Attribute::Directory, &format!(
            " {:>left$}{}{:>right$} ",
            "", dir_label, "",
            left = left_pad,
            right = right_pad,
        ));
    }
}

// ── Cloud status symbol ───────────────────────────────────────────────────────

/// Display cloud status symbol with configured color.
///
/// Port of: CResultsDisplayerNormal::DisplayCloudStatusSymbol
fn display_cloud_status_symbol(console: &mut Console, config: &Config, status: CloudStatus) {
    let (attr, symbol) = match status {
        CloudStatus::None      => (Attribute::Default,                            ' '),
        CloudStatus::CloudOnly => (Attribute::CloudStatusCloudOnly,               cloud_status::CIRCLE_HOLLOW),
        CloudStatus::Local     => (Attribute::CloudStatusLocallyAvailable,        cloud_status::CIRCLE_HALF_FILLED),
        CloudStatus::Pinned    => (Attribute::CloudStatusAlwaysLocallyAvailable,  cloud_status::CIRCLE_FILLED),
    };

    let color = config.attributes[attr as usize];
    console.printf(color, &format!("{} ", symbol));
}

/// Display a file owner string, padded to `max_width`.
///
/// Port of: CResultsDisplayerNormal::DisplayFileOwner
fn display_file_owner(console: &mut Console, config: &Config, owner: &str, max_width: usize) {
    let color = config.attributes[Attribute::Owner as usize];
    let padding = if max_width > owner.len() { max_width - owner.len() } else { 0 };
    console.printf(color, &format!("{}{:width$} ", owner, "", width = padding));
}

// ── Directory summary ─────────────────────────────────────────────────────────

/// Display summary line: "X dirs, Y files using Z bytes"
///
/// Port of: CResultsDisplayerWithHeaderAndFooter::DisplayDirectorySummary
fn display_directory_summary(console: &mut Console, di: &DirectoryInfo) {
    let dirs_word  = if di.subdirectory_count == 1 { " dir, " } else { " dirs, " };
    let files_word = if di.file_count == 1 { " file using " } else { " files using " };
    let bytes_word = if di.bytes_used == 1 { " byte" } else { " bytes" };

    console.color_printf(&format!(
        "\n{{Information}} {{InformationHighlight}}{}{{Information}}{}{{InformationHighlight}}{}{{Information}}{}{{InformationHighlight}}{}{{Information}}{}",
        di.subdirectory_count,
        dirs_word,
        di.file_count,
        files_word,
        format_number_with_separators(di.bytes_used),
        bytes_word,
    ));

    if di.stream_count > 0 {
        let streams_word = if di.stream_count == 1 { " stream using " } else { " streams using " };
        let sbytes_word  = if di.stream_bytes_used == 1 { " byte" } else { " bytes" };
        console.color_printf(&format!(
            "{{Information}}, {{InformationHighlight}}{}{{Information}}{}{{InformationHighlight}}{}{{Information}}{}",
            di.stream_count,
            streams_word,
            format_number_with_separators(di.stream_bytes_used),
            sbytes_word,
        ));
    }

    console.color_puts("");
}

// ── Volume footer ─────────────────────────────────────────────────────────────

/// Display free space on volume.
///
/// Port of: CResultsDisplayerWithHeaderAndFooter::DisplayVolumeFooter
fn display_volume_footer(console: &mut Console, di: &DirectoryInfo) {
    let dir_wide: Vec<u16> = di.dir_path.as_os_str().encode_wide().chain(Some(0)).collect();

    let mut free_bytes_available = 0u64;
    let mut total_bytes = 0u64;
    let mut total_free_bytes = 0u64;

    let success = unsafe {
        windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
            windows::core::PCWSTR(dir_wide.as_ptr()),
            Some(&mut free_bytes_available),
            Some(&mut total_bytes),
            Some(&mut total_free_bytes),
        )
    };

    if success.is_err() {
        return;
    }

    let bytes_word = if total_free_bytes == 1 { " byte free on volume" } else { " bytes free on volume" };
    console.color_printf(&format!(
        "{{InformationHighlight}} {}{{Information}}{}\n",
        format_number_with_separators(total_free_bytes),
        bytes_word,
    ));

    // Show quota info if different from total free bytes
    if free_bytes_available != total_free_bytes {
        display_footer_quota_info(console, free_bytes_available);
    }
}

/// Display quota-limited free space info.
///
/// Port of: CResultsDisplayerWithHeaderAndFooter::DisplayFooterQuotaInfo
fn display_footer_quota_info(console: &mut Console, free_bytes_available: u64) {
    // Get current username
    let mut username_buf = vec![0u16; 257]; // UNLEN + 1
    let mut username_len = username_buf.len() as u32;

    let username = unsafe {
        let success = windows::Win32::System::WindowsProgramming::GetUserNameW(
            Some(windows::core::PWSTR(username_buf.as_mut_ptr())),
            &mut username_len,
        );
        if success.is_ok() && username_len > 1 {
            String::from_utf16_lossy(&username_buf[..(username_len as usize - 1)])
        } else {
            "<Unknown>".to_string()
        }
    };

    let bytes_word = if free_bytes_available == 1 { " byte available to " } else { " bytes available to " };
    console.color_printf(&format!(
        "{{Information}} {{InformationHighlight}}{}{{Information}}{}{{InformationHighlight}}{}{{Information}} due to quota\n",
        format_number_with_separators(free_bytes_available),
        bytes_word,
        username,
    ));
}

// ── Listing summary (recursive) ──────────────────────────────────────────────

/// Display full recursive summary.
///
/// Port of: CResultsDisplayerWithHeaderAndFooter::DisplayListingSummary
fn display_listing_summary(console: &mut Console, di: &DirectoryInfo, totals: &ListingTotals) {
    let max_count = totals.file_count.max(totals.directory_count);
    let max_digits = if max_count > 0 {
        let d = (max_count as f64).log10() as usize + 1;
        d + d / 3 // Add space for commas
    } else {
        1
    };

    let files_word = if totals.file_count == 1 { " file using " } else { " files using " };
    let bytes_word = if totals.file_bytes == 1 { " byte" } else { " bytes" };
    let dirs_word  = if totals.directory_count == 1 { " subdirectory" } else { " subdirectories" };

    console.color_printf(&format!(
        "{{Information}} Total files listed:\n\n{{InformationHighlight}}    {:>width$}{{Information}}{}{{InformationHighlight}}{}{{Information}}{}\n{{InformationHighlight}}    {:>width$}{{Information}}{}\n",
        format_number_with_separators(totals.file_count as u64),
        files_word,
        format_number_with_separators(totals.file_bytes),
        bytes_word,
        format_number_with_separators(totals.directory_count as u64),
        dirs_word,
        width = max_digits,
    ));

    if totals.stream_count > 0 {
        let streams_word = if totals.stream_count == 1 { " stream using " } else { " streams using " };
        let sbytes_word  = if totals.stream_bytes == 1 { " byte" } else { " bytes" };
        console.color_printf(&format!(
            "{{InformationHighlight}}    {:>width$}{{Information}}{}{{InformationHighlight}}{}{{Information}}{}\n",
            format_number_with_separators(totals.stream_count as u64),
            streams_word,
            format_number_with_separators(totals.stream_bytes),
            sbytes_word,
            width = max_digits,
        ));
    }

    display_volume_footer(console, di);

    console.puts(Attribute::Default, "");
}

// ── Helper functions ──────────────────────────────────────────────────────────

/// Calculate the string length of the largest file size with thousands separators.
///
/// Port of: CResultsDisplayerWithHeaderAndFooter::GetStringLengthOfMaxFileSize
fn get_string_length_of_max_file_size(largest: u64) -> usize {
    if largest == 0 {
        return 1;
    }
    let digits = (largest as f64).log10() as usize + 1;
    digits + (digits - 1) / 3 // Add space for comma separators
}

/// Format a number with locale-aware thousands separators.
///
/// Port of: CResultsDisplayerWithHeaderAndFooter::FormatNumberWithSeparators
///
/// Uses Win32 GetNumberFormatEx for locale-aware formatting.
pub fn format_number_with_separators(n: u64) -> String {
    // Format the number as a simple string first
    let num_str = n.to_string();

    // Use Win32 GetNumberFormatEx for locale-aware formatting
    let num_wide: Vec<u16> = num_str.encode_utf16().chain(Some(0)).collect();
    let mut out_buf = [0u16; 40]; // Enough for any u64 with separators

    // NUMBERFMTW with 0 decimal digits
    let fmt = windows::Win32::Globalization::NUMBERFMTW {
        NumDigits:     0,
        LeadingZero:   0,
        Grouping:      3,
        lpDecimalSep:  windows::core::PWSTR(c".".as_ptr() as *mut u16),
        lpThousandSep: windows::core::PWSTR(c",".as_ptr() as *mut u16),
        NegativeOrder: 1,
    };

    let result = unsafe {
        windows::Win32::Globalization::GetNumberFormatEx(
            windows::core::PCWSTR(std::ptr::null()),
            0,
            windows::core::PCWSTR(num_wide.as_ptr()),
            Some(&fmt),
            Some(&mut out_buf),
        )
    };

    if result > 0 {
        let len = out_buf.iter().position(|&c| c == 0).unwrap_or(out_buf.len());
        String::from_utf16_lossy(&out_buf[..len])
    } else {
        // Fallback: manual grouping with commas
        format_with_commas(n)
    }
}

/// Fallback: format a number with commas as thousands separator.
fn format_with_commas(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    let len = s.len();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(ch);
    }
    result
}

// ── WideDisplayer ─────────────────────────────────────────────────────────────

/// Wide format displayer — multi-column filenames with [dir] brackets.
///
/// Port of: CResultsDisplayerWide
pub struct WideDisplayer {
    console: Console,
    cmd:     Arc<CommandLine>,
    config:  Arc<Config>,
}

impl WideDisplayer {
    pub fn new(console: Console, cmd: Arc<CommandLine>, config: Arc<Config>) -> Self {
        WideDisplayer { console, cmd, config }
    }

    pub fn into_console(self) -> Console {
        self.console
    }

    pub fn console_mut(&mut self) -> &mut Console {
        &mut self.console
    }
}

impl ResultsDisplayer for WideDisplayer {
    /// Display results for a single directory using wide format.
    ///
    /// Port of: CResultsDisplayerWithHeaderAndFooter::DisplayResults (header/footer)
    ///        + CResultsDisplayerWide::DisplayFileResults (column layout)
    fn display_results(&mut self, drive_info: &DriveInfo, dir_info: &DirectoryInfo, level: DirectoryLevel) {
        // Skip empty subdirectories
        if level == DirectoryLevel::Subdirectory && dir_info.matches.is_empty() {
            return;
        }

        if level == DirectoryLevel::Initial {
            display_drive_header(&mut self.console, drive_info);
        }

        display_path_header(&mut self.console, dir_info);

        if dir_info.matches.is_empty() {
            display_empty_directory_message(&mut self.console, dir_info);
        } else {
            display_wide_file_results(&mut self.console, &self.config, dir_info);
            display_directory_summary(&mut self.console, dir_info);

            if !self.cmd.recurse {
                display_volume_footer(&mut self.console, dir_info);
            }
        }

        self.console.puts(Attribute::Default, "");
        self.console.puts(Attribute::Default, "");

        let _ = self.console.flush();
    }

    fn display_recursive_summary(&mut self, dir_info: &DirectoryInfo, totals: &ListingTotals) {
        display_listing_summary(&mut self.console, dir_info, totals);
    }
}

/// Display files in column-major wide format.
///
/// Port of: CResultsDisplayerWide::DisplayFileResults
fn display_wide_file_results(console: &mut Console, config: &Config, di: &DirectoryInfo) {
    if di.largest_file_name == 0 || di.matches.is_empty() {
        return;
    }

    let console_width = console.width() as usize;

    // Account for brackets on directories: [dirname] adds 2 chars
    let max_name_len = di.matches.iter().map(|fi| {
        let base_len = fi.file_name.to_string_lossy().len();
        if fi.is_directory() { base_len + 2 } else { base_len }
    }).max().unwrap_or(0);

    // Calculate column count and widths — Port of: GetColumnInfo
    let (columns, column_width) = if max_name_len + 1 > console_width {
        (1, console_width)
    } else {
        let cols = console_width / (max_name_len + 1);
        (cols, console_width / cols)
    };

    let total_items     = di.matches.len();
    let rows            = total_items.div_ceil(columns);
    let items_in_last_row = total_items % columns;

    // Display in column-major order — Port of: DisplayFileResults loop
    for row in 0..rows {
        for col in 0..columns {
            if row * columns + col >= total_items {
                break;
            }

            // Column-major index calculation matching TCDir exactly
            let full_rows = if items_in_last_row != 0 { rows - 1 } else { rows };
            let mut idx = row + (col * full_rows);

            // Adjust for items in the last row
            if col < items_in_last_row {
                idx += col;
            } else {
                idx += items_in_last_row;
            }

            if idx >= total_items {
                break;
            }

            let fi = &di.matches[idx];
            let text_attr = config.get_text_attr_for_file(fi.file_attributes, &fi.file_name);

            // Format filename: [dirname] for dirs, plain name for files
            let name = fi.file_name.to_string_lossy();
            let display_name = if fi.is_directory() {
                format!("[{}]", name)
            } else {
                name.to_string()
            };

            console.printf(text_attr, &display_name);

            // Pad to column width
            if column_width > display_name.len() {
                console.printf_attr(Attribute::Default, &format!(
                    "{:width$}", "", width = column_width - display_name.len(),
                ));
            }
        }

        console.puts(Attribute::Default, "");
    }
}

// ── BareDisplayer ─────────────────────────────────────────────────────────────

/// Bare format displayer — filenames only, no decoration.
///
/// Port of: CResultsDisplayerBare
pub struct BareDisplayer {
    console: Console,
    cmd:     Arc<CommandLine>,
    config:  Arc<Config>,
}

impl BareDisplayer {
    pub fn new(console: Console, cmd: Arc<CommandLine>, config: Arc<Config>) -> Self {
        BareDisplayer { console, cmd, config }
    }

    pub fn into_console(self) -> Console {
        self.console
    }

    pub fn console_mut(&mut self) -> &mut Console {
        &mut self.console
    }
}

impl ResultsDisplayer for BareDisplayer {
    /// Display results for a single directory using bare format.
    ///
    /// Port of: CResultsDisplayerBare::DisplayResults
    fn display_results(&mut self, _drive_info: &DriveInfo, dir_info: &DirectoryInfo, _level: DirectoryLevel) {
        for fi in &dir_info.matches {
            let text_attr = self.config.get_text_attr_for_file(fi.file_attributes, &fi.file_name);

            if self.cmd.recurse {
                // When recursing, show full path
                let full_path = dir_info.dir_path.join(&fi.file_name);
                let path_str = full_path.to_string_lossy();
                console_printf_line(&mut self.console, text_attr, &path_str);
            } else {
                let name = fi.file_name.to_string_lossy();
                console_printf_line(&mut self.console, text_attr, &name);
            }
        }

        let _ = self.console.flush();
    }

    /// Bare mode doesn't display recursive summary.
    ///
    /// Port of: CResultsDisplayerBare::DisplayRecursiveSummary
    fn display_recursive_summary(&mut self, _dir_info: &DirectoryInfo, _totals: &ListingTotals) {
        // No summary in bare mode
    }
}

/// Helper: Printf a line with color + newline.
fn console_printf_line(console: &mut Console, attr: u16, text: &str) {
    console.printf(attr, &format!("{}\n", text));
}

// ── Displayer enum (polymorphic wrapper) ──────────────────────────────────────

/// Polymorphic displayer wrapping Normal, Wide, or Bare variants.
///
/// Provides `into_console()` and `console_mut()` without trait object issues.
pub enum Displayer {
    Normal(NormalDisplayer),
    Wide(WideDisplayer),
    Bare(BareDisplayer),
}

impl Displayer {
    /// Create the appropriate displayer based on command-line switches.
    ///
    /// Priority: bare > wide > normal (matching TCDir)
    pub fn new(console: Console, cmd: Arc<CommandLine>, config: Arc<Config>) -> Self {
        if cmd.bare_listing {
            Displayer::Bare(BareDisplayer::new(console, cmd, config))
        } else if cmd.wide_listing {
            Displayer::Wide(WideDisplayer::new(console, cmd, config))
        } else {
            Displayer::Normal(NormalDisplayer::new(console, cmd, config))
        }
    }

    /// Consume the displayer and return the Console.
    pub fn into_console(self) -> Console {
        match self {
            Displayer::Normal(d) => d.into_console(),
            Displayer::Wide(d)   => d.into_console(),
            Displayer::Bare(d)   => d.into_console(),
        }
    }

    /// Get a mutable reference to the console.
    pub fn console_mut(&mut self) -> &mut Console {
        match self {
            Displayer::Normal(d) => d.console_mut(),
            Displayer::Wide(d)   => d.console_mut(),
            Displayer::Bare(d)   => d.console_mut(),
        }
    }
}

impl ResultsDisplayer for Displayer {
    fn display_results(&mut self, drive_info: &DriveInfo, dir_info: &DirectoryInfo, level: DirectoryLevel) {
        match self {
            Displayer::Normal(d) => d.display_results(drive_info, dir_info, level),
            Displayer::Wide(d)   => d.display_results(drive_info, dir_info, level),
            Displayer::Bare(d)   => d.display_results(drive_info, dir_info, level),
        }
    }

    fn display_recursive_summary(&mut self, dir_info: &DirectoryInfo, totals: &ListingTotals) {
        match self {
            Displayer::Normal(d) => d.display_recursive_summary(dir_info, totals),
            Displayer::Wide(d)   => d.display_recursive_summary(dir_info, totals),
            Displayer::Bare(d)   => d.display_recursive_summary(dir_info, totals),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_number_zero() {
        let result = format_number_with_separators(0);
        assert_eq!(result, "0");
    }

    #[test]
    fn format_number_small() {
        let result = format_number_with_separators(42);
        assert_eq!(result, "42");
    }

    #[test]
    fn format_number_thousands() {
        let result = format_number_with_separators(1234);
        assert_eq!(result, "1,234");
    }

    #[test]
    fn format_number_millions() {
        let result = format_number_with_separators(1234567);
        assert_eq!(result, "1,234,567");
    }

    #[test]
    fn max_file_size_width_zero() {
        assert_eq!(get_string_length_of_max_file_size(0), 1);
    }

    #[test]
    fn max_file_size_width_small() {
        // 999 → 3 digits, no commas → width 3
        assert_eq!(get_string_length_of_max_file_size(999), 3);
    }

    #[test]
    fn max_file_size_width_thousands() {
        // 1234 → 4 digits + 1 comma → width 5
        assert_eq!(get_string_length_of_max_file_size(1234), 5);
    }

    #[test]
    fn format_with_commas_basic() {
        assert_eq!(format_with_commas(0), "0");
        assert_eq!(format_with_commas(1), "1");
        assert_eq!(format_with_commas(123), "123");
        assert_eq!(format_with_commas(1234), "1,234");
        assert_eq!(format_with_commas(1234567), "1,234,567");
    }

    #[test]
    fn directory_level_equality() {
        assert_eq!(DirectoryLevel::Initial, DirectoryLevel::Initial);
        assert_ne!(DirectoryLevel::Initial, DirectoryLevel::Subdirectory);
    }
}
