// command_line.rs — CLI argument parsing (custom, no clap)
//
// Port of: CommandLine.h, CommandLine.cpp
// Windows-style /switch and -switch prefixes, compound switches (/a:hs, /o:-d),
// long switches (--env, /owner), trailing - disable (/m-).

use std::ffi::OsString;

use crate::config::Config;
use crate::ehm::AppError;

// ── Enums ─────────────────────────────────────────────────────────────────────

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

// ── CommandLine struct ────────────────────────────────────────────────────────

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
    pub show_help:        bool,
    pub switch_prefix:    char,
    pub time_field:       TimeField,
    pub show_owner:       bool,
    pub show_streams:     bool,
    pub debug:            bool,
}

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
            show_help:       false,
            switch_prefix:   '-',
            time_field:      TimeField::Written,
            show_owner:      false,
            show_streams:    false,
            debug:           false,
        }
    }
}

impl CommandLine {
    /// Parse command-line arguments into a CommandLine struct.
    /// Args should NOT include argv[0] (program name).
    ///
    /// Port of: CCommandLine::Parse
    pub fn parse_from<I, S>(args: I) -> Result<Self, AppError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut cmd = CommandLine::default();

        for arg_ref in args {
            let arg = arg_ref.as_ref();
            if arg.is_empty() {
                continue;
            }

            let first_char = arg.chars().next().unwrap();

            match first_char {
                '-' | '/' => {
                    cmd.switch_prefix = first_char;

                    let switch_arg;
                    let mut is_double_dash = false;

                    // Check for '--' prefix
                    if first_char == '-' && arg.len() > 1 && arg.as_bytes()[1] == b'-' {
                        switch_arg = &arg[2..];
                        is_double_dash = true;
                    } else {
                        switch_arg = &arg[1..];
                    }

                    // Detect long switch: 3+ chars without ':' or '-' at position 1
                    let looks_like_long = switch_arg.len() >= 3
                        && switch_arg.as_bytes().get(1) != Some(&b':')
                        && switch_arg.as_bytes().get(1) != Some(&b'-');

                    // Reject single-dash long switches (e.g., -env → error)
                    if looks_like_long && !is_double_dash && first_char == '-' {
                        return Err(AppError::InvalidArg(String::new()));
                    }

                    cmd.handle_switch(switch_arg)?;
                }
                _ => {
                    // Positional argument (file mask)
                    cmd.masks.push(OsString::from(arg));
                }
            }
        }

