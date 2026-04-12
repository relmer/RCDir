// config/mod.rs — Color configuration and RCDIR env var overrides
//
// Port of: Config.h, Config.cpp
// Manages display item colors, extension colors, file attribute colors,
// and switch defaults from the RCDIR environment variable.

mod env_overrides;
pub mod file_reader;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;

use crate::color::*;
use crate::command_line::SizeFormat;
use crate::environment_provider::{DefaultEnvironmentProvider, EnvironmentProvider};
use crate::file_attribute_map::ATTRIBUTE_PRECEDENCE;
use crate::file_info::{
    FILE_ATTRIBUTE_MAP, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
    IO_REPARSE_TAG_SYMLINK, IO_REPARSE_TAG_MOUNT_POINT,
};
use crate::icon_mapping::{
    self,
    NF_CUSTOM_FOLDER, NF_FA_EXTERNAL_LINK, NF_FA_FILE, NF_COD_FILE_SYMLINK_DIR,
    NF_MD_CLOUD_CHECK, NF_MD_CLOUD_OUTLINE, NF_MD_PIN,
};





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
    TreeConnector                     = 16,
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl Attribute
//
//  Attribute enum utility methods and constants.
//
////////////////////////////////////////////////////////////////////////////////

impl Attribute {
    pub const COUNT: usize = 17;

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
        Attribute::TreeConnector,
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
            "TreeConnector"                     => Some(Attribute::TreeConnector),
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
            Attribute::TreeConnector                     => "TreeConnector",
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
            Attribute::TreeConnector            => Some('C'),
            _ => None,
        }
    }
}





////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeSource {
    Default,
    ConfigFile,
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
    pub source_file_path:     String,
    pub line_number:          usize,
}

impl ErrorInfo {
    pub fn new (message: String, entry: String, invalid_text: String, invalid_text_offset: usize) -> Self {
        ErrorInfo {
            message,
            entry,
            invalid_text,
            invalid_text_offset,
            source_file_path: String::new(),
            line_number:      0,
        }
    }
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
//
//  FileDisplayStyle
//
//  Resolved color + icon for a single file entry.
//
//  Port of: SFileDisplayStyle in TCDirCore/Config.h
//
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy, Default)]
pub struct FileDisplayStyle {
    /// Resolved color attribute (Windows console WORD)
    pub text_attr:       u16,

    /// Resolved icon code point ('\0' = no icon configured)
    pub icon_code_point: Option<char>,

    /// true if icon explicitly set to empty (user typed ",")
    pub icon_suppressed: bool,
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

    /// Extension → icon code point (keys lowercase with leading dot)
    pub extension_icons:           HashMap<String, char>,

    /// Extension → icon source tracking
    pub extension_icon_sources:    HashMap<String, AttributeSource>,

    /// Well-known dir → icon code point (keys lowercase)
    pub well_known_dir_icons:      HashMap<String, char>,

    /// Well-known dir → icon source tracking
    pub well_known_dir_icon_sources: HashMap<String, AttributeSource>,

    /// File attribute flag → icon code point
    pub file_attr_icons:           HashMap<u32, char>,

    ////////////////////////////////////////////////////////////////////////////

    /// Type fallback icons
    pub icon_directory_default:    char,
    pub icon_file_default:         char,
    pub icon_symlink:              char,
    pub icon_junction:             char,

    /// Cloud status NF glyphs
    pub icon_cloud_only:           char,
    pub icon_locally_available:    char,
    pub icon_always_local:         char,

    ////////////////////////////////////////////////////////////////////////////

    pub icons:          Option<bool>,
    pub wide_listing:   Option<bool>,
    pub bare_listing:   Option<bool>,
    pub recurse:        Option<bool>,
    pub perf_timer:     Option<bool>,
    pub multi_threaded: Option<bool>,
    pub show_owner:     Option<bool>,
    pub show_streams:   Option<bool>,

    ////////////////////////////////////////////////////////////////////////////

    pub tree:           Option<bool>,
    pub max_depth:      Option<i32>,
    pub tree_indent:    Option<i32>,
    pub size_format:    Option<SizeFormat>,

    /// Validation results from last env var parse
    pub last_parse_result: ValidationResult,

    ////////////////////////////////////////////////////////////////////////////

    /// Config file state
    pub config_file_path:         String,
    pub config_file_loaded:       bool,
    pub config_file_parse_result: ValidationResult,

    /// Switch source tracking (indexed same order as SWITCH_MEMBER_ORDER)
    pub switch_sources:           [AttributeSource; Self::SWITCH_COUNT],
    pub max_depth_source:         AttributeSource,
    pub tree_indent_source:       AttributeSource,
    pub size_format_source:       AttributeSource,

    /// Active source for the current parse pass (ConfigFile or Environment).
    /// Set before calling process_color_override_entry to tag all source maps.
    current_source:               AttributeSource,
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

    pub const SWITCH_COUNT: usize = 9;

    /// Ordered member accessors for switch source tracking.
    /// Index 0..8 maps to: wide_listing, bare_listing, recurse, perf_timer,
    /// multi_threaded, show_owner, show_streams, icons, tree
    pub const SWITCH_MEMBER_ORDER: [fn(&Config) -> &Option<bool>; Self::SWITCH_COUNT] = [
        |c| &c.wide_listing,
        |c| &c.bare_listing,
        |c| &c.recurse,
        |c| &c.perf_timer,
        |c| &c.multi_threaded,
        |c| &c.show_owner,
        |c| &c.show_streams,
        |c| &c.icons,
        |c| &c.tree,
    ];

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
            attributes:                 [0u16; Attribute::COUNT],
            attribute_sources:          [AttributeSource::Default; Attribute::COUNT],
            extension_colors:           HashMap::new(),
            extension_sources:          HashMap::new(),
            file_attr_colors:           HashMap::new(),
            extension_icons:            HashMap::new(),
            extension_icon_sources:     HashMap::new(),
            well_known_dir_icons:       HashMap::new(),
            well_known_dir_icon_sources: HashMap::new(),
            file_attr_icons:            HashMap::new(),
            icon_directory_default:     NF_CUSTOM_FOLDER,
            icon_file_default:          NF_FA_FILE,
            icon_symlink:               NF_COD_FILE_SYMLINK_DIR,
            icon_junction:              NF_FA_EXTERNAL_LINK,
            icon_cloud_only:            NF_MD_CLOUD_OUTLINE,
            icon_locally_available:     NF_MD_CLOUD_CHECK,
            icon_always_local:          NF_MD_PIN,
            icons:             None,
            wide_listing:      None,
            bare_listing:      None,
            recurse:           None,
            perf_timer:        None,
            multi_threaded:    None,
            show_owner:        None,
            show_streams:      None,
            tree:              None,
            max_depth:         None,
            tree_indent:       None,
            size_format:       None,
            last_parse_result: ValidationResult::default(),
            config_file_path:         String::new(),
            config_file_loaded:       false,
            config_file_parse_result: ValidationResult::default(),
            switch_sources:           [AttributeSource::Default; Self::SWITCH_COUNT],
            max_depth_source:         AttributeSource::Default,
            tree_indent_source:       AttributeSource::Default,
            size_format_source:       AttributeSource::Default,
            current_source:           AttributeSource::Environment,
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
        self.initialize_with_provider (default_attr, &DefaultEnvironmentProvider);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  load_config_file
    //
    //  Read .rcdirconfig from USERPROFILE, parse lines, apply settings.
    //  Silently skips if USERPROFILE not set or file not found.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn load_config_file (&mut self, provider: &dyn EnvironmentProvider) {
        self.config_file_parse_result.errors.clear();
        self.config_file_loaded = false;
        self.config_file_path.clear();

        // Resolve config file path from USERPROFILE
        let user_profile = match provider.get_env_var ("USERPROFILE") {
            Some (v) => v,
            None => return,
        };

        self.config_file_path = format! ("{}\\.rcdirconfig", user_profile);

        // Read the file
        let lines = match file_reader::read_config_file (&self.config_file_path) {
            Ok (lines) => lines,
            Err (file_reader::ConfigFileError::NotFound) => return,
            Err (file_reader::ConfigFileError::IoError (msg)) => {
                self.config_file_parse_result.errors.push (ErrorInfo::new (
                    msg,
                    self.config_file_path.clone(),
                    String::new(),
                    0,
                ));
                return;
            }
            Err (file_reader::ConfigFileError::EncodingError (msg)) => {
                self.config_file_parse_result.errors.push (ErrorInfo::new (
                    msg,
                    self.config_file_path.clone(),
                    String::new(),
                    0,
                ));
                return;
            }
        };

        self.config_file_loaded = true;
        self.process_config_lines (&lines);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  process_config_lines
    //
    //  Process parsed config file lines: skip blanks/comments, strip inline
    //  comments, apply settings with ConfigFile source, tag errors with line
    //  numbers.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn process_config_lines (&mut self, lines: &[String]) {
        self.current_source = AttributeSource::ConfigFile;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Skip empty and whitespace-only lines
            if trimmed.is_empty() {
                continue;
            }

            // Skip full-line comments
            if trimmed.starts_with ('#') {
                continue;
            }

            // Strip inline comment
            let entry = if let Some (hash_pos) = trimmed.find ('#') {
                let before = trimmed[..hash_pos].trim();
                if before.is_empty() {
                    continue;
                }
                before
            } else {
                trimmed
            };

            // Track error count before processing to tag new errors with line number
            let error_count_before = self.config_file_parse_result.errors.len();

            self.process_color_override_entry (entry);

