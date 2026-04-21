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
use crate::path_ellipsis::ELLIPSIS;

use super::column_layout::compute_column_layout;
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
            display_wide_file_results (&mut self.console, &self.cmd, &self.config, dir_info, self.icons_active);
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
//  Display files in column-major wide format with variable-width columns.
//  Port of: CResultsDisplayerWide::DisplayFileResults
//
////////////////////////////////////////////////////////////////////////////////

fn display_wide_file_results (console: &mut Console, cmd: &CommandLine, config: &Config, di: &DirectoryInfo, icons_active: bool) {
    if di.largest_file_name == 0 || di.matches.is_empty() {
        return;
    }

    let console_width = console.width() as usize;
    let in_sync_root  = cloud_status::is_under_sync_root (di.dir_path.as_os_str());
    let ellipsize     = cmd.ellipsize.unwrap_or (true);

    // Build per-entry display widths vector (T011).
    // Each entry's width = filename + optional brackets/icon/cloud.

    let display_widths: Vec<usize> = di.matches.iter().map (|fi| {
        let mut w = fi.file_name.to_string_lossy().len();

        // Directory brackets [name] when icons are off
        if fi.is_directory() && !icons_active {
            w += 2;
        }

        // Icon space is always consumed when icons are active (even for
        // suppressed icons, the render loop emits a 2-char placeholder).
        if icons_active {
            w += 2;
        }

        // Cloud status symbol + space
        if in_sync_root {
            w += if icons_active { 4 } else { 3 };
        }

        w
    }).collect();

    // Compute variable-width column layout (T012)

    let layout = compute_column_layout (&display_widths, console_width, ellipsize);

    // Display in column-major order (T013 + T014)

    let total_items       = di.matches.len();
    let items_in_last_row = total_items % layout.columns;

    for row in 0..layout.rows {
        for col in 0..layout.columns {
            if row * layout.columns + col >= total_items {
                break;
            }

            // Column-major index calculation matching TCDir exactly
            let full_rows = if items_in_last_row != 0 { layout.rows - 1 } else { layout.rows };
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
                cch_name += if icons_active { 4 } else { 3 };
            }

            // Icon glyph before filename (when icons are active)
            if icons_active {
                if let Some(icon) = style.icon_code_point {
                    if !style.icon_suppressed {
                        console.writef (text_attr, format_args! ("{} ", icon));
                        cch_name += 2;
                    } else {
                        console.printf (text_attr, "  ");
                        cch_name += 2;
                    }
                } else {
                    console.printf (text_attr, "  ");
                    cch_name += 2;
                }
            }

            // Format filename, with outlier truncation when trunc_cap is active (T014)
            let name = fi.file_name.to_string_lossy();

            if fi.is_directory() && !icons_active {
                // Directory with [brackets]
                let display_len = name.len() + 2;
                let effective_cch = cch_name + display_len;

                if layout.trunc_cap > 0 && effective_cch > layout.trunc_cap {
                    let over = effective_cch - layout.trunc_cap;
                    if over + 1 < display_len {
                        let keep = name.len() - over - 1;
                        console.writef (text_attr, format_args! ("[{}{}", &name[..keep], ELLIPSIS));
                        cch_name = layout.trunc_cap;
                    } else {
                        console.writef (text_attr, format_args! ("[{}]", name));
                        cch_name += display_len;
                    }
                } else {
                    console.writef (text_attr, format_args! ("[{}]", name));
                    cch_name += display_len;
                }
            } else {
                // Plain filename
                let effective_cch = cch_name + name.len();

                if layout.trunc_cap > 0 && effective_cch > layout.trunc_cap {
                    let over = effective_cch - layout.trunc_cap;
                    if over + 1 < name.len() {
                        let keep = name.len() - over - 1;
                        console.writef (text_attr, format_args! ("{}{}", &name[..keep], ELLIPSIS));
                        cch_name = layout.trunc_cap;
                    } else {
                        console.printf (text_attr, &name);
                        cch_name += name.len();
                    }
                } else {
                    console.printf (text_attr, &name);
                    cch_name += name.len();
                }
            }

            // Pad to per-column width; last column gets no trailing space
            let col_width = if col < layout.columns - 1 { layout.column_widths[col] } else { 0 };

            if col_width > cch_name {
                console.writef_attr (Attribute::Default, format_args! (
                    "{:width$}", "", width = col_width - cch_name,
                ));
            }
        }

        console.puts (Attribute::Default, "");
    }
}
