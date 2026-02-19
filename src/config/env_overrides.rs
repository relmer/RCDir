// env_overrides.rs — RCDIR environment variable parsing and override application
//
// Port of: CConfig::ApplyUserColorOverrides and related parsing logic
//
// Extends impl Config with all methods that parse the RCDIR environment
// variable and apply color, icon, and switch overrides.

use crate::color::parse_color_name;
use crate::environment_provider::EnvironmentProvider;
use crate::file_info::FILE_ATTRIBUTE_MAP;

use super::{
    Attribute, AttributeSource, Config, ErrorInfo, FileAttrStyle,
    RCDIR_ENV_VAR_NAME,
};





////////////////////////////////////////////////////////////////////////////////
//
//  impl Config — env var parsing methods
//
//  All methods in this block handle reading and applying the RCDIR
//  environment variable.  Separated from the main impl block for
//  readability.
//
////////////////////////////////////////////////////////////////////////////////

impl Config {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  apply_user_color_overrides
    //
    //  Parse the RCDIR environment variable for user color overrides and
    //  switch defaults.
    //
    //  Port of: CConfig::ApplyUserColorOverrides
    //
    ////////////////////////////////////////////////////////////////////////////

    pub(super) fn apply_user_color_overrides(&mut self, provider: &dyn EnvironmentProvider) {
        self.last_parse_result.errors.clear();

        let env_value = match provider.get_env_var (RCDIR_ENV_VAR_NAME) {
            Some (v) => v,
            None => return,
        };

        for entry_raw in env_value.split (';') {
            let entry = entry_raw.trim();
            if entry.is_empty() {
                continue;
            }
            self.process_color_override_entry (entry);
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_icon_value
    //
    //  Parse an icon specification string into a code point.
    //
    //  Formats:
    //    ""           → suppressed (icon_suppressed = true, returns None)
    //    "X"          → single BMP codepoint (literal glyph)
    //    "U+XXXX"     → hex code point notation (4–6 hex digits)
    //
    //  Note: Rust's char type cannot represent UTF-16 surrogates, so a
    //  two-character string is always routed to U+XXXX parsing (which
    //  will reject it if it lacks the "U+" prefix).
    //
    ////////////////////////////////////////////////////////////////////////////

    fn parse_icon_value(icon_spec: &str, entry: &str, errors: &mut Vec<ErrorInfo>) -> Option<(char, bool)> {
        let trimmed = icon_spec.trim();

        // Empty → suppressed
        if trimmed.is_empty() {
            return Some (('\0', true));
        }

        let chars: Vec<char> = trimmed.chars().collect();

        match chars.len() {
            1 => Self::parse_single_glyph (&chars, trimmed, entry, errors),
            _ => Self::parse_uplus_notation (trimmed, entry, errors),
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_single_glyph
    //
    //  Parse a single BMP character as an icon glyph.  Rejects lone
    //  surrogates (can't happen with Rust's char, but be safe).
    //
    ////////////////////////////////////////////////////////////////////////////

    fn parse_single_glyph(chars: &[char], trimmed: &str, entry: &str, errors: &mut Vec<ErrorInfo>) -> Option<(char, bool)> {
        let c  = chars[0];
        let cp = c as u32;

        if (0xD800..=0xDFFF).contains (&cp) {
            errors.push (ErrorInfo {
                message:             "Invalid icon: lone surrogate".into(),
                entry:               entry.into(),
                invalid_text:        trimmed.into(),
                invalid_text_offset: entry.find (trimmed).unwrap_or (0),
            });
            return None;
        }

        Some ((c, false))
    }











    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_uplus_notation
    //
    //  Parse a "U+XXXX" hex code point notation (4–6 hex digits) into an
    //  icon character.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn parse_uplus_notation(trimmed: &str, entry: &str, errors: &mut Vec<ErrorInfo>) -> Option<(char, bool)> {

        fn try_parse(trimmed: &str) -> Result<(char, bool), &'static str> {
            if !trimmed.starts_with ("U+") && !trimmed.starts_with ("u+") {
                return Err ("Invalid icon: expected single glyph or U+XXXX");
            }

            let hex_str = &trimmed[2..];
            if hex_str.len() < 4 || hex_str.len() > 6 {
                return Err ("Invalid icon: U+ requires 4-6 hex digits");
            }

            let cp = u32::from_str_radix (hex_str, 16)
                .ok()
                .filter (|&cp| (1..=0x10FFFF).contains (&cp) && !(0xD800..=0xDFFF).contains (&cp))
                .ok_or ("Invalid icon: code point out of range (U+0001..U+10FFFF)")?;

            char::from_u32 (cp)
                .map (|c| (c, false))
                .ok_or ("Invalid icon: code point is not a valid Unicode character")
        }

        match try_parse (trimmed) {
            Ok (result) => Some (result),
            Err (message) => {
                errors.push (ErrorInfo {
                    message:             message.into(),
                    entry:               entry.into(),
                    invalid_text:        trimmed.into(),
                    invalid_text_offset: entry.find (trimmed).unwrap_or (0),
                });
                None
            }
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  process_color_override_entry
    //
    //  Process a single entry from the RCDIR env var.
    //
    //  Port of: CConfig::ProcessColorOverrideEntry
    //
    ////////////////////////////////////////////////////////////////////////////

    fn process_color_override_entry(&mut self, entry: &str) {
        // Check for switch prefixes (/, -, --) — not allowed in env var
        if entry.starts_with('/') || entry.starts_with('-') {
            let prefix_len = if entry.starts_with("--") { 2 } else { 1 };
            self.last_parse_result.errors.push (ErrorInfo {
                message:             "Switch prefixes (/, -, --) are not allowed in env var".into(),
                entry:               entry.into(),
                invalid_text:        entry[..prefix_len].into(),
                invalid_text_offset: 0,
            });
            return;
        }

        // Check if it's a switch name
        if is_switch_name (entry) {
            self.process_switch_override (entry);
            return;
        }

        // Parse key=value
        let (key, value) = match parse_key_and_value (entry) {
            Some (kv) => kv,
            None => {
                self.last_parse_result.errors.push (ErrorInfo {
                    message:             "Invalid entry format (expected key = value)".into(),
                    entry:               entry.into(),
                    invalid_text:        entry.into(),
                    invalid_text_offset: 0,
                });
                return;
            }
        };

        // Parse color and icon from value
        let (color_attr, icon_result) = match self.parse_color_and_icon (entry, value) {
            Some (pair) => pair,
            None => return,
        };

        // Dispatch to the right apply function
        self.apply_key_override (key, color_attr, icon_result, entry);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_color_and_icon
    //
    //  Split a value string on the first comma into "color,icon", parse
    //  each part, and return the results.  Returns None only when the
    //  color portion fails (icon failures are non-fatal — the color
    //  portion is still applied).
    //
    ////////////////////////////////////////////////////////////////////////////

    fn parse_color_and_icon(&mut self, entry: &str, value: &str)
        -> Option<(Option<u16>, Option<(char, bool)>)>
    {
        let (color_view, icon_view, has_comma) = if let Some (comma_pos) = value.find (',') {
            let color_part = value[..comma_pos].trim();
            let icon_part  = &value[comma_pos + 1..];
            (color_part, Some (icon_part), true)
        } else {
            (value, None, false)
        };

        let color_attr = if !color_view.is_empty() {
            match self.parse_color_value (entry, color_view) {
                Some (c) => Some (c),
                None => return None,
            }
        } else {
            None
        };

        let mut icon_result: Option<(char, bool)> = None;
        if has_comma {
            let icon_spec = icon_view.unwrap_or ("");
            icon_result = Self::parse_icon_value (icon_spec, entry, &mut self.last_parse_result.errors);
        }

        Some ((color_attr, icon_result))
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  apply_key_override
    //
    //  Dispatch a parsed color+icon override to the correct apply function
    //  based on the key prefix (.ext, dir:name, attr:x, or single char).
    //
    //  Port of: second half of CConfig::ProcessColorOverrideEntry
    //
    ////////////////////////////////////////////////////////////////////////////

    fn apply_key_override(&mut self, key: &str, color_attr: Option<u16>, icon_result: Option<(char, bool)>, entry: &str) {
        type ColorHandler = fn(&mut Config, &str, u16, &str);
        type IconHandler  = fn(&mut Config, &str, char, bool, &str);

        let (color_handler, icon_handler): (Option<ColorHandler>, Option<IconHandler>) = match classify_key (key) {
            KeyType::Extension      => (Some (Config::process_file_extension_override),    Some (Config::apply_extension_icon_override)),
            KeyType::WellKnownDir   => (None,                                              Some (Config::apply_well_known_dir_icon_override)),
            KeyType::FileAttribute  => (Some (Config::process_file_attribute_override),    Some (Config::apply_file_attribute_icon_override)),
            KeyType::DisplayAttr    => (Some (Config::process_display_attribute_override), None),
            KeyType::Invalid => {
                self.last_parse_result.errors.push (ErrorInfo {
                    message:             "Invalid key (expected single character, .extension, dir:name, or attr:x)".into(),
                    entry:               entry.into(),
                    invalid_text:        key.into(),
                    invalid_text_offset: entry.find (key).unwrap_or (0),
                });
                return;
            }
        };

        if let (Some (handler), Some (attr)) = (color_handler, color_attr) {
            handler (self, key, attr, entry);
        }

        if let (Some (handler), Some ((cp, suppressed))) = (icon_handler, icon_result) {
            handler (self, key, cp, suppressed, entry);
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_color_value
    //
    //  Parse a color value in format: "FgColor [on BgColor]"
    //
    //  Port of: CConfig::ParseColorValue
    //
    ////////////////////////////////////////////////////////////////////////////

    fn parse_color_value(&mut self, entry: &str, value: &str) -> Option<u16> {

        fn try_parse(value: &str) -> Result<u16, (&'static str, &str)> {
            let lower = value.to_ascii_lowercase();
            let (fore_str, back_str) = match lower.find (" on ") {
                Some (pos) => (value[..pos].trim(), Some (value[pos + 4..].trim())),
                None       => (value.trim(), None),
            };

            let fore = parse_color_name (fore_str, false)
                .map_err (|_| ("Invalid foreground color", fore_str))?;

            let back = match back_str {
                Some (bs) if !bs.is_empty() => parse_color_name (bs, true)
                    .map_err (|_| ("Invalid background color", bs))?,
                _ => 0,
            };

            if back != 0 && fore == (back >> 4) {
                return Err (("Foreground and background colors are the same", value));
            }

            Ok (fore | back)
        }

        match try_parse (value) {
            Ok (attr) => Some (attr),
            Err ((message, bad_text)) => {
                self.last_parse_result.errors.push (ErrorInfo {
                    message:             message.into(),
                    entry:               entry.into(),
                    invalid_text:        bad_text.into(),
                    invalid_text_offset: entry.find (bad_text).unwrap_or (0),
                });
                None
            }
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  process_switch_override
    //
    //  Look up entry in the SWITCH_MAPPINGS table (case-insensitive).
    //  On match, set the corresponding Option<bool> field.
    //  On miss, record a parse error.
    //
    //  Port of: CConfig::ProcessSwitchOverride
    //
    ////////////////////////////////////////////////////////////////////////////

    fn process_switch_override(&mut self, entry: &str) {
        for &(name, value, accessor) in SWITCH_MAPPINGS {
            if entry.eq_ignore_ascii_case (name) {
                *accessor (self) = Some (value);
                return;
            }
        }

        self.last_parse_result.errors.push (ErrorInfo {
            message:             "Invalid switch (expected W, S, P, M, B, Owner, or Streams)".into(),
            entry:               entry.into(),
            invalid_text:        entry.into(),
            invalid_text_offset: 0,
        });
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  process_file_extension_override
    //
    //  Apply a file extension color override from the env var.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn process_file_extension_override(&mut self, key: &str, color_attr: u16, _entry: &str) {
        let lower_key = key.to_ascii_lowercase();
        self.extension_colors.insert (lower_key.clone(), color_attr);
        self.extension_sources.insert (lower_key, AttributeSource::Environment);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  process_display_attribute_override
    //
    //  Apply a display attribute color override from the env var.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn process_display_attribute_override(&mut self, key: &str, color_attr: u16, entry: &str) {
        let ch = key.chars().next().unwrap();
        let ch_upper = ch.to_ascii_uppercase();

        // Find matching attribute by env key
        for attr in &Attribute::ALL {
            if attr.env_key() == Some(ch_upper) {
                self.attributes[*attr as usize] = color_attr;
                self.attribute_sources[*attr as usize] = AttributeSource::Environment;
                return;
            }
        }

        self.last_parse_result.errors.push(ErrorInfo {
            message:             "Invalid display attribute character (valid: D,T,A,-,S,R,I,H,E,F,O)".into(),
            entry:               entry.into(),
            invalid_text:        ch.to_string(),
            invalid_text_offset: 0,
        });
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  process_file_attribute_override
    //
    //  Apply a file attribute color override from the env var.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn process_file_attribute_override(&mut self, key: &str, color_attr: u16, entry: &str) {
        // Format: attr:X where X is the attribute char
        if key.len() != 6 || !key[..5].eq_ignore_ascii_case("attr:") {
            self.last_parse_result.errors.push(ErrorInfo {
                message:             "Invalid file attribute key (expected attr:<x>)".into(),
                entry:               entry.into(),
                invalid_text:        key.into(),
                invalid_text_offset: entry.find(key).unwrap_or(0),
            });
            return;
        }

        let attr_char = key.as_bytes()[5].to_ascii_uppercase() as char;

        // Look up in FILE_ATTRIBUTE_MAP
        for &(flag, map_char) in &FILE_ATTRIBUTE_MAP {
            if map_char == attr_char {
                self.file_attr_colors.insert(flag, FileAttrStyle {
                    attr:   color_attr,
                    source: AttributeSource::Environment,
                });
                return;
            }
        }

        let key_pos = entry.find(key).unwrap_or(0);
        self.last_parse_result.errors.push(ErrorInfo {
            message:             "Invalid file attribute character (expected R, H, S, A, T, E, C, P or 0)".into(),
            entry:               entry.into(),
            invalid_text:        attr_char.to_string(),
            invalid_text_offset: key_pos + 5,
        });
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  apply_extension_icon_override
    //
    //  Apply a user icon override for a file extension from the env var.
    //  First-write-wins for environment overrides.
    //
    //  Port of: CConfig::ApplyIconOverride (extension path)
    //
    ////////////////////////////////////////////////////////////////////////////

    fn apply_extension_icon_override(&mut self, key: &str, icon_cp: char, suppressed: bool, _entry: &str) {
        let lower_key = key.to_ascii_lowercase();

        // First-write-wins: if already set by environment, report duplicate
        if let Some (&AttributeSource::Environment) = self.extension_icon_sources.get (&lower_key) {
            self.last_parse_result.errors.push (ErrorInfo {
                message:             "Duplicate extension icon override (first value wins)".into(),
                entry:               String::new(),
                invalid_text:        lower_key.clone(),
                invalid_text_offset: 0,
            });
            return;
        }

        let glyph = if suppressed { '\0' } else { icon_cp };
        self.extension_icons.insert (lower_key.clone(), glyph);
        self.extension_icon_sources.insert (lower_key, AttributeSource::Environment);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  apply_well_known_dir_icon_override
    //
    //  Apply a user icon override for a well-known directory from the env var.
    //  First-write-wins for environment overrides.
    //
    //  Port of: CConfig::ApplyIconOverride (well-known dir path)
    //
    ////////////////////////////////////////////////////////////////////////////

    fn apply_well_known_dir_icon_override(&mut self, key: &str, icon_cp: char, suppressed: bool, _entry: &str) {
        let dir_name = &key[4..];
        let lower_key = dir_name.to_ascii_lowercase();

        if let Some (&AttributeSource::Environment) = self.well_known_dir_icon_sources.get (&lower_key) {
            self.last_parse_result.errors.push (ErrorInfo {
                message:             "Duplicate well-known dir icon override (first value wins)".into(),
                entry:               String::new(),
                invalid_text:        lower_key.clone(),
                invalid_text_offset: 0,
            });
            return;
        }

        let glyph = if suppressed { '\0' } else { icon_cp };
        self.well_known_dir_icons.insert (lower_key.clone(), glyph);
        self.well_known_dir_icon_sources.insert (lower_key, AttributeSource::Environment);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  apply_file_attribute_icon_override
    //
    //  Apply a user icon override for a file attribute from the env var.
    //
    //  Port of: CConfig::ProcessFileAttributeIconOverride
    //
    ////////////////////////////////////////////////////////////////////////////

    fn apply_file_attribute_icon_override(&mut self, key: &str, icon_cp: char, suppressed: bool, _entry: &str) {
        let attr_char = key.as_bytes()[5] as char;
        let attr_upper = attr_char.to_ascii_uppercase();
        let glyph = if suppressed { '\0' } else { icon_cp };

        for &(flag, map_char) in &FILE_ATTRIBUTE_MAP {
            if map_char == attr_upper {
                self.file_attr_icons.insert (flag, glyph);
                return;
            }
        }
        // Invalid attr char — error already handled by color override path
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  KeyType
//
//  Classification of an override key from the RCDIR env var.
//
////////////////////////////////////////////////////////////////////////////////

enum KeyType {
    Extension,
    WellKnownDir,
    FileAttribute,
    DisplayAttr,
    Invalid,
}





////////////////////////////////////////////////////////////////////////////////
//
//  classify_key
//
//  Classify an override key by its prefix/shape.  Pure function with no
//  side effects — just pattern recognition.
//
////////////////////////////////////////////////////////////////////////////////

fn classify_key(key: &str) -> KeyType {
    match key.as_bytes() {
        [b'.', ..]                                                         => KeyType::Extension,
        [_, _, _, _, _, ..] if key[..4].eq_ignore_ascii_case ("dir:")      => KeyType::WellKnownDir,
        [_, _, _, _, _, _] if key[..5].eq_ignore_ascii_case ("attr:")      => KeyType::FileAttribute,
        [_]                                                                => KeyType::DisplayAttr,
        _                                                                  => KeyType::Invalid,
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  SWITCH_MAPPINGS
//
//  Table-driven switch dispatch.  Each entry maps a name
//  (case-insensitive) to a boolean value and a field accessor.
//
//  Port of: CConfig::s_switchMappings[]
//
////////////////////////////////////////////////////////////////////////////////

type SwitchAccessor = fn(&mut Config) -> &mut Option<bool>;

const SWITCH_MAPPINGS: &[(&str, bool, SwitchAccessor)] = &[
    ("s",       true,  |c| &mut c.recurse),
    ("s-",      false, |c| &mut c.recurse),
    ("w",       true,  |c| &mut c.wide_listing),
    ("w-",      false, |c| &mut c.wide_listing),
    ("b",       true,  |c| &mut c.bare_listing),
    ("b-",      false, |c| &mut c.bare_listing),
    ("p",       true,  |c| &mut c.perf_timer),
    ("p-",      false, |c| &mut c.perf_timer),
    ("m",       true,  |c| &mut c.multi_threaded),
    ("m-",      false, |c| &mut c.multi_threaded),
    ("owner",   true,  |c| &mut c.show_owner),
    ("streams", true,  |c| &mut c.show_streams),
    ("icons",   true,  |c| &mut c.icons),
    ("icons-",  false, |c| &mut c.icons),
];





////////////////////////////////////////////////////////////////////////////////
//
//  is_switch_name
//
//  Check if entry matches any name in the SWITCH_MAPPINGS table
//  (case-insensitive).
//
//  Port of: CConfig::IsSwitchName
//
////////////////////////////////////////////////////////////////////////////////

fn is_switch_name(entry: &str) -> bool {
    SWITCH_MAPPINGS.iter().any (|&(name, _, _)| entry.eq_ignore_ascii_case (name))
}





////////////////////////////////////////////////////////////////////////////////
//
//  parse_key_and_value
//
//  Split an entry on '=' into key and value, trimming whitespace.
//
////////////////////////////////////////////////////////////////////////////////

fn parse_key_and_value(entry: &str) -> Option<(&str, &str)> {
    let eq_pos = entry.find('=')?;
    let key = entry[..eq_pos].trim();
    let value = entry[eq_pos + 1..].trim();

    if key.is_empty() || value.is_empty() {
        return None;
    }

    Some((key, value))
}
