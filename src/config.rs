// config.rs — Color configuration and RCDIR env var overrides
//
// Port of: Config.h, Config.cpp
// Manages display item colors, extension colors, file attribute colors,
// and switch defaults from the RCDIR environment variable.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;

use crate::color::*;
use crate::environment_provider::{DefaultEnvironmentProvider, EnvironmentProvider};
use crate::file_info::FILE_ATTRIBUTE_MAP;





/// Environment variable name
pub const RCDIR_ENV_VAR_NAME: &str = "RCDIR";





////////////////////////////////////////////////////////////////////////////////

/// Display item attribute indices — determines what color is used for each UI element.
/// Port of: Config.h → EAttribute (X-Macro EATTRIBUTE_LIST)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum Attribute {
    Default                           = 0,
    Date                              = 1,
    Time                              = 2,
    FileAttributePresent              = 3,
    FileAttributeNotPresent           = 4,
    Size                              = 5,
    Directory                         = 6,
    Information                       = 7,
    InformationHighlight              = 8,
    SeparatorLine                     = 9,
    Error                             = 10,
    Owner                             = 11,
    Stream                            = 12,
    CloudStatusCloudOnly              = 13,
    CloudStatusLocallyAvailable       = 14,
    CloudStatusAlwaysLocallyAvailable = 15,
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl Attribute
//
//  Attribute enum utility methods and constants.
//
////////////////////////////////////////////////////////////////////////////////

impl Attribute {
    pub const COUNT: usize = 16;

    /// All attribute variants in order, for iteration.
    pub const ALL: [Attribute; Self::COUNT] = [
        Attribute::Default,
        Attribute::Date,
        Attribute::Time,
        Attribute::FileAttributePresent,
        Attribute::FileAttributeNotPresent,
        Attribute::Size,
        Attribute::Directory,
        Attribute::Information,
        Attribute::InformationHighlight,
        Attribute::SeparatorLine,
        Attribute::Error,
        Attribute::Owner,
        Attribute::Stream,
        Attribute::CloudStatusCloudOnly,
        Attribute::CloudStatusLocallyAvailable,
        Attribute::CloudStatusAlwaysLocallyAvailable,
    ];

    ////////////////////////////////////////////////////////////////////////////
    //
    //  from_name
    //
    //  Lookup attribute by name (for {MarkerName} color markers in ColorPrintf).
    //  Case-sensitive match to match TCDir's X-Macro generated table.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn from_name(name: &str) -> Option<Attribute> {
        match name {
            "Default"                           => Some(Attribute::Default),
            "Date"                              => Some(Attribute::Date),
            "Time"                              => Some(Attribute::Time),
            "FileAttributePresent"              => Some(Attribute::FileAttributePresent),
            "FileAttributeNotPresent"           => Some(Attribute::FileAttributeNotPresent),
            "Size"                              => Some(Attribute::Size),
            "Directory"                         => Some(Attribute::Directory),
            "Information"                       => Some(Attribute::Information),
            "InformationHighlight"              => Some(Attribute::InformationHighlight),
            "SeparatorLine"                     => Some(Attribute::SeparatorLine),
            "Error"                             => Some(Attribute::Error),
            "Owner"                             => Some(Attribute::Owner),
            "Stream"                            => Some(Attribute::Stream),
            "CloudStatusCloudOnly"              => Some(Attribute::CloudStatusCloudOnly),
            "CloudStatusLocallyAvailable"       => Some(Attribute::CloudStatusLocallyAvailable),
            "CloudStatusAlwaysLocallyAvailable" => Some(Attribute::CloudStatusAlwaysLocallyAvailable),
            _ => None,
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  name
    //
    //  Get the display name of this attribute.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn name(&self) -> &'static str {
        match self {
            Attribute::Default                           => "Default",
            Attribute::Date                              => "Date",
            Attribute::Time                              => "Time",
            Attribute::FileAttributePresent              => "FileAttributePresent",
            Attribute::FileAttributeNotPresent           => "FileAttributeNotPresent",
            Attribute::Size                              => "Size",
            Attribute::Directory                         => "Directory",
            Attribute::Information                       => "Information",
            Attribute::InformationHighlight              => "InformationHighlight",
            Attribute::SeparatorLine                     => "SeparatorLine",
            Attribute::Error                             => "Error",
            Attribute::Owner                             => "Owner",
            Attribute::Stream                            => "Stream",
            Attribute::CloudStatusCloudOnly              => "CloudStatusCloudOnly",
            Attribute::CloudStatusLocallyAvailable       => "CloudStatusLocallyAvailable",
            Attribute::CloudStatusAlwaysLocallyAvailable => "CloudStatusAlwaysLocallyAvailable",
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_key
    //
    //  Single-char key used in RCDIR env var for display attribute overrides.
    //  Returns None for attributes without a short key.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn env_key(&self) -> Option<char> {
        match self {
            Attribute::Date                    => Some('D'),
            Attribute::Time                    => Some('T'),
            Attribute::FileAttributePresent    => Some('A'),
            Attribute::FileAttributeNotPresent => Some('-'),
            Attribute::Size                    => Some('S'),
            Attribute::Directory               => Some('R'),
            Attribute::Information             => Some('I'),
            Attribute::InformationHighlight    => Some('H'),
            Attribute::Error                   => Some('E'),
            Attribute::Default                 => Some('F'),
            Attribute::Owner                   => Some('O'),
            Attribute::Stream                  => Some('M'),
            _ => None,
        }
    }
}





////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeSource {
    Default,
    Environment,
}





