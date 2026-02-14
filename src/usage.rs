// usage.rs — Help screen, env-var help, config display, error display
//
// Port of: Usage.h, Usage.cpp
// Implements T031-T035 (US-15: Help Display)

use crate::color::*;
use crate::config::{Attribute, AttributeSource, RCDIR_ENV_VAR_NAME};
use crate::console::Console;





////////////////////////////////////////////////////////////////////////////////

pub const VERSION_STRING:    &str = env!("RCDIR_VERSION_STRING");
pub const VERSION_YEAR:      &str = env!("RCDIR_VERSION_YEAR");
pub const BUILD_TIMESTAMP:   &str = env!("RCDIR_BUILD_TIMESTAMP");





////////////////////////////////////////////////////////////////////////////////
//
//  architecture
//
//  Returns the current CPU architecture as a display string.
//
////////////////////////////////////////////////////////////////////////////////

fn architecture() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "ARM64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else {
        "unknown"
    }
}





////////////////////////////////////////////////////////////////////////////////

pub const CIRCLE_HOLLOW:      char = '\u{25CB}';  // ○ Cloud-only
pub const CIRCLE_HALF_FILLED: char = '\u{25D0}';  // ◐ Locally available
pub const CIRCLE_FILLED:      char = '\u{25CF}';  // ● Always locally available
pub const LINE_HORIZONTAL:    char = '\u{2500}';  // ─ Horizontal line
pub const COPYRIGHT:          char = '\u{00A9}';  // ©
pub const OVERLINE:           char = '\u{203E}';  // ‾





////////////////////////////////////////////////////////////////////////////////

struct DisplayItemInfo {
    name: &'static str,
    attr: Attribute,
}





const DISPLAY_ITEM_INFOS: &[DisplayItemInfo] = &[
    DisplayItemInfo { name: "Default",                 attr: Attribute::Default },
    DisplayItemInfo { name: "Date",                    attr: Attribute::Date },
    DisplayItemInfo { name: "Time",                    attr: Attribute::Time },
    DisplayItemInfo { name: "File attribute present",  attr: Attribute::FileAttributePresent },
    DisplayItemInfo { name: "File attribute absent",   attr: Attribute::FileAttributeNotPresent },
    DisplayItemInfo { name: "Size",                    attr: Attribute::Size },
    DisplayItemInfo { name: "Directory",               attr: Attribute::Directory },
    DisplayItemInfo { name: "Information",             attr: Attribute::Information },
    DisplayItemInfo { name: "Info highlight",          attr: Attribute::InformationHighlight },
    DisplayItemInfo { name: "Separator line",          attr: Attribute::SeparatorLine },
    DisplayItemInfo { name: "Error",                   attr: Attribute::Error },
    DisplayItemInfo { name: "Owner",                   attr: Attribute::Owner },
    DisplayItemInfo { name: "Stream",                  attr: Attribute::Stream },
];





////////////////////////////////////////////////////////////////////////////////

struct CloudStatusInfo {
    attr:      Attribute,
    base_name: &'static str,
    symbol:    char,
}





const CLOUD_STATUS_INFOS: &[CloudStatusInfo] = &[
    CloudStatusInfo { attr: Attribute::CloudStatusCloudOnly,              base_name: "CloudOnly",              symbol: CIRCLE_HOLLOW },
    CloudStatusInfo { attr: Attribute::CloudStatusLocallyAvailable,       base_name: "LocallyAvailable",       symbol: CIRCLE_HALF_FILLED },
    CloudStatusInfo { attr: Attribute::CloudStatusAlwaysLocallyAvailable, base_name: "AlwaysLocallyAvailable", symbol: CIRCLE_FILLED },
];





////////////////////////////////////////////////////////////////////////////////

struct FileAttrInfo {
    name:      &'static str,
    letter:    char,
    attribute: u32,
}





