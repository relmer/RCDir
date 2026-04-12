// command_line.rs — CLI argument parsing (custom, no clap)
//
// Port of: CommandLine.h, CommandLine.cpp
// Windows-style /switch and -switch prefixes, compound switches (/a:hs, /o:-d),
// long switches (--env, /owner), trailing - disable (/m-).

use std::ffi::OsString;

use crate::config::Config;
use crate::ehm::AppError;





////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Default,    // Unsorted (filesystem order)
    Name,       // /O:N — alphabetic by name
    Extension,  // /O:E — alphabetic by extension
    Size,       // /O:S — smallest first
    Date,       // /O:D — oldest first
}





pub const SORT_ORDER_COUNT: usize = 5;





#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}





#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeField {
    Written,    // /T:W — ftLastWriteTime (default)
    Creation,   // /T:C — ftCreationTime
    Access,     // /T:A — ftLastAccessTime
}





#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeFormat {
    Default,    // Not explicitly set; tree mode uses Auto, non-tree uses Bytes
    Auto,       // Explorer-style abbreviated (1024-based, 3 sig digits, 7-char)
    Bytes,      // Exact byte count with comma separators (existing behavior)
}





////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct CommandLine {
    pub recurse:          bool,
    pub attrs_required:   u32,
    pub attrs_excluded:   u32,
    pub sort_order:       SortOrder,
    pub sort_direction:   SortDirection,
    pub sort_preference:  [SortOrder; SORT_ORDER_COUNT],
    pub masks:            Vec<OsString>,
    pub wide_listing:     bool,
    pub bare_listing:     bool,
    pub perf_timer:       bool,
    pub multi_threaded:   bool,
    pub show_env_help:    bool,
    pub show_config:      bool,
    pub show_settings:    bool,
    pub show_help:        bool,
    pub switch_prefix:    char,
    pub time_field:       TimeField,
    pub show_owner:       bool,
    pub show_streams:     bool,
    pub icons:            Option<bool>,
    pub debug:            bool,
    pub tree:             Option<bool>,
    pub max_depth:        i32,
    pub tree_indent:      i32,
    pub size_format:      SizeFormat,
    pub set_aliases:      bool,
    pub get_aliases:      bool,
    pub remove_aliases:   bool,
    pub what_if:          bool,
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl Default for CommandLine
//
//  Returns default CommandLine values.
//
////////////////////////////////////////////////////////////////////////////////