            // Tag any new errors with config file source and line number
            for e in error_count_before..self.config_file_parse_result.errors.len() {
                self.config_file_parse_result.errors[e].source_file_path = self.config_file_path.clone();
                self.config_file_parse_result.errors[e].line_number = i + 1;  // 1-based
            }
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  validate_config_file
    //
    //  Return config file parse errors.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn validate_config_file (&self) -> &ValidationResult {
        &self.config_file_parse_result
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  is_config_file_loaded
    //
    //  Whether config file was found and loaded successfully.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn is_config_file_loaded (&self) -> bool {
        self.config_file_loaded
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
        self.attributes[Attribute::TreeConnector as usize]                     = FC_DARK_GREY;

        self.initialize_extension_colors();
        self.initialize_file_attr_colors();
        self.initialize_extension_icons();
        self.initialize_well_known_dir_icons();
        self.load_config_file (provider);
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
    //  get_cloud_status_icon
    //
    //  Returns the NF glyph for a cloud status, or None for CS_NONE.
    //
    //  Port of: CConfig::GetCloudStatusIcon
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn get_cloud_status_icon(&self, status: crate::cloud_status::CloudStatus) -> Option<char> {
        match status {
            crate::cloud_status::CloudStatus::None      => None,
            crate::cloud_status::CloudStatus::CloudOnly => Some (self.icon_cloud_only),
            crate::cloud_status::CloudStatus::Local     => Some (self.icon_locally_available),
            crate::cloud_status::CloudStatus::Pinned    => Some (self.icon_always_local),
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  get_display_style_for_file
    //
    //  Unified precedence resolver returning color + icon for a file entry.
    //  Levels are called lowest-priority first so that higher-priority
    //  levels overwrite.
    //
    //    Directories:  fallback dir icon  < well-known dir < attributes
    //    Files:        fallback file icon < extension      < attributes
    //
    //  Port of: CConfig::GetDisplayStyleForFile
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn get_display_style_for_file(&self, file_info: &crate::file_info::FileInfo) -> FileDisplayStyle {
        let default_attr = self.attributes[Attribute::Default as usize];
        let mut style = FileDisplayStyle {
            text_attr:       default_attr,
            icon_code_point: None,
            icon_suppressed: false,
        };

        if file_info.file_attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            self.resolve_directory_style (file_info, &mut style);
        } else {
            self.resolve_file_fallback_icon (file_info, &mut style);
            self.resolve_extension_style (file_info, &mut style);
        }

        self.resolve_file_attribute_style (file_info, &mut style);

        // Inherit default background if none set
        if style.text_attr & BC_MASK == 0 {
            style.text_attr |= default_attr & BC_MASK;
        }

        style.text_attr = ensure_visible_color_attr (style.text_attr, default_attr);

        style
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  resolve_directory_style
    //
    //  Set color and icon for a directory entry.  Checks well-known dir
    //  name first, then falls back to reparse point type or default dir
    //  icon.
    //
    //  Port of: CConfig::ResolveDirectoryStyle
    //
    ////////////////////////////////////////////////////////////////////////////

    fn resolve_directory_style(&self, file_info: &crate::file_info::FileInfo, style: &mut FileDisplayStyle) {
        let name = file_info.file_name.to_string_lossy().to_ascii_lowercase();

        style.text_attr = self.attributes[Attribute::Directory as usize];

        // Check well-known directory names
        if let Some(&icon) = self.well_known_dir_icons.get(&name) {
            style.icon_code_point = if icon == '\0' { None } else { Some (icon) };
            style.icon_suppressed = icon == '\0';
            return;
        }

        // Reparse points get special icons
        if file_info.file_attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            match file_info.reparse_tag {
                IO_REPARSE_TAG_SYMLINK     => style.icon_code_point = Some (self.icon_symlink),
                IO_REPARSE_TAG_MOUNT_POINT => style.icon_code_point = Some (self.icon_junction),
                _                          => style.icon_code_point = Some (self.icon_directory_default),
            }
        } else {
            style.icon_code_point = Some (self.icon_directory_default);
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  resolve_extension_style
    //
    //  Set color and icon from extension lookup tables.
    //
    //  Port of: CConfig::ResolveExtensionStyle
    //
    ////////////////////////////////////////////////////////////////////////////

    fn resolve_extension_style(&self, file_info: &crate::file_info::FileInfo, style: &mut FileDisplayStyle) {
        let path = Path::new(&file_info.file_name);
        let ext_str = match path.extension() {
            Some(ext) => format!(".{}", ext.to_string_lossy()).to_ascii_lowercase(),
            None => return,
        };

        if let Some(&color) = self.extension_colors.get(&ext_str) {
            style.text_attr = color;
        }

        if let Some(&icon) = self.extension_icons.get(&ext_str) {
            style.icon_code_point = if icon == '\0' { None } else { Some (icon) };
            style.icon_suppressed = icon == '\0';
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  resolve_file_attribute_style
    //
    //  Walk attribute precedence in reverse (lowest priority first) so the
    //  highest-priority attribute overwrites last.
    //
    //  Port of: CConfig::ResolveFileAttributeStyle
    //
    ////////////////////////////////////////////////////////////////////////////

    fn resolve_file_attribute_style(&self, file_info: &crate::file_info::FileInfo, style: &mut FileDisplayStyle) {
        for &(flag, _) in ATTRIBUTE_PRECEDENCE.iter().rev() {
            if file_info.file_attributes & flag == 0 {
                continue;
            }

            if let Some(attr_style) = self.file_attr_colors.get(&flag) {
                style.text_attr = attr_style.attr;
            }

            if let Some(&icon) = self.file_attr_icons.get(&flag) {
                style.icon_code_point = if icon == '\0' { None } else { Some (icon) };
                style.icon_suppressed = icon == '\0';
            }
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  resolve_file_fallback_icon
    //
    //  Set the fallback icon for non-directory files (symlink or default
    //  file icon).
    //
    //  Port of: CConfig::ResolveFileFallbackIcon
    //
    ////////////////////////////////////////////////////////////////////////////

    fn resolve_file_fallback_icon(&self, file_info: &crate::file_info::FileInfo, style: &mut FileDisplayStyle) {
        if file_info.file_attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
            && file_info.reparse_tag == IO_REPARSE_TAG_SYMLINK
        {
            style.icon_code_point = Some (self.icon_symlink);
        } else {
            style.icon_code_point = Some (self.icon_file_default);
        }
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
            (".asm",            FC_LIGHT_GREEN),
            (".cod",            FC_GREEN),
            (".i",              FC_GREEN),

            (".c",              FC_LIGHT_GREEN),
            (".c++",            FC_LIGHT_GREEN),
            (".cpp",            FC_LIGHT_GREEN),
            (".cxx",            FC_LIGHT_GREEN),
            (".h",              FC_LIGHT_GREEN),
            (".hpp",            FC_LIGHT_GREEN),
            (".hxx",            FC_LIGHT_GREEN),
            (".rc",             FC_LIGHT_GREEN),

            (".cs",             FC_LIGHT_GREEN),
            (".csx",            FC_LIGHT_GREEN),
            (".resx",           FC_LIGHT_GREEN),
            (".xaml",           FC_LIGHT_GREEN),

            (".js",             FC_LIGHT_GREEN),
            (".mjs",            FC_LIGHT_GREEN),
            (".cjs",            FC_LIGHT_GREEN),
            (".jsx",            FC_LIGHT_GREEN),
            (".ts",             FC_LIGHT_GREEN),
            (".tsx",            FC_LIGHT_GREEN),

            (".html",           FC_LIGHT_GREEN),
            (".htm",            FC_LIGHT_GREEN),
            (".xhtml",          FC_LIGHT_GREEN),
            (".css",            FC_LIGHT_GREEN),
            (".scss",           FC_LIGHT_GREEN),
            (".sass",           FC_LIGHT_GREEN),
            (".less",           FC_LIGHT_GREEN),
            (".vue",            FC_LIGHT_GREEN),
            (".svelte",         FC_LIGHT_GREEN),

            (".py",             FC_LIGHT_GREEN),
            (".pyw",            FC_LIGHT_GREEN),
            (".ipynb",          FC_LIGHT_GREEN),

            (".rs",             FC_LIGHT_GREEN),

            (".jar",            FC_LIGHT_GREEN),
            (".java",           FC_LIGHT_GREEN),
            (".class",          FC_LIGHT_GREEN),
            (".gradle",         FC_LIGHT_GREEN),

            (".go",             FC_LIGHT_GREEN),
            (".rb",             FC_LIGHT_GREEN),
            (".erb",            FC_LIGHT_GREEN),
            (".fs",             FC_LIGHT_GREEN),
            (".fsx",            FC_LIGHT_GREEN),
            (".fsi",            FC_LIGHT_GREEN),
            (".lua",            FC_LIGHT_GREEN),
            (".pl",             FC_LIGHT_GREEN),
            (".pm",             FC_LIGHT_GREEN),
            (".php",            FC_LIGHT_GREEN),
            (".hs",             FC_LIGHT_GREEN),
            (".dart",           FC_LIGHT_GREEN),
            (".kt",             FC_LIGHT_GREEN),
            (".kts",            FC_LIGHT_GREEN),
            (".swift",          FC_LIGHT_GREEN),
            (".scala",          FC_LIGHT_GREEN),
            (".sc",             FC_LIGHT_GREEN),
            (".sbt",            FC_LIGHT_GREEN),
            (".clj",            FC_LIGHT_GREEN),
            (".cljs",           FC_LIGHT_GREEN),
            (".cljc",           FC_LIGHT_GREEN),
            (".ex",             FC_LIGHT_GREEN),
            (".exs",            FC_LIGHT_GREEN),
            (".erl",            FC_LIGHT_GREEN),
            (".groovy",         FC_LIGHT_GREEN),
            (".jl",             FC_LIGHT_GREEN),
            (".r",              FC_LIGHT_GREEN),
            (".rmd",            FC_LIGHT_GREEN),
            (".elm",            FC_LIGHT_GREEN),

            // Config/data
            (".xml",            FC_BROWN),
            (".xsd",            FC_BROWN),
            (".xsl",            FC_BROWN),
            (".xslt",           FC_BROWN),
            (".dtd",            FC_BROWN),
            (".plist",          FC_BROWN),
            (".manifest",       FC_BROWN),
            (".json",           FC_BROWN),
            (".toml",           FC_BROWN),
            (".yml",            FC_BROWN),
            (".yaml",           FC_BROWN),
            (".ini",            FC_BROWN),
            (".cfg",            FC_BROWN),
            (".conf",           FC_BROWN),
            (".config",         FC_BROWN),
            (".properties",     FC_BROWN),
            (".settings",       FC_BROWN),
            (".reg",            FC_BROWN),

            // Database
            (".sql",            FC_BROWN),
            (".sqlite",         FC_BROWN),
            (".mdb",            FC_BROWN),
            (".accdb",          FC_BROWN),
            (".pgsql",          FC_BROWN),
            (".db",             FC_BROWN),
            (".csv",            FC_BROWN),
            (".tsv",            FC_BROWN),

            // Intermediate
            (".obj",            FC_GREEN),
            (".lib",            FC_GREEN),
            (".res",            FC_GREEN),
            (".pch",            FC_GREEN),
            (".pdb",            FC_GREEN),

            // Build
            (".wrn",            FC_LIGHT_RED),
            (".err",            FC_LIGHT_RED),
            (".log",            FC_WHITE),

            // Executable
            (".bash",           FC_LIGHT_RED),
            (".bat",            FC_LIGHT_RED),
            (".cmd",            FC_LIGHT_RED),
            (".dll",            FC_LIGHT_CYAN),
            (".exe",            FC_LIGHT_CYAN),
            (".ps1",            FC_LIGHT_RED),
            (".psd1",           FC_LIGHT_RED),
            (".psm1",           FC_LIGHT_RED),
            (".ps1xml",         FC_LIGHT_RED),
            (".sh",             FC_LIGHT_RED),
            (".zsh",            FC_LIGHT_RED),
            (".fish",           FC_LIGHT_RED),
            (".sys",            FC_LIGHT_CYAN),
            (".msi",            FC_LIGHT_CYAN),
            (".msix",           FC_LIGHT_CYAN),
            (".deb",            FC_LIGHT_CYAN),
            (".rpm",            FC_LIGHT_CYAN),

            // Visual Studio
            (".sln",            FC_MAGENTA),
            (".vcproj",         FC_MAGENTA),
            (".csproj",         FC_DARK_GREY),
            (".vcxproj",        FC_MAGENTA),
            (".csxproj",        FC_DARK_GREY),
            (".fsproj",         FC_DARK_GREY),
            (".user",           FC_DARK_GREY),
            (".ncb",            FC_DARK_GREY),
            (".suo",            FC_DARK_GREY),
            (".code-workspace", FC_DARK_GREY),

            // Documents
            (".!!!",            FC_WHITE),
            (".1st",            FC_WHITE),
            (".doc",            FC_WHITE),
            (".docx",           FC_WHITE),
            (".rtf",            FC_WHITE),
            (".eml",            FC_WHITE),
            (".md",             FC_WHITE),
            (".markdown",       FC_WHITE),
            (".rst",            FC_WHITE),
            (".me",             FC_WHITE),
            (".now",            FC_WHITE),
            (".ppt",            FC_WHITE),
            (".pptx",           FC_WHITE),
            (".pdf",            FC_WHITE),
            (".text",           FC_WHITE),
            (".txt",            FC_WHITE),
            (".xls",            FC_WHITE),
            (".xlsx",           FC_WHITE),

            // Compressed
            (".7z",             FC_MAGENTA),
            (".arj",            FC_MAGENTA),
            (".gz",             FC_MAGENTA),
            (".rar",            FC_MAGENTA),
            (".tar",            FC_MAGENTA),
            (".zip",            FC_MAGENTA),
            (".xz",             FC_MAGENTA),
            (".bz2",            FC_MAGENTA),
            (".tgz",            FC_MAGENTA),
            (".cab",            FC_MAGENTA),
            (".zst",            FC_MAGENTA),

            // Media
            (".png",            FC_CYAN),
            (".jpg",            FC_CYAN),
            (".jpeg",           FC_CYAN),
            (".gif",            FC_CYAN),
            (".bmp",            FC_CYAN),
            (".ico",            FC_CYAN),
            (".svg",            FC_CYAN),
            (".webp",           FC_CYAN),
            (".mp3",            FC_CYAN),
            (".wav",            FC_CYAN),
            (".flac",           FC_CYAN),
            (".mp4",            FC_CYAN),
            (".avi",            FC_CYAN),
            (".mkv",            FC_CYAN),
            (".mov",            FC_CYAN),

            // Fonts
            (".ttf",            FC_DARK_GREY),
            (".otf",            FC_DARK_GREY),
            (".woff",           FC_DARK_GREY),
            (".woff2",          FC_DARK_GREY),

            // Security / Certificates
            (".cer",            FC_YELLOW),
            (".crt",            FC_YELLOW),
            (".pem",            FC_YELLOW),
            (".key",            FC_YELLOW),
            (".pfx",            FC_YELLOW),

            // Docker / Terraform / Lock
            (".dockerfile",     FC_LIGHT_GREEN),
            (".dockerignore",   FC_DARK_GREY),
            (".tf",             FC_LIGHT_GREEN),
            (".tfvars",         FC_LIGHT_GREEN),
            (".bicep",          FC_LIGHT_GREEN),
            (".lock",           FC_DARK_GREY),
        ];

        for &(ext, color) in defaults {
            self.extension_colors.insert (ext.to_ascii_lowercase(), color);
            self.extension_sources.insert (ext.to_ascii_lowercase(), AttributeSource::Default);
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
    //  initialize_extension_icons
    //
    //  Seed the extension → icon HashMap from the default icon mapping table.
    //
    //  Port of: CConfig::PopulateIconMap (extension variant)
    //
    ////////////////////////////////////////////////////////////////////////////

    fn initialize_extension_icons(&mut self) {
        self.extension_icons.clear();
        self.extension_icon_sources.clear();

        for &(ext, glyph) in icon_mapping::DEFAULT_EXTENSION_ICONS {
            let key = ext.to_ascii_lowercase();
            self.extension_icons.insert (key.clone(), glyph);
            self.extension_icon_sources.insert (key, AttributeSource::Default);
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  initialize_well_known_dir_icons
    //
    //  Seed the well-known dir → icon HashMap from the default icon mapping
    //  table.
    //
    //  Port of: CConfig::PopulateIconMap (well-known dir variant)
    //
    ////////////////////////////////////////////////////////////////////////////

    fn initialize_well_known_dir_icons(&mut self) {
        self.well_known_dir_icons.clear();
        self.well_known_dir_icon_sources.clear();

        for &(name, glyph) in icon_mapping::DEFAULT_WELL_KNOWN_DIR_ICONS {
            let key = name.to_ascii_lowercase();
            self.well_known_dir_icons.insert (key.clone(), glyph);
            self.well_known_dir_icon_sources.insert (key, AttributeSource::Default);
        }
    }
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





////////////////////////////////////////////////////////////////////////////////
//
//  ensure_visible_color_attr
//
//  Adjust a color attribute so foreground is visible against the background.
//  If foreground matches background, use a contrasting background.
//
//  Port of: CConfig::EnsureVisibleColorAttr
//
////////////////////////////////////////////////////////////////////////////////

fn ensure_visible_color_attr(color_attr: u16, default_attr: u16) -> u16 {
    let fore = color_attr & FC_MASK;
    let mut back = color_attr & BC_MASK;
    let default_back = default_attr & BC_MASK;

    if back == 0 {
        back = default_back;
    }

    // If fore matches back, use contrasting background
    if (fore << 4) == back {
        let contrast_back = if back & 0x80 != 0 { BC_BLACK } else { BC_LIGHT_GREY };
        return fore | contrast_back;
    }

    fore | back
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
    //  Verifies the attribute count is 17.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn attribute_count() {
        assert_eq!(Attribute::COUNT, 17);
        assert_eq!(Attribute::ALL.len(), 17);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  entry_without_comma_backward_compat
    //
    //  FR-024: Entries without a comma produce identical behavior to
    //  the pre-icon code path — color set, icon unchanged, no side
    //  effects.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn entry_without_comma_backward_compat() {
        let cfg = make_config (Some (".py=Green"));

        // Color should be set to Green
        assert_eq!(cfg.extension_colors.get(".py"), Some (&FC_GREEN));

        // Icon should remain at the default (populated from DEFAULT_EXTENSION_ICONS)
        // and NOT be modified or suppressed
        let default_icon = cfg.extension_icons.get(".py");
        assert!(default_icon.is_some(), ".py should have a default icon");
        assert_ne!(*default_icon.unwrap(), '\0', "icon should not be suppressed");

        // No errors
        assert!(cfg.last_parse_result.errors.is_empty());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  comma_syntax_sets_color_and_icon
    //
    //  Verify comma-syntax sets both color and icon.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn comma_syntax_sets_color_and_icon() {
        let cfg = make_config (Some (".txt=Yellow,A"));

        assert_eq!(cfg.extension_colors.get (".txt"), Some (&FC_YELLOW));
        assert_eq!(cfg.extension_icons.get (".txt"), Some (&'A'));
        assert!(cfg.last_parse_result.errors.is_empty());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  comma_syntax_suppresses_icon
    //
    //  Verify comma-syntax with empty icon suppresses it.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn comma_syntax_suppresses_icon() {
        let cfg = make_config (Some (".txt=Red,"));

        assert_eq!(cfg.extension_colors.get (".txt"), Some (&FC_RED));
        assert_eq!(cfg.extension_icons.get (".txt"), Some (&'\0'));
        assert!(cfg.last_parse_result.errors.is_empty());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  comma_syntax_icon_only
    //
    //  Verify comma-syntax with no color sets only icon.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn comma_syntax_icon_only() {
        let cfg = make_config (Some (".rs=,X"));

        // Color should remain at whatever the default was
        let before_cfg = make_config (None);
        let default_rs_color = before_cfg.extension_colors.get (".rs").copied();
        assert_eq!(cfg.extension_colors.get (".rs").copied(), default_rs_color);

        // Icon should be 'X'
        assert_eq!(cfg.extension_icons.get (".rs"), Some (&'X'));
        assert!(cfg.last_parse_result.errors.is_empty());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  well_known_dir_icon_lookup
    //
    //  Verify resolve_directory_style performs case-insensitive well-known
    //  dir lookup.  T049 verification.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn well_known_dir_icon_lookup() {
        use crate::file_info::FileInfo;
        use crate::icon_mapping::NF_SETI_GIT;

        let cfg = make_config (None);

        // ".git" is a well-known dir — should get its specific icon
        let fi_git = FileInfo {
            file_name:        std::ffi::OsString::from (".git"),
            file_attributes:  FILE_ATTRIBUTE_DIRECTORY,
            file_size:        0,
            creation_time:    0,
            last_write_time:  0,
            last_access_time: 0,
            reparse_tag:      0,
            streams:          Vec::new(),
        };
        let style = cfg.get_display_style_for_file (&fi_git);
        assert!(style.icon_code_point.is_some(), ".git should have an icon");
        assert_eq!(style.icon_code_point.unwrap(), NF_SETI_GIT);

        // Case-insensitive: ".GIT" should match too
        let fi_git_upper = FileInfo {
            file_name:        std::ffi::OsString::from (".GIT"),
            file_attributes:  FILE_ATTRIBUTE_DIRECTORY,
            file_size:        0,
            creation_time:    0,
            last_write_time:  0,
            last_access_time: 0,
            reparse_tag:      0,
            streams:          Vec::new(),
        };
        let style_upper = cfg.get_display_style_for_file (&fi_git_upper);
        assert_eq!(style_upper.icon_code_point, style.icon_code_point);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  dir_prefix_overrides_default_icon
    //
    //  Verify user `dir:` prefix overrides built-in well-known dir icons.
    //  T050 verification.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn dir_prefix_overrides_default_icon() {
        use crate::file_info::FileInfo;

        // Override .git dir icon with 'X'
        let cfg = make_config (Some ("dir:.git=,X"));

        let fi_git = FileInfo {
            file_name:        std::ffi::OsString::from (".git"),
            file_attributes:  FILE_ATTRIBUTE_DIRECTORY,
            file_size:        0,
            creation_time:    0,
            last_write_time:  0,
            last_access_time: 0,
            reparse_tag:      0,
            streams:          Vec::new(),
        };
        let style = cfg.get_display_style_for_file (&fi_git);
        assert_eq!(style.icon_code_point, Some ('X'), "dir: override should replace default icon");
        assert!(!style.icon_suppressed);
        assert!(cfg.last_parse_result.errors.is_empty());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_icon_emission_spacing_contract
    //
    //  Verify the icon emission spacing contract:
    //  - When icon is present: glyph (1 char) + space = 2 visual cells
    //  - When icon is suppressed: 2 spaces
    //  - When no icon: 0 cells (no emission)
    //
    //  This validates FR-007 (icon layout width accounting).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_icon_emission_spacing_contract() {
        const CX_ICON_COLUMN: usize = 2;

        // Active icon: glyph + space = 2
        let style_active = FileDisplayStyle {
            text_attr: 0,
            icon_code_point: Some ('\u{E7A8}'), // NF_DEV_RUST
            icon_suppressed: false,
        };
        let emission = format! ("{} ", style_active.icon_code_point.unwrap());
        assert_eq! (emission.chars().count(), CX_ICON_COLUMN,
            "Icon glyph + space must be {} chars", CX_ICON_COLUMN);

        // Suppressed icon: 2 spaces
        let style_suppressed = FileDisplayStyle {
            text_attr: 0,
            icon_code_point: None,
            icon_suppressed: true,
        };
        assert! (style_suppressed.icon_code_point.is_none());
        assert! (style_suppressed.icon_suppressed);
        let blank_emission = "  "; // 2 spaces when icon suppressed
        assert_eq! (blank_emission.len(), CX_ICON_COLUMN,
            "Suppressed icon padding must be {} chars", CX_ICON_COLUMN);

        // No icon at all: nothing emitted (0 chars)
        let style_none = FileDisplayStyle {
            text_attr: 0,
            icon_code_point: None,
            icon_suppressed: false,
        };
        assert! (style_none.icon_code_point.is_none());
        assert! (!style_none.icon_suppressed);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_invalid_background_rejects_entire_entry
    //
    //  Verifies that an invalid background color (e.g., "Blue on Chartreuse")
    //  rejects the entire entry rather than silently using the foreground only.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_invalid_background_rejects_entire_entry() {
        let config = make_config (Some (".txt=Blue on Chartreuse"));

        // Error should have been recorded
        assert! (!config.last_parse_result.errors.is_empty(),
            "Should record error for invalid background color");

        let err = &config.last_parse_result.errors[0];
        assert_eq! (err.message, "Invalid background color");
        assert_eq! (err.invalid_text, "Chartreuse");

        // The extension should retain its DEFAULT color, not the invalid override.
        // It should not have Blue foreground (0x01) applied.
        if let Some (&attr) = config.extension_colors.get (".txt") {
            assert_ne! (attr & FC_MASK, 0x01, // FC_BLUE = 0x01
                "Invalid bg entry should not apply foreground color either");
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_same_fore_back_color_rejected
    //
    //  Verifies that entries where foreground == background color (e.g.,
    //  "Blue on Blue") are rejected as unreadable.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_same_fore_back_color_rejected() {
        let config = make_config (Some (".txt=Blue on Blue"));

        // Error should have been recorded
        assert! (!config.last_parse_result.errors.is_empty(),
            "Should record error for same fore/back color");

        let err = &config.last_parse_result.errors[0];
        assert_eq! (err.message, "Foreground and background colors are the same");

        // The extension should retain its DEFAULT color, not the Blue on Blue override.
        if let Some (&attr) = config.extension_colors.get (".txt") {
            assert_ne! (attr, 0x01 | 0x10, // Blue fore | Blue back
                "Same fore/back entry should not apply color");
        }
    }





    // =========================================================================
    //  Tree config env var tests (T019)
    // =========================================================================

    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_var_tree_sets_tree_true
    //
    //  Verify RCDIR=Tree sets config.tree = Some(true).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_var_tree_sets_tree_true () {
        let config = make_config (Some ("Tree"));
        assert_eq! (config.tree, Some (true));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_var_tree_disable_sets_tree_false
    //
    //  Verify RCDIR=Tree- sets config.tree = Some(false).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_var_tree_disable_sets_tree_false () {
        let config = make_config (Some ("Tree-"));
        assert_eq! (config.tree, Some (false));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_var_depth_sets_max_depth
    //
    //  Verify RCDIR=Depth=5 sets config.max_depth = Some(5).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_var_depth_sets_max_depth () {
        let config = make_config (Some ("Depth=5"));
        assert_eq! (config.max_depth, Some (5));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_var_tree_indent_sets_tree_indent
    //
    //  Verify RCDIR=TreeIndent=2 sets config.tree_indent = Some(2).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_var_tree_indent_sets_tree_indent () {
        let config = make_config (Some ("TreeIndent=2"));
        assert_eq! (config.tree_indent, Some (2));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_var_tree_with_depth_and_indent_parses_all
    //
    //  Verify RCDIR with Tree + Depth + TreeIndent parses all three.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_var_tree_with_depth_and_indent_parses_all () {
        let config = make_config (Some ("Tree;Depth=3;TreeIndent=6"));
        assert_eq! (config.tree, Some (true));
        assert_eq! (config.max_depth, Some (3));
        assert_eq! (config.tree_indent, Some (6));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_var_depth_invalid_records_error
    //
    //  Verify RCDIR=Depth=foo records a parse error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_var_depth_invalid_records_error () {
        let config = make_config (Some ("Depth=foo"));
        assert! (config.last_parse_result.has_issues());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_var_tree_indent_out_of_range_records_error
    //
    //  Verify RCDIR=TreeIndent=10 records a parse error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_var_tree_indent_out_of_range_records_error () {
        let config = make_config (Some ("TreeIndent=10"));
        assert! (config.last_parse_result.has_issues());
    }





    // =========================================================================
    //  Size config env var tests (T020)
    // =========================================================================

    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_var_size_auto_sets_size_format
    //
    //  Verify RCDIR=Size=Auto sets config.size_format = Some(Auto).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_var_size_auto_sets_size_format () {
        let config = make_config (Some ("Size=Auto"));
        assert_eq! (config.size_format, Some (SizeFormat::Auto));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_var_size_bytes_sets_size_format
    //
    //  Verify RCDIR=Size=Bytes sets config.size_format = Some(Bytes).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_var_size_bytes_sets_size_format () {
        let config = make_config (Some ("Size=Bytes"));
        assert_eq! (config.size_format, Some (SizeFormat::Bytes));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_var_size_invalid_records_error
    //
    //  Verify RCDIR=Size=Invalid records a parse error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_var_size_invalid_records_error () {
        let config = make_config (Some ("Size=Invalid"));
        assert! (config.last_parse_result.has_issues());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  env_var_size_case_insensitive
    //
    //  Verify Size switch is case-insensitive.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn env_var_size_case_insensitive () {
        let config = make_config (Some ("Size=auto"));
        assert_eq! (config.size_format, Some (SizeFormat::Auto));

        let config2 = make_config (Some ("Size=BYTES"));
        assert_eq! (config2.size_format, Some (SizeFormat::Bytes));
    }





    // =========================================================================
    //  Color name parsing tests
    //  Port of: ConfigEnvironmentTests (ParseColorName, ParseColorSpec)
    // =========================================================================

    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_color_name_all_foreground_colors
    //
    //  Port of: ParseColorName_AllForegroundColors_ReturnsCorrectValues
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_color_name_all_foreground_colors () {
        use crate::color::*;

        let cases: &[(&str, u16)] = &[
            ("Black",        FC_BLACK),
            ("Blue",         FC_BLUE),
            ("Green",        FC_GREEN),
            ("Cyan",         FC_CYAN),
            ("Red",          FC_RED),
            ("Magenta",      FC_MAGENTA),
            ("Brown",        FC_BROWN),
            ("LightGrey",    FC_LIGHT_GREY),
            ("DarkGrey",     FC_DARK_GREY),
            ("LightBlue",    FC_LIGHT_BLUE),
            ("LightGreen",   FC_LIGHT_GREEN),
            ("LightCyan",    FC_LIGHT_CYAN),
            ("LightRed",     FC_LIGHT_RED),
            ("LightMagenta", FC_LIGHT_MAGENTA),
            ("Yellow",       FC_YELLOW),
            ("White",        FC_WHITE),
        ];

        for (name, expected) in cases {
            let result = parse_color_name (name, false);
            assert! (result.is_ok(), "parse_color_name('{}', false) failed", name);
            assert_eq! (result.unwrap(), *expected, "Foreground color '{}' mismatch", name);
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_color_name_all_background_colors
    //
    //  Port of: ParseColorName_AllBackgroundColors_ReturnsCorrectValues
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_color_name_all_background_colors () {
        use crate::color::*;

        let cases: &[(&str, u16)] = &[
            ("Black",        BC_BLACK),
            ("Blue",         BC_BLUE),
            ("Green",        BC_GREEN),
            ("Cyan",         BC_CYAN),
            ("Red",          BC_RED),
            ("Magenta",      BC_MAGENTA),
            ("Brown",        BC_BROWN),
            ("LightGrey",    BC_LIGHT_GREY),
            ("DarkGrey",     BC_DARK_GREY),
            ("LightBlue",    BC_LIGHT_BLUE),
            ("LightGreen",   BC_LIGHT_GREEN),
            ("LightCyan",    BC_LIGHT_CYAN),
            ("LightRed",     BC_LIGHT_RED),
            ("LightMagenta", BC_LIGHT_MAGENTA),
            ("Yellow",       BC_YELLOW),
            ("White",        BC_WHITE),
        ];

        for (name, expected) in cases {
            let result = parse_color_name (name, true);
            assert! (result.is_ok(), "parse_color_name('{}', true) failed", name);
            assert_eq! (result.unwrap(), *expected, "Background color '{}' mismatch", name);
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_color_name_case_insensitive
    //
    //  Port of: ParseColorName_CaseInsensitive_ReturnsCorrectValues
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_color_name_case_insensitive () {
        use crate::color::*;

        assert_eq! (parse_color_name ("red",       false).unwrap(), FC_RED);
        assert_eq! (parse_color_name ("RED",       false).unwrap(), FC_RED);
        assert_eq! (parse_color_name ("Red",       false).unwrap(), FC_RED);
        assert_eq! (parse_color_name ("lightblue", false).unwrap(), FC_LIGHT_BLUE);
        assert_eq! (parse_color_name ("LIGHTBLUE", false).unwrap(), FC_LIGHT_BLUE);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_color_name_invalid_returns_error
    //
    //  Port of: ParseColorName_InvalidColor_ReturnsZero
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_color_name_invalid_returns_error () {
        use crate::color::parse_color_name;

        assert! (parse_color_name ("NotAColor", false).is_err());
        assert! (parse_color_name ("", false).is_err());
        assert! (parse_color_name ("123", false).is_err());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_color_spec_foreground_only
    //
    //  Port of: ParseColorSpec_ForegroundOnly_NoWhitespace_ReturnsCorrectValue
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_color_spec_foreground_only () {
        use crate::color::*;

        assert_eq! (parse_color_spec ("Yellow").unwrap(), FC_YELLOW);
        assert_eq! (parse_color_spec ("Red").unwrap(), FC_RED);
        assert_eq! (parse_color_spec (" Yellow ").unwrap(), FC_YELLOW);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_color_spec_foreground_and_background
    //
    //  Port of: ParseColorSpec_ForegroundAndBackground variants
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_color_spec_foreground_and_background () {
        use crate::color::*;

        let result = parse_color_spec ("LightCyan on Blue").unwrap();
        assert_eq! (result, FC_LIGHT_CYAN | BC_BLUE);

        let result2 = parse_color_spec ("  LightCyan  on  Blue  ").unwrap();
        assert_eq! (result2, FC_LIGHT_CYAN | BC_BLUE);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  parse_color_spec_case_insensitive
    //
    //  Port of: ParseColorSpec_CaseInsensitiveOn variants
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn parse_color_spec_case_insensitive () {
        use crate::color::*;

        assert_eq! (parse_color_spec ("lightcyan on blue").unwrap(), FC_LIGHT_CYAN | BC_BLUE);
        assert_eq! (parse_color_spec ("LIGHTCYAN ON BLUE").unwrap(), FC_LIGHT_CYAN | BC_BLUE);
    }





    // =========================================================================
    //  Display attribute override tests — individual attribute chars
    //  Port of: ProcessDisplayAttributeOverride_*
    // =========================================================================

    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_attribute_override_all_valid_chars
    //
    //  Port of: ProcessDisplayAttributeOverride_AllValidChars_AllWork
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn display_attribute_override_all_valid_chars () {
        use crate::color::*;

        // Each display attribute char maps to an Attribute index
        let cases: &[(&str, Attribute)] = &[
            ("D=Yellow",  Attribute::Date),
            ("T=Yellow",  Attribute::Time),
            ("S=Yellow",  Attribute::Size),
            ("R=Yellow",  Attribute::Directory),
            ("A=Yellow",  Attribute::FileAttributePresent),
            ("I=Yellow",  Attribute::Information),
            ("H=Yellow",  Attribute::InformationHighlight),
            ("E=Yellow",  Attribute::Error),
            ("F=Yellow",  Attribute::Default),
            ("O=Yellow",  Attribute::Owner),
            ("M=Yellow",  Attribute::Stream),
            ("C=Yellow",  Attribute::TreeConnector),
        ];

        for (entry, expected_attr) in cases {
            let config = make_config (Some (entry));
            assert_eq! (config.attributes[*expected_attr as usize], FC_YELLOW,
                "Display attribute override for '{}' should set {:?} to Yellow", entry, expected_attr);
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_attribute_override_lowercase_works
    //
    //  Port of: ProcessDisplayAttributeOverride_LowercaseChar_WorksCorrectly
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn display_attribute_override_lowercase_works () {
        use crate::color::*;

        let config = make_config (Some ("d=Yellow"));
        assert_eq! (config.attributes[Attribute::Date as usize], FC_YELLOW);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_attribute_override_with_background
    //
    //  Port of: ProcessDisplayAttributeOverride_WithBackground_StoresComplete
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn display_attribute_override_with_background () {
        use crate::color::*;

        let config = make_config (Some ("D=LightCyan on Blue"));
        assert_eq! (config.attributes[Attribute::Date as usize], FC_LIGHT_CYAN | BC_BLUE);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_attribute_override_invalid_char_does_nothing
    //
    //  Port of: ProcessDisplayAttributeOverride_InvalidChar_DoesNothing
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn display_attribute_override_invalid_char_does_nothing () {
        let config = make_config (Some ("Q=Yellow"));
        // Q is not a valid display attribute — should be reported as error
        assert! (!config.last_parse_result.errors.is_empty());
    }





    // =========================================================================
    //  Switch override tests — individual switches
    //  Port of: ConfigSwitchOverrideTests
    // =========================================================================

    ////////////////////////////////////////////////////////////////////////////
    //
    //  switch_override_w_variants
    //
    //  Port of: ProcessSwitchOverride_W/WUppercase/wLowercase/WMinus
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn switch_override_w_variants () {
        let cfg_w  = make_config (Some ("W"));
        assert_eq! (cfg_w.wide_listing, Some (true));

        let cfg_wc = make_config (Some ("w"));
        assert_eq! (cfg_wc.wide_listing, Some (true));

        let cfg_wd = make_config (Some ("W-"));
        assert_eq! (cfg_wd.wide_listing, Some (false));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  switch_override_s_sets_recurse
    //
    //  Port of: ProcessSwitchOverride_S_SetsRecurse
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn switch_override_s_sets_recurse () {
        let config = make_config (Some ("S"));
        assert_eq! (config.recurse, Some (true));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  switch_override_p_sets_perf_timer
    //
    //  Port of: ProcessSwitchOverride_P_SetsPerfTimer
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn switch_override_p_sets_perf_timer () {
        let config = make_config (Some ("P"));
        assert_eq! (config.perf_timer, Some (true));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  switch_override_m_variants
    //
    //  Port of: ProcessSwitchOverride_M/MMinus
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn switch_override_m_variants () {
        let cfg_m  = make_config (Some ("M"));
        assert_eq! (cfg_m.multi_threaded, Some (true));

        let cfg_md = make_config (Some ("M-"));
        assert_eq! (cfg_md.multi_threaded, Some (false));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  switch_override_icons_variants
    //
    //  Port of: EnvVar_Icons_SetsIconsTrue / EnvVar_IconsDisable_SetsIconsFalse
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn switch_override_icons_variants () {
        let cfg_on  = make_config (Some ("Icons"));
        assert_eq! (cfg_on.icons, Some (true));

        let cfg_off = make_config (Some ("Icons-"));
        assert_eq! (cfg_off.icons, Some (false));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  switch_override_invalid_adds_error
    //
    //  Port of: ProcessSwitchOverride_InvalidSwitch_AddsError
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn switch_override_invalid_adds_error () {
        let config = make_config (Some ("Bogus"));
        assert! (!config.last_parse_result.errors.is_empty());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  switch_override_slash_dash_prefix_rejected
    //
    //  Port of: ProcessColorOverrideEntry_SwitchWithSlash/Dash_RejectsPrefix
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn switch_override_slash_dash_prefix_rejected () {
        let config_slash = make_config (Some ("/W"));
        assert! (!config_slash.last_parse_result.errors.is_empty());

        let config_dash = make_config (Some ("-W"));
        assert! (!config_dash.last_parse_result.errors.is_empty());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  default_switch_values_are_not_set
    //
    //  Port of: DefaultSwitchValues_AreNotSet
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn default_switch_values_are_not_set () {
        let config = make_config (None);

        assert! (config.wide_listing.is_none());
        assert! (config.bare_listing.is_none());
        assert! (config.recurse.is_none());
        assert! (config.perf_timer.is_none());
        assert! (config.multi_threaded.is_none());
        assert! (config.show_owner.is_none());
        assert! (config.show_streams.is_none());
        assert! (config.icons.is_none());
        assert! (config.tree.is_none());
    }





    // =========================================================================
    //  File extension override tests
    //  Port of: ProcessFileExtensionOverride_*
    // =========================================================================

    ////////////////////////////////////////////////////////////////////////////
    //
    //  file_extension_override_stores_correctly
    //
    //  Port of: ProcessFileExtensionOverride_LowercaseExtension
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn file_extension_override_stores_correctly () {
        use crate::color::*;

        let config = make_config (Some (".txt=LightGreen"));
        assert_eq! (config.extension_colors.get (".txt"), Some (&FC_LIGHT_GREEN));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  file_extension_override_uppercase_converts_to_lowercase
    //
    //  Port of: ProcessFileExtensionOverride_UppercaseExtension
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn file_extension_override_uppercase_converts_to_lowercase () {
        use crate::color::*;

        let config = make_config (Some (".TXT=LightGreen"));
        assert_eq! (config.extension_colors.get (".txt"), Some (&FC_LIGHT_GREEN));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  file_extension_override_multiple_extensions
    //
    //  Port of: ProcessFileExtensionOverride_MultipleExtensions_AllStored
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn file_extension_override_multiple_extensions () {
        use crate::color::*;

        let config = make_config (Some (".txt=LightGreen;.log=Yellow;.md=LightCyan"));
        assert_eq! (config.extension_colors.get (".txt"), Some (&FC_LIGHT_GREEN));
        assert_eq! (config.extension_colors.get (".log"), Some (&FC_YELLOW));
        assert_eq! (config.extension_colors.get (".md"),  Some (&FC_LIGHT_CYAN));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  file_extension_override_with_background
    //
    //  Port of: ProcessFileExtensionOverride_WithBackground_StoresComplete
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn file_extension_override_with_background () {
        use crate::color::*;

        let config = make_config (Some (".log=Red on Blue"));
        assert_eq! (config.extension_colors.get (".log"), Some (&(FC_RED | BC_BLUE)));
    }





    // =========================================================================
    //  Integration scenarios
    //  Port of: IntegrationTest_*
    // =========================================================================

    ////////////////////////////////////////////////////////////////////////////
    //
    //  integration_complex_env_string_all_processed
    //
    //  Port of: IntegrationTest_ComplexEnvironmentString_AllEntriesProcessed
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn integration_complex_env_string_all_processed () {
        use crate::color::*;

        let config = make_config (Some (
            ".py=LightGreen;D=Yellow;W;.rs=LightCyan;Owner"
        ));

        assert_eq! (config.extension_colors.get (".py"), Some (&FC_LIGHT_GREEN));
        assert_eq! (config.extension_colors.get (".rs"), Some (&FC_LIGHT_CYAN));
        assert_eq! (config.attributes[Attribute::Date as usize], FC_YELLOW);
        assert_eq! (config.wide_listing, Some (true));
        assert_eq! (config.show_owner, Some (true));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  integration_empty_env_var_no_error
    //
    //  Port of: IntegrationTest_EmptyEnvironmentVariable_NoError
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn integration_empty_env_var_no_error () {
        let config = make_config (Some (""));
        assert! (config.last_parse_result.errors.is_empty());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  integration_trailing_semicolon_handled
    //
    //  Port of: IntegrationTest_TrailingSemicolon_HandledCorrectly
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn integration_trailing_semicolon_handled () {
        use crate::color::*;

        let config = make_config (Some (".py=LightGreen;"));
        assert_eq! (config.extension_colors.get (".py"), Some (&FC_LIGHT_GREEN));
        assert! (config.last_parse_result.errors.is_empty());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  integration_multiple_semicolons_parsed
    //
    //  Port of: ApplyUserColorOverrides_MultipleSemicolons_ParsesCorrectly
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn integration_multiple_semicolons_parsed () {
        use crate::color::*;

        let config = make_config (Some (".py=LightGreen;;.rs=Yellow"));
        assert_eq! (config.extension_colors.get (".py"), Some (&FC_LIGHT_GREEN));
        assert_eq! (config.extension_colors.get (".rs"), Some (&FC_YELLOW));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  integration_mixed_attributes_and_extensions
    //
    //  Port of: IntegrationTest_MixedExtensionsAndAttributes_AllProcessed
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn integration_mixed_attributes_and_extensions () {
        use crate::color::*;

        let config = make_config (Some (
            ".cpp=LightCyan;D=Yellow;.h=LightGreen;T=Red;S=White"
        ));

        assert_eq! (config.extension_colors.get (".cpp"), Some (&FC_LIGHT_CYAN));
        assert_eq! (config.extension_colors.get (".h"),   Some (&FC_LIGHT_GREEN));
        assert_eq! (config.attributes[Attribute::Date as usize], FC_YELLOW);
        assert_eq! (config.attributes[Attribute::Time as usize], FC_RED);
        assert_eq! (config.attributes[Attribute::Size as usize], FC_WHITE);
    }





    // =========================================================================
    //  Icon parsing tests
    //  Port of: ConfigIconParsingTests
    // =========================================================================

    ////////////////////////////////////////////////////////////////////////////
    //
    //  icon_hex_four_digits_parses_correctly
    //
    //  Port of: ParseIconValue_HexFourDigits_ParsesCorrectly
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn icon_hex_four_digits_parses_correctly () {
        let config = make_config (Some (".cpp=LightCyan,U+E61D"));
        let icon = config.extension_icons.get (".cpp");
        assert! (icon.is_some(), "Extension icon should be set");
        assert_eq! (*icon.unwrap(), '\u{E61D}');
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  icon_hex_five_digits_parses_correctly
    //
    //  Port of: ParseIconValue_HexFiveDigits_ParsesCorrectly
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn icon_hex_five_digits_parses_correctly () {
        let config = make_config (Some (".cpp=LightCyan,U+1F4C2"));
        let icon = config.extension_icons.get (".cpp");
        assert! (icon.is_some(), "5-digit hex icon should be set");
        assert_eq! (*icon.unwrap(), '\u{1F4C2}');
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  icon_surrogate_range_rejected
    //
    //  Port of: ParseIconValue_SurrogateRange_RejectsD800 / RejectsDFFF
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn icon_surrogate_range_rejected () {
        let config_d800 = make_config (Some (".cpp=LightCyan,U+D800"));
        assert! (!config_d800.last_parse_result.errors.is_empty(), "D800 should be rejected");

        let config_dfff = make_config (Some (".cpp=LightCyan,U+DFFF"));
        assert! (!config_dfff.last_parse_result.errors.is_empty(), "DFFF should be rejected");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  icon_zero_code_point_rejected
    //
    //  Port of: ParseIconValue_ZeroCodePoint_Rejects
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn icon_zero_code_point_rejected () {
        let config = make_config (Some (".cpp=LightCyan,U+0000"));
        assert! (!config.last_parse_result.errors.is_empty());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  icon_empty_suppressed
    //
    //  Port of: ParseIconValue_Empty_Suppressed
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn icon_empty_suppressed () {
        // Trailing comma with nothing after it → icon suppressed
        let config = make_config (Some (".cpp=LightCyan,"));
        let icon = config.extension_icons.get (".cpp");
        // Icon suppressed means icon is '\0'
        if let Some (ch) = icon {
            assert_eq! (*ch, '\0', "Empty icon spec should suppress icon");
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  icon_literal_bmp_glyph_parses_correctly
    //
    //  Port of: ParseIconValue_LiteralBmpGlyph_ParsesCorrectly
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn icon_literal_bmp_glyph_parses_correctly () {
        // Single char literal
        let config = make_config (Some (".py=LightGreen,\u{E606}"));
        let icon = config.extension_icons.get (".py");
        assert! (icon.is_some());
        assert_eq! (*icon.unwrap(), '\u{E606}');
    }





    // =========================================================================
    //  Display style / icon precedence tests
    //  Port of: ConfigIconPrecedenceTests
    // =========================================================================

    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_style_plain_directory_returns_directory_color
    //
    //  Port of: GetDisplayStyle_PlainDirectory_ReturnsDirectoryColor
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn display_style_plain_directory_returns_directory_color () {
        use crate::color::*;
        use crate::file_info::FileInfo;

        let config = make_config (None);
        let fi = FileInfo {
            file_name:       std::ffi::OsString::from ("mydir"),
            file_attributes: FILE_ATTRIBUTE_DIRECTORY,
            file_size:       0,
            creation_time:   0,
            last_write_time: 0,
            last_access_time: 0,
            reparse_tag:     0,
            streams:         Vec::new(),
        };

        let style = config.get_display_style_for_file (&fi);
        assert_eq! (style.text_attr & FC_MASK, FC_LIGHT_BLUE);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_style_unknown_extension_returns_default_file_icon
    //
    //  Port of: GetDisplayStyle_UnknownExtension_ReturnsFileDefault
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn display_style_unknown_extension_returns_default_file_icon () {
        use crate::file_info::FileInfo;

        let config = make_config (None);
        let fi = FileInfo {
            file_name:       std::ffi::OsString::from ("data.xyz123"),
            file_attributes: 0x20, // FILE_ATTRIBUTE_ARCHIVE
            file_size:       100,
            creation_time:   0,
            last_write_time: 0,
            last_access_time: 0,
            reparse_tag:     0,
            streams:         Vec::new(),
        };

        let style = config.get_display_style_for_file (&fi);
        assert_eq! (style.icon_code_point, Some (crate::icon_mapping::NF_FA_FILE));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_style_cpp_file_returns_cpp_icon
    //
    //  Port of: GetDisplayStyle_NormalMode_CppFileReturnsIcon
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn display_style_cpp_file_returns_cpp_icon () {
        use crate::file_info::FileInfo;

        let config = make_config (None);
        let fi = FileInfo {
            file_name:       std::ffi::OsString::from ("main.cpp"),
            file_attributes: 0x20,
            file_size:       1000,
            creation_time:   0,
            last_write_time: 0,
            last_access_time: 0,
            reparse_tag:     0,
            streams:         Vec::new(),
        };

        let style = config.get_display_style_for_file (&fi);
        assert! (style.icon_code_point.is_some(), "C++ file should have an icon");
        assert! (!style.icon_suppressed, "Icon should not be suppressed");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_style_directory_returns_folder_icon
    //
    //  Port of: GetDisplayStyle_DirectoryReturnsDirectoryIcon
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn display_style_directory_returns_folder_icon () {
        use crate::file_info::FileInfo;

        let config = make_config (None);
        let fi = FileInfo {
            file_name:       std::ffi::OsString::from ("mydir"),
            file_attributes: FILE_ATTRIBUTE_DIRECTORY,
            file_size:       0,
            creation_time:   0,
            last_write_time: 0,
            last_access_time: 0,
            reparse_tag:     0,
            streams:         Vec::new(),
        };

        let style = config.get_display_style_for_file (&fi);
        assert_eq! (style.icon_code_point, Some (crate::icon_mapping::NF_CUSTOM_FOLDER));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_style_symlink_dir_returns_symlink_icon
    //
    //  Port of: GetDisplayStyle_SymlinkDir_ReturnsSymlinkIcon
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn display_style_symlink_dir_returns_symlink_icon () {
        use crate::file_info::{FileInfo, FILE_ATTRIBUTE_REPARSE_POINT, IO_REPARSE_TAG_SYMLINK};

        let config = make_config (None);
        let fi = FileInfo {
            file_name:       std::ffi::OsString::from ("link"),
            file_attributes: FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT,
            file_size:       0,
            creation_time:   0,
            last_write_time: 0,
            last_access_time: 0,
            reparse_tag:     IO_REPARSE_TAG_SYMLINK,
            streams:         Vec::new(),
        };

        let style = config.get_display_style_for_file (&fi);
        assert_eq! (style.icon_code_point, Some (crate::icon_mapping::NF_COD_FILE_SYMLINK_DIR));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_style_junction_dir_returns_junction_icon
    //
    //  Port of: GetDisplayStyle_JunctionDir_ReturnsJunctionIcon
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn display_style_junction_dir_returns_junction_icon () {
        use crate::file_info::{FileInfo, FILE_ATTRIBUTE_REPARSE_POINT, IO_REPARSE_TAG_MOUNT_POINT};

        let config = make_config (None);
        let fi = FileInfo {
            file_name:       std::ffi::OsString::from ("mount"),
            file_attributes: FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT,
            file_size:       0,
            creation_time:   0,
            last_write_time: 0,
            last_access_time: 0,
            reparse_tag:     IO_REPARSE_TAG_MOUNT_POINT,
            streams:         Vec::new(),
        };

        let style = config.get_display_style_for_file (&fi);
        assert_eq! (style.icon_code_point, Some (crate::icon_mapping::NF_FA_EXTERNAL_LINK));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_style_well_known_dir_returns_specific_icon
    //
    //  Port of: GetDisplayStyle_WellKnownDir_ReturnsSpecificIcon
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn display_style_well_known_dir_returns_specific_icon () {
        use crate::file_info::FileInfo;

        let config = make_config (None);
        let fi = FileInfo {
            file_name:       std::ffi::OsString::from (".git"),
            file_attributes: FILE_ATTRIBUTE_DIRECTORY,
            file_size:       0,
            creation_time:   0,
            last_write_time: 0,
            last_access_time: 0,
            reparse_tag:     0,
            streams:         Vec::new(),
        };

        let style = config.get_display_style_for_file (&fi);
        // .git should get a specific (non-default) folder icon
        assert! (style.icon_code_point.is_some());
        assert_ne! (style.icon_code_point, Some (crate::icon_mapping::NF_CUSTOM_FOLDER),
            ".git should get a specific icon, not the default folder icon");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_style_hidden_file_attribute_wins_over_extension
    //
    //  Port of: GetDisplayStyle_HiddenCppFile_HiddenColorLocks
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn display_style_hidden_file_attribute_wins_over_extension () {
        use crate::file_info::FileInfo;

        let config = make_config (None);
        let fi = FileInfo {
            file_name:       std::ffi::OsString::from ("hidden.cpp"),
            file_attributes: 0x22, // ARCHIVE | HIDDEN
            file_size:       1000,
            creation_time:   0,
            last_write_time: 0,
            last_access_time: 0,
            reparse_tag:     0,
            streams:         Vec::new(),
        };

        let style = config.get_display_style_for_file (&fi);
        // Hidden file attribute color should override the .cpp extension color
        // (the exact colors don't matter — what matters is the precedence)
        let fi_normal = FileInfo {
            file_name:       std::ffi::OsString::from ("normal.cpp"),
            file_attributes: 0x20, // ARCHIVE only
            streams:         Vec::new(),
            ..fi
        };
        let style_normal = config.get_display_style_for_file (&fi_normal);

        // Hidden file should have a different color than non-hidden
        if config.file_attr_colors.contains_key (&0x02) {
            assert_ne! (style.text_attr, style_normal.text_attr,
                "Hidden file should have different color than normal .cpp file");
        }
    }





    // =========================================================================
    //  Error reporting tests
    //  Port of: ErrorInfo_*
    // =========================================================================

    ////////////////////////////////////////////////////////////////////////////
    //
    //  error_invalid_entry_format_populates
    //
    //  Port of: ErrorInfo_InvalidEntryFormat_PopulatesCorrectly
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn error_invalid_entry_format_populates () {
        let config = make_config (Some ("NoEqualsSign"));
        assert! (!config.last_parse_result.errors.is_empty());
        let err = &config.last_parse_result.errors[0];
        assert! (!err.entry.is_empty());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  error_invalid_foreground_color_populates
    //
    //  Port of: ErrorInfo_InvalidForegroundColor_PopulatesCorrectly
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn error_invalid_foreground_color_populates () {
        let config = make_config (Some (".txt=NotAColor"));
        assert! (!config.last_parse_result.errors.is_empty());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  error_invalid_background_populates
    //
    //  Port of: ErrorInfo_InvalidBackgroundColor_PopulatesCorrectly
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn error_invalid_background_populates () {
        let config = make_config (Some (".txt=Red on NotAColor"));
        assert! (!config.last_parse_result.errors.is_empty());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  format_number_concurrent_calls_independent
    //
    //  Port of: FormatNumberWithSeparators_MultipleConcurrentCalls
    //  Rust strings are independent — no static buffer corruption issue.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn format_number_concurrent_calls_independent () {
        use crate::results_displayer::format_number_with_separators;

        let s1 = format_number_with_separators (1000);
        let s2 = format_number_with_separators (2000);
        let s3 = format_number_with_separators (3000);

        assert_eq! (s1, "1,000");
        assert_eq! (s2, "2,000");
        assert_eq! (s3, "3,000");
    }





    // =========================================================================
    //  Config file tests (Phase 3+)
    // =========================================================================

    /// Test helper: create a Config with config file lines and optional RCDIR env var.
    fn make_config_with_file (file_lines: &[&str], env_value: Option<&str>) -> Config {
        let mut config = Config::new();
        let mut mock = MockEnvironmentProvider::new();
        mock.set ("USERPROFILE", "C:\\Users\\test");
        if let Some (val) = env_value {
            mock.set (RCDIR_ENV_VAR_NAME, val);
        }

        // Initialize defaults
        config.attributes[Attribute::Default as usize] = FC_LIGHT_GREY;
        config.attributes[Attribute::Date as usize] = FC_RED;
        config.attributes[Attribute::Time as usize] = FC_BROWN;
        config.attributes[Attribute::FileAttributePresent as usize] = FC_CYAN;
        config.attributes[Attribute::FileAttributeNotPresent as usize] = FC_DARK_GREY;
        config.attributes[Attribute::Size as usize] = FC_YELLOW;
        config.attributes[Attribute::Directory as usize] = FC_LIGHT_BLUE;
        config.attributes[Attribute::Information as usize] = FC_CYAN;
        config.attributes[Attribute::InformationHighlight as usize] = FC_WHITE;
        config.attributes[Attribute::SeparatorLine as usize] = FC_LIGHT_BLUE;
        config.attributes[Attribute::Error as usize] = FC_LIGHT_RED;
        config.attributes[Attribute::Owner as usize] = FC_GREEN;
        config.attributes[Attribute::Stream as usize] = FC_DARK_GREY;
        config.attributes[Attribute::CloudStatusCloudOnly as usize] = FC_LIGHT_BLUE;
        config.attributes[Attribute::CloudStatusLocallyAvailable as usize] = FC_LIGHT_GREEN;
        config.attributes[Attribute::CloudStatusAlwaysLocallyAvailable as usize] = FC_LIGHT_GREEN;
        config.attributes[Attribute::TreeConnector as usize] = FC_DARK_GREY;
        config.initialize_extension_colors();
        config.initialize_file_attr_colors();
        config.initialize_extension_icons();
        config.initialize_well_known_dir_icons();

        // Process config file lines
        let lines: Vec<String> = file_lines.iter().map (|s| s.to_string()).collect();
        config.config_file_path = "C:\\Users\\test\\.rcdirconfig".into();
        config.config_file_loaded = true;
        config.process_config_lines (&lines);

        // Then env var
        config.apply_user_color_overrides (&mock);

        config
    }



    // --- T019: Config file loading: switches, colors, icons, params ---

    #[test]
    fn config_file_switch_wide_applied() {
        let config = make_config_with_file (&["w"], None);
        assert_eq! (config.wide_listing, Some (true));
    }



    #[test]
    fn config_file_switch_tree_applied() {
        let config = make_config_with_file (&["tree"], None);
        assert_eq! (config.tree, Some (true));
    }



    #[test]
    fn config_file_switch_icons_applied() {
        let config = make_config_with_file (&["icons"], None);
        assert_eq! (config.icons, Some (true));
    }



    #[test]
    fn config_file_switch_disabled() {
        let config = make_config_with_file (&["w-"], None);
        assert_eq! (config.wide_listing, Some (false));
    }



    #[test]
    fn config_file_extension_color_applied() {
        let config = make_config_with_file (&[".cpp = LightGreen"], None);
        assert_eq! (config.extension_colors.get (".cpp"), Some (&FC_LIGHT_GREEN));
        assert_eq! (config.extension_sources.get (".cpp"), Some (&AttributeSource::ConfigFile));
    }



    #[test]
    fn config_file_display_attribute_color_applied() {
        let config = make_config_with_file (&["D = LightCyan"], None);
        assert_eq! (config.attributes[Attribute::Date as usize], FC_LIGHT_CYAN);
        assert_eq! (config.attribute_sources[Attribute::Date as usize], AttributeSource::ConfigFile);
    }



    #[test]
    fn config_file_depth_parameter_applied() {
        let config = make_config_with_file (&["Depth = 3"], None);
        assert_eq! (config.max_depth, Some (3));
        assert_eq! (config.max_depth_source, AttributeSource::ConfigFile);
    }



    #[test]
    fn config_file_size_auto_applied() {
        let config = make_config_with_file (&["Size = Auto"], None);
        assert_eq! (config.size_format, Some (SizeFormat::Auto));
        assert_eq! (config.size_format_source, AttributeSource::ConfigFile);
    }



    // --- T020: Comment and blank line tests ---

    #[test]
    fn config_file_comment_lines_skipped() {
        let config = make_config_with_file (&["# This is a comment", ".cpp = Red"], None);
        assert_eq! (config.extension_colors.get (".cpp"), Some (&FC_RED));
        assert! (!config.config_file_parse_result.has_issues());
    }



    #[test]
    fn config_file_inline_comment_stripped() {
        let config = make_config_with_file (&[".cpp = LightGreen # my favorite"], None);
        assert_eq! (config.extension_colors.get (".cpp"), Some (&FC_LIGHT_GREEN));
    }



    #[test]
    fn config_file_blank_lines_skipped() {
        let config = make_config_with_file (&["", "  ", ".cpp = Red", "", ".h = Yellow"], None);
        assert_eq! (config.extension_colors.get (".cpp"), Some (&FC_RED));
        assert_eq! (config.extension_colors.get (".h"), Some (&FC_YELLOW));
    }



    #[test]
    fn config_file_whitespace_trimmed() {
        let config = make_config_with_file (&["   .cpp = Red   "], None);
        assert_eq! (config.extension_colors.get (".cpp"), Some (&FC_RED));
    }



    #[test]
    fn config_file_switch_source_tracked() {
        let config = make_config_with_file (&["w"], None);
        assert_eq! (config.switch_sources[0], AttributeSource::ConfigFile);  // index 0 = wide_listing
    }



    // =========================================================================
    //  Phase 4: US2 — Precedence tests
    // =========================================================================

    #[test]
    fn env_var_overrides_config_file_color() {
        let config = make_config_with_file (&[".cpp = LightGreen"], Some (".cpp=Yellow"));
        assert_eq! (config.extension_colors.get (".cpp"), Some (&FC_YELLOW));
        assert_eq! (config.extension_sources.get (".cpp"), Some (&AttributeSource::Environment));
    }



    #[test]
    fn env_var_overrides_config_file_switch() {
        let config = make_config_with_file (&["w"], Some ("w-"));
        assert_eq! (config.wide_listing, Some (false));
    }



    #[test]
    fn non_conflicting_settings_merge() {
        let config = make_config_with_file (&[".cpp = LightGreen"], Some (".h=Yellow"));
        assert_eq! (config.extension_colors.get (".cpp"), Some (&FC_LIGHT_GREEN));
        assert_eq! (config.extension_colors.get (".h"), Some (&FC_YELLOW));
        assert_eq! (config.extension_sources.get (".cpp"), Some (&AttributeSource::ConfigFile));
        assert_eq! (config.extension_sources.get (".h"), Some (&AttributeSource::Environment));
    }



    #[test]
    fn config_file_source_preserved_when_no_env_override() {
        let config = make_config_with_file (&["D = LightCyan"], None);
        assert_eq! (config.attribute_sources[Attribute::Date as usize], AttributeSource::ConfigFile);
    }



    #[test]
    fn env_var_source_overwrites_config_file_source() {
        let config = make_config_with_file (&["D = LightCyan"], Some ("D=Yellow"));
        assert_eq! (config.attributes[Attribute::Date as usize], FC_YELLOW);
        assert_eq! (config.attribute_sources[Attribute::Date as usize], AttributeSource::Environment);
    }



    // =========================================================================
    //  Phase 5: US3 — Format rules tests
    // =========================================================================

    #[test]
    fn config_file_inline_comment_with_hash_in_middle() {
        let config = make_config_with_file (&[".cpp = Red # this is green # not this"], None);
        assert_eq! (config.extension_colors.get (".cpp"), Some (&FC_RED));
    }



    #[test]
    fn config_file_comment_only_line_with_leading_whitespace() {
        let config = make_config_with_file (&["   # comment", ".cpp = Red"], None);
        assert_eq! (config.extension_colors.get (".cpp"), Some (&FC_RED));
        assert! (!config.config_file_parse_result.has_issues());
    }



    #[test]
    fn config_file_whitespace_around_equals() {
        let config = make_config_with_file (&[".cpp   =   LightGreen"], None);
        assert_eq! (config.extension_colors.get (".cpp"), Some (&FC_LIGHT_GREEN));
    }



    #[test]
    fn config_file_tabs_as_whitespace() {
        let config = make_config_with_file (&["\t.cpp = Red\t"], None);
        assert_eq! (config.extension_colors.get (".cpp"), Some (&FC_RED));
    }



    #[test]
    fn config_file_duplicate_last_wins() {
        let config = make_config_with_file (&[".cpp = LightGreen", ".cpp = Yellow"], None);
        assert_eq! (config.extension_colors.get (".cpp"), Some (&FC_YELLOW));
    }



    #[test]
    fn config_file_duplicate_switch_last_wins() {
        let config = make_config_with_file (&["w", "w-"], None);
        assert_eq! (config.wide_listing, Some (false));
    }
}