const FILE_ATTR_INFOS: &[FileAttrInfo] = &[
    FileAttrInfo { name: "Read-only",     letter: 'R', attribute: 0x0001 },
    FileAttrInfo { name: "Hidden",        letter: 'H', attribute: 0x0002 },
    FileAttrInfo { name: "System",        letter: 'S', attribute: 0x0004 },
    FileAttrInfo { name: "Archive",       letter: 'A', attribute: 0x0020 },
    FileAttrInfo { name: "Temporary",     letter: 'T', attribute: 0x0040 },
    FileAttrInfo { name: "Encrypted",     letter: 'E', attribute: 0x4000 },
    FileAttrInfo { name: "Compressed",    letter: 'C', attribute: 0x0800 },
    FileAttrInfo { name: "Reparse point", letter: 'P', attribute: 0x0400 },
    FileAttrInfo { name: "Sparse file",   letter: '0', attribute: 0x0200 },
];





////////////////////////////////////////////////////////////////////////////////

struct SwitchInfo {
    name:        &'static str,
    description: &'static str,
}





const SWITCH_INFOS: &[SwitchInfo] = &[
    SwitchInfo { name: "W",       description: "Wide listing format" },
    SwitchInfo { name: "S",       description: "Recurse into subdirectories" },
    SwitchInfo { name: "P",       description: "Display performance timing" },
    SwitchInfo { name: "M",       description: "Multi-threaded enumeration" },
    SwitchInfo { name: "B",       description: "Bare listing format" },
    SwitchInfo { name: "Owner",   description: "Display file ownership" },
    SwitchInfo { name: "Streams", description: "Display alternate data streams" },
];





////////////////////////////////////////////////////////////////////////////////
//
//  is_powershell
//
//  Check whether the current shell is PowerShell.
//
////////////////////////////////////////////////////////////////////////////////

fn is_powershell() -> bool {
    std::env::var("PSModulePath").is_ok()
}





////////////////////////////////////////////////////////////////////////////////
//
//  is_env_var_set
//
//  Check whether an environment variable is defined.
//
////////////////////////////////////////////////////////////////////////////////