#[derive(Debug, Clone)]
pub struct FileAttrStyle {
    pub attr:   u16,
    pub source: AttributeSource,
}





////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct ErrorInfo {
    pub message:              String,
    pub entry:                String,
    pub invalid_text:         String,
    pub invalid_text_offset:  usize,
}





#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    pub errors: Vec<ErrorInfo>,
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl ValidationResult
//
//  Returns true if there are any validation errors.
//
////////////////////////////////////////////////////////////////////////////////

impl ValidationResult {
    pub fn has_issues(&self) -> bool {
        !self.errors.is_empty()
    }
}





////////////////////////////////////////////////////////////////////////////////

pub struct Config {
    /// Display item colors (indexed by Attribute enum)
    pub attributes:         [u16; Attribute::COUNT],

    /// Source tracking for display item colors
    pub attribute_sources:  [AttributeSource; Attribute::COUNT],

    /// Extension → WORD color mapping (keys are lowercase with leading dot)
    pub extension_colors:   HashMap<String, u16>,

    /// Extension → source tracking
    pub extension_sources:  HashMap<String, AttributeSource>,

    /// File attribute flag → color+source
    pub file_attr_colors:   HashMap<u32, FileAttrStyle>,

    ////////////////////////////////////////////////////////////////////////////

    pub wide_listing:   Option<bool>,
    pub bare_listing:   Option<bool>,
    pub recurse:        Option<bool>,
    pub perf_timer:     Option<bool>,
    pub multi_threaded: Option<bool>,
    pub show_owner:     Option<bool>,
    pub show_streams:   Option<bool>,