        Ok(cmd)
    }

    /// Apply switch defaults from Config (parsed from RCDIR environment variable).
    /// Env defaults are applied BEFORE command-line parsing overrides them.
    ///
    /// Port of: CCommandLine::ApplyConfigDefaults
    pub fn apply_config_defaults(&mut self, config: &Config) {
        if let Some(v) = config.wide_listing   { self.wide_listing   = v; }
        if let Some(v) = config.bare_listing   { self.bare_listing   = v; }
        if let Some(v) = config.recurse        { self.recurse        = v; }
        if let Some(v) = config.perf_timer     { self.perf_timer     = v; }
        if let Some(v) = config.multi_threaded { self.multi_threaded = v; }
        if let Some(v) = config.show_owner     { self.show_owner     = v; }
        if let Some(v) = config.show_streams   { self.show_streams   = v; }
    }

    /// Route a switch argument to the appropriate handler.
    ///
    /// Port of: CCommandLine::HandleSwitch
    fn handle_switch(&mut self, switch_arg: &str) -> Result<(), AppError> {
        // Check for long switch (3+ chars, no ':' or '-' at position 1)
        if switch_arg.len() >= 3
            && switch_arg.as_bytes().get(1) != Some(&b':')
            && switch_arg.as_bytes().get(1) != Some(&b'-')
        {
            return self.handle_long_switch(switch_arg);
        }

        // Single-character switch
        let ch = switch_arg.chars().next()
            .ok_or_else(|| AppError::InvalidArg(String::new()))?;

        // Check for trailing '-' to disable
        let disable = switch_arg.len() >= 2 && switch_arg.as_bytes()[1] == b'-';

        let ch_lower = ch.to_ascii_lowercase();
        match ch_lower {
            's' => { self.recurse       = !disable; Ok(()) }
            'w' => { self.wide_listing  = !disable; Ok(()) }
            'b' => { self.bare_listing  = !disable; Ok(()) }
            'p' => { self.perf_timer    = !disable; Ok(()) }
            'm' => { self.multi_threaded = !disable; Ok(()) }
            '?' => { self.show_help     = true;     Ok(()) }
            'o' => self.order_by_handler(&switch_arg[1..]),
            'a' => self.attribute_handler(&switch_arg[1..]),
            't' => self.time_field_handler(&switch_arg[1..]),
            _   => Err(AppError::InvalidArg(String::new())),
        }
    }

    /// Handle long switches: env, config, owner, streams, debug
    ///
    /// Port of: CCommandLine::HandleLongSwitch
    fn handle_long_switch(&mut self, switch_arg: &str) -> Result<(), AppError> {
        if switch_arg.eq_ignore_ascii_case("env") {
            self.show_env_help = true;
            Ok(())
        } else if switch_arg.eq_ignore_ascii_case("config") {
            self.show_config = true;
            Ok(())
        } else if switch_arg.eq_ignore_ascii_case("owner") {
            self.show_owner = true;
            Ok(())
        } else if switch_arg.eq_ignore_ascii_case("streams") {
            self.show_streams = true;
            Ok(())
        } else if cfg!(debug_assertions) && switch_arg.eq_ignore_ascii_case("debug") {
            self.debug = true;
            Ok(())
        } else {
            Err(AppError::InvalidArg(String::new()))
        }
    }

    /// Handle /O sort order switch.
    /// Format: [:][-]<key> where key is N/E/S/D (case-insensitive)
    ///
    /// Port of: CCommandLine::OrderByHandler
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

        // Read sort key (only first char matters; rest silently ignored)
        let key = chars.next().unwrap().to_ascii_lowercase();
        let order = match key {
            'n' => SortOrder::Name,
            'e' => SortOrder::Extension,
            's' => SortOrder::Size,
            'd' => SortOrder::Date,
            _   => return Err(AppError::InvalidArg(String::new())),
        };

        self.sort_order = order;
        self.sort_preference[0] = order;
        Ok(())
    }

    /// Handle /A attribute filter switch.
    /// Format: [:]<chars> where chars are attribute codes, - toggles exclude mode.
    ///
    /// Port of: CCommandLine::AttributeHandler
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

    /// Handle /T time field switch.
    /// Format: [:]<field> where field is C/A/W (case-insensitive)
    ///
    /// Port of: CCommandLine::TimeFieldHandler
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

    // ── Boolean switches ──────────────────────────────────────────────────

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

    #[test]
    fn parse_slash_switches() {
        let cmd = CommandLine::parse_from(["/s", "/w", "/p"]).unwrap();
        assert!(cmd.recurse);
        assert!(cmd.wide_listing);
        assert!(cmd.perf_timer);
        assert_eq!(cmd.switch_prefix, '/');
    }

    #[test]
    fn parse_dash_switches() {
        let cmd = CommandLine::parse_from(["-s", "-b"]).unwrap();
        assert!(cmd.recurse);
        assert!(cmd.bare_listing);
        assert_eq!(cmd.switch_prefix, '-');
    }

    #[test]
    fn parse_disable_with_trailing_dash() {
        let cmd = CommandLine::parse_from(["/m-"]).unwrap();
        assert!(!cmd.multi_threaded);
    }

    #[test]
    fn parse_help() {
        let cmd = CommandLine::parse_from(["/?"].iter()).unwrap();
        assert!(cmd.show_help);
    }

    // ── Sort order ────────────────────────────────────────────────────────

    #[test]
    fn parse_sort_name() {
        let cmd = CommandLine::parse_from(["/on"]).unwrap();
        assert_eq!(cmd.sort_order, SortOrder::Name);
        assert_eq!(cmd.sort_direction, SortDirection::Ascending);
    }

    #[test]
    fn parse_sort_with_colon() {
        let cmd = CommandLine::parse_from(["/o:s"]).unwrap();
        assert_eq!(cmd.sort_order, SortOrder::Size);
    }

    #[test]
    fn parse_sort_descending() {
        let cmd = CommandLine::parse_from(["/o:-d"]).unwrap();
        assert_eq!(cmd.sort_order, SortOrder::Date);
        assert_eq!(cmd.sort_direction, SortDirection::Descending);
    }

    #[test]
    fn parse_sort_case_insensitive() {
        let cmd = CommandLine::parse_from(["/O:E"]).unwrap();
        assert_eq!(cmd.sort_order, SortOrder::Extension);
    }

    #[test]
    fn parse_sort_empty_errors() {
        assert!(CommandLine::parse_from(["/o"]).is_err());
        assert!(CommandLine::parse_from(["/o:"]).is_err());
    }

    // ── Attribute filter ──────────────────────────────────────────────────

    #[test]
    fn parse_attrs_required() {
        let cmd = CommandLine::parse_from(["/a:d"]).unwrap();
        assert_ne!(cmd.attrs_required & 0x10, 0); // FILE_ATTRIBUTE_DIRECTORY = 0x10
    }

    #[test]
    fn parse_attrs_excluded() {
        let cmd = CommandLine::parse_from(["/a:-h"]).unwrap();
        assert_ne!(cmd.attrs_excluded & 0x02, 0); // FILE_ATTRIBUTE_HIDDEN = 0x02
    }

    #[test]
    fn parse_attrs_mixed() {
        // /a:h-ds → require H, exclude D, require S
        let cmd = CommandLine::parse_from(["/a:h-ds"]).unwrap();
        assert_ne!(cmd.attrs_required & 0x02, 0); // H required
        assert_ne!(cmd.attrs_excluded & 0x10, 0);  // D excluded
        assert_ne!(cmd.attrs_required & 0x04, 0); // S required
    }

    #[test]
    fn parse_attrs_double_dash_error() {
        assert!(CommandLine::parse_from(["/a:--d"]).is_err());
    }

    // ── Time field ────────────────────────────────────────────────────────

    #[test]
    fn parse_time_creation() {
        let cmd = CommandLine::parse_from(["/t:c"]).unwrap();
        assert_eq!(cmd.time_field, TimeField::Creation);
    }

    #[test]
    fn parse_time_access() {
        let cmd = CommandLine::parse_from(["/t:a"]).unwrap();
        assert_eq!(cmd.time_field, TimeField::Access);
    }

    #[test]
    fn parse_time_no_colon() {
        let cmd = CommandLine::parse_from(["/tw"]).unwrap();
        assert_eq!(cmd.time_field, TimeField::Written);
    }

    // ── Long switches ─────────────────────────────────────────────────────

    #[test]
    fn parse_long_switch_env_double_dash() {
        let cmd = CommandLine::parse_from(["--env"]).unwrap();
        assert!(cmd.show_env_help);
    }

    #[test]
    fn parse_long_switch_env_slash() {
        let cmd = CommandLine::parse_from(["/env"]).unwrap();
        assert!(cmd.show_env_help);
    }

    #[test]
    fn parse_long_switch_single_dash_error() {
        // -env (single dash + long) should be an error
        assert!(CommandLine::parse_from(["-env"]).is_err());
    }

    #[test]
    fn parse_long_switch_config() {
        let cmd = CommandLine::parse_from(["/config"]).unwrap();
        assert!(cmd.show_config);
    }

    #[test]
    fn parse_long_switch_owner() {
        let cmd = CommandLine::parse_from(["--owner"]).unwrap();
        assert!(cmd.show_owner);
    }

    #[test]
    fn parse_long_switch_streams() {
        let cmd = CommandLine::parse_from(["/streams"]).unwrap();
        assert!(cmd.show_streams);
    }

    #[test]
    fn parse_long_switch_case_insensitive() {
        let cmd = CommandLine::parse_from(["/ENV"]).unwrap();
        assert!(cmd.show_env_help);

        let cmd2 = CommandLine::parse_from(["--Owner"]).unwrap();
        assert!(cmd2.show_owner);
    }

    // ── File masks ────────────────────────────────────────────────────────

    #[test]
    fn parse_masks() {
        let cmd = CommandLine::parse_from(["*.rs", "*.toml"]).unwrap();
        assert_eq!(cmd.masks.len(), 2);
        assert_eq!(cmd.masks[0], "*.rs");
        assert_eq!(cmd.masks[1], "*.toml");
    }

    #[test]
    fn parse_mixed_masks_and_switches() {
        let cmd = CommandLine::parse_from(["*.rs", "/s", "*.toml", "/o:n"]).unwrap();
        assert_eq!(cmd.masks.len(), 2);
        assert!(cmd.recurse);
        assert_eq!(cmd.sort_order, SortOrder::Name);
    }

    // ── Config defaults ───────────────────────────────────────────────────

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

    // ── Switch prefix tracking ────────────────────────────────────────────

    #[test]
    fn switch_prefix_tracks_last_used() {
        let cmd = CommandLine::parse_from(["-s", "/w"]).unwrap();
        assert_eq!(cmd.switch_prefix, '/');

        let cmd2 = CommandLine::parse_from(["/s", "-w"]).unwrap();
        assert_eq!(cmd2.switch_prefix, '-');
    }

    // ── Sort preference tiebreaker ────────────────────────────────────────

    #[test]
    fn sort_preference_default() {
        let cmd = CommandLine::default();
        assert_eq!(cmd.sort_preference[0], SortOrder::Default);
        assert_eq!(cmd.sort_preference[1], SortOrder::Name);
        assert_eq!(cmd.sort_preference[2], SortOrder::Date);
        assert_eq!(cmd.sort_preference[3], SortOrder::Extension);
        assert_eq!(cmd.sort_preference[4], SortOrder::Size);
    }

    #[test]
    fn sort_preference_updated_on_sort() {
        let cmd = CommandLine::parse_from(["/o:s"]).unwrap();
        assert_eq!(cmd.sort_preference[0], SortOrder::Size);
        // Rest of chain unchanged
        assert_eq!(cmd.sort_preference[1], SortOrder::Name);
    }
}