fn is_env_var_set(name: &str) -> bool {
    std::env::var(name).is_ok()
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_usage
//
//  Display the main usage/help screen.
//  Port of: CUsage::DisplayUsage
//
////////////////////////////////////////////////////////////////////////////////

pub fn display_usage(console: &mut Console, prefix: char) {
    let short = if prefix == '-' { "-" } else { "/" };
    let long  = if prefix == '-' { "--" } else { "/" };
    let m_dis = if prefix == '-' { " -M-" } else { " /M-" };
    let lpad  = if prefix == '-' { "" } else { " " };

    // "Technicolor" with rainbow per-character cycling
    console.puts(Attribute::Default, "");
    console.print_colorful_string("Rusticolor");

    // Header: product name continuation, version, copyright
    console.color_printf(&format!("\
{{Information}} Directory version {ver} {arch} ({ts})
Copyright {copy} 2004-{year} by Robert Elmer

{{InformationHighlight}}RCDIR{{Information}} \
         [{{InformationHighlight}}drive:{{Information}}]\
         [{{InformationHighlight}}path{{Information}}]\
         [{{InformationHighlight}}filename{{Information}}] \
         [{{InformationHighlight}}{short}A{{Information}}[[:]{{InformationHighlight}}attributes{{Information}}]] \
         [{{InformationHighlight}}{short}O{{Information}}[[:]{{InformationHighlight}}sortorder{{Information}}]] \
         [{{InformationHighlight}}{short}T{{Information}}[[:]{{InformationHighlight}}timefield{{Information}}]] \
         [{{InformationHighlight}}{short}S{{Information}}] \
         [{{InformationHighlight}}{short}W{{Information}}] \
         [{{InformationHighlight}}{short}B{{Information}}] \
         [{{InformationHighlight}}{short}P{{Information}}] \
         [{{InformationHighlight}}{short}M{{Information}}] \
         [{{InformationHighlight}}{long}Env{{Information}}] \
         [{{InformationHighlight}}{long}Config{{Information}}] \
         [{{InformationHighlight}}{long}Owner{{Information}}] \
         [{{InformationHighlight}}{long}Streams{{Information}}]",
        ver  = VERSION_STRING,
        arch = architecture(),
        ts   = BUILD_TIMESTAMP,
        copy = COPYRIGHT,
        year = VERSION_YEAR,
    ));

    #[cfg(debug_assertions)]
    console.color_printf(&format!(
        "{{Information}} [{{InformationHighlight}}{long}Debug{{Information}}]"
    ));

    // Body: switch descriptions, attribute codes, cloud symbols, sort/time fields
    // Multiline string literal — source indentation = output indentation (WYSIWYG).
    // Only \ continuation is used mid-line to join two-column attribute pairs.
    console.color_puts(&format!("\
{{Information}}


  [drive:][path][filename]
              Specifies drive, directory, and/or files to list.

  {{InformationHighlight}}{short}A{{Information}}          Displays files with specified attributes.
  attributes   {{InformationHighlight}}D{{Information}}  Directories                {{InformationHighlight}}R{{Information}}  Read-only files
               {{InformationHighlight}}H{{Information}}  Hidden files               {{InformationHighlight}}A{{Information}}  Files ready for archiving
               {{InformationHighlight}}S{{Information}}  System files               {{InformationHighlight}}T{{Information}}  Temporary files
               {{InformationHighlight}}E{{Information}}  Encrypted files            {{InformationHighlight}}C{{Information}}  Compressed files
               {{InformationHighlight}}P{{Information}}  Reparse points             {{InformationHighlight}}0{{Information}}  Sparse files
               {{InformationHighlight}}X{{Information}}  Not content indexed        {{InformationHighlight}}I{{Information}}  Integrity stream (ReFS)
               {{InformationHighlight}}B{{Information}}  No scrub data (ReFS)       {{InformationHighlight}}O{{Information}}  Cloud-only (not local)
               {{InformationHighlight}}L{{Information}}  Locally available          {{InformationHighlight}}V{{Information}}  Always locally available
               {{InformationHighlight}}-{{Information}}  Prefix meaning not

  Cloud status symbols shown between file size and name:
               {{CloudStatusCloudOnly}}{CIRCLE_HOLLOW}{{Information}}  Cloud-only (not locally available)
               {{CloudStatusLocallyAvailable}}{CIRCLE_HALF_FILLED}{{Information}}  Locally available (can be freed)
               {{CloudStatusAlwaysLocallyAvailable}}{CIRCLE_FILLED}{{Information}}  Always locally available (pinned)

  {{InformationHighlight}}{short}O{{Information}}          List by files in sorted order.
  sortorder    {{InformationHighlight}}N{{Information}}  By name (alphabetic)       {{InformationHighlight}}S{{Information}}  By size (smallest first)
               {{InformationHighlight}}E{{Information}}  By extension (alphabetic)  {{InformationHighlight}}D{{Information}}  By date/time (oldest first)
               {{InformationHighlight}}-{{Information}}  Prefix to reverse order

  {{InformationHighlight}}{short}T{{Information}}          Selects the time field for display and sorting.
  timefield    {{InformationHighlight}}C{{Information}}  Creation time              {{InformationHighlight}}A{{Information}}  Last access time
               {{InformationHighlight}}W{{Information}}  Last write time (default)

  {{InformationHighlight}}{short}S{{Information}}          Displays files in specified directory and all subdirectories.
  {{InformationHighlight}}{short}W{{Information}}          Displays results in a wide listing format.
  {{InformationHighlight}}{short}B{{Information}}          Displays bare file names only (no headers, footers, or details).
  {{InformationHighlight}}{short}P{{Information}}          Displays performance timing information.
  {{InformationHighlight}}{short}M{{Information}}          Enables multi-threaded enumeration (default). Use{{InformationHighlight}}{m_dis}{{Information}} to disable.
  {{InformationHighlight}}{long}Env{{Information}}       {lpad}Displays {RCDIR_ENV_VAR_NAME} help, syntax, and current value.
  {{InformationHighlight}}{long}Config{{Information}}    {lpad}Displays current color configuration for all items and extensions.
  {{InformationHighlight}}{long}Owner{{Information}}     {lpad}Displays file owner (DOMAIN\\User) for each file.
  {{InformationHighlight}}{long}Streams{{Information}}   {lpad}Displays alternate data streams (NTFS only)."
    ));

    #[cfg(debug_assertions)]
    console.color_puts(&format!(
        "\n  {{InformationHighlight}}{long}Debug{{Information}}     {lpad}Displays raw file attributes in hex for diagnosing edge cases."
    ));
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_env_var_help
//
//  Display RCDIR environment variable help with syntax, colors, example,
//  current value.
//  Port of: CUsage::DisplayEnvVarHelp
//
////////////////////////////////////////////////////////////////////////////////

pub fn display_env_var_help(console: &mut Console, prefix: char) {
    let (syntax_cmd, syntax_suffix, example_cmd) = if is_powershell() {
        (
            format!("  {{InformationHighlight}}$env:{RCDIR_ENV_VAR_NAME}{{Information}} = \""),
            "\"",
            format!("{{Information}}  Example: {{InformationHighlight}}$env:{RCDIR_ENV_VAR_NAME}{{Information}} = \"W;D=LightGreen;S=Yellow;Attr:H=DarkGrey;.cpp=White on Blue\""),
        )
    } else {
        (
            format!("  set {{InformationHighlight}}{RCDIR_ENV_VAR_NAME}{{Information}} ="),
            "",
            format!("{{Information}}  Example: {{InformationHighlight}}set {RCDIR_ENV_VAR_NAME}{{Information}} = W;D=LightGreen;S=Yellow;Attr:H=DarkGrey;.cpp=White on Blue"),
        )
    };

    // Multiline string literal — source indentation = output indentation (WYSIWYG).
    // The syntax line uses \\ continuation to build a single long output line.
    console.color_puts(&format!("
{{Information}}Set the {{InformationHighlight}}{RCDIR_ENV_VAR_NAME}{{Information}} environment variable to override default colors for \
display items, file attributes, or file extensions:
{syntax_cmd}[{{InformationHighlight}}<Switch>{{Information}}] | \
[{{InformationHighlight}}<Item>{{Information}} | \
{{InformationHighlight}}Attr:<fileattr>{{Information}} | \
{{InformationHighlight}}<.ext>{{Information}}] = \
{{InformationHighlight}}<Fore>{{Information}} [on {{InformationHighlight}}<Back>{{Information}}][;...]\
{syntax_suffix}

  {{InformationHighlight}}<Switch>{{Information}}    A command-line switch:
                  {{InformationHighlight}}W{{Information}}        Wide listing format
                  {{InformationHighlight}}P{{Information}}        Display performance timing information
                  {{InformationHighlight}}S{{Information}}        Recurse into subdirectories
                  {{InformationHighlight}}M{{Information}}        Enables multi-threaded enumeration (default); use {{InformationHighlight}}M-{{Information}} to disable
                  {{InformationHighlight}}Owner{{Information}}    Display file ownership
                  {{InformationHighlight}}Streams{{Information}}  Display alternate data streams (NTFS)

  {{InformationHighlight}}<Item>{{Information}}      A display item:
                  {{InformationHighlight}}D{{Information}}  Date                     {{InformationHighlight}}T{{Information}}  Time
                  {{InformationHighlight}}S{{Information}}  Size                     {{InformationHighlight}}R{{Information}}  Directory name
                  {{InformationHighlight}}I{{Information}}  Information              {{InformationHighlight}}H{{Information}}  Information highlight
                  {{InformationHighlight}}E{{Information}}  Error                    {{InformationHighlight}}F{{Information}}  File (default)
                  {{InformationHighlight}}O{{Information}}  Owner                    {{InformationHighlight}}M{{Information}}  Stream

              Cloud status (use full name, e.g., {{InformationHighlight}}CloudOnly=Blue{{Information}}):
                  {{InformationHighlight}}CloudOnly{{Information}}                   {{InformationHighlight}}LocallyAvailable{{Information}}
                  {{InformationHighlight}}AlwaysLocallyAvailable{{Information}}

  {{InformationHighlight}}<.ext>{{Information}}      A file extension, including the leading period.

  {{InformationHighlight}}<FileAttr>{{Information}}  A file attribute (see file attributes below)
                  {{InformationHighlight}}R{{Information}}  Read-only                {{InformationHighlight}}H{{Information}}  Hidden
                  {{InformationHighlight}}S{{Information}}  System                   {{InformationHighlight}}A{{Information}}  Archive
                  {{InformationHighlight}}T{{Information}}  Temporary                {{InformationHighlight}}E{{Information}}  Encrypted
                  {{InformationHighlight}}C{{Information}}  Compressed               {{InformationHighlight}}P{{Information}}  Reparse point
                  {{InformationHighlight}}0{{Information}}  Sparse file

  {{InformationHighlight}}<Fore>{{Information}}      Foreground color
  {{InformationHighlight}}<Back>{{Information}}      Background color"
    ));

    display_color_chart(console);

    console.color_puts(&format!("{{Default}}{example_cmd}\n"));

    if is_env_var_set(RCDIR_ENV_VAR_NAME) {
        display_env_var_current_value(console, RCDIR_ENV_VAR_NAME);
        display_env_var_decoded_settings(console);
        display_env_var_issues(console, prefix, false);
    } else {
        console.color_puts(&format!(
            "  {{InformationHighlight}}{RCDIR_ENV_VAR_NAME}{{Information}} environment variable is not set."
        ));
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_current_configuration
//
//  Display current color configuration with source tracking.
//  Port of: CUsage::DisplayCurrentConfiguration
//
////////////////////////////////////////////////////////////////////////////////

pub fn display_current_configuration(console: &mut Console, prefix: char) {
    if is_env_var_set(RCDIR_ENV_VAR_NAME) {
        display_env_var_issues(console, prefix, true);
    }

    display_configuration_table(console);
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_env_var_issues
//
//  Display validation errors from the RCDIR env var.
//  Port of: CUsage::DisplayEnvVarIssues
//
////////////////////////////////////////////////////////////////////////////////

pub fn display_env_var_issues(console: &mut Console, prefix: char, show_hint: bool) {
    let long = if prefix == '-' { "--" } else { "/" };

    let config = console.config_arc();
    let result = config.validate_environment_variable();

    if !result.has_issues() {
        return;
    }

    let hint = if show_hint {
        format!(" (see {}env for help)", long)
    } else {
        String::new()
    };

    console.color_printf(&format!(
        "{{Default}}\n{{Error}}There are some problems with your {} environment variable{}:\n",
        RCDIR_ENV_VAR_NAME, hint
    ));

    for error in &result.errors {
        let prefix_len = 2 + error.message.len() + 5 + error.invalid_text_offset;
        let underline: String = std::iter::repeat_n(OVERLINE, error.invalid_text.len()).collect();

        console.color_printf(&format!(
            "{{Error}}  {} in \"{}\"\n", error.message, error.entry
        ));
        console.color_printf(&format!(
            "{{Default}}{:>width$}{{Error}}{}\n\n", "", underline, width = prefix_len
        ));
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_configuration_table
//
//  Display color configuration sections (display items, file attrs, extensions).
//
////////////////////////////////////////////////////////////////////////////////

fn display_configuration_table(console: &mut Console) {
    let column_width_attr   = 27;
    let column_width_source = 15;

    display_attribute_configuration(console, column_width_attr, column_width_source);
    display_file_attribute_configuration(console, column_width_attr, column_width_source);
    display_extension_configuration(console, column_width_attr, column_width_source);
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_attribute_configuration
//
//  Display current display-item color assignments with source tracking.
//
////////////////////////////////////////////////////////////////////////////////

fn display_attribute_configuration(console: &mut Console, col_attr: usize, col_source: usize) {
    console.puts(Attribute::Information, "\nCurrent display item configuration:\n");

    let config = console.config_arc();

    for info in DISPLAY_ITEM_INFOS {
        let attr = config.attributes[info.attr as usize];
        let is_env = config.attribute_sources[info.attr as usize] == AttributeSource::Environment;

        display_item_and_source(console, info.name, attr, is_env, col_attr, col_source);
    }

    for info in CLOUD_STATUS_INFOS {
        let attr = config.attributes[info.attr as usize];
        let is_env = config.attribute_sources[info.attr as usize] == AttributeSource::Environment;
        let display = format!("{} ({})", info.base_name, info.symbol);

        display_item_and_source(console, &display, attr, is_env, col_attr, col_source);
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_file_attribute_configuration
//
//  Display file-attribute color configuration with source tracking.
//
////////////////////////////////////////////////////////////////////////////////

fn display_file_attribute_configuration(console: &mut Console, col_attr: usize, col_source: usize) {
    console.puts(Attribute::Information, "\nFile attribute color configuration:\n");

    let config = console.config_arc();

    for info in FILE_ATTR_INFOS {
        if let Some(style) = config.file_attr_colors.get(&info.attribute) {
            let is_env = style.source == AttributeSource::Environment;
            let label = format!("{} {}", info.letter, info.name);

            display_item_and_source(console, &label, style.attr, is_env, col_attr, col_source);
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_extension_configuration
//
//  Display file-extension color configuration.
//
////////////////////////////////////////////////////////////////////////////////

fn display_extension_configuration(console: &mut Console, _col_attr: usize, _col_source: usize) {
    console.puts(Attribute::Information, "\nFile extension color configuration:");

    let config = console.config_arc();
    let width = console.width() as usize;

    // Collect and sort extensions
    let mut extensions: Vec<(&String, &u16)> = config.extension_colors.iter().collect();
    extensions.sort_by_key(|(ext, _)| ext.as_str());

    let max_ext_len = extensions.iter().map(|(e, _)| e.len()).max().unwrap_or(6);
    let source_width = "Environment".len();
    let indent = 2;
    let available = if width > indent { width - indent } else { width };
    let min_col_width = max_ext_len + 2 + source_width + 3;

    let columns = if min_col_width > 0 && min_col_width <= available {
        std::cmp::max(1, available / min_col_width)
    } else {
        1
    };

    if columns == 1 {
        for (ext, color) in &extensions {
            let is_env = config.extension_sources.get(*ext)
                .is_some_and(|s| *s == AttributeSource::Environment);
            display_item_and_source(console, ext, **color, is_env, max_ext_len, source_width);
        }
    } else {
        display_extension_multi_column(console, &extensions, max_ext_len, source_width, available, columns);
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_extension_multi_column
//
//  Render extension colors in a multi-column layout.
//
////////////////////////////////////////////////////////////////////////////////

fn display_extension_multi_column(
    console: &mut Console,
    extensions: &[(&String, &u16)],
    max_ext_len: usize,
    source_width: usize,
    available: usize,
    columns: usize,
) {
    let col_width = std::cmp::max(1, available / columns);
    let rows = extensions.len().div_ceil(columns);
    let items_in_last_row = extensions.len() % columns;
    let full_rows = if items_in_last_row != 0 { rows - 1 } else { rows };

    console.puts(Attribute::Information, "");

    let config = console.config_arc();

    for row in 0..rows {
        console.printf_attr(Attribute::Information, "  ");

        for col in 0..columns {
            if row * columns + col >= extensions.len() {
                break;
            }

            // Column-major ordering
            let mut idx = row + (col * full_rows);
            if col < items_in_last_row {
                idx += col;
            } else {
                idx += items_in_last_row;
            }

            if idx >= extensions.len() {
                break;
            }

            let (ext, color) = extensions[idx];
            let is_env = config.extension_sources.get(ext)
                .is_some_and(|s| *s == AttributeSource::Environment);

            let bg_attr = config.attributes[Attribute::Default as usize] & BC_MASK;
            let source_attr = bg_attr | if is_env { FC_CYAN } else { FC_DARK_GREY };
            let source = if is_env { "Environment" } else { "Default" };
            let pad = if max_ext_len > ext.len() { max_ext_len - ext.len() } else { 0 };
            let used = max_ext_len + 2 + source_width;

            console.printf(*color, ext);
            console.printf_attr(Attribute::Information, &format!("{:pad$}  ", "", pad = pad));
            console.printf(source_attr, &format!("{:<width$}", source, width = source_width));

            if col_width > used {
                console.printf_attr(Attribute::Information, &format!("{:pad$}", "", pad = col_width - used));
            }
        }

        console.puts(Attribute::Default, "");
    }

    console.puts(Attribute::Default, "");
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_width
//
//  Returns the display width (in terminal columns) of a string.
//  Uses char count rather than byte length so that multi-byte Unicode
//  symbols like ○ ◐ ● (each 3 bytes, 1 column) are measured correctly.
//
////////////////////////////////////////////////////////////////////////////////

fn display_width(s: &str) -> usize {
    s.chars().count()
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_item_and_source
//
//  Display a configuration item name with its color and source label.
//
////////////////////////////////////////////////////////////////////////////////

fn display_item_and_source(console: &mut Console, item: &str, attr: u16, is_env: bool, col_item: usize, col_source: usize) {
    let config = console.config_arc();
    let bg_attr = config.attributes[Attribute::Default as usize] & BC_MASK;
    let source_attr = bg_attr | if is_env { FC_CYAN } else { FC_DARK_GREY };
    let source = if is_env { "Environment" } else { "Default" };
    let item_width = display_width(item);
    let pad = col_item.saturating_sub(item_width);

    console.printf_attr(Attribute::Information, "  ");
    console.printf(attr, item);
    console.printf_attr(Attribute::Information, &format!("{:pad$}  ", "", pad = pad));
    console.printf(source_attr, &format!("{:<width$}", source, width = col_source));
    console.printf(config.attributes[Attribute::Default as usize], "\n");
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_color_chart
//
//  Display the 16-color chart showing all foreground color names.
//
////////////////////////////////////////////////////////////////////////////////

fn display_color_chart(console: &mut Console) {
    const ROWS: &[(&str, &str)] = &[
        ("Black",     "DarkGrey"),
        ("Blue",      "LightBlue"),
        ("Green",     "LightGreen"),
        ("Cyan",      "LightCyan"),
        ("Red",       "LightRed"),
        ("Magenta",   "LightMagenta"),
        ("Brown",     "Yellow"),
        ("LightGrey", "White"),
    ];

    const LEFT_WIDTH: usize = 18;

    for &(left, right) in ROWS {
        let left_attr = get_color_attribute(console, left);
        let right_attr = get_color_attribute(console, right);
        let pad = if LEFT_WIDTH > left.len() { LEFT_WIDTH - left.len() } else { 0 };

        console.printf(console.config().attributes[Attribute::Default as usize], "                  ");
        console.printf(left_attr, left);
        console.printf(console.config().attributes[Attribute::Default as usize], &format!("{:pad$}", "", pad = pad));
        console.printf(right_attr, right);
        console.printf(console.config().attributes[Attribute::Default as usize], "\n");
    }

    console.puts(Attribute::Default, "");
}





////////////////////////////////////////////////////////////////////////////////
//
//  ensure_visible_color_attr
//
//  Adjust a color attribute so foreground is visible against the background.
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
        let contrast_back = if back & 0x80 != 0 { 0x00u16 } else { 0x70u16 }; // BC_Black or BC_LightGrey
        return fore | contrast_back;
    }

    fore | back
}





////////////////////////////////////////////////////////////////////////////////
//
//  get_color_attribute
//
//  Parse a color name and return a visible console attribute.
//
////////////////////////////////////////////////////////////////////////////////

fn get_color_attribute(console: &Console, color_name: &str) -> u16 {
    let default_attr = console.config().attributes[Attribute::Default as usize];
    let fore = parse_color_name(color_name, false).unwrap_or(FC_LIGHT_GREY);
    ensure_visible_color_attr(fore, default_attr)
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_env_var_current_value
//
//  Display the current value of the RCDIR env var with color-coded segments.
//
////////////////////////////////////////////////////////////////////////////////

fn display_env_var_current_value(console: &mut Console, env_name: &str) {
    let env_value = match std::env::var(env_name) {
        Ok(v) => v,
        Err(_) => return,
    };

    console.color_printf(&format!(
        "{{Information}}Your settings:{{Default}}\n\n  {{Information}}{env_name}{{Default}} = "
    ));

    if env_value.is_empty() {
        console.puts(Attribute::InformationHighlight, "<empty>");
        return;
    }

    console.printf(console.config().attributes[Attribute::Default as usize], "\"");

    let mut first = true;
    for segment in env_value.split(';') {
        if segment.is_empty() {
            continue;
        }

        if !first {
            console.printf(console.config().attributes[Attribute::Default as usize], ";");
        }
        first = false;

        display_env_var_segment(console, segment);
    }

    console.puts(Attribute::Default, "\"");
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_env_var_segment
//
//  Display a single semicolon-delimited segment of the env var value.
//
////////////////////////////////////////////////////////////////////////////////

fn display_env_var_segment(console: &mut Console, segment: &str) {
    let default_attr = console.config().attributes[Attribute::Default as usize];

    let eq_pos = match segment.find('=') {
        Some(p) => p,
        None => {
            console.printf(default_attr, segment);
            return;
        }
    };

    let key = &segment[..eq_pos];
    let value = &segment[eq_pos + 1..];
    let trimmed = value.trim();

    match parse_color_spec(trimmed) {
        Ok(color_attr) => {
            let visible = ensure_visible_color_attr(color_attr, default_attr);
            console.printf(visible, key);
            console.printf(default_attr, "=");
            console.printf(visible, value);
        }
        Err(_) => {
            console.printf(default_attr, segment);
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  display_env_var_decoded_settings
//
//  Display decoded switch and color settings from the env var.
//
////////////////////////////////////////////////////////////////////////////////

fn display_env_var_decoded_settings(console: &mut Console) {
    let config = console.config_arc();

    let has_switches = config.wide_listing.is_some()
        || config.bare_listing.is_some()
        || config.recurse.is_some()
        || config.perf_timer.is_some()
        || config.multi_threaded.is_some()
        || config.show_owner.is_some()
        || config.show_streams.is_some();

    let has_display_items = DISPLAY_ITEM_INFOS.iter().any(|i| {
        config.attribute_sources[i.attr as usize] == AttributeSource::Environment
    }) || CLOUD_STATUS_INFOS.iter().any(|i| {
        config.attribute_sources[i.attr as usize] == AttributeSource::Environment
    });

    let has_file_attrs = config.file_attr_colors.values().any(|s| {
        s.source == AttributeSource::Environment
    });

    let has_extensions = config.extension_sources.values().any(|s| {
        *s == AttributeSource::Environment
    });

    if !has_switches && !has_display_items && !has_file_attrs && !has_extensions {
        return;
    }

    if has_switches {
        console.puts(Attribute::Information, "    Switches:");
        let switch_values: [&Option<bool>; 7] = [
            &config.wide_listing,
            &config.recurse,
            &config.perf_timer,
            &config.multi_threaded,
            &config.bare_listing,
            &config.show_owner,
            &config.show_streams,
        ];
        for (i, info) in SWITCH_INFOS.iter().enumerate() {
            if switch_values[i].is_some() {
                console.printf(
                    config.attributes[Attribute::Default as usize],
                    &format!("      {:<8} {}\n", info.name, info.description),
                );
            }
        }
    }

    if has_display_items {
        console.color_puts("{Default}\n    {Information}Display item colors:");
        for info in DISPLAY_ITEM_INFOS {
            if config.attribute_sources[info.attr as usize] == AttributeSource::Environment {
                let attr = config.attributes[info.attr as usize];
                console.printf(config.attributes[Attribute::Default as usize], "      ");
                console.printf(attr, info.name);
                console.puts(Attribute::Default, "");
            }
        }

        for info in CLOUD_STATUS_INFOS {
            if config.attribute_sources[info.attr as usize] == AttributeSource::Environment {
                let attr = config.attributes[info.attr as usize];
                let display = format!("{} ({})", info.base_name, info.symbol);
                console.printf(config.attributes[Attribute::Default as usize], "      ");
                console.printf(attr, &display);
                console.puts(Attribute::Default, "");
            }
        }
    }

    if has_file_attrs {
        console.color_puts("{Default}\n    {Information}File attribute colors:");
        for info in FILE_ATTR_INFOS {
            if let Some(style) = config.file_attr_colors.get(&info.attribute)
                && style.source == AttributeSource::Environment
            {
                console.printf(config.attributes[Attribute::Default as usize], "      ");
                console.printf(style.attr, &format!("{} {}", info.letter, info.name));
                console.puts(Attribute::Default, "");
            }
        }
    }

    if has_extensions {
        console.color_puts("{Default}\n    {Information}File extension colors:");
        let mut env_exts: Vec<(&String, &u16)> = config.extension_colors.iter()
            .filter(|(ext, _)| {
                config.extension_sources.get(*ext)
                    .is_some_and(|s| *s == AttributeSource::Environment)
            })
            .collect();
        env_exts.sort_by_key(|(ext, _)| ext.as_str());

        for (ext, attr) in &env_exts {
            console.printf(config.attributes[Attribute::Default as usize], "      ");
            console.printf(**attr, ext);
            console.puts(Attribute::Default, "");
        }
    }
}
