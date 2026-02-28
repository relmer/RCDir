// tree.rs — Tree format displayer: hierarchical directory listing with
//           Unicode box-drawing connectors
//
// Port of: CResultsDisplayerTree (TCDir)
//
// Wraps NormalDisplayer via composition, overriding the display flow to
// prepend tree connectors before the filename column.  The MT lister
// drives the interleaved display (each entry is printed immediately
// followed by its children when it is a directory).

use std::sync::Arc;

use crate::cloud_status;
use crate::command_line::CommandLine;
use crate::config::{Attribute, Config};
use crate::console::Console;
use crate::directory_info::DirectoryInfo;
use crate::drive_info::DriveInfo;
use crate::file_info::FileInfo;
use crate::listing_totals::ListingTotals;
use crate::owner;
use crate::tree_connector_state::TreeConnectorState;

use super::common::{
    display_cloud_status_symbol,
    display_drive_header,
    display_empty_directory_message,
    display_listing_summary,
    display_path_header,
    format_number_with_separators,
    get_string_length_of_max_file_size,
};
use super::normal::{
    display_attributes,
    display_date_and_time,
    display_file_owner,
    display_file_size,
    get_time_field_for_display,
};
#[cfg(debug_assertions)]
use super::normal::display_raw_attributes;
use super::{DirectoryLevel, NormalDisplayer, ResultsDisplayer};





////////////////////////////////////////////////////////////////////////////////

/// Per-directory display state saved/restored around child recursion to
/// preserve column alignment.
///
/// Port of: CResultsDisplayerTree::SDirectoryDisplayState
pub struct DirectoryDisplayState {
    pub largest_file_size_str_len: usize,
    pub in_sync_root:             bool,
    pub owners:                   Vec<String>,
    pub max_owner_len:            usize,
}





////////////////////////////////////////////////////////////////////////////////

/// Tree format displayer — wraps NormalDisplayer via composition.
///
/// Port of: CResultsDisplayerTree (TCDir)
pub struct TreeDisplayer {
    inner:                       NormalDisplayer,
    cmd:                         Arc<CommandLine>,
    config:                      Arc<Config>,
    icons_active:                bool,

