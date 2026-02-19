// wide.rs — Multi-column wide format displayer
//
// Port of: CResultsDisplayerWide

use std::sync::Arc;

use crate::cloud_status;
use crate::command_line::CommandLine;
use crate::config::{Attribute, Config};
use crate::console::Console;
use crate::directory_info::DirectoryInfo;
use crate::drive_info::DriveInfo;
use crate::listing_totals::ListingTotals;

use super::common::{
    display_cloud_status_symbol,
    display_directory_summary,
    display_drive_header,
    display_empty_directory_message,
    display_listing_summary,
    display_path_header,
    display_volume_footer,
};
use super::{DirectoryLevel, ResultsDisplayer};





////////////////////////////////////////////////////////////////////////////////

/// Wide format displayer — multi-column filenames with [dir] brackets.
///
/// Port of: CResultsDisplayerWide
pub struct WideDisplayer {
    console:      Console,
    cmd:          Arc<CommandLine>,
    config:       Arc<Config>,
    icons_active: bool,
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl WideDisplayer
//
//  Wide displayer construction and console access.
//
////////////////////////////////////////////////////////////////////////////////

impl WideDisplayer {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  new
    //
    //  Create a new WideDisplayer.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn new(console: Console, cmd: Arc<CommandLine>, config: Arc<Config>, icons_active: bool) -> Self {
        WideDisplayer { console, cmd, config, icons_active }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  into_console
    //
    //  Consume the displayer and return the Console.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn into_console(self) -> Console {
        self.console
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  console_mut
    //
    //  Get a mutable reference to the console.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn console_mut(&mut self) -> &mut Console {
        &mut self.console
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl ResultsDisplayer for WideDisplayer
//
//  Wide-format directory listing and recursive summary.
//
////////////////////////////////////////////////////////////////////////////////

impl ResultsDisplayer for WideDisplayer {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_results
    //
    //  Display results for a single directory using wide format.
    //  Port of: CResultsDisplayerWithHeaderAndFooter::DisplayResults
    //  and CResultsDisplayerWide::DisplayFileResults (column layout).
    //
    ////////////////////////////////////////////////////////////////////////////

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
            display_wide_file_results(&mut self.console, &self.config, dir_info, self.icons_active);
            display_directory_summary(&mut self.console, dir_info);

            if !self.cmd.recurse {
                display_volume_footer(&mut self.console, dir_info);
            }
        }

        self.console.puts(Attribute::Default, "");
        self.console.puts(Attribute::Default, "");

        let _ = self.console.flush();
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_recursive_summary
    //
    //  Display recursive summary after all directories.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn display_recursive_summary(&mut self, dir_info: &DirectoryInfo, totals: &ListingTotals) {
        display_listing_summary(&mut self.console, dir_info, totals);
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_wide_file_results
//
//  Display files in column-major wide format.
//  Port of: CResultsDisplayerWide::DisplayFileResults
//
////////////////////////////////////////////////////////////////////////////////

fn display_wide_file_results(console: &mut Console, config: &Config, di: &DirectoryInfo, icons_active: bool) {
    if di.largest_file_name == 0 || di.matches.is_empty() {
        return;
    }

    let console_width = console.width() as usize;
    let in_sync_root = cloud_status::is_under_sync_root (di.dir_path.as_os_str());

    // Account for brackets on directories (only when icons are NOT active).
    // When icons are active, the folder icon provides the visual distinction.
    let max_name_len = di.matches.iter().map(|fi| {
        let base_len = fi.file_name.to_string_lossy().len();
        if fi.is_directory() && !icons_active { base_len + 2 } else { base_len }
    }).max().unwrap_or(0);

    // When icons are active, account for icon + space (+2) in column width
    let mut adjusted_max = if icons_active { max_name_len + 2 } else { max_name_len };

    // When in sync root, cloud status symbol + space adds +2
    if in_sync_root {
        adjusted_max += 2;
    }

    // Calculate column count and widths — Port of: GetColumnInfo
    let (columns, column_width) = if adjusted_max + 1 > console_width {
        (1, console_width)
    } else {
        let cols = console_width / (adjusted_max + 1);
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
            let style = config.get_display_style_for_file (fi);
            let text_attr = style.text_attr;
            let mut cch_name: usize = 0;

            // Cloud status symbol (when in sync root)
            if in_sync_root {
                let cloud = cloud_status::get_cloud_status (fi.file_attributes, true);
                display_cloud_status_symbol (console, config, cloud, icons_active);
                cch_name += 2;
            }

            // Icon glyph before filename (when icons are active)
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
                cch_name += 2; // icon + space
            }

            // Format filename: [dirname] for dirs (classic mode only), plain name for files
            let name = fi.file_name.to_string_lossy();
            if fi.is_directory() && !icons_active {
                console.writef (text_attr, format_args! ("[{}]", name));
                cch_name += name.len() + 2;
            } else {
                console.printf (text_attr, &name);
                cch_name += name.len();
            }

            // Pad to column width
            if column_width > cch_name {
                console.writef_attr (Attribute::Default, format_args! (
                    "{:width$}", "", width = column_width - cch_name,
                ));
            }
        }

        console.puts(Attribute::Default, "");
    }
}
