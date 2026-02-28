// common.rs — Shared display helpers used by Normal, Wide, and Bare displayers
//
// Port of: CResultsDisplayerWithHeaderAndFooter (header/footer/summary methods)

use crate::cloud_status::CloudStatus;
use crate::config::{Attribute, Config};
use crate::console::Console;
use crate::directory_info::DirectoryInfo;
use crate::drive_info::DriveInfo;
use crate::listing_totals::ListingTotals;





////////////////////////////////////////////////////////////////////////////////
//
//  display_drive_header
//
//  Display drive/volume header.
//  Port of: CResultsDisplayerWithHeaderAndFooter::DisplayDriveHeader
//
////////////////////////////////////////////////////////////////////////////////

pub(super) fn display_drive_header(console: &mut Console, drive_info: &DriveInfo) {
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

    console.color_printf(&format!(
        "{{Information}} ({{InformationHighlight}}{}{{Information}})\n",
        drive_info.file_system_name,
    ));

    // Volume name (second line)
    if !drive_info.volume_name.is_empty() {
        console.color_printf(&format!(
            "{{Information}} Volume name is \"{{InformationHighlight}}{}{{Information}}\"\n",
            drive_info.volume_name,
        ));
    } else {
        console.color_puts("{Information} Volume has no name");
    }

    console.color_puts("");
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_path_header
//
//  Display "Directory of <path>" header.
//  Port of: CResultsDisplayerWithHeaderAndFooter::DisplayPathHeader
//
////////////////////////////////////////////////////////////////////////////////

pub(super) fn display_path_header(console: &mut Console, dir_info: &DirectoryInfo) {
    console.color_printf(&format!(
        "{{Information}} Directory of {{InformationHighlight}}{}{{Information}}\n\n",
        dir_info.dir_path.display(),
    ));
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_empty_directory_message
//
//  Display message when directory has no matches.
//  Port of: CResultsDisplayerWithHeaderAndFooter::DisplayEmptyDirectoryMessage
//
////////////////////////////////////////////////////////////////////////////////

pub(super) fn display_empty_directory_message(console: &mut Console, dir_info: &DirectoryInfo) {
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





////////////////////////////////////////////////////////////////////////////////
//
//  display_cloud_status_symbol
//
//  Display cloud status symbol with configured color.
//  Port of: CResultsDisplayerNormal::DisplayCloudStatusSymbol
//
////////////////////////////////////////////////////////////////////////////////

pub(super) fn display_cloud_status_symbol(console: &mut Console, config: &Config, status: CloudStatus, icons_active: bool) {
    let attr = match status {
        CloudStatus::None      => Attribute::Default,
        CloudStatus::CloudOnly => Attribute::CloudStatusCloudOnly,
        CloudStatus::Local     => Attribute::CloudStatusLocallyAvailable,
        CloudStatus::Pinned    => Attribute::CloudStatusAlwaysLocallyAvailable,
    };
    let color = config.attributes[attr as usize];

    if icons_active {
        // NF glyph path — leading space + 2-col icon + trailing space (4 visual cols)
        if let Some (icon) = config.get_cloud_status_icon (status) {
            console.writef (color, format_args! (" {} ", icon));
        } else {
            console.printf (config.attributes[Attribute::Default as usize], "    ");
        }
    } else {
        // Unicode circle path — leading space + symbol + trailing space (3 chars)
        let symbol = status.symbol();
        console.writef (color, format_args! (" {} ", symbol));
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_directory_summary
//
//  Display summary line: "X dirs, Y files using Z bytes".
//  Port of: CResultsDisplayerWithHeaderAndFooter::DisplayDirectorySummary
//
////////////////////////////////////////////////////////////////////////////////

pub(super) fn display_directory_summary(console: &mut Console, di: &DirectoryInfo) {
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





////////////////////////////////////////////////////////////////////////////////
//
//  display_volume_footer
//
//  Display free space on volume.
//  Port of: CResultsDisplayerWithHeaderAndFooter::DisplayVolumeFooter
//
////////////////////////////////////////////////////////////////////////////////

pub(super) fn display_volume_footer(console: &mut Console, di: &DirectoryInfo) {
    let dir_wide: Vec<u16> = std::os::windows::ffi::OsStrExt::encode_wide(di.dir_path.as_os_str())
        .chain(Some(0)).collect();

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





////////////////////////////////////////////////////////////////////////////////
//
//  display_footer_quota_info
//
//  Display quota-limited free space info.
//  Port of: CResultsDisplayerWithHeaderAndFooter::DisplayFooterQuotaInfo
//
////////////////////////////////////////////////////////////////////////////////

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





////////////////////////////////////////////////////////////////////////////////
//
//  display_listing_summary
//
//  Display full recursive summary.
//  Port of: CResultsDisplayerWithHeaderAndFooter::DisplayListingSummary
//
////////////////////////////////////////////////////////////////////////////////

pub(super) fn display_listing_summary(console: &mut Console, di: &DirectoryInfo, totals: &ListingTotals) {
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





////////////////////////////////////////////////////////////////////////////////
//
//  get_string_length_of_max_file_size
//
//  Calculate the string length of the largest file size with thousands
//  separators.
//  Port of: CResultsDisplayerWithHeaderAndFooter::GetStringLengthOfMaxFileSize
//
////////////////////////////////////////////////////////////////////////////////

pub(super) fn get_string_length_of_max_file_size(largest: u64) -> usize {
    if largest == 0 {
        return 1;
    }
    let digits = (largest as f64).log10() as usize + 1;
    digits + (digits - 1) / 3 // Add space for comma separators
}





////////////////////////////////////////////////////////////////////////////////
//
//  format_number_with_separators
//
//  Format a number with locale-aware thousands separators.
//  Uses Win32 GetNumberFormatEx for locale-aware formatting.
//  Port of: CResultsDisplayerWithHeaderAndFooter::FormatNumberWithSeparators
//
////////////////////////////////////////////////////////////////////////////////

pub fn format_number_with_separators(n: u64) -> String {
    // Format the number as a simple string first
    let num_str = n.to_string();

    // Use Win32 GetNumberFormatEx for locale-aware formatting
    let num_wide: Vec<u16> = num_str.encode_utf16().chain(Some(0)).collect();
    let mut out_buf = [0u16; 40]; // Enough for any u64 with separators

    // NUMBERFMTW with 0 decimal digits
    // Use proper UTF-16 arrays — the Win32 API expects PWSTR (wide string
    // pointers), not C-string pointers reinterpreted as u16.
    let decimal_sep:  [u16; 2] = [b'.' as u16, 0];
    let thousand_sep: [u16; 2] = [b',' as u16, 0];

    let fmt = windows::Win32::Globalization::NUMBERFMTW {
        NumDigits:     0,
        LeadingZero:   0,
        Grouping:      3,
        lpDecimalSep:  windows::core::PWSTR(decimal_sep.as_ptr() as *mut u16),
        lpThousandSep: windows::core::PWSTR(thousand_sep.as_ptr() as *mut u16),
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





////////////////////////////////////////////////////////////////////////////////
//
//  format_with_commas
//
//  Fallback: format a number with commas as thousands separator.
//
////////////////////////////////////////////////////////////////////////////////

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





////////////////////////////////////////////////////////////////////////////////
//
//  format_abbreviated_size
//
//  Explorer-style abbreviated file size: 1024-based division with 3
//  significant digits and a fixed 7-character width.  The numeric
//  portion is right-justified in a 4-character field, followed by a
//  space separator, followed by the unit label left-justified in a
//  2-character field.  This ensures numbers and suffixes each align
//  in their own sub-column.
//
//  Range                 Format       Example
//  0                     0 B          "   0 B "
//  1-999                 ### B        " 426 B "
//  1000-1023             1 KB         "   1 KB"  (Explorer rounding)
//  1024-10239            X.XX KB      "4.61 KB"
//  10240-102399          XX.X KB      "17.1 KB"
//  102400-1048575        ### KB       " 976 KB"
//  1 MB+                 same 3-sig   "16.7 MB"
//  1 GB+                 same         "1.39 GB"
//  1 TB+                 same         "1.00 TB"
//
//  Port of: CResultsDisplayerNormal::FormatAbbreviatedSize
//
////////////////////////////////////////////////////////////////////////////////

pub fn format_abbreviated_size (cb_size: u64) -> String {

    static SUFFIXES: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB", "EB"];



    // Bytes range: 0-999 displayed as integer bytes
    if cb_size < 1000 {
        return format! ("{:>4} {:<2}", cb_size, "B");
    }

    // 1000-1023 bytes: Explorer shows "1 KB" (rounds up)
    if cb_size < 1024 {
        return "   1 KB".to_string();
    }

    // Divide by 1024 repeatedly until value fits in 3 significant digits.
    let mut value     = cb_size as f64;
    let mut idx_suffix = 0usize;

    while value >= 1024.0 && idx_suffix + 1 < SUFFIXES.len() {
        value /= 1024.0;
        idx_suffix += 1;
    }

    // Three-significant-digit formatting:
    //   <10    → X.XX  (e.g., "4.61 KB")
    //   <100   → XX.X  (e.g., "17.1 KB")
    //   >=100  → ###   (e.g., " 976 KB")
    let suffix = SUFFIXES[idx_suffix];

    if value < 10.0 {
        format! ("{:>4.2} {:<2}", value, suffix)
    } else if value < 100.0 {
        format! ("{:>4.1} {:<2}", value, suffix)
    } else {
        format! ("{:>4.0} {:<2}", value, suffix)
    }
}





#[cfg(test)]
mod tests {
    use super::*;


    ////////////////////////////////////////////////////////////////////////////
    //
    //  format_number_zero
    //
    //  Test formatting of zero.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn format_number_zero() {
        let result = format_number_with_separators(0);
        assert_eq!(result, "0");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  format_number_small
    //
    //  Test formatting of a small number.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn format_number_small() {
        let result = format_number_with_separators(42);
        assert_eq!(result, "42");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  format_number_thousands
    //
    //  Test formatting with thousands separator.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn format_number_thousands() {
        let result = format_number_with_separators(1234);
        assert_eq!(result, "1,234");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  format_number_millions
    //
    //  Test formatting with millions separator.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn format_number_millions() {
        let result = format_number_with_separators(1234567);
        assert_eq!(result, "1,234,567");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  max_file_size_width_zero
    //
    //  Test width calculation for zero.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn max_file_size_width_zero() {
        assert_eq!(get_string_length_of_max_file_size(0), 1);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  max_file_size_width_small
    //
    //  Test width calculation for a small number.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn max_file_size_width_small() {
        // 999 → 3 digits, no commas → width 3
        assert_eq!(get_string_length_of_max_file_size(999), 3);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  max_file_size_width_thousands
    //
    //  Test width calculation with thousands separator.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn max_file_size_width_thousands() {
        // 1234 → 4 digits + 1 comma → width 5
        assert_eq!(get_string_length_of_max_file_size(1234), 5);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  format_with_commas_basic
    //
    //  Test fallback comma formatting.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn format_with_commas_basic() {
        assert_eq!(format_with_commas(0), "0");
        assert_eq!(format_with_commas(1), "1");
        assert_eq!(format_with_commas(123), "123");
        assert_eq!(format_with_commas(1234), "1,234");
        assert_eq!(format_with_commas(1234567), "1,234,567");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  directory_level_equality
    //
    //  Test DirectoryLevel equality and inequality.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn directory_level_equality() {
        use super::super::DirectoryLevel;
        assert_eq!(DirectoryLevel::Initial, DirectoryLevel::Initial);
        assert_ne!(DirectoryLevel::Initial, DirectoryLevel::Subdirectory);
    }





    // =========================================================================
    //  Abbreviated size formatter tests (T024)
    // =========================================================================

    ////////////////////////////////////////////////////////////////////////////
    //
    //  abbreviated_size_zero
    //
    //  Zero bytes displays as "   0 B ".
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn abbreviated_size_zero () {
        assert_eq! (format_abbreviated_size (0), "   0 B ");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  abbreviated_size_small_bytes
    //
    //  Values 1-999 display as right-justified integer bytes.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn abbreviated_size_small_bytes () {
        assert_eq! (format_abbreviated_size (1),   "   1 B ");
        assert_eq! (format_abbreviated_size (426), " 426 B ");
        assert_eq! (format_abbreviated_size (999), " 999 B ");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  abbreviated_size_1000_rounds_to_kb
    //
    //  Values 1000-1023 display as "   1 KB" (Explorer rounding).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn abbreviated_size_1000_rounds_to_kb () {
        assert_eq! (format_abbreviated_size (1000), "   1 KB");
        assert_eq! (format_abbreviated_size (1023), "   1 KB");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  abbreviated_size_1kb
    //
    //  Exactly 1024 bytes → "1.00 KB".
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn abbreviated_size_1kb () {
        assert_eq! (format_abbreviated_size (1024), "1.00 KB");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  abbreviated_size_fractional_kb
    //
    //  4720 bytes → 4.609375 KB → "4.61 KB".
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn abbreviated_size_fractional_kb () {
        assert_eq! (format_abbreviated_size (4720), "4.61 KB");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  abbreviated_size_tens_kb
    //
    //  17510 bytes → 17.099... KB → "17.1 KB".
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn abbreviated_size_tens_kb () {
        assert_eq! (format_abbreviated_size (17510), "17.1 KB");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  abbreviated_size_hundreds_kb
    //
    //  999424 bytes → 976 KB → " 976 KB".
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn abbreviated_size_hundreds_kb () {
        assert_eq! (format_abbreviated_size (999424), " 976 KB");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  abbreviated_size_1mb
    //
    //  1048576 bytes → exactly 1 MB → "1.00 MB".
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn abbreviated_size_1mb () {
        assert_eq! (format_abbreviated_size (1_048_576), "1.00 MB");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  abbreviated_size_tens_mb
    //
    //  17563648 bytes → 16.75 MB → "16.8 MB".
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn abbreviated_size_tens_mb () {
        assert_eq! (format_abbreviated_size (17_563_648), "16.8 MB");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  abbreviated_size_1gb
    //
    //  1073741824 bytes → exactly 1 GB → "1.00 GB".
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn abbreviated_size_1gb () {
        assert_eq! (format_abbreviated_size (1_073_741_824), "1.00 GB");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  abbreviated_size_fractional_gb
    //
    //  1493172224 bytes → 1.39... GB → "1.39 GB".
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn abbreviated_size_fractional_gb () {
        assert_eq! (format_abbreviated_size (1_493_172_224), "1.39 GB");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  abbreviated_size_1tb
    //
    //  1099511627776 bytes → exactly 1 TB → "1.00 TB".
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn abbreviated_size_1tb () {
        assert_eq! (format_abbreviated_size (1_099_511_627_776), "1.00 TB");
    }
}