    // Per-directory state set by begin_directory()
    largest_file_size_str_len:   usize,
    in_sync_root:                bool,
    owners:                      Vec<String>,
    max_owner_len:               usize,
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
            inner:                     NormalDisplayer::new (console, Arc::clone (&cmd), Arc::clone (&config), icons_active),
            cmd,
            config,
            icons_active,
            largest_file_size_str_len: 0,
            in_sync_root:              false,
            owners:                    Vec::new(),
            max_owner_len:             0,
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





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_tree_root_header
    //
    //  Display drive header and path header for the root directory.
    //  Port of: CResultsDisplayerTree::DisplayTreeRootHeader
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn display_tree_root_header (&mut self, drive_info: &DriveInfo, dir_info: &DirectoryInfo) {
        let console = self.inner.console_mut();
        display_drive_header (console, drive_info);
        display_path_header (console, dir_info);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_tree_empty_root_message
    //
    //  Display empty directory message and trailing blank lines.
    //  Port of: CResultsDisplayerTree::DisplayTreeEmptyRootMessage
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn display_tree_empty_root_message (&mut self, dir_info: &DirectoryInfo) {
        let console = self.inner.console_mut();
        display_empty_directory_message (console, dir_info);
        console.puts (Attribute::Default, "");
        console.puts (Attribute::Default, "");
        let _ = console.flush();
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_tree_root_summary
    //
    //  Display trailing blank lines and flush after tree output completes.
    //  Port of: CResultsDisplayerTree::DisplayTreeRootSummary
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn display_tree_root_summary (&mut self) {
        let console = self.inner.console_mut();
        console.puts (Attribute::Default, "");
        console.puts (Attribute::Default, "");
        let _ = console.flush();
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  begin_directory
    //
    //  Compute per-directory display state (max file size width, sync root
    //  status, owner column data) before rendering entries.
    //  Port of: CResultsDisplayerTree::BeginDirectory
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn begin_directory (&mut self, dir_info: &DirectoryInfo) {
        self.largest_file_size_str_len = get_string_length_of_max_file_size (dir_info.largest_file_size);
        self.in_sync_root             = cloud_status::is_under_sync_root (dir_info.dir_path.as_os_str());
        self.owners.clear();
        self.max_owner_len = 0;

        if self.cmd.show_owner {
            let (owners, max_len) = owner::get_file_owners (dir_info);
            self.owners           = owners;
            self.max_owner_len    = max_len;
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  save_directory_state
    //
    //  Save per-directory display state before recursing into a child.
    //  Port of: CResultsDisplayerTree::SaveDirectoryState
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn save_directory_state (&self) -> DirectoryDisplayState {
        DirectoryDisplayState {
            largest_file_size_str_len: self.largest_file_size_str_len,
            in_sync_root:             self.in_sync_root,
            owners:                   self.owners.clone(),
            max_owner_len:            self.max_owner_len,
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  restore_directory_state
    //
    //  Restore per-directory display state after returning from a child.
    //  Port of: CResultsDisplayerTree::RestoreDirectoryState
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn restore_directory_state (&mut self, state: DirectoryDisplayState) {
        self.largest_file_size_str_len = state.largest_file_size_str_len;
        self.in_sync_root             = state.in_sync_root;
        self.owners                   = state.owners;
        self.max_owner_len            = state.max_owner_len;
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_single_entry
    //
    //  Render one file/directory entry with tree connector prefix.
    //  Column order matches Normal mode but inserts tree prefix between
    //  metadata columns and icon/filename.
    //  Port of: CResultsDisplayerTree::DisplaySingleEntry
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn display_single_entry (
        &mut self,
        file_info: &FileInfo,
        tree_state: &mut TreeConnectorState,
        is_last_entry: bool,
        idx_file: usize,
    ) {
        let console     = self.inner.console_mut();
        let style       = self.config.get_display_style_for_file (file_info);
        let text_attr   = style.text_attr;
        let size_format = self.cmd.resolved_size_format();

        // Date and time
        let time_value = get_time_field_for_display (file_info, self.cmd.time_field);
        display_date_and_time (console, time_value);

        // Attributes
        display_attributes (console, &self.config, file_info.file_attributes);

        // File size or <DIR>
        display_file_size (console, file_info, self.largest_file_size_str_len, size_format);

        // Cloud status symbol
        let cloud = cloud_status::get_cloud_status (file_info.file_attributes, self.in_sync_root);
        display_cloud_status_symbol (console, &self.config, cloud, self.icons_active);

        // Debug attribute display (debug builds only, gated by --debug)
        #[cfg(debug_assertions)]
        if self.cmd.debug {
            display_raw_attributes (console, &self.config, file_info);
        }

        // Owner column (if --owner)
        if let (true, Some(owner_str)) = (self.cmd.show_owner, self.owners.get (idx_file)) {
            display_file_owner (console, &self.config, owner_str, self.max_owner_len);
        }

        // Tree connector prefix (before icon/filename)
        let prefix = tree_state.get_prefix (is_last_entry);
        if !prefix.is_empty() {
            let tree_color = self.config.attributes[Attribute::TreeConnector as usize];
            console.printf (tree_color, &prefix);
        }

        // Icon glyph (when icons are active and not suppressed)
        if self.icons_active {
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

        // Alternate data streams (if --streams and this entry has them)
        if self.cmd.show_streams && !file_info.streams.is_empty() {
            self.display_file_streams_with_tree_prefix (file_info, tree_state);
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_file_streams_with_tree_prefix
    //
    //  Display alternate data streams below a file entry, with tree
    //  continuation prefix characters for proper connector alignment.
    //  Port of: CResultsDisplayerTree::DisplayFileStreamsWithTreePrefix
    //
    ////////////////////////////////////////////////////////////////////////////

    fn display_file_streams_with_tree_prefix (
        &mut self,
        file_info: &FileInfo,
        tree_state: &TreeConnectorState,
    ) {
        let continuation_prefix = tree_state.get_stream_continuation();
        let max_file_size       = self.largest_file_size_str_len.max (5);
        let owner_padding       = if self.max_owner_len > 0 { self.max_owner_len + 1 } else { 0 };
        let file_name           = file_info.file_name.to_string_lossy();

        let console   = self.inner.console_mut();
        let default_c = self.config.attributes[Attribute::Default as usize];
        let size_c    = self.config.attributes[Attribute::Size as usize];
        let stream_c  = self.config.attributes[Attribute::Stream as usize];
        let tree_c    = self.config.attributes[Attribute::TreeConnector as usize];

        // Cloud status gap: leading space + symbol/icon + trailing space
        let cloud_gap = if self.icons_active { "    " } else { "   " };

        for si in &file_info.streams {
            let formatted_size = format_number_with_separators (si.size as u64);

            // 30 chars indentation (date/time 21 + attributes 9)
            console.writef (default_c, format_args! ("{:30}", ""));
            console.writef (size_c, format_args! ("  {:>width$}", formatted_size, width = max_file_size));
            console.writef (default_c, format_args! ("{}  {:width$}", cloud_gap, "", width = owner_padding));

            // Tree continuation prefix
            if !continuation_prefix.is_empty() {
                console.printf (tree_c, &continuation_prefix);
            }

            console.writef_line (stream_c, format_args! ("{}{}", file_name, si.name));
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl ResultsDisplayer for TreeDisplayer
//
//  Tree-format directory listing.  Delegates to inner NormalDisplayer for
//  non-tree paths.  The tree-walking flow is driven externally by
//  MultiThreadedLister::print_directory_tree_mode.
//
////////////////////////////////////////////////////////////////////////////////

impl ResultsDisplayer for TreeDisplayer {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_results
    //
    //  Not normally called in tree mode.  Delegates to inner NormalDisplayer
    //  for safety.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn display_results (&mut self, drive_info: &DriveInfo, dir_info: &DirectoryInfo, level: DirectoryLevel) {
        self.inner.display_results (drive_info, dir_info, level);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_recursive_summary
    //
    //  Display the recursive summary (total files, bytes, dirs).
    //  Delegates to the common listing summary display.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn display_recursive_summary (&mut self, dir_info: &DirectoryInfo, totals: &ListingTotals) {
        let console = self.inner.console_mut();
        display_listing_summary (console, dir_info, totals);
    }
}