impl Default for CommandLine {
    fn default() -> Self {
        CommandLine {
            recurse:         false,
            attrs_required:  0,
            attrs_excluded:  0,
            sort_order:      SortOrder::Default,
            sort_direction:  SortDirection::Ascending,
            sort_preference: [
                SortOrder::Default,   // [0] = overwritten with requested sort
                SortOrder::Name,      // [1] = first tiebreaker
                SortOrder::Date,      // [2]
                SortOrder::Extension, // [3]
                SortOrder::Size,      // [4]
            ],
            masks:           Vec::new(),
            wide_listing:    false,
            bare_listing:    false,
            perf_timer:      false,
            multi_threaded:  true,
            show_env_help:   false,
            show_config:     false,
            show_settings:   false,
            show_help:       false,
            switch_prefix:   '-',
            time_field:      TimeField::Written,
            show_owner:      false,
            show_streams:    false,
            icons:           None,
            debug:           false,
            tree:            None,
            max_depth:       0,
            tree_indent:     4,
            size_format:     SizeFormat::Default,
            set_aliases:     false,
            get_aliases:     false,
            remove_aliases:  false,
            what_if:         false,
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl CommandLine
//
//  Command-line argument parsing and switch handling.
//
////////////////////////////////////////////////////////////////////////////////

impl CommandLine {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_from
    //
    //  Parse command-line arguments into a CommandLine struct.
    //  Args should NOT include argv[0] (program name).
    //
    //  Port of: CCommandLine::Parse
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn parse_from<I, S>(args: I) -> Result<Self, AppError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut cmd = CommandLine::default();
        let args: Vec<String> = args.into_iter().map (|s| s.as_ref().to_string()).collect();
        let mut i = 0;

        while i < args.len() {
            let arg = &args[i];
            if arg.is_empty() {
                i += 1;
                continue;
            }

            let first_char = arg.chars().next().unwrap();

            match first_char {
                '-' | '/' => {
                    cmd.switch_prefix = first_char;
                    cmd.parse_switch (arg, first_char, &args, &mut i)?;
                }
                _ => {
                    // Positional argument (file mask)
                    cmd.masks.push (OsString::from (arg.as_str()));
                }
            }

            i += 1;
        }

        cmd.validate_switch_combinations()?;
        Ok(cmd)
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_switch
    //
    //  Strip the switch prefix (-, --, /) from a raw argument, validate
    //  single-dash long switch rejection, and dispatch to handle_switch.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn parse_switch(&mut self, arg: &str, prefix: char, args: &[String], idx: &mut usize) -> Result<(), AppError> {
        let switch_arg;
        let mut is_double_dash = false;

        // Check for '--' prefix
        if prefix == '-' && arg.len() > 1 && arg.as_bytes()[1] == b'-' {
            switch_arg = &arg[2..];
            is_double_dash = true;
        } else {
            switch_arg = &arg[1..];
        }

        // Detect long switch: 3+ chars without ':' or '-' at position 1
        let looks_like_long = switch_arg.len() >= 3
            && switch_arg.as_bytes().get (1) != Some (&b':')
            && switch_arg.as_bytes().get (1) != Some (&b'-');

        // Reject single-dash long switches (e.g., -env) — must use --env
        if looks_like_long && !is_double_dash && prefix == '-' {
            return Err (Self::reject_single_dash_long_switch (switch_arg));
        }

        self.handle_switch (switch_arg, args, idx)
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  reject_single_dash_long_switch
    //
    //  Build a descriptive error for a mis-prefixed long switch.
    //  If the switch name matches a known long switch, includes a
    //  "Did you mean --<name>?" hint.
    //  Port of: CCommandLine::RejectSingleDashLongSwitch
    //
    ////////////////////////////////////////////////////////////////////////////

    fn reject_single_dash_long_switch(switch_arg: &str) -> AppError {
        // Extract just the switch name (before any '=' separator)
        let switch_name = match switch_arg.find ('=') {
            Some (pos) => &switch_arg[..pos],
            None       => switch_arg,
        };

        // Strip trailing '-' (negation suffix) for lookup
        let lookup = switch_name.strip_suffix ('-').unwrap_or (switch_name);

        if Self::is_recognized_long_switch (lookup) {
            AppError::InvalidArg (format! ("Unknown switch: -{switch_name}.  Did you mean --{switch_name}?"))
        } else {
            AppError::InvalidArg (format! ("Unknown switch: -{switch_name}."))
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  is_recognized_long_switch
    //
    //  Returns true if the given name (case-insensitive, without prefix
    //  or '=') matches any known long switch.
    //  Port of: CCommandLine::IsRecognizedLongSwitch
    //
    ////////////////////////////////////////////////////////////////////////////

    fn is_recognized_long_switch(name: &str) -> bool {
        const RECOGNIZED: &[&str] = &[
            "env",
            "config",
            "owner",
            "streams",
            "debug",
            "icons",
            "tree",
            "depth",
            "treeindent",
            "size",
            "set-aliases",
            "get-aliases",
            "remove-aliases",
            "whatif",
        ];



        RECOGNIZED.iter().any (|&s| s.eq_ignore_ascii_case (name))
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  validate_switch_combinations
    //
    //  Post-parse validation of switch conflicts and dependencies.
    //  Called at the end of parse_from, before config defaults are applied.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn validate_switch_combinations(&self) -> Result<(), AppError> {
        let tree = self.tree.unwrap_or (false);

        //
        // Alias switches are mutually exclusive with each other
        // and with all directory listing switches.
        //

        let alias_count = self.set_aliases as u8
                        + self.get_aliases as u8
                        + self.remove_aliases as u8;

        if alias_count > 1 {
            return Err (AppError::InvalidArg (
                "--set-aliases, --get-aliases, and --remove-aliases are mutually exclusive".into()
            ));
        }

        if alias_count == 1
            && (tree || self.wide_listing || self.bare_listing || self.recurse
                || self.show_owner || self.show_streams || self.show_env_help
                || self.show_config || self.show_settings
                || self.sort_order != SortOrder::Default
                || self.attrs_required != 0 || self.attrs_excluded != 0)
            {
                return Err (AppError::InvalidArg (
                    "Alias switches cannot be combined with directory listing switches".into()
                ));
            }

        if self.what_if && !self.set_aliases && !self.remove_aliases {
            return Err (AppError::InvalidArg (
                "--whatif is only valid with --set-aliases or --remove-aliases".into()
            ));
        }

        if tree {
            if self.wide_listing {
                return Err (AppError::InvalidArg (
                    "--Tree cannot be combined with /W (wide listing)".into()
                ));
            }
            if self.bare_listing {
                return Err (AppError::InvalidArg (
                    "--Tree cannot be combined with /B (bare listing)".into()
                ));
            }
            if self.recurse {
                return Err (AppError::InvalidArg (
                    "--Tree cannot be combined with /S (recurse)".into()
                ));
            }
            if self.show_owner {
                return Err (AppError::InvalidArg (
                    "--Tree cannot be combined with --Owner".into()
                ));
            }
            if self.size_format == SizeFormat::Bytes {
                return Err (AppError::InvalidArg (
                    "--Tree cannot be combined with --Size=Bytes".into()
                ));
            }
        }

        if self.max_depth > 0 && !tree {
            return Err (AppError::InvalidArg (
                "--Depth requires --Tree".into()
            ));
        }

        if self.tree_indent != 4 && !tree {
            return Err (AppError::InvalidArg (
                "--TreeIndent requires --Tree".into()
            ));
        }

        Ok(())
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  resolved_size_format
    //
    //  Resolve SizeFormat::Default based on tree mode:
    //  - Tree mode:     Default → Auto (abbreviated sizes)
    //  - Non-tree mode: Default → Bytes (exact byte counts)
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn resolved_size_format(&self) -> SizeFormat {
        match self.size_format {
            SizeFormat::Default => {
                if self.tree.unwrap_or (false) {
                    SizeFormat::Auto
                } else {
                    SizeFormat::Bytes
                }
            }
            other => other,
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  apply_config_defaults
    //
    //  Apply switch defaults from Config (RCDIR environment variable).
    //  CLI-set values take priority over config values.
    //
    //  Port of: CCommandLine::ApplyConfigDefaults
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn apply_config_defaults(&mut self, config: &Config) {
        if let Some(v) = config.wide_listing   { self.wide_listing   = v; }
        if let Some(v) = config.bare_listing   { self.bare_listing   = v; }
        if let Some(v) = config.recurse        { self.recurse        = v; }
        if let Some(v) = config.perf_timer     { self.perf_timer     = v; }
        if let Some(v) = config.multi_threaded { self.multi_threaded = v; }
        if let Some(v) = config.show_owner     { self.show_owner     = v; }
        if let Some(v) = config.show_streams   { self.show_streams   = v; }

        // Icons: conditional merge — only apply config default if CLI didn't specify
        if config.icons.is_some() && self.icons.is_none() {
            self.icons = config.icons;
        }

        // Tree: conditional merge — only apply config default if CLI didn't specify
        if self.tree.is_none() {
            self.tree = config.tree;
        }

        // Depth: only apply if CLI didn't set and tree is active
        if self.max_depth == 0
            && let Some (d) = config.max_depth
            && self.tree.unwrap_or (false)
        {
            self.max_depth = d;
        }

        // TreeIndent: only apply if CLI didn't change from default and tree active
        if self.tree_indent == 4
            && let Some (ti) = config.tree_indent
            && self.tree.unwrap_or (false)
        {
            self.tree_indent = ti;
        }

        // SizeFormat: only apply if CLI didn't set
        if self.size_format == SizeFormat::Default
            && let Some (sf) = config.size_format
        {
            self.size_format = sf;
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  handle_switch
    //
    //  Route a switch argument to the appropriate handler.
    //
    //  Port of: CCommandLine::HandleSwitch
    //
    ////////////////////////////////////////////////////////////////////////////

    fn handle_switch(&mut self, switch_arg: &str, args: &[String], idx: &mut usize) -> Result<(), AppError> {
        // Check for long switch (3+ chars, no ':' or '-' at position 1)
        if switch_arg.len() >= 3
            && switch_arg.as_bytes().get(1) != Some(&b':')
            && switch_arg.as_bytes().get(1) != Some(&b'-')
        {
            return self.handle_long_switch (switch_arg, args, idx);
        }

        // Single-character switch
        let ch = switch_arg.chars().next()
            .ok_or_else(|| AppError::InvalidArg(String::new()))?;

        // Check for trailing '-' to disable
        let disable = switch_arg.len() >= 2 && switch_arg.as_bytes()[1] == b'-';

        let ch_lower = ch.to_ascii_lowercase();
        match ch_lower {
            's' => { self.recurse        = !disable; Ok(()) }
            'w' => { self.wide_listing   = !disable; Ok(()) }
            'b' => { self.bare_listing   = !disable; Ok(()) }
            'p' => { self.perf_timer     = !disable; Ok(()) }
            'm' => { self.multi_threaded = !disable; Ok(()) }
            '?' => { self.show_help      = true;     Ok(()) }
            'o' => self.order_by_handler(&switch_arg[1..]),
            'a' => self.attribute_handler(&switch_arg[1..]),
            't' => self.time_field_handler(&switch_arg[1..]),
            _   => Err(AppError::InvalidArg(String::new())),
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  handle_long_switch
    //
    //  Handle long switches: env, config, owner, streams, icons, debug.
    //  Table-driven dispatch with case-insensitive matching.
    //
    //  Port of: CCommandLine::HandleLongSwitch
    //
    ////////////////////////////////////////////////////////////////////////////

    fn handle_long_switch(&mut self, switch_arg: &str, args: &[String], idx: &mut usize) -> Result<(), AppError> {
        // Split on '=' to extract key and optional value
        let (key, inline_value) = match switch_arg.find ('=') {
            Some (pos) => (&switch_arg[..pos], Some (&switch_arg[pos + 1..])),
            None       => (switch_arg, None),
        };

        // Boolean switches (no value expected)
        type Setter = fn(&mut CommandLine);

        let bool_switches: &[(&str, Setter)] = &[
            ("env",      |cmd| cmd.show_env_help = true),
            ("config",   |cmd| cmd.show_config   = true),
            ("settings", |cmd| cmd.show_settings = true),
            ("owner",    |cmd| cmd.show_owner    = true),
            ("streams", |cmd| cmd.show_streams  = true),
            ("icons",   |cmd| cmd.icons = Some (true)),
            ("icons-",  |cmd| cmd.icons = Some (false)),
            ("tree",    |cmd| cmd.tree = Some (true)),
            ("tree-",   |cmd| cmd.tree = Some (false)),
            ("set-aliases",    |cmd| cmd.set_aliases    = true),
            ("get-aliases",    |cmd| cmd.get_aliases    = true),
            ("remove-aliases", |cmd| cmd.remove_aliases = true),
            ("whatif",         |cmd| cmd.what_if        = true),
            #[cfg(debug_assertions)]
            ("debug",   |cmd| cmd.debug = true),
        ];

        for &(name, setter) in bool_switches {
            if key.eq_ignore_ascii_case (name) {
                setter (self);
                return Ok(());
            }
        }

        // Parameterized switches — need a value (from '=' or next arg)
        let value = match inline_value {
            Some (v) => v.to_string(),
            None     => {
                let next_idx = *idx + 1;
                if next_idx < args.len() {
                    *idx = next_idx;
                    args[next_idx].clone()
                } else {
                    return Err (AppError::InvalidArg (
                        format! ("Switch --{} requires a value", key)
                    ));
                }
            }
        };

        let key_lower = key.to_ascii_lowercase();
        match key_lower.as_str() {
            "depth" => {
                let n: i32 = value.parse().map_err (|_| {
                    AppError::InvalidArg (format! ("Invalid depth value: {}", value))
                })?;
                if n <= 0 {
                    return Err (AppError::InvalidArg (
                        format! ("--Depth must be a positive integer, got {}", n)
                    ));
                }
                self.max_depth = n;
                Ok(())
            }
            "treeindent" => {
                let n: i32 = value.parse().map_err (|_| {
                    AppError::InvalidArg (format! ("Invalid tree indent value: {}", value))
                })?;
                if !(1..=8).contains (&n) {
                    return Err (AppError::InvalidArg (
                        format! ("--TreeIndent must be between 1 and 8, got {}", n)
                    ));
                }
                self.tree_indent = n;
                Ok(())
            }
            "size" => {
                if value.eq_ignore_ascii_case ("auto") {
                    self.size_format = SizeFormat::Auto;
                } else if value.eq_ignore_ascii_case ("bytes") {
                    self.size_format = SizeFormat::Bytes;
                } else {
                    return Err (AppError::InvalidArg (
                        format! ("Invalid --Size value '{}'. Use Auto or Bytes", value)
                    ));
                }
                Ok(())
            }
            _ => Err (AppError::InvalidArg (String::new())),
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  order_by_handler
    //
    //  Handle /O sort order switch.
    //
    //  Port of: CCommandLine::OrderByHandler
    //
    ////////////////////////////////////////////////////////////////////////////

    fn order_by_handler(&mut self, arg: &str) -> Result<(), AppError> {
        let mut chars = arg.chars().peekable();

        // Skip optional colon
        if chars.peek() == Some(&':') {
            chars.next();
        }

        // Must have at least one char
        if chars.peek().is_none() {
            return Err(AppError::InvalidArg(String::new()));
        }

        // Check for reverse direction
        if chars.peek() == Some(&'-') {
            self.sort_direction = SortDirection::Descending;
            chars.next();

            if chars.peek().is_none() {
                return Err(AppError::InvalidArg(String::new()));
            }
        }

        // Read sort key
        let key = chars.next().unwrap().to_ascii_lowercase();
        let order = match key {
            'n' => SortOrder::Name,
            'e' => SortOrder::Extension,
            's' => SortOrder::Size,
            'd' => SortOrder::Date,
            _   => return Err(AppError::InvalidArg(String::new())),
        };

        // Trailing characters are an error (e.g. /o:d- is invalid; use /o:-d)
        if chars.next().is_some() {
            return Err(AppError::InvalidArg(String::new()));
        }

        self.sort_order = order;
        self.sort_preference[0] = order;
        Ok(())
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  attribute_handler
    //
    //  Handle /A attribute filter switch.
    //
    //  Port of: CCommandLine::AttributeHandler
    //
    ////////////////////////////////////////////////////////////////////////////

    fn attribute_handler(&mut self, arg: &str) -> Result<(), AppError> {
        let mut chars = arg.chars().peekable();

        // Skip optional colon prefix
        if chars.peek() == Some(&':') {
            chars.next();
        }

        // Must have at least one char
        if chars.peek().is_none() {
            return Err(AppError::InvalidArg(String::new()));
        }

        // Start in "required" mode
        let mut excluding = false;

        for ch in chars {
            let ch_lower = ch.to_ascii_lowercase();

            if ch_lower == '-' {
                // Double '-' is an error
                if excluding {
                    return Err(AppError::InvalidArg(String::new()));
                }
                excluding = true;
                continue;
            }

            // Map character to Win32 attribute flag
            let flag = match ch_lower {
                'd' => windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY.0,
                'h' => windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN.0,
                's' => windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_SYSTEM.0,
                'r' => windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_READONLY.0,
                'a' => windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_ARCHIVE.0,
                't' => windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_TEMPORARY.0,
                'e' => windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_ENCRYPTED.0,
                'c' => windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_COMPRESSED.0,
                'p' => windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT.0,
                '0' => windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_SPARSE_FILE.0,
                'x' => windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NOT_CONTENT_INDEXED.0,
                'i' => windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_INTEGRITY_STREAM.0,
                'b' => windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NO_SCRUB_DATA.0,
                'o' => {
                    // Cloud-only composite: OFFLINE | RECALL_ON_OPEN | RECALL_ON_DATA_ACCESS
                    windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_OFFLINE.0
                        | windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_RECALL_ON_OPEN.0
                        | windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS.0
                }
                'l' => windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_UNPINNED.0,
                'v' => windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_PINNED.0,
                _   => 0, // Unknown chars silently ignored (matches TCDir behavior)
            };

            if flag != 0 {
                if excluding {
                    self.attrs_excluded |= flag;
                } else {
                    self.attrs_required |= flag;
                }
            }

            // Reset to required mode after each attribute char
            excluding = false;
        }

        Ok(())
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  time_field_handler
    //
    //  Handle /T time field switch.
    //
    //  Port of: CCommandLine::TimeFieldHandler
    //
    ////////////////////////////////////////////////////////////////////////////

    fn time_field_handler(&mut self, arg: &str) -> Result<(), AppError> {
        let mut chars = arg.chars();

        // Skip optional colon
        let first = chars.next()
            .ok_or_else(|| AppError::InvalidArg(String::new()))?;

        let field_char = if first == ':' {
            chars.next()
                .ok_or_else(|| AppError::InvalidArg(String::new()))?
        } else {
            first
        };

        match field_char.to_ascii_lowercase() {
            'c' => { self.time_field = TimeField::Creation; Ok(()) }
            'a' => { self.time_field = TimeField::Access;   Ok(()) }
            'w' => { self.time_field = TimeField::Written;  Ok(()) }
            _   => Err(AppError::InvalidArg(String::new())),
        }
    }
}





#[cfg(test)]
mod tests {
    use super::*;

    ////////////////////////////////////////////////////////////////////////////
    //
    //  default_values
    //
    //  Verify default CommandLine values.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn default_values() {
        let cmd = CommandLine::default();
        assert!(!cmd.recurse);
        assert!(!cmd.wide_listing);
        assert!(!cmd.bare_listing);
        assert!(!cmd.perf_timer);
        assert!(cmd.multi_threaded);
        assert!(!cmd.show_help);
        assert_eq!(cmd.sort_order, SortOrder::Default);
        assert_eq!(cmd.sort_direction, SortDirection::Ascending);
        assert_eq!(cmd.time_field, TimeField::Written);
        assert_eq!(cmd.switch_prefix, '-');
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_slash_switches
    //
    //  Verify parsing of slash-prefixed switches.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_slash_switches() {
        let cmd = CommandLine::parse_from(["/s", "/w", "/p"]).unwrap();
        assert!(cmd.recurse);
        assert!(cmd.wide_listing);
        assert!(cmd.perf_timer);
        assert_eq!(cmd.switch_prefix, '/');
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_dash_switches
    //
    //  Verify parsing of dash-prefixed switches.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_dash_switches() {
        let cmd = CommandLine::parse_from(["-s", "-b"]).unwrap();
        assert!(cmd.recurse);
        assert!(cmd.bare_listing);
        assert_eq!(cmd.switch_prefix, '-');
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_disable_with_trailing_dash
    //
    //  Verify trailing dash disables a switch.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_disable_with_trailing_dash() {
        let cmd = CommandLine::parse_from(["/m-"]).unwrap();
        assert!(!cmd.multi_threaded);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_help
    //
    //  Verify /? enables show_help.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_help() {
        let cmd = CommandLine::parse_from(["/?"].iter()).unwrap();
        assert!(cmd.show_help);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_sort_name
    //
    //  Verify /O:N sets sort order to Name.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_sort_name() {
        let cmd = CommandLine::parse_from(["/on"]).unwrap();
        assert_eq!(cmd.sort_order, SortOrder::Name);
        assert_eq!(cmd.sort_direction, SortDirection::Ascending);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_sort_with_colon
    //
    //  Verify /O:S with colon prefix.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_sort_with_colon() {
        let cmd = CommandLine::parse_from(["/o:s"]).unwrap();
        assert_eq!(cmd.sort_order, SortOrder::Size);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_sort_descending
    //
    //  Verify /O:-D sets descending direction.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_sort_descending() {
        let cmd = CommandLine::parse_from(["/o:-d"]).unwrap();
        assert_eq!(cmd.sort_order, SortOrder::Date);
        assert_eq!(cmd.sort_direction, SortDirection::Descending);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_sort_case_insensitive
    //
    //  Verify sort switch is case-insensitive.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_sort_case_insensitive() {
        let cmd = CommandLine::parse_from(["/O:E"]).unwrap();
        assert_eq!(cmd.sort_order, SortOrder::Extension);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_sort_empty_errors
    //
    //  Verify empty sort arg returns error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_sort_empty_errors() {
        assert!(CommandLine::parse_from(["/o"]).is_err());
        assert!(CommandLine::parse_from(["/o:"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_sort_trailing_chars_error
    //
    //  Verify trailing characters after the sort key produce an error.
    //  e.g. /o:d- is invalid — use /o:-d for descending date.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_sort_trailing_chars_error() {
        assert!(CommandLine::parse_from(["/o:d-"]).is_err());
        assert!(CommandLine::parse_from(["/o:nx"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_attrs_required
    //
    //  Verify /A:D sets required attribute.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_attrs_required() {
        let cmd = CommandLine::parse_from(["/a:d"]).unwrap();
        assert_ne!(cmd.attrs_required & 0x10, 0); // FILE_ATTRIBUTE_DIRECTORY = 0x10
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_attrs_excluded
    //
    //  Verify /A:-H sets excluded attribute.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_attrs_excluded() {
        let cmd = CommandLine::parse_from(["/a:-h"]).unwrap();
        assert_ne!(cmd.attrs_excluded & 0x02, 0); // FILE_ATTRIBUTE_HIDDEN = 0x02
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_attrs_mixed
    //
    //  Verify mixed required and excluded attributes.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_attrs_mixed() {
        // /a:h-ds → require H, exclude D, require S
        let cmd = CommandLine::parse_from(["/a:h-ds"]).unwrap();
        assert_ne!(cmd.attrs_required & 0x02, 0); // H required
        assert_ne!(cmd.attrs_excluded & 0x10, 0);  // D excluded
        assert_ne!(cmd.attrs_required & 0x04, 0); // S required
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_attrs_double_dash_error
    //
    //  Verify double dash in /A returns error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_attrs_double_dash_error() {
        assert!(CommandLine::parse_from(["/a:--d"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_time_creation
    //
    //  Verify /T:C sets creation time field.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_time_creation() {
        let cmd = CommandLine::parse_from(["/t:c"]).unwrap();
        assert_eq!(cmd.time_field, TimeField::Creation);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_time_access
    //
    //  Verify /T:A sets access time field.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_time_access() {
        let cmd = CommandLine::parse_from(["/t:a"]).unwrap();
        assert_eq!(cmd.time_field, TimeField::Access);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_time_no_colon
    //
    //  Verify /TW without colon works.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_time_no_colon() {
        let cmd = CommandLine::parse_from(["/tw"]).unwrap();
        assert_eq!(cmd.time_field, TimeField::Written);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_long_switch_env_double_dash
    //
    //  Verify --env enables show_env_help.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_long_switch_env_double_dash() {
        let cmd = CommandLine::parse_from(["--env"]).unwrap();
        assert!(cmd.show_env_help);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_long_switch_env_slash
    //
    //  Verify /env enables show_env_help.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_long_switch_env_slash() {
        let cmd = CommandLine::parse_from(["/env"]).unwrap();
        assert!(cmd.show_env_help);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_long_switch_single_dash_error
    //
    //  Verify -env (single dash) returns error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_long_switch_single_dash_error() {
        // -env (single dash + long) should produce descriptive error
        let err = CommandLine::parse_from(["-env"]).unwrap_err();
        assert_eq!(err.to_string(), "Unknown switch: -env.  Did you mean --env?");

        // Unrecognized long switch with single dash
        let err = CommandLine::parse_from(["-notaswitch"]).unwrap_err();
        assert_eq!(err.to_string(), "Unknown switch: -notaswitch.");

        // Single-dash with '=' separator — strips = for lookup
        let err = CommandLine::parse_from(["-icons=true"]).unwrap_err();
        assert!(err.to_string().contains("Did you mean"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_long_switch_config
    //
    //  Verify /config enables show_config.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_long_switch_config() {
        let cmd = CommandLine::parse_from(["/config"]).unwrap();
        assert!(cmd.show_config);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_long_switch_owner
    //
    //  Verify --owner enables show_owner.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_long_switch_owner() {
        let cmd = CommandLine::parse_from(["--owner"]).unwrap();
        assert!(cmd.show_owner);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_long_switch_streams
    //
    //  Verify /streams enables show_streams.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_long_switch_streams() {
        let cmd = CommandLine::parse_from(["/streams"]).unwrap();
        assert!(cmd.show_streams);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_long_switch_case_insensitive
    //
    //  Verify long switches are case-insensitive.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_long_switch_case_insensitive() {
        let cmd = CommandLine::parse_from(["/ENV"]).unwrap();
        assert!(cmd.show_env_help);

        let cmd2 = CommandLine::parse_from(["--Owner"]).unwrap();
        assert!(cmd2.show_owner);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_masks
    //
    //  Verify positional arguments parsed as masks.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_masks() {
        let cmd = CommandLine::parse_from(["*.rs", "*.toml"]).unwrap();
        assert_eq!(cmd.masks.len(), 2);
        assert_eq!(cmd.masks[0], "*.rs");
        assert_eq!(cmd.masks[1], "*.toml");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_mixed_masks_and_switches
    //
    //  Verify masks and switches can be intermixed.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_mixed_masks_and_switches() {
        let cmd = CommandLine::parse_from(["*.rs", "/s", "*.toml", "/o:n"]).unwrap();
        assert_eq!(cmd.masks.len(), 2);
        assert!(cmd.recurse);
        assert_eq!(cmd.sort_order, SortOrder::Name);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  apply_config_defaults
    //
    //  Verify Config defaults are applied to CommandLine.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn apply_config_defaults() {
        let mut cmd = CommandLine::default();
        let mut config = Config::new();
        config.wide_listing = Some(true);
        config.perf_timer = Some(true);
        config.multi_threaded = Some(false);

        cmd.apply_config_defaults(&config);

        assert!(cmd.wide_listing);
        assert!(cmd.perf_timer);
        assert!(!cmd.multi_threaded);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  switch_prefix_tracks_last_used
    //
    //  Verify switch_prefix tracks the last prefix used.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn switch_prefix_tracks_last_used() {
        let cmd = CommandLine::parse_from(["-s", "/w"]).unwrap();
        assert_eq!(cmd.switch_prefix, '/');

        let cmd2 = CommandLine::parse_from(["/s", "-w"]).unwrap();
        assert_eq!(cmd2.switch_prefix, '-');
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  sort_preference_default
    //
    //  Verify default sort preference chain.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn sort_preference_default() {
        let cmd = CommandLine::default();
        assert_eq!(cmd.sort_preference[0], SortOrder::Default);
        assert_eq!(cmd.sort_preference[1], SortOrder::Name);
        assert_eq!(cmd.sort_preference[2], SortOrder::Date);
        assert_eq!(cmd.sort_preference[3], SortOrder::Extension);
        assert_eq!(cmd.sort_preference[4], SortOrder::Size);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  sort_preference_updated_on_sort
    //
    //  Verify sort preference[0] updated on sort.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn sort_preference_updated_on_sort() {
        let cmd = CommandLine::parse_from(["/o:s"]).unwrap();
        assert_eq!(cmd.sort_preference[0], SortOrder::Size);
        // Rest of chain unchanged
        assert_eq!(cmd.sort_preference[1], SortOrder::Name);
    }





    // =========================================================================
    //  Tree switch parsing tests (T013)
    // =========================================================================

    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_tree_switch_double_dash
    //
    //  Verify --Tree enables tree mode.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_tree_switch_double_dash () {
        let cmd = CommandLine::parse_from (["--Tree"]).unwrap();
        assert_eq! (cmd.tree, Some (true));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_tree_switch_slash
    //
    //  Verify /Tree enables tree mode.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_tree_switch_slash () {
        let cmd = CommandLine::parse_from (["/Tree"]).unwrap();
        assert_eq! (cmd.tree, Some (true));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_tree_disable_switch_double_dash
    //
    //  Verify --Tree- disables tree mode.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_tree_disable_switch_double_dash () {
        let cmd = CommandLine::parse_from (["--Tree-"]).unwrap();
        assert_eq! (cmd.tree, Some (false));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_tree_switch_case_insensitive
    //
    //  Verify tree switch is case-insensitive.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_tree_switch_case_insensitive () {
        let cmd = CommandLine::parse_from (["--tree"]).unwrap();
        assert_eq! (cmd.tree, Some (true));

        let cmd2 = CommandLine::parse_from (["--TREE"]).unwrap();
        assert_eq! (cmd2.tree, Some (true));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_alias_switches
    //
    //  Verify parsing of --set-aliases, --get-aliases, --remove-aliases.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_alias_switches () {
        let cmd = CommandLine::parse_from (["--set-aliases"]).unwrap();
        assert! (cmd.set_aliases);
        assert! (!cmd.get_aliases);
        assert! (!cmd.remove_aliases);

        let cmd = CommandLine::parse_from (["--get-aliases"]).unwrap();
        assert! (cmd.get_aliases);
        assert! (!cmd.set_aliases);

        let cmd = CommandLine::parse_from (["--remove-aliases"]).unwrap();
        assert! (cmd.remove_aliases);
        assert! (!cmd.set_aliases);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_whatif_with_alias_switch
    //
    //  Verify --whatif works with --set-aliases and --remove-aliases.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_whatif_with_alias_switch () {
        let cmd = CommandLine::parse_from (["--set-aliases", "--whatif"]).unwrap();
        assert! (cmd.set_aliases);
        assert! (cmd.what_if);

        let cmd = CommandLine::parse_from (["--remove-aliases", "--whatif"]).unwrap();
        assert! (cmd.remove_aliases);
        assert! (cmd.what_if);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  alias_switches_mutually_exclusive
    //
    //  Verify --set-aliases, --get-aliases, --remove-aliases are mutually
    //  exclusive.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn alias_switches_mutually_exclusive () {
        assert! (CommandLine::parse_from (["--set-aliases", "--get-aliases"]).is_err());
        assert! (CommandLine::parse_from (["--set-aliases", "--remove-aliases"]).is_err());
        assert! (CommandLine::parse_from (["--get-aliases", "--remove-aliases"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  whatif_without_alias_switch_errors
    //
    //  Verify --whatif without an alias switch produces an error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn whatif_without_alias_switch_errors () {
        assert! (CommandLine::parse_from (["--whatif"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  alias_switches_reject_listing_switches
    //
    //  Verify alias switches cannot be combined with directory listing
    //  switches.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn alias_switches_reject_listing_switches () {
        assert! (CommandLine::parse_from (["--set-aliases", "/w"]).is_err());
        assert! (CommandLine::parse_from (["--get-aliases", "/s"]).is_err());
        assert! (CommandLine::parse_from (["--remove-aliases", "--tree"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_tree_switch_single_dash_fails
    //
    //  Verify -tree (single dash) produces error with hint.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_tree_switch_single_dash_fails () {
        let err = CommandLine::parse_from (["-tree"]).unwrap_err();
        assert! (err.to_string().contains ("Did you mean"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_depth_single_dash_fails
    //
    //  Verify -depth=5 (single dash) produces error with hint.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_depth_single_dash_fails () {
        let err = CommandLine::parse_from (["-depth=5"]).unwrap_err();
        assert! (err.to_string().contains ("Did you mean"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_depth_with_equals
    //
    //  Verify --Depth=5 parses depth with = separator.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_depth_with_equals () {
        let cmd = CommandLine::parse_from (["--Tree", "--Depth=5"]).unwrap();
        assert_eq! (cmd.max_depth, 5);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_depth_with_space
    //
    //  Verify --Depth 5 parses depth with space separator.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_depth_with_space () {
        let cmd = CommandLine::parse_from (["--Tree", "--Depth", "5"]).unwrap();
        assert_eq! (cmd.max_depth, 5);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_tree_indent_with_equals
    //
    //  Verify --TreeIndent=2 parses tree indent.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_tree_indent_with_equals () {
        let cmd = CommandLine::parse_from (["--Tree", "--TreeIndent=2"]).unwrap();
        assert_eq! (cmd.tree_indent, 2);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_tree_with_wide_fails
    //
    //  Verify --Tree with /W produces conflict error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_tree_with_wide_fails () {
        assert! (CommandLine::parse_from (["--Tree", "/w"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_tree_with_bare_fails
    //
    //  Verify --Tree with /B produces conflict error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_tree_with_bare_fails () {
        assert! (CommandLine::parse_from (["--Tree", "/b"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_tree_with_recurse_fails
    //
    //  Verify --Tree with /S produces conflict error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_tree_with_recurse_fails () {
        assert! (CommandLine::parse_from (["--Tree", "/s"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_tree_with_owner_fails
    //
    //  Verify --Tree with --Owner produces conflict error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_tree_with_owner_fails () {
        assert! (CommandLine::parse_from (["--Tree", "--owner"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_depth_without_tree_fails
    //
    //  Verify --Depth without --Tree produces error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_depth_without_tree_fails () {
        assert! (CommandLine::parse_from (["--Depth=5"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_tree_indent_without_tree_fails
    //
    //  Verify --TreeIndent without --Tree produces error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_tree_indent_without_tree_fails () {
        assert! (CommandLine::parse_from (["--TreeIndent=2"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_tree_indent_out_of_range_fails
    //
    //  Verify --TreeIndent=10 (>8) produces error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_tree_indent_out_of_range_fails () {
        assert! (CommandLine::parse_from (["--Tree", "--TreeIndent=10"]).is_err());
        assert! (CommandLine::parse_from (["--Tree", "--TreeIndent=0"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_depth_zero_fails
    //
    //  Verify --Depth=0 produces error (0 = unlimited is default, not user-set).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_depth_zero_fails () {
        assert! (CommandLine::parse_from (["--Tree", "--Depth=0"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_depth_negative_fails
    //
    //  Verify --Depth=-1 produces error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_depth_negative_fails () {
        assert! (CommandLine::parse_from (["--Tree", "--Depth=-1"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_tree_with_owner_fails_even_with_icons
    //
    //  Verify --Tree + --Owner fails regardless of --Icons.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_tree_with_owner_fails_even_with_icons () {
        assert! (CommandLine::parse_from (["--Tree", "--owner", "--icons"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_tree_with_depth_and_indent_succeeds
    //
    //  Verify --Tree + --Depth + --TreeIndent all work together.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_tree_with_depth_and_indent_succeeds () {
        let cmd = CommandLine::parse_from (["--Tree", "--Depth=3", "--TreeIndent=6"]).unwrap();
        assert_eq! (cmd.tree, Some (true));
        assert_eq! (cmd.max_depth, 3);
        assert_eq! (cmd.tree_indent, 6);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  apply_config_defaults_tree_transfers
    //
    //  Verify config tree=true transfers to CommandLine.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn apply_config_defaults_tree_transfers () {
        let mut cmd = CommandLine::default();
        let mut config = Config::new();
        config.tree = Some (true);

        cmd.apply_config_defaults (&config);

        assert_eq! (cmd.tree, Some (true));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  apply_config_defaults_tree_with_depth_transfers
    //
    //  Verify config tree + depth both transfer.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn apply_config_defaults_tree_with_depth_transfers () {
        let mut cmd = CommandLine::default();
        let mut config = Config::new();
        config.tree      = Some (true);
        config.max_depth  = Some (5);

        cmd.apply_config_defaults (&config);

        assert_eq! (cmd.tree, Some (true));
        assert_eq! (cmd.max_depth, 5);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  apply_config_defaults_depth_without_tree_silently_ignored
    //
    //  Verify config depth without tree is silently ignored.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn apply_config_defaults_depth_without_tree_silently_ignored () {
        let mut cmd = CommandLine::default();
        let mut config = Config::new();
        config.max_depth = Some (5);

        cmd.apply_config_defaults (&config);

        assert_eq! (cmd.max_depth, 0);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  cli_tree_disable_overrides_env_var_tree
    //
    //  Verify CLI --Tree- overrides env var Tree.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn cli_tree_disable_overrides_env_var_tree () {
        let mut cmd = CommandLine::parse_from (["--Tree-"]).unwrap();
        let mut config = Config::new();
        config.tree = Some (true);

        cmd.apply_config_defaults (&config);

        assert_eq! (cmd.tree, Some (false));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  cli_depth_overrides_env_var_depth
    //
    //  Verify CLI --Depth overrides env var Depth.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn cli_depth_overrides_env_var_depth () {
        let mut cmd = CommandLine::parse_from (["--Tree", "--Depth=3"]).unwrap();
        let mut config = Config::new();
        config.max_depth = Some (10);

        cmd.apply_config_defaults (&config);

        assert_eq! (cmd.max_depth, 3);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_size_default_resolves_to_bytes_without_tree
    //
    //  Verify SizeFormat::Default resolves to Bytes without tree mode.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_size_default_resolves_to_bytes_without_tree () {
        let cmd = CommandLine::default();
        assert_eq! (cmd.resolved_size_format(), SizeFormat::Bytes);
    }





    // =========================================================================
    //  Size switch parsing tests (T014)
    // =========================================================================

    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_size_bytes_without_tree
    //
    //  Verify --Size=Bytes works without tree mode.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_size_bytes_without_tree () {
        let cmd = CommandLine::parse_from (["--Size=Bytes"]).unwrap();
        assert_eq! (cmd.size_format, SizeFormat::Bytes);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_size_auto_without_tree
    //
    //  Verify --Size=Auto works without tree mode.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_size_auto_without_tree () {
        let cmd = CommandLine::parse_from (["--Size=Auto"]).unwrap();
        assert_eq! (cmd.size_format, SizeFormat::Auto);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_size_invalid_fails
    //
    //  Verify --Size=Invalid produces error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_size_invalid_fails () {
        assert! (CommandLine::parse_from (["--Size=Invalid"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_size_case_insensitive
    //
    //  Verify --Size switch is case-insensitive.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_size_case_insensitive () {
        let cmd = CommandLine::parse_from (["--Size=auto"]).unwrap();
        assert_eq! (cmd.size_format, SizeFormat::Auto);

        let cmd2 = CommandLine::parse_from (["--Size=BYTES"]).unwrap();
        assert_eq! (cmd2.size_format, SizeFormat::Bytes);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_size_auto_with_tree
    //
    //  Verify --Tree + --Size=Auto succeeds.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_size_auto_with_tree () {
        let cmd = CommandLine::parse_from (["--Tree", "--Size=Auto"]).unwrap();
        assert_eq! (cmd.size_format, SizeFormat::Auto);
        assert_eq! (cmd.tree, Some (true));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_size_bytes_with_tree_fails
    //
    //  Verify --Tree + --Size=Bytes produces conflict error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_size_bytes_with_tree_fails () {
        assert! (CommandLine::parse_from (["--Tree", "--Size=Bytes"]).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_size_default_resolves_to_auto_with_tree
    //
    //  Verify SizeFormat::Default resolves to Auto with tree mode.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_size_default_resolves_to_auto_with_tree () {
        let cmd = CommandLine::parse_from (["--Tree"]).unwrap();
        assert_eq! (cmd.resolved_size_format(), SizeFormat::Auto);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  apply_config_defaults_size_auto_transfers
    //
    //  Verify config size_format=Auto transfers to CommandLine.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn apply_config_defaults_size_auto_transfers () {
        let mut cmd = CommandLine::default();
        let mut config = Config::new();
        config.size_format = Some (SizeFormat::Auto);

        cmd.apply_config_defaults (&config);

        assert_eq! (cmd.size_format, SizeFormat::Auto);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  apply_config_defaults_size_bytes_not_overridden_by_cli
    //
    //  Verify CLI-set size_format is not overridden by config.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn apply_config_defaults_size_bytes_not_overridden_by_cli () {
        let mut cmd = CommandLine::parse_from (["--Size=Bytes"]).unwrap();
        let mut config = Config::new();
        config.size_format = Some (SizeFormat::Auto);

        cmd.apply_config_defaults (&config);

        assert_eq! (cmd.size_format, SizeFormat::Bytes);
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_long_switch_settings_double_dash
    //
    //  Verify --settings enables show_settings.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_long_switch_settings_double_dash () {
        let cmd = CommandLine::parse_from (["--settings"]).unwrap();
        assert! (cmd.show_settings);
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  alias_switch_rejects_settings
    //
    //  Verify alias switches cannot be combined with --settings.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn alias_switch_rejects_settings () {
        let result = CommandLine::parse_from (["--set-aliases", "--settings"]);
        assert! (result.is_err());
    }
}