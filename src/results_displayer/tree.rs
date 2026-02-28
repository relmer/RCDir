// tree.rs — Tree format displayer: hierarchical directory listing with
//           Unicode box-drawing connectors
//
// Port of: CResultsDisplayerTree (TCDir)
//
// Wraps NormalDisplayer via composition, overriding the display flow to
// prepend tree connectors before the filename column.

use std::sync::Arc;

use crate::command_line::CommandLine;
use crate::config::Config;
use crate::console::Console;
use crate::directory_info::DirectoryInfo;
use crate::drive_info::DriveInfo;
use crate::listing_totals::ListingTotals;
use crate::tree_connector_state::TreeConnectorState;

use super::normal::NormalDisplayer;
use super::{DirectoryLevel, ResultsDisplayer};





////////////////////////////////////////////////////////////////////////////////

/// Per-directory display state saved/restored around child recursion to
/// preserve column alignment.
pub struct DirectoryDisplayState {
    pub largest_file_size_str_len: usize,
    pub in_sync_root:             bool,
}





////////////////////////////////////////////////////////////////////////////////

/// Tree format displayer — wraps NormalDisplayer via composition.
///
/// Port of: CResultsDisplayerTree (TCDir)
pub struct TreeDisplayer {
    inner: NormalDisplayer,
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl TreeDisplayer
//
//  Tree displayer construction, console access, and tree-specific display
//  methods.
//
////////////////////////////////////////////////////////////////////////////////

impl TreeDisplayer {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  new
    //
    //  Create a new TreeDisplayer wrapping a NormalDisplayer.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn new (console: Console, cmd: Arc<CommandLine>, config: Arc<Config>, icons_active: bool) -> Self {
        TreeDisplayer {
            inner: NormalDisplayer::new (console, cmd, config, icons_active),
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  into_console
    //
    //  Consume the displayer and return the Console for further use.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn into_console (self) -> Console {
        self.inner.into_console()
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  console_mut
    //
    //  Get a mutable reference to the console (for flushing).
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn console_mut (&mut self) -> &mut Console {
        self.inner.console_mut()
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl ResultsDisplayer for TreeDisplayer
//
//  Tree-format directory listing.  Delegates to inner NormalDisplayer for
//  non-tree paths.
//
////////////////////////////////////////////////////////////////////////////////

impl ResultsDisplayer for TreeDisplayer {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_results
    //
    //  Delegates to inner NormalDisplayer for base display.
    //  Tree-walking flow is driven externally by
    //  MultiThreadedLister::print_directory_tree_mode.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn display_results (&mut self, drive_info: &DriveInfo, dir_info: &DirectoryInfo, level: DirectoryLevel) {
        self.inner.display_results (drive_info, dir_info, level);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_recursive_summary
    //
    //  Delegates to inner NormalDisplayer for final summary display.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn display_recursive_summary (&mut self, dir_info: &DirectoryInfo, totals: &ListingTotals) {
        self.inner.display_recursive_summary (dir_info, totals);
    }
}
