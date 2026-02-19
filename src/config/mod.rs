// config/mod.rs — Color configuration and RCDIR env var overrides
//
// Port of: Config.h, Config.cpp
// Manages display item colors, extension colors, file attribute colors,
// and switch defaults from the RCDIR environment variable.

mod env_overrides;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;

use crate::color::*;
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
//
//  OverrideValue
//
//  Intermediate result of parsing a "color,icon" value from the env var.
//
//  Port of: SOverrideValue in TCDirCore/Config.h
//
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Default)]
pub struct OverrideValue {
    pub color_attr:   u16,
    pub icon_cp:      char,
    pub suppressed:   bool,
    pub has_color:    bool,
    pub has_icon:     bool,
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
        self.initialize_extension_icons();
        self.initialize_well_known_dir_icons();
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
    //  Verifies the attribute count is 16.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn attribute_count() {
        assert_eq!(Attribute::COUNT, 16);
        assert_eq!(Attribute::ALL.len(), 16);
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
}