    /// Validation results from last env var parse
    pub last_parse_result: ValidationResult,
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl Default for Config
//
//  Default trait implementation for Config.
//
////////////////////////////////////////////////////////////////////////////////

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl Config
//
//  Configuration initialization, color management, and environment parsing.
//
////////////////////////////////////////////////////////////////////////////////

impl Config {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  new
    //
    //  Create a new Config with no defaults initialized.
    //  Call initialize() to set up default colors and parse env var.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn new() -> Self {
        Config {
            attributes:        [0u16; Attribute::COUNT],
            attribute_sources: [AttributeSource::Default; Attribute::COUNT],
            extension_colors:  HashMap::new(),
            extension_sources: HashMap::new(),
            file_attr_colors:  HashMap::new(),
            wide_listing:      None,
            bare_listing:      None,
            recurse:           None,
            perf_timer:        None,
            multi_threaded:    None,
            show_owner:        None,
            show_streams:      None,
            last_parse_result: ValidationResult::default(),
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  initialize
    //
    //  Initialize with default colors and parse RCDIR env var.
    //  default_attr is the console's default text attribute (typically
    //  LightGrey on Black = 0x07).
    //
    //  Port of: CConfig::Initialize
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn initialize(&mut self, default_attr: u16) {
        self.initialize_with_provider(default_attr, &DefaultEnvironmentProvider);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  initialize_with_provider
    //
    //  Initialize with a specific environment provider (for testing).
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn initialize_with_provider(&mut self, default_attr: u16, provider: &dyn EnvironmentProvider) {
        // Set default display item colors
        // Port of: CConfig::Initialize() hardcoded defaults
        self.attributes[Attribute::Default as usize]                           = default_attr;
        self.attributes[Attribute::Date as usize]                              = FC_RED;
        self.attributes[Attribute::Time as usize]                              = FC_BROWN;
        self.attributes[Attribute::FileAttributePresent as usize]              = FC_CYAN;
        self.attributes[Attribute::FileAttributeNotPresent as usize]           = FC_DARK_GREY;
        self.attributes[Attribute::Size as usize]                              = FC_YELLOW;
        self.attributes[Attribute::Directory as usize]                         = FC_LIGHT_BLUE;
        self.attributes[Attribute::Information as usize]                       = FC_CYAN;
        self.attributes[Attribute::InformationHighlight as usize]              = FC_WHITE;
        self.attributes[Attribute::SeparatorLine as usize]                     = FC_LIGHT_BLUE;
        self.attributes[Attribute::Error as usize]                             = FC_LIGHT_RED;
        self.attributes[Attribute::Owner as usize]                             = FC_GREEN;
        self.attributes[Attribute::Stream as usize]                            = FC_DARK_GREY;
        self.attributes[Attribute::CloudStatusCloudOnly as usize]              = FC_LIGHT_BLUE;
        self.attributes[Attribute::CloudStatusLocallyAvailable as usize]       = FC_LIGHT_GREEN;
        self.attributes[Attribute::CloudStatusAlwaysLocallyAvailable as usize] = FC_LIGHT_GREEN;

        self.initialize_extension_colors();
        self.initialize_file_attr_colors();
        self.apply_user_color_overrides(provider);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  get_text_attr_for_file
    //
    //  Resolve which color to use for a file based on priority:
    //  1. File attribute colors (in fixed precedence order)
    //  2. Directory color
    //  3. Extension color
    //  4. Default filename color
    //
    //  Port of: CConfig::GetTextAttrForFile
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn get_text_attr_for_file(&self, file_attributes: u32, file_name: &OsStr) -> u16 {
        let default_attr = self.attributes[Attribute::Default as usize];

        // Check file attribute colors in precedence order
        for &(flag, _) in &FILE_ATTRIBUTE_MAP {
            if (file_attributes & flag) == 0 {
                continue;
            }
            if let Some(style) = self.file_attr_colors.get(&flag) {
                let mut attr = style.attr;
                // Inherit background from default if none set
                if attr & BC_MASK == 0 {
                    attr |= default_attr & BC_MASK;
                }
                return attr;
            }
        }

        // Directory color
        if file_attributes & 0x10 != 0 {
            // FILE_ATTRIBUTE_DIRECTORY = 0x10
            let mut attr = self.attributes[Attribute::Directory as usize];
            if attr & BC_MASK == 0 {
                attr |= default_attr & BC_MASK;
            }
            return attr;
        }

        // Extension color
        let path = Path::new(file_name);
        if let Some(ext_os) = path.extension() {
            let ext_str = format!(".{}", ext_os.to_string_lossy()).to_ascii_lowercase();
            if let Some(&color) = self.extension_colors.get(&ext_str) {
                let mut attr = color;
                if attr & BC_MASK == 0 {
                    attr |= default_attr & BC_MASK;
                }
                return attr;
            }
        }

        // Default
        let mut attr = default_attr;
        if attr & BC_MASK == 0 {
            attr |= default_attr & BC_MASK;
        }
        attr
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  validate_environment_variable
    //
    //  Return the validation result from the last env var parse.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn validate_environment_variable(&self) -> &ValidationResult {
        &self.last_parse_result
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  initialize_extension_colors
    //
    //  Port of: CConfig::InitializeExtensionToTextAttrMap + s_rgTextAttrs[]
    //
    ////////////////////////////////////////////////////////////////////////////

    fn initialize_extension_colors(&mut self) {
        let defaults: &[(&str, u16)] = &[
            // Code — source
            (".asm",   FC_LIGHT_GREEN),
            (".cod",   FC_GREEN),
            (".i",     FC_GREEN),
            (".c",     FC_LIGHT_GREEN),
            (".cpp",   FC_LIGHT_GREEN),
            (".cxx",   FC_LIGHT_GREEN),
            (".h",     FC_LIGHT_GREEN),
            (".hpp",   FC_LIGHT_GREEN),
            (".hxx",   FC_LIGHT_GREEN),
            (".rc",    FC_LIGHT_GREEN),
            (".cs",    FC_LIGHT_GREEN),
            (".resx",  FC_LIGHT_GREEN),
            (".rcml",  FC_LIGHT_GREEN),
            (".js",    FC_LIGHT_GREEN),
            (".jsx",   FC_LIGHT_GREEN),
            (".ts",    FC_LIGHT_GREEN),
            (".tsx",   FC_LIGHT_GREEN),
            (".html",  FC_LIGHT_GREEN),
            (".htm",   FC_LIGHT_GREEN),
            (".css",   FC_LIGHT_GREEN),
            (".scss",  FC_LIGHT_GREEN),
            (".less",  FC_LIGHT_GREEN),
            (".py",    FC_LIGHT_GREEN),
            (".pyw",   FC_LIGHT_GREEN),
            (".jar",   FC_LIGHT_GREEN),
            (".java",  FC_LIGHT_GREEN),
            (".class", FC_LIGHT_GREEN),
            // Config/data
            (".xml",   FC_BROWN),
            (".json",  FC_BROWN),
            (".yml",   FC_BROWN),
            (".yaml",  FC_BROWN),
            // Intermediate
            (".obj",   FC_GREEN),
            (".lib",   FC_GREEN),
            (".res",   FC_GREEN),
            (".pch",   FC_GREEN),
            // Build
            (".wrn",   FC_LIGHT_RED),
            (".err",   FC_LIGHT_RED),
            (".log",   FC_WHITE),
            // Executable
            (".bash",  FC_LIGHT_RED),
            (".bat",   FC_LIGHT_RED),
            (".cmd",   FC_LIGHT_RED),
            (".dll",   FC_LIGHT_CYAN),
            (".exe",   FC_LIGHT_CYAN),
            (".ps1",   FC_LIGHT_RED),
            (".psd1",  FC_LIGHT_RED),
            (".psm1",  FC_LIGHT_RED),
            (".sh",    FC_LIGHT_RED),
            (".sys",   FC_LIGHT_CYAN),
            // Visual Studio
            (".sln",     FC_MAGENTA),
            (".vcproj",  FC_MAGENTA),
            (".csproj",  FC_DARK_GREY),
            (".vcxproj", FC_MAGENTA),
            (".csxproj", FC_DARK_GREY),
            (".user",    FC_DARK_GREY),
            (".ncb",     FC_DARK_GREY),
            // Documents
            (".!!!",   FC_WHITE),
            (".1st",   FC_WHITE),
            (".doc",   FC_WHITE),
            (".docx",  FC_WHITE),
            (".eml",   FC_WHITE),
            (".md",    FC_WHITE),
            (".me",    FC_WHITE),
            (".now",   FC_WHITE),
            (".ppt",   FC_WHITE),
            (".pptx",  FC_WHITE),
            (".text",  FC_WHITE),
            (".txt",   FC_WHITE),
            (".xls",   FC_WHITE),
            (".xlsx",  FC_WHITE),
            // Compressed
            (".7z",    FC_MAGENTA),
            (".arj",   FC_MAGENTA),
            (".gz",    FC_MAGENTA),
            (".rar",   FC_MAGENTA),
            (".tar",   FC_MAGENTA),
            (".zip",   FC_MAGENTA),
        ];

        for &(ext, color) in defaults {
            self.extension_colors.insert(ext.to_ascii_lowercase(), color);
            self.extension_sources.insert(ext.to_ascii_lowercase(), AttributeSource::Default);
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  initialize_file_attr_colors
    //
    //  Port of: CConfig::InitializeFileAttributeToTextAttrMap
    //
    ////////////////////////////////////////////////////////////////////////////

    fn initialize_file_attr_colors(&mut self) {
        self.file_attr_colors.clear();

        // Hidden files → DarkGrey
        self.file_attr_colors.insert(0x02, FileAttrStyle {
            attr:   FC_DARK_GREY,
            source: AttributeSource::Default,
        });

        // Encrypted files → LightGreen
        self.file_attr_colors.insert(0x4000, FileAttrStyle {
            attr:   FC_LIGHT_GREEN,
            source: AttributeSource::Default,
        });
    }





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

    fn apply_user_color_overrides(&mut self, provider: &dyn EnvironmentProvider) {
        self.last_parse_result.errors.clear();

        let env_value = match provider.get_env_var(RCDIR_ENV_VAR_NAME) {
            Some(v) => v,
            None => return,
        };

        for entry_raw in env_value.split(';') {
            let entry = entry_raw.trim();
            if entry.is_empty() {
                continue;
            }
            self.process_color_override_entry(entry);
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
            self.last_parse_result.errors.push(ErrorInfo {
                message:             "Switch prefixes (/, -, --) are not allowed in env var".into(),
                entry:               entry.into(),
                invalid_text:        entry[..prefix_len].into(),
                invalid_text_offset: 0,
            });
            return;
        }

        // Check if it's a switch name
        if is_switch_name(entry) {
            self.process_switch_override(entry);
            return;
        }

        // Parse key=value
        let (key, value) = match parse_key_and_value(entry) {
            Some(kv) => kv,
            None => {
                self.last_parse_result.errors.push(ErrorInfo {
                    message:             "Invalid entry format (expected key = value)".into(),
                    entry:               entry.into(),
                    invalid_text:        entry.into(),
                    invalid_text_offset: 0,
                });
                return;
            }
        };

        // Parse color value
        let color_attr = match self.parse_color_value(entry, value) {
            Some(c) => c,
            None => return, // Error already recorded
        };

        // Apply based on key type
        if key.starts_with('.') {
            // Extension color override
            self.process_file_extension_override(key, color_attr);
        } else if key.len() == 6
            && key[..5].eq_ignore_ascii_case("attr:")
        {
            // File attribute color override
            self.process_file_attribute_override(key, color_attr, entry);
        } else if key.len() == 1 {
            // Display attribute override
            self.process_display_attribute_override(key.chars().next().unwrap(), color_attr, entry);
        } else {
            self.last_parse_result.errors.push(ErrorInfo {
                message:             "Invalid key (expected single character, .extension, or attr:x)".into(),
                entry:               entry.into(),
                invalid_text:        key.into(),
                invalid_text_offset: entry.find(key).unwrap_or(0),
            });
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
        let lower = value.to_ascii_lowercase();
        let on_pos = lower.find(" on ");

        let (fore_str, back_str) = if let Some(pos) = on_pos {
            (value[..pos].trim(), Some(value[pos + 4..].trim()))
        } else {
            (value.trim(), None)
        };

        let fore = match parse_color_name(fore_str, false) {
            Ok(v) => v,
            Err(_) => {
                let equal_pos = entry.find('=').unwrap_or(0);
                let fore_offset = equal_pos + 1 + entry[equal_pos + 1..].find(|c: char| !c.is_whitespace()).unwrap_or(0);
                self.last_parse_result.errors.push(ErrorInfo {
                    message:             "Invalid foreground color".into(),
                    entry:               entry.into(),
                    invalid_text:        fore_str.into(),
                    invalid_text_offset: fore_offset,
                });
                return None;
            }
        };

        let back = if let Some(bs) = back_str {
            if !bs.is_empty() {
                parse_color_name(bs, true).unwrap_or_else(|_| {
                    let equal_pos = entry.find('=').unwrap_or(0);
                    let on_in_entry = lower.find(" on ").unwrap_or(0);
                    let back_offset = equal_pos + 1 + on_in_entry + 4;
                    self.last_parse_result.errors.push(ErrorInfo {
                        message:             "Invalid background color".into(),
                        entry:               entry.into(),
                        invalid_text:        bs.into(),
                        invalid_text_offset: back_offset,
                    });
                    0
                })
            } else {
                0
            }
        } else {
            0
        };

        Some(fore | back)
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  process_switch_override
    //
    //  Process a switch override from env var (e.g., "W", "M-", "Owner",
    //  "Streams").
    //
    //  Port of: CConfig::ProcessSwitchOverride + ProcessLongSwitchOverride
    //
    ////////////////////////////////////////////////////////////////////////////

    fn process_switch_override(&mut self, entry: &str) {
        // Try long switches first
        if entry.len() >= 5 {
            if entry.eq_ignore_ascii_case("owner") {
                self.show_owner = Some(true);
                return;
            }
            if entry.eq_ignore_ascii_case("streams") {
                self.show_streams = Some(true);
                return;
            }
        }

        // Short switches: single char, optional trailing '-'
        let (ch, value) = match entry.len() {
            1 => (entry.chars().next().unwrap(), true),
            2 if entry.as_bytes()[1] == b'-' => (entry.chars().next().unwrap(), false),
            _ => {
                self.last_parse_result.errors.push(ErrorInfo {
                    message:             "Invalid switch (expected W, S, P, M, B, Owner, or Streams)".into(),
                    entry:               entry.into(),
                    invalid_text:        entry.into(),
                    invalid_text_offset: 0,
                });
                return;
            }
        };

        match ch.to_ascii_lowercase() {
            's' => { self.recurse        = Some(value); }
            'w' => { self.wide_listing   = Some(value); }
            'b' => { self.bare_listing   = Some(value); }
            'p' => { self.perf_timer     = Some(value); }
            'm' => { self.multi_threaded = Some(value); }
            _ => {
                self.last_parse_result.errors.push(ErrorInfo {
                    message:             "Invalid switch (expected W, S, P, M, B, Owner, or Streams)".into(),
                    entry:               entry.into(),
                    invalid_text:        entry[..1].into(),
                    invalid_text_offset: 0,
                });
            }
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  process_file_extension_override
    //
    //  Apply a file extension color override from the env var.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn process_file_extension_override(&mut self, key: &str, color_attr: u16) {
        let lower_key = key.to_ascii_lowercase();
        self.extension_colors.insert(lower_key.clone(), color_attr);
        self.extension_sources.insert(lower_key, AttributeSource::Environment);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  process_display_attribute_override
    //
    //  Apply a display attribute color override from the env var.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn process_display_attribute_override(&mut self, ch: char, color_attr: u16, entry: &str) {
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
}





////////////////////////////////////////////////////////////////////////////////
//
//  is_switch_name
//
//  Check if an entry is a valid switch name (no prefix).
//  Valid: W, S, P, M, B, M-, Owner, Streams (case-insensitive)
//
//  Port of: CConfig::IsSwitchName
//
////////////////////////////////////////////////////////////////////////////////

fn is_switch_name(entry: &str) -> bool {
    // Single-letter switches (optionally with '-' suffix)
    if entry.len() == 1 || (entry.len() == 2 && entry.as_bytes()[1] == b'-') {
        let ch = entry.as_bytes()[0].to_ascii_lowercase();
        return ch == b'w' || ch == b's' || ch == b'p' || ch == b'm' || ch == b'b';
    }

    // Long switch names
    if entry.len() == 5 && entry.eq_ignore_ascii_case("owner") {
        return true;
    }
    if entry.len() == 7 && entry.eq_ignore_ascii_case("streams") {
        return true;
    }

    false
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





////////////////////////////////////////////////////////////////////////////////
//
//  impl std::fmt::Debug for Config
//
//  Debug trait implementation for Config.
//
////////////////////////////////////////////////////////////////////////////////

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("extension_count", &self.extension_colors.len())
            .field("file_attr_count", &self.file_attr_colors.len())
            .finish()
    }
}





#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment_provider::MockEnvironmentProvider;


    ////////////////////////////////////////////////////////////////////////////
    //
    //  make_config
    //
    //  Test helper: creates a Config with optional RCDIR env var value.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn make_config(env_value: Option<&str>) -> Config {
        let mut config = Config::new();
        let mut mock = MockEnvironmentProvider::new();
        if let Some(val) = env_value {
            mock.set(RCDIR_ENV_VAR_NAME, val);
        }
        config.initialize_with_provider(FC_LIGHT_GREY, &mock);
        config
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  default_date_color
    //
    //  Verifies the default date color is Red.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn default_date_color() {
        let config = make_config(None);
        assert_eq!(config.attributes[Attribute::Date as usize], FC_RED);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  default_size_color
    //
    //  Verifies the default size color is Yellow.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn default_size_color() {
        let config = make_config(None);
        assert_eq!(config.attributes[Attribute::Size as usize], FC_YELLOW);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  default_directory_color
    //
    //  Verifies the default directory color is LightBlue.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn default_directory_color() {
        let config = make_config(None);
        assert_eq!(config.attributes[Attribute::Directory as usize], FC_LIGHT_BLUE);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  default_extension_count
    //
    //  Verifies the default extension count is at least 70.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn default_extension_count() {
        let config = make_config(None);
        // Should have all the default extensions
        assert!(config.extension_colors.len() >= 70);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  default_cpp_color
    //
    //  Verifies the default .cpp color is LightGreen.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn default_cpp_color() {
        let config = make_config(None);
        assert_eq!(*config.extension_colors.get(".cpp").unwrap(), FC_LIGHT_GREEN);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  default_zip_color
    //
    //  Verifies the default .zip color is Magenta.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn default_zip_color() {
        let config = make_config(None);
        assert_eq!(*config.extension_colors.get(".zip").unwrap(), FC_MAGENTA);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_switch_wide
    //
    //  Verifies the W switch enables wide listing.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_switch_wide() {
        let config = make_config(Some("W"));
        assert_eq!(config.wide_listing, Some(true));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_switch_disable
    //
    //  Verifies the M- switch disables multi-threading.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_switch_disable() {
        let config = make_config(Some("M-"));
        assert_eq!(config.multi_threaded, Some(false));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_switch_multiple
    //
    //  Verifies multiple switches (W;S;P) are parsed correctly.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_switch_multiple() {
        let config = make_config(Some("W;S;P"));
        assert_eq!(config.wide_listing, Some(true));
        assert_eq!(config.recurse, Some(true));
        assert_eq!(config.perf_timer, Some(true));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_switch_owner
    //
    //  Verifies the Owner switch enables owner display.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_switch_owner() {
        let config = make_config(Some("Owner"));
        assert_eq!(config.show_owner, Some(true));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_switch_streams
    //
    //  Verifies the Streams switch enables streams display.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_switch_streams() {
        let config = make_config(Some("Streams"));
        assert_eq!(config.show_streams, Some(true));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_display_attribute_override
    //
    //  Verifies a display attribute color override (D=LightGreen).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_display_attribute_override() {
        let config = make_config(Some("D=LightGreen"));
        assert_eq!(config.attributes[Attribute::Date as usize], FC_LIGHT_GREEN);
        assert_eq!(config.attribute_sources[Attribute::Date as usize], AttributeSource::Environment);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_extension_override
    //
    //  Verifies an extension color override (.rs=Cyan).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_extension_override() {
        let config = make_config(Some(".rs=Cyan"));
        assert_eq!(*config.extension_colors.get(".rs").unwrap(), FC_CYAN);
        assert_eq!(*config.extension_sources.get(".rs").unwrap(), AttributeSource::Environment);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_extension_override_case_insensitive
    //
    //  Verifies extension overrides are case-insensitive.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_extension_override_case_insensitive() {
        let config = make_config(Some(".RS=Yellow"));
        // Stored lowercase
        assert_eq!(*config.extension_colors.get(".rs").unwrap(), FC_YELLOW);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_color_with_background
    //
    //  Verifies a color override with background (D=LightCyan on Blue).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_color_with_background() {
        let config = make_config(Some("D=LightCyan on Blue"));
        assert_eq!(config.attributes[Attribute::Date as usize], FC_LIGHT_CYAN | BC_BLUE);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_file_attribute_override
    //
    //  Verifies a file attribute color override (Attr:H=Yellow).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_file_attribute_override() {
        let config = make_config(Some("Attr:H=Yellow"));
        let style = config.file_attr_colors.get(&0x02).unwrap();
        assert_eq!(style.attr, FC_YELLOW);
        assert_eq!(style.source, AttributeSource::Environment);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_prefix_error
    //
    //  Verifies switch prefixes (/, -) produce an error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_prefix_error() {
        let config = make_config(Some("/W"));
        assert!(config.last_parse_result.has_issues());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_invalid_color_error
    //
    //  Verifies an invalid color name produces an error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_invalid_color_error() {
        let config = make_config(Some("D=Purple"));
        assert!(config.last_parse_result.has_issues());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_combined_valid_and_invalid
    //
    //  Verifies valid and invalid entries can coexist in the env var.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_combined_valid_and_invalid() {
        let config = make_config(Some("W;D=Purple;S=Yellow"));
        assert_eq!(config.wide_listing, Some(true));
        // D=Purple fails, S=Yellow succeeds
        assert_eq!(config.attributes[Attribute::Size as usize], FC_YELLOW);
        assert!(config.last_parse_result.has_issues());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  text_attr_for_cpp_file
    //
    //  Verifies .cpp files get the LightGreen color.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn text_attr_for_cpp_file() {
        let config = make_config(None);
        let attr = config.get_text_attr_for_file(0x20, OsStr::new("test.cpp"));
        // .cpp default = FC_LIGHT_GREEN
        assert_eq!(attr & FC_MASK, FC_LIGHT_GREEN);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  text_attr_for_directory
    //
    //  Verifies directories get the LightBlue color.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn text_attr_for_directory() {
        let config = make_config(None);
        let attr = config.get_text_attr_for_file(0x10, OsStr::new("subdir"));
        assert_eq!(attr & FC_MASK, FC_LIGHT_BLUE);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  text_attr_for_hidden_file
    //
    //  Verifies hidden files get the DarkGrey color.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn text_attr_for_hidden_file() {
        let config = make_config(None);
        let attr = config.get_text_attr_for_file(0x02, OsStr::new("hidden.txt"));
        assert_eq!(attr & FC_MASK, FC_DARK_GREY);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  text_attr_hidden_overrides_extension
    //
    //  Verifies hidden attribute overrides extension color.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn text_attr_hidden_overrides_extension() {
        let config = make_config(None);
        // Hidden + .cpp → hidden color wins (file attr has priority over extension)
        let attr = config.get_text_attr_for_file(0x22, OsStr::new("secret.cpp"));
        assert_eq!(attr & FC_MASK, FC_DARK_GREY);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  text_attr_for_unknown_extension
    //
    //  Verifies unknown extensions get the default color.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn text_attr_for_unknown_extension() {
        let config = make_config(None);
        let attr = config.get_text_attr_for_file(0x20, OsStr::new("data.xyz"));
        // Should get default color
        assert_eq!(attr & FC_MASK, FC_LIGHT_GREY);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  attribute_from_name_valid
    //
    //  Verifies valid attribute names resolve correctly.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn attribute_from_name_valid() {
        assert_eq!(Attribute::from_name("Date"), Some(Attribute::Date));
        assert_eq!(Attribute::from_name("Error"), Some(Attribute::Error));
        assert_eq!(Attribute::from_name("Default"), Some(Attribute::Default));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  attribute_from_name_invalid
    //
    //  Verifies invalid attribute names return None.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn attribute_from_name_invalid() {
        assert_eq!(Attribute::from_name("invalid"), None);
        assert_eq!(Attribute::from_name(""), None);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  attribute_name_roundtrip
    //
    //  Verifies all attribute names round-trip through from_name.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn attribute_name_roundtrip() {
        for attr in &Attribute::ALL {
            let name = attr.name();
            assert_eq!(Attribute::from_name(name), Some(*attr));
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  attribute_count
    //
    //  Verifies the attribute count is 16.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn attribute_count() {
        assert_eq!(Attribute::COUNT, 16);
        assert_eq!(Attribute::ALL.len(), 16);
    }
}
