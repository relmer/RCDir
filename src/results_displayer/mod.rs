// results_displayer — Display formatting for directory listings
//
// Port of: IResultsDisplayer.h, ResultsDisplayerNormal.h/.cpp,
//          ResultsDisplayerWithHeaderAndFooter.h/.cpp,
//          ResultsDisplayerWide.h/.cpp,
//          ResultsDisplayerBare.h/.cpp
//
// Provides the ResultsDisplayer trait including NormalDisplayer, WideDisplayer,
// and BareDisplayer implementations, plus a Displayer enum wrapper.
//
// Module structure:
//   mod.rs    — shared types (DirectoryLevel, ResultsDisplayer trait, Displayer enum)
//   common.rs — shared helpers (headers, footers, summaries, number formatting)
//   normal.rs — NormalDisplayer + normal-specific display routines
//   wide.rs   — WideDisplayer + column-major wide display routines
//   bare.rs   — BareDisplayer + bare (filename-only) display

mod bare;
mod common;
mod normal;
mod wide;

use std::sync::Arc;

use crate::command_line::CommandLine;
use crate::config::Config;
use crate::console::Console;
use crate::directory_info::DirectoryInfo;
use crate::drive_info::DriveInfo;
use crate::listing_totals::ListingTotals;

pub use self::bare::BareDisplayer;
pub use self::common::format_number_with_separators;
pub use self::normal::NormalDisplayer;
pub use self::wide::WideDisplayer;





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

    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_results
    //
    //  Display results for a single directory.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn display_results(&mut self, drive_info: &DriveInfo, dir_info: &DirectoryInfo, level: DirectoryLevel);

    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_recursive_summary
    //
    //  Display recursive summary after all directories.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn display_recursive_summary(&mut self, dir_info: &DirectoryInfo, totals: &ListingTotals);
}





////////////////////////////////////////////////////////////////////////////////

/// Polymorphic displayer wrapping Normal, Wide, or Bare variants.
///
/// Provides `into_console()` and `console_mut()` without trait object issues.
pub enum Displayer {
    Normal(NormalDisplayer),
    Wide(WideDisplayer),
    Bare(BareDisplayer),
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl Displayer
//
//  Polymorphic displayer construction and console access.
//
////////////////////////////////////////////////////////////////////////////////

impl Displayer {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  new
    //
    //  Create the appropriate displayer based on command-line switches.
    //  Priority: bare > wide > normal (matching TCDir).
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn new(console: Console, cmd: Arc<CommandLine>, config: Arc<Config>, icons_active: bool) -> Self {
        if cmd.bare_listing {
            Displayer::Bare(BareDisplayer::new(console, cmd, config, icons_active))
        } else if cmd.wide_listing {
            Displayer::Wide(WideDisplayer::new(console, cmd, config, icons_active))
        } else {
            Displayer::Normal(NormalDisplayer::new(console, cmd, config, icons_active))
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  into_console
    //
    //  Consume the displayer and return the Console.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn into_console(self) -> Console {
        match self {
            Displayer::Normal(d) => d.into_console(),
            Displayer::Wide(d)   => d.into_console(),
            Displayer::Bare(d)   => d.into_console(),
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  console_mut
    //
    //  Get a mutable reference to the console.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn console_mut(&mut self) -> &mut Console {
        match self {
            Displayer::Normal(d) => d.console_mut(),
            Displayer::Wide(d)   => d.console_mut(),
            Displayer::Bare(d)   => d.console_mut(),
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl ResultsDisplayer for Displayer
//
//  Dispatch to the underlying Normal, Wide, or Bare displayer variant.
//
////////////////////////////////////////////////////////////////////////////////

impl ResultsDisplayer for Displayer {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_results
    //
    //  Dispatch display_results to the underlying displayer variant.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn display_results(&mut self, drive_info: &DriveInfo, dir_info: &DirectoryInfo, level: DirectoryLevel) {
        match self {
            Displayer::Normal(d) => d.display_results(drive_info, dir_info, level),
            Displayer::Wide(d)   => d.display_results(drive_info, dir_info, level),
            Displayer::Bare(d)   => d.display_results(drive_info, dir_info, level),
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_recursive_summary
    //
    //  Dispatch display_recursive_summary to the underlying displayer variant.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn display_recursive_summary(&mut self, dir_info: &DirectoryInfo, totals: &ListingTotals) {
        match self {
            Displayer::Normal(d) => d.display_recursive_summary(dir_info, totals),
            Displayer::Wide(d)   => d.display_recursive_summary(dir_info, totals),
            Displayer::Bare(d)   => d.display_recursive_summary(dir_info, totals),
        }
    }
}
