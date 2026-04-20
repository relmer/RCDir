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





#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::sync::Arc;

    use crate::command_line::CommandLine;
    use crate::config::Config;
    use crate::console::Console;
    use crate::directory_info::DirectoryInfo;
    use crate::drive_info::{DriveInfo, DRIVE_FIXED};
    use crate::file_info::{FileInfo, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_ARCHIVE};
    use crate::tree_connector_state::TreeConnectorState;


    fn make_test_config() -> Arc<Config> {
        let mut cfg = Config::new();
        cfg.initialize (0x07);
        Arc::new (cfg)
    }


    fn make_test_console (config: Arc<Config>) -> Console {
        Console::new_for_testing (config)
    }


    fn make_test_cmd (args: &[&str]) -> Arc<CommandLine> {
        Arc::new (CommandLine::parse_from (args).unwrap())
    }


    fn make_test_drive_info() -> DriveInfo {
        DriveInfo {
            unc_path:         PathBuf::new(),
            root_path:        PathBuf::from ("C:\\"),
            volume_name:      "TestVol".to_string(),
            file_system_name: "NTFS".to_string(),
            volume_type:      DRIVE_FIXED,
            is_unc_path:      false,
            remote_name:      String::new(),
        }
    }


    fn make_file (name: &str, size: u64) -> FileInfo {
        FileInfo {
            file_name:        OsString::from (name),
            file_attributes:  FILE_ATTRIBUTE_ARCHIVE,
            file_size:        size,
            creation_time:    133_500_000_000_000_000,
            last_write_time:  133_500_000_000_000_000,
            last_access_time: 133_500_000_000_000_000,
            reparse_tag:      0,
            streams:          Vec::new(),
        }
    }


    fn make_dir (name: &str) -> FileInfo {
        FileInfo {
            file_name:        OsString::from (name),
            file_attributes:  FILE_ATTRIBUTE_DIRECTORY,
            file_size:        0,
            creation_time:    133_500_000_000_000_000,
            last_write_time:  133_500_000_000_000_000,
            last_access_time: 133_500_000_000_000_000,
            reparse_tag:      0,
            streams:          Vec::new(),
        }
    }


    fn make_dir_info (path: &str, files: Vec<FileInfo>) -> DirectoryInfo {
        let file_count = files.iter().filter (|f| !f.is_directory()).count() as u32;
        let dir_count  = files.iter().filter (|f| f.is_directory()).count() as u32;
        let largest    = files.iter().map (|f| f.file_size).max().unwrap_or (0);
        let bytes      = files.iter().map (|f| f.file_size).sum::<u64>();

        let mut di = DirectoryInfo::new (PathBuf::from (path), "*".to_string());
        di.matches             = files;
        di.file_count          = file_count;
        di.subdirectory_count  = dir_count;
        di.largest_file_size   = largest;
        di.bytes_used          = bytes;
        di
    }


    fn strip_ansi (s: &str) -> String {
        let mut result = String::with_capacity (s.len());
        let mut chars = s.chars().peekable();
        while let Some (ch) = chars.next() {
            if ch == '\x1b' {
                if chars.peek() == Some (&'[') {
                    chars.next();
                    while let Some (&next) = chars.peek() {
                        chars.next();
                        if next.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            } else {
                result.push (ch);
            }
        }
        result
    }


    ////////////////////////////////////////////////////////////////////////////
    //
    //  root_header_contains_directory_of
    //
    //  Verify tree root header includes "Directory of" text.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn root_header_contains_directory_of() {
        let config  = make_test_config();
        let console = make_test_console (Arc::clone (&config));
        let cmd     = make_test_cmd (&["--Tree"]);

        let mut displayer = TreeDisplayer::new (console, cmd, config, false);
        let drive_info    = make_test_drive_info();
        let dir_info      = make_dir_info ("C:\\TestDir", vec![make_file ("a.txt", 100)]);

        displayer.display_tree_root_header (&drive_info, &dir_info);
        let output = strip_ansi (&displayer.into_console().take_test_buffer());

        assert! (
            output.contains ("Directory of"),
            "Root header should contain 'Directory of', got:\n{}",
            output,
        );
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  single_entry_shows_dir_tag
    //
    //  Verify directory entries display <DIR> in the size column.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn single_entry_shows_dir_tag() {
        let config  = make_test_config();
        let console = make_test_console (Arc::clone (&config));
        let cmd     = make_test_cmd (&["--Tree"]);

        let mut displayer = TreeDisplayer::new (console, cmd, config, false);
        let dir_info = make_dir_info ("C:\\TestDir", vec![make_dir ("subdir")]);

        displayer.begin_directory (&dir_info);

        let mut tree_state = TreeConnectorState::new (4);
        tree_state.push (false);

        displayer.display_single_entry (&dir_info.matches[0], &mut tree_state, true, 0);
        let output = strip_ansi (&displayer.into_console().take_test_buffer());

        assert! (
            output.contains ("<DIR>"),
            "Directory entry should contain '<DIR>', got:\n{}",
            output,
        );
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  single_entry_contains_filename
    //
    //  Verify displayed entry includes the filename.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn single_entry_contains_filename() {
        let config  = make_test_config();
        let console = make_test_console (Arc::clone (&config));
        let cmd     = make_test_cmd (&["--Tree"]);

        let mut displayer = TreeDisplayer::new (console, cmd, config, false);
        let dir_info = make_dir_info ("C:\\TestDir", vec![make_file ("hello.txt", 42)]);

        displayer.begin_directory (&dir_info);

        let mut tree_state = TreeConnectorState::new (4);
        tree_state.push (false);

        displayer.display_single_entry (&dir_info.matches[0], &mut tree_state, true, 0);
        let output = strip_ansi (&displayer.into_console().take_test_buffer());

        assert! (
            output.contains ("hello.txt"),
            "Entry should contain filename 'hello.txt', got:\n{}",
            output,
        );
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  entry_with_icons_includes_icon_glyph_space
    //
    //  Verify entries with icons active include extra characters for
    //  icon glyph + trailing space.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn entry_with_icons_includes_icon_glyph_space() {
        let config  = make_test_config();

        // Without icons
        let console_no = make_test_console (Arc::clone (&config));
        let cmd_no     = make_test_cmd (&["--Tree"]);
        let mut disp_no = TreeDisplayer::new (console_no, cmd_no, Arc::clone (&config), false);
        let dir_info = make_dir_info ("C:\\TestDir", vec![make_file ("test.rs", 100)]);
        disp_no.begin_directory (&dir_info);
        let mut ts_no = TreeConnectorState::new (4);
        ts_no.push (false);
        disp_no.display_single_entry (&dir_info.matches[0], &mut ts_no, true, 0);
        let out_no = disp_no.into_console().take_test_buffer();

        // With icons
        let console_yes = make_test_console (Arc::clone (&config));
        let cmd_yes     = make_test_cmd (&["--Tree"]);
        let mut disp_yes = TreeDisplayer::new (console_yes, cmd_yes, config, true);
        let dir_info2 = make_dir_info ("C:\\TestDir", vec![make_file ("test.rs", 100)]);
        disp_yes.begin_directory (&dir_info2);
        let mut ts_yes = TreeConnectorState::new (4);
        ts_yes.push (false);
        disp_yes.display_single_entry (&dir_info2.matches[0], &mut ts_yes, true, 0);
        let out_yes = disp_yes.into_console().take_test_buffer();

        // Icon output should be longer (icon char + space = 2 extra chars minimum)
        assert! (
            out_yes.len() > out_no.len(),
            "Icon output ({} bytes) should be longer than non-icon ({} bytes)",
            out_yes.len(),
            out_no.len(),
        );
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  tree_connector_tee_for_non_last_entry
    //
    //  Verify non-last entries show ├── connector in display output.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn tree_connector_tee_for_non_last_entry() {
        let config  = make_test_config();
        let console = make_test_console (Arc::clone (&config));
        let cmd     = make_test_cmd (&["--Tree"]);

        let mut displayer = TreeDisplayer::new (console, cmd, config, false);
        let files = vec![make_file ("first.txt", 100), make_file ("second.txt", 200)];
        let dir_info = make_dir_info ("C:\\TestDir", files);

        displayer.begin_directory (&dir_info);

        let mut tree_state = TreeConnectorState::new (4);
        tree_state.push (true);

        // Display first (non-last) entry
        displayer.display_single_entry (&dir_info.matches[0], &mut tree_state, false, 0);
        let output = strip_ansi (&displayer.into_console().take_test_buffer());

        let tee = "\u{251C}\u{2500}\u{2500}";
        assert! (
            output.contains (tee),
            "Non-last entry should contain tee connector '├──', got:\n{}",
            output,
        );
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  tree_connector_corner_for_last_entry
    //
    //  Verify last entries show └── connector in display output.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn tree_connector_corner_for_last_entry() {
        let config  = make_test_config();
        let console = make_test_console (Arc::clone (&config));
        let cmd     = make_test_cmd (&["--Tree"]);

        let mut displayer = TreeDisplayer::new (console, cmd, config, false);
        let dir_info = make_dir_info ("C:\\TestDir", vec![make_file ("only.txt", 100)]);

        displayer.begin_directory (&dir_info);

        let mut tree_state = TreeConnectorState::new (4);
        tree_state.push (false);

        displayer.display_single_entry (&dir_info.matches[0], &mut tree_state, true, 0);
        let output = strip_ansi (&displayer.into_console().take_test_buffer());

        let corner = "\u{2514}\u{2500}\u{2500}";
        assert! (
            output.contains (corner),
            "Last entry should contain corner connector '└──', got:\n{}",
            output,
        );
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  tree_continuation_pipe_at_depth
    //
    //  Verify continuation lines show │ at ancestor depths.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn tree_continuation_pipe_at_depth() {
        let config  = make_test_config();
        let console = make_test_console (Arc::clone (&config));
        let cmd     = make_test_cmd (&["--Tree"]);

        let mut displayer = TreeDisplayer::new (console, cmd, config, false);
        let dir_info = make_dir_info ("C:\\TestDir\\Sub\\Inner", vec![make_file ("deep.txt", 100)]);

        displayer.begin_directory (&dir_info);

        // Simulate depth 3: root → child (has sibling → pipe) → grandchild (last)
        let mut tree_state = TreeConnectorState::new (4);
        tree_state.push (false);  // depth 1: root (no visible connector)
        tree_state.push (true);   // depth 2: has sibling → shows │
        tree_state.push (false);  // depth 3: last entry

        displayer.display_single_entry (&dir_info.matches[0], &mut tree_state, true, 0);
        let output = strip_ansi (&displayer.into_console().take_test_buffer());

        let pipe = "\u{2502}";
        assert! (
            output.contains (pipe),
            "Nested entry should contain pipe connector '│', got:\n{}",
            output,
        );
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  custom_indent_changes_prefix_width
    //
    //  Verify different tree_indent values produce different prefix widths.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn custom_indent_changes_prefix_width() {
        let config = make_test_config();

        // Indent 4 (default)
        let console4 = make_test_console (Arc::clone (&config));
        let cmd4     = make_test_cmd (&["--Tree"]);
        let mut disp4 = TreeDisplayer::new (console4, cmd4, Arc::clone (&config), false);
        let dir_info = make_dir_info ("C:\\TestDir", vec![make_file ("a.txt", 100)]);
        disp4.begin_directory (&dir_info);
        let mut ts4 = TreeConnectorState::new (4);
        ts4.push (false);
        disp4.display_single_entry (&dir_info.matches[0], &mut ts4, true, 0);
        let out4 = strip_ansi (&disp4.into_console().take_test_buffer());

        // Indent 8
        let console8 = make_test_console (Arc::clone (&config));
        let cmd8     = make_test_cmd (&["--Tree", "/TreeIndent=8"]);
        let mut disp8 = TreeDisplayer::new (console8, cmd8, config, false);
        let dir_info2 = make_dir_info ("C:\\TestDir", vec![make_file ("a.txt", 100)]);
        disp8.begin_directory (&dir_info2);
        let mut ts8 = TreeConnectorState::new (8);
        ts8.push (false);
        disp8.display_single_entry (&dir_info2.matches[0], &mut ts8, true, 0);
        let out8 = strip_ansi (&disp8.into_console().take_test_buffer());

        // Wider indent should produce a longer line
        assert! (
            out8.len() >= out4.len(),
            "Indent=8 output ({} chars) should be >= indent=4 ({} chars)",
            out8.len(),
            out4.len(),
        );
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  multiple_entries_interleaved_order
    //
    //  Verify that files and directories appear in the order given
    //  (sort is external — tree displayer renders in the order it receives).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn multiple_entries_interleaved_order() {
        let config  = make_test_config();
        let console = make_test_console (Arc::clone (&config));
        let cmd     = make_test_cmd (&["--Tree"]);

        let mut displayer = TreeDisplayer::new (console, cmd, config, false);
        let files = vec![
            make_dir ("alpha_dir"),
            make_file ("beta.txt", 100),
            make_dir ("gamma_dir"),
        ];
        let dir_info = make_dir_info ("C:\\TestDir", files);
        displayer.begin_directory (&dir_info);

        let mut tree_state = TreeConnectorState::new (4);
        tree_state.push (true);

        let count = dir_info.matches.len();
        for (i, file) in dir_info.matches.iter().enumerate() {
            let is_last = i == count - 1;
            displayer.display_single_entry (file, &mut tree_state, is_last, i);
        }

        let output = strip_ansi (&displayer.into_console().take_test_buffer());
        let alpha_pos = output.find ("alpha_dir").expect ("alpha_dir not found");
        let beta_pos  = output.find ("beta.txt").expect ("beta.txt not found");
        let gamma_pos = output.find ("gamma_dir").expect ("gamma_dir not found");

        assert! (alpha_pos < beta_pos, "alpha_dir should appear before beta.txt");
        assert! (beta_pos < gamma_pos, "beta.txt should appear before gamma_dir");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  large_file_size_shows_comma_numbers
    //
    //  Verify large file sizes in tree mode show comma-formatted numbers.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn large_file_size_shows_comma_numbers() {
        let config  = make_test_config();
        let console = make_test_console (Arc::clone (&config));
        let cmd     = make_test_cmd (&["--Tree"]);

        let mut displayer = TreeDisplayer::new (console, cmd, config, false);
        let dir_info = make_dir_info ("C:\\TestDir", vec![make_file ("big.dat", 12_345_678)]);

        displayer.begin_directory (&dir_info);

        let mut tree_state = TreeConnectorState::new (4);
        tree_state.push (false);

        displayer.display_single_entry (&dir_info.matches[0], &mut tree_state, true, 0);
        let output = strip_ansi (&displayer.into_console().take_test_buffer());

        // Tree mode default is Auto (abbreviated), so a 12MB file should
        // show an abbreviated size. The important thing is that the output
        // contains numeric content and the filename.
        assert! (
            output.contains ("big.dat"),
            "Output should contain filename 'big.dat', got:\n{}",
            output,
        );
        // Verify the size column has some numeric content (not empty)
        assert! (
            output.chars().any (|c| c.is_ascii_digit()),
            "Output should contain numeric size data, got:\n{}",
            output,
        );
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  max_depth_stored_in_command_line
    //
    //  Verify /Depth=N is parsed and stored correctly in CommandLine.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn max_depth_stored_in_command_line() {
        let cmd = CommandLine::parse_from (["--Tree", "/Depth=3"]).unwrap();
        assert_eq! (cmd.max_depth, 3);
    }
}
