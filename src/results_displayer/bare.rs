// bare.rs — Bare format displayer: filenames only, no decoration
//
// Port of: CResultsDisplayerBare

use std::sync::Arc;

use crate::command_line::CommandLine;
use crate::config::{Attribute, Config};
use crate::console::Console;
use crate::directory_info::DirectoryInfo;
use crate::drive_info::DriveInfo;
use crate::listing_totals::ListingTotals;

use super::{DirectoryLevel, ResultsDisplayer};





////////////////////////////////////////////////////////////////////////////////

/// Bare format displayer — filenames only, no decoration.
///
/// Port of: CResultsDisplayerBare
pub struct BareDisplayer {
    console:      Console,
    cmd:          Arc<CommandLine>,
    config:       Arc<Config>,
    icons_active: bool,
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl BareDisplayer
//
//  Bare displayer construction and console access.
//
////////////////////////////////////////////////////////////////////////////////

impl BareDisplayer {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  new
    //
    //  Create a new BareDisplayer.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn new(console: Console, cmd: Arc<CommandLine>, config: Arc<Config>, icons_active: bool) -> Self {
        BareDisplayer { console, cmd, config, icons_active }
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
//  impl ResultsDisplayer for BareDisplayer
//
//  Bare-format directory listing and recursive summary.
//
////////////////////////////////////////////////////////////////////////////////

impl ResultsDisplayer for BareDisplayer {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_results
    //
    //  Display results for a single directory using bare format.
    //  Port of: CResultsDisplayerBare::DisplayResults
    //
    ////////////////////////////////////////////////////////////////////////////

    fn display_results(&mut self, _drive_info: &DriveInfo, dir_info: &DirectoryInfo, _level: DirectoryLevel) {
        for fi in &dir_info.matches {
            let style = self.config.get_display_style_for_file (fi);
            let text_attr = style.text_attr;

            // Icon glyph before filename (when icons are active)
            if self.icons_active {
                if let Some(icon) = style.icon_code_point {
                    if !style.icon_suppressed {
                        self.console.writef (text_attr, format_args! ("{} ", icon));
                    } else {
                        self.console.printf (text_attr, "  ");
                    }
                } else {
                    self.console.printf (text_attr, "  ");
                }
            }

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





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_recursive_summary
    //
    //  Bare mode doesn't display recursive summary.
    //  Port of: CResultsDisplayerBare::DisplayRecursiveSummary
    //
    ////////////////////////////////////////////////////////////////////////////

    fn display_recursive_summary(&mut self, _dir_info: &DirectoryInfo, _totals: &ListingTotals) {
        // No summary in bare mode
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  console_printf_line
//
//  Helper: Printf a line with color + newline.
//
////////////////////////////////////////////////////////////////////////////////

fn console_printf_line(console: &mut Console, attr: u16, text: &str) {
    console.writef_line (attr, format_args! ("{}", text));
}
