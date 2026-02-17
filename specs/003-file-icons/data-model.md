# Data Model: Nerd Font File & Folder Icons (Rust)

**Feature**: 003-file-icons | **Date**: 2026-02-16

---

## Entity Overview

```
Config (extended)
├── FileDisplayStyle           # Resolved color + icon for a file entry
├── extension_icons            # HashMap<String, char> — extension → icon glyph
├── extension_icon_sources     # HashMap<String, AttributeSource> — source tracking
├── well_known_dir_icons       # HashMap<String, char> — dir name → icon glyph
├── well_known_dir_icon_sources # HashMap<String, AttributeSource> — source tracking
├── file_attr_icons            # HashMap<u32, char> — attribute flag → icon glyph
├── icon_directory_default     # char — default directory icon
├── icon_file_default          # char — default file icon
├── icon_symlink               # char — symlink icon
├── icon_junction              # char — junction point icon
├── icon_cloud_only            # char — cloud-only NF glyph
├── icon_locally_available     # char — locally available NF glyph
├── icon_always_local          # char — always-local / pinned NF glyph
└── icons                      # Option<bool> — icons switch from RCDIR env var

NerdFontDetector (new module)
├── IconActivation             # Tri-state: Auto / ForceOn / ForceOff
├── DetectionResult            # Detected / NotDetected / Inconclusive
└── FontProber trait           # GDI calls injectable for testing

IconMapping (new module)
├── NF_* constants             # Named char constants for every NF code point
├── DEFAULT_EXTENSION_ICONS    # &[(&str, char)] static array
├── DEFAULT_WELL_KNOWN_DIR_ICONS # &[(&str, char)] static array
└── ATTRIBUTE_PRECEDENCE       # &[(u32, char)] static array (PSHERC0TA order)

CommandLine (extended)
└── icons                      # Option<bool> — /Icons, /Icons-
```

---

## New Types

### `icon_mapping.rs` — Nerd Font Constants

```rust
// Named constants replace magic hex literals throughout the codebase.
// Grouped by Nerd Font prefix, alphabetical within each group.
// Rust char is a Unicode scalar value (0–0x10FFFF, excluding surrogates).

// --- Custom (nf-custom-*) ---
pub const NF_CUSTOM_ASM:              char = '\u{E6AB}';
pub const NF_CUSTOM_ELIXIR:          char = '\u{E62D}';
pub const NF_CUSTOM_ELM:             char = '\u{E62C}';
pub const NF_CUSTOM_FOLDER:          char = '\u{E5FF}';
pub const NF_CUSTOM_FOLDER_CONFIG:   char = '\u{E5FC}';
pub const NF_CUSTOM_KOTLIN:          char = '\u{E634}';
pub const NF_CUSTOM_MSDOS:           char = '\u{E629}';

// --- Seti (nf-seti-*) ---
pub const NF_SETI_BICEP:             char = '\u{E63B}';
pub const NF_SETI_CONFIG:            char = '\u{E615}';
pub const NF_SETI_DB:                char = '\u{E64D}';
pub const NF_SETI_GIT:               char = '\u{E65D}';    // DEVIATION from TI
pub const NF_SETI_GITHUB:            char = '\u{E65B}';    // DEVIATION from TI
pub const NF_SETI_HTML:              char = '\u{E60E}';
pub const NF_SETI_JSON:              char = '\u{E60B}';
pub const NF_SETI_JULIA:             char = '\u{E624}';
pub const NF_SETI_LUA:               char = '\u{E620}';
pub const NF_SETI_MAKEFILE:          char = '\u{E673}';
pub const NF_SETI_NPM:               char = '\u{E616}';    // DEVIATION from TI
pub const NF_SETI_PROJECT:           char = '\u{E601}';
pub const NF_SETI_PYTHON:            char = '\u{E606}';    // DEVIATION from TI
pub const NF_SETI_SHELL:             char = '\u{E691}';
pub const NF_SETI_SVELTE:            char = '\u{E697}';
pub const NF_SETI_SWIFT:             char = '\u{E699}';
pub const NF_SETI_TERRAFORM:         char = '\u{E69A}';
pub const NF_SETI_TYPESCRIPT:        char = '\u{E628}';

// --- Dev (nf-dev-*) ---
pub const NF_DEV_AWS:                char = '\u{E7AD}';
pub const NF_DEV_CLOJURE:            char = '\u{E768}';
pub const NF_DEV_CSS3:               char = '\u{E749}';
pub const NF_DEV_DART:               char = '\u{E798}';
pub const NF_DEV_DATABASE:           char = '\u{E706}';
pub const NF_DEV_DOCKER:             char = '\u{E7B0}';
pub const NF_DEV_ERLANG:             char = '\u{E7B1}';
pub const NF_DEV_FSHARP:             char = '\u{E7A7}';
pub const NF_DEV_GO:                 char = '\u{E724}';
pub const NF_DEV_GROOVY:             char = '\u{E775}';
pub const NF_DEV_HASKELL:            char = '\u{E777}';
pub const NF_DEV_JAVASCRIPT_ALT:     char = '\u{E74E}';
pub const NF_DEV_LESS:               char = '\u{E758}';
pub const NF_DEV_MARKDOWN:           char = '\u{E73E}';
pub const NF_DEV_PERL:               char = '\u{E769}';
pub const NF_DEV_PHP:                char = '\u{E73D}';
pub const NF_DEV_REACT:              char = '\u{E7BA}';
pub const NF_DEV_RUST:               char = '\u{E7A8}';
pub const NF_DEV_SASS:               char = '\u{E74B}';
pub const NF_DEV_SCALA:              char = '\u{E737}';
pub const NF_DEV_VISUAL_STUDIO:      char = '\u{E70C}';
pub const NF_DEV_VSCODE:             char = '\u{E8DA}';    // DEVIATION from TI

// --- Font Awesome Extension (nf-fae-*) ---
pub const NF_FAE_JAVA:               char = '\u{E256}';

// --- Octicons (nf-oct-*) ---
pub const NF_OCT_FILE_BINARY:        char = '\u{F471}';
pub const NF_OCT_FILE_MEDIA:         char = '\u{F40F}';
pub const NF_OCT_FILE_ZIP:           char = '\u{F410}';
pub const NF_OCT_REPO:               char = '\u{F401}';
pub const NF_OCT_RUBY:               char = '\u{F43B}';
pub const NF_OCT_TERMINAL:           char = '\u{F489}';

// --- Font Awesome (nf-fa-*) ---
pub const NF_FA_ARCHIVE:             char = '\u{F187}';
pub const NF_FA_CERTIFICATE:         char = '\u{F0A3}';
pub const NF_FA_ENVELOPE:            char = '\u{F0E0}';
pub const NF_FA_EXTERNAL_LINK:       char = '\u{F08E}';
pub const NF_FA_FILE:                char = '\u{F15B}';
pub const NF_FA_FILE_AUDIO_O:        char = '\u{F1C7}';
pub const NF_FA_FILE_IMAGE_O:        char = '\u{F1C5}';
pub const NF_FA_FILE_PDF_O:          char = '\u{F1C1}';
pub const NF_FA_FILE_VIDEO_O:        char = '\u{F1C8}';
pub const NF_FA_FONT:                char = '\u{F031}';
pub const NF_FA_GEAR:                char = '\u{F013}';
pub const NF_FA_GITHUB_ALT:          char = '\u{F113}';
pub const NF_FA_KEY:                 char = '\u{F084}';
pub const NF_FA_LIST:                char = '\u{F03A}';
pub const NF_FA_LOCK:                char = '\u{F023}';
pub const NF_FA_USERS:               char = '\u{F0C0}';
pub const NF_FA_WINDOWS:             char = '\u{F17A}';

// --- Material Design (nf-md-*) — supplementary plane (surrogate pair in UTF-16) ---
pub const NF_MD_APPLICATION:         char = '\u{F08C6}';
pub const NF_MD_APPS:                char = '\u{F003B}';
pub const NF_MD_CACHED:              char = '\u{F00E8}';
pub const NF_MD_CLOUD_CHECK:         char = '\u{F0160}';
pub const NF_MD_CLOUD_OUTLINE:       char = '\u{F0163}';
pub const NF_MD_CONSOLE_LINE:        char = '\u{F07B7}';
pub const NF_MD_CONTACTS:            char = '\u{F06CB}';
pub const NF_MD_DESKTOP_CLASSIC:     char = '\u{F07C0}';
pub const NF_MD_FILE_DOCUMENT:       char = '\u{F0219}';
pub const NF_MD_FILE_EXCEL:          char = '\u{F021B}';
pub const NF_MD_FILE_POWERPOINT:     char = '\u{F0227}';
pub const NF_MD_FILE_WORD:           char = '\u{F022C}';
pub const NF_MD_FOLDER_DOWNLOAD:     char = '\u{F024D}';
pub const NF_MD_FOLDER_IMAGE:        char = '\u{F024F}';
pub const NF_MD_FOLDER_STAR:         char = '\u{F069D}';
pub const NF_MD_FORMAT_ALIGN_LEFT:   char = '\u{F0262}';
pub const NF_MD_LANGUAGE_C:          char = '\u{F0671}';
pub const NF_MD_LANGUAGE_CPP:        char = '\u{F0672}';
pub const NF_MD_LANGUAGE_CSHARP:     char = '\u{F031B}';
pub const NF_MD_LANGUAGE_R:          char = '\u{F07D4}';
pub const NF_MD_LANGUAGE_XAML:       char = '\u{F0673}';
pub const NF_MD_MICROSOFT_AZURE:     char = '\u{F0805}';
pub const NF_MD_MICROSOFT_ONEDRIVE:  char = '\u{F03CA}';
pub const NF_MD_MOVIE:               char = '\u{F0381}';
pub const NF_MD_MUSIC_BOX_MULTIPLE:  char = '\u{F0333}';
pub const NF_MD_NOTEBOOK:            char = '\u{F082E}';
pub const NF_MD_ELEPHANT:            char = '\u{F07C6}';
pub const NF_MD_PACKAGE_VARIANT:     char = '\u{F03D6}';
pub const NF_MD_PIN:                 char = '\u{F0403}';
pub const NF_MD_SHIP_WHEEL:          char = '\u{F0833}';
pub const NF_MD_SVG:                 char = '\u{F0721}';
pub const NF_MD_TEST_TUBE:           char = '\u{F0668}';
pub const NF_MD_VUEJS:               char = '\u{F0844}';
pub const NF_MD_TIMER:               char = '\u{F13AB}';
pub const NF_MD_XML:                 char = '\u{F05C0}';

// --- Codicons (nf-cod-*) ---
pub const NF_COD_FILE_SYMLINK_DIR:   char = '\u{EAED}';
pub const NF_COD_FOLDER_LIBRARY:     char = '\u{EBDF}';
pub const NF_COD_OUTPUT:             char = '\u{EB9D}';
pub const NF_COD_PACKAGE:            char = '\u{EB29}';
pub const NF_COD_PREVIEW:            char = '\u{EB2F}';
```

**Purpose**: Single source of truth for all Nerd Font code points. Every usage site references a named constant — no magic hex literals. Rust `char` handles the full Unicode range natively (including supplementary plane), so no `WideCharPair` / `CodePointToWideChars` equivalent is needed.

### `icon_mapping.rs` — Static Default Tables

```rust
/// Default extension → icon mappings.
/// Aligned to Terminal-Icons devblackops default theme (NF v3.4.0).
/// Keys are lowercase with leading dot (matching Config::extension_colors convention).
pub const DEFAULT_EXTENSION_ICONS: &[(&str, char)] = &[
    // C/C++
    (".c",       NF_MD_LANGUAGE_C),
    (".h",       NF_MD_LANGUAGE_C),
    (".cpp",     NF_MD_LANGUAGE_CPP),
    (".cxx",     NF_MD_LANGUAGE_CPP),
    (".c++",     NF_MD_LANGUAGE_CPP),
    (".hpp",     NF_MD_LANGUAGE_CPP),
    (".hxx",     NF_MD_LANGUAGE_CPP),
    (".asm",     NF_CUSTOM_ASM),
    (".cod",     NF_CUSTOM_ASM),
    (".i",       NF_CUSTOM_ASM),

    // C# / .NET
    (".cs",      NF_MD_LANGUAGE_CSHARP),
    (".csx",     NF_MD_LANGUAGE_CSHARP),
    (".resx",    NF_MD_XML),
    (".xaml",    NF_MD_LANGUAGE_XAML),

    // JavaScript / TypeScript
    (".js",      NF_DEV_JAVASCRIPT_ALT),
    (".mjs",     NF_DEV_JAVASCRIPT_ALT),
    (".cjs",     NF_DEV_JAVASCRIPT_ALT),
    (".jsx",     NF_DEV_REACT),
    (".ts",      NF_SETI_TYPESCRIPT),
    (".tsx",     NF_DEV_REACT),

    // Web
    (".html",    NF_SETI_HTML),
    (".htm",     NF_SETI_HTML),
    (".xhtml",   NF_SETI_HTML),
    (".css",     NF_DEV_CSS3),
    (".scss",    NF_DEV_SASS),
    (".sass",    NF_DEV_SASS),
    (".less",    NF_DEV_LESS),
    (".vue",     NF_MD_VUEJS),
    (".svelte",  NF_SETI_SVELTE),

    // Python (DEVIATION: nf-seti-python)
    (".py",      NF_SETI_PYTHON),
    (".pyw",     NF_SETI_PYTHON),
    (".ipynb",   NF_MD_NOTEBOOK),

    // Java
    (".java",    NF_FAE_JAVA),
    (".jar",     NF_FAE_JAVA),
    (".class",   NF_FAE_JAVA),
    (".gradle",  NF_MD_ELEPHANT),

    // Rust
    (".rs",      NF_DEV_RUST),

    // Go
    (".go",      NF_DEV_GO),

    // Ruby
    (".rb",      NF_OCT_RUBY),
    (".erb",     NF_OCT_RUBY),

    // F#
    (".fs",      NF_DEV_FSHARP),
    (".fsx",     NF_DEV_FSHARP),
    (".fsi",     NF_DEV_FSHARP),

    // Lua
    (".lua",     NF_SETI_LUA),

    // Perl
    (".pl",      NF_DEV_PERL),
    (".pm",      NF_DEV_PERL),

    // PHP
    (".php",     NF_DEV_PHP),

    // Haskell
    (".hs",      NF_DEV_HASKELL),

    // Dart
    (".dart",    NF_DEV_DART),

    // Kotlin
    (".kt",      NF_CUSTOM_KOTLIN),
    (".kts",     NF_CUSTOM_KOTLIN),

    // Swift
    (".swift",   NF_SETI_SWIFT),

    // Scala
    (".scala",   NF_DEV_SCALA),
    (".sc",      NF_DEV_SCALA),
    (".sbt",     NF_DEV_SCALA),

    // Clojure
    (".clj",     NF_DEV_CLOJURE),
    (".cljs",    NF_DEV_CLOJURE),
    (".cljc",    NF_DEV_CLOJURE),

    // Elixir / Erlang
    (".ex",      NF_CUSTOM_ELIXIR),
    (".exs",     NF_CUSTOM_ELIXIR),
    (".erl",     NF_DEV_ERLANG),

    // Groovy
    (".groovy",  NF_DEV_GROOVY),

    // Julia
    (".jl",      NF_SETI_JULIA),

    // R
    (".r",       NF_MD_LANGUAGE_R),
    (".rmd",     NF_MD_LANGUAGE_R),

    // Elm
    (".elm",     NF_CUSTOM_ELM),

    // Data formats
    (".xml",     NF_MD_XML),
    (".xsd",     NF_MD_XML),
    (".xsl",     NF_MD_XML),
    (".xslt",    NF_MD_XML),
    (".dtd",     NF_MD_XML),
    (".plist",   NF_MD_XML),
    (".manifest", NF_MD_XML),
    (".json",    NF_SETI_JSON),
    (".toml",    NF_FA_GEAR),
    (".yml",     NF_MD_FORMAT_ALIGN_LEFT),
    (".yaml",    NF_MD_FORMAT_ALIGN_LEFT),

    // Config / Settings
    (".ini",     NF_FA_GEAR),
    (".cfg",     NF_FA_GEAR),
    (".conf",    NF_FA_GEAR),
    (".config",  NF_FA_GEAR),
    (".properties", NF_FA_GEAR),
    (".settings", NF_FA_GEAR),
    (".reg",     NF_FA_GEAR),

    // Database / SQL
    (".sql",     NF_DEV_DATABASE),
    (".sqlite",  NF_DEV_DATABASE),
    (".mdb",     NF_DEV_DATABASE),
    (".accdb",   NF_DEV_DATABASE),
    (".pgsql",   NF_DEV_DATABASE),
    (".db",      NF_SETI_DB),
    (".csv",     NF_MD_FILE_EXCEL),
    (".tsv",     NF_MD_FILE_EXCEL),

    // Build artifacts
    (".obj",     NF_OCT_FILE_BINARY),
    (".lib",     NF_OCT_FILE_BINARY),
    (".res",     NF_OCT_FILE_BINARY),
    (".pch",     NF_OCT_FILE_BINARY),
    (".pdb",     NF_DEV_DATABASE),

    // Logs
    (".wrn",     NF_FA_LIST),
    (".err",     NF_FA_LIST),
    (".log",     NF_FA_LIST),

    // Shell
    (".bash",    NF_OCT_TERMINAL),
    (".sh",      NF_OCT_TERMINAL),
    (".zsh",     NF_OCT_TERMINAL),
    (".fish",    NF_OCT_TERMINAL),
    (".bat",     NF_CUSTOM_MSDOS),
    (".cmd",     NF_CUSTOM_MSDOS),

    // PowerShell
    (".ps1",     NF_MD_CONSOLE_LINE),
    (".psd1",    NF_MD_CONSOLE_LINE),
    (".psm1",    NF_MD_CONSOLE_LINE),
    (".ps1xml",  NF_MD_CONSOLE_LINE),

    // Executables
    (".exe",     NF_MD_APPLICATION),
    (".sys",     NF_MD_APPLICATION),
    (".dll",     NF_FA_ARCHIVE),

    // Installers
    (".msi",     NF_MD_PACKAGE_VARIANT),
    (".msix",    NF_MD_PACKAGE_VARIANT),
    (".deb",     NF_MD_PACKAGE_VARIANT),
    (".rpm",     NF_MD_PACKAGE_VARIANT),

    // Visual Studio
    (".sln",     NF_DEV_VISUAL_STUDIO),
    (".vcproj",  NF_DEV_VISUAL_STUDIO),
    (".vcxproj", NF_DEV_VISUAL_STUDIO),
    (".csproj",  NF_DEV_VISUAL_STUDIO),
    (".csxproj", NF_DEV_VISUAL_STUDIO),
    (".fsproj",  NF_DEV_FSHARP),
    (".user",    NF_DEV_VISUAL_STUDIO),
    (".ncb",     NF_DEV_VISUAL_STUDIO),
    (".suo",     NF_DEV_VISUAL_STUDIO),
    (".code-workspace", NF_DEV_VISUAL_STUDIO),

    // Documents
    (".doc",     NF_MD_FILE_WORD),
    (".docx",    NF_MD_FILE_WORD),
    (".rtf",     NF_MD_FILE_WORD),
    (".ppt",     NF_MD_FILE_POWERPOINT),
    (".pptx",    NF_MD_FILE_POWERPOINT),
    (".xls",     NF_MD_FILE_EXCEL),
    (".xlsx",    NF_MD_FILE_EXCEL),
    (".pdf",     NF_FA_FILE_PDF_O),

    // Markdown
    (".md",      NF_DEV_MARKDOWN),
    (".markdown", NF_DEV_MARKDOWN),
    (".rst",     NF_DEV_MARKDOWN),

    // Text
    (".txt",     NF_MD_FILE_DOCUMENT),
    (".text",    NF_MD_FILE_DOCUMENT),
    (".!!!",     NF_MD_FILE_DOCUMENT),
    (".1st",     NF_MD_FILE_DOCUMENT),
    (".me",      NF_MD_FILE_DOCUMENT),
    (".now",     NF_MD_FILE_DOCUMENT),

    // Email
    (".eml",     NF_FA_ENVELOPE),

    // Images
    (".png",     NF_FA_FILE_IMAGE_O),
    (".jpg",     NF_FA_FILE_IMAGE_O),
    (".jpeg",    NF_FA_FILE_IMAGE_O),
    (".gif",     NF_FA_FILE_IMAGE_O),
    (".bmp",     NF_FA_FILE_IMAGE_O),
    (".ico",     NF_FA_FILE_IMAGE_O),
    (".tif",     NF_FA_FILE_IMAGE_O),
    (".tiff",    NF_FA_FILE_IMAGE_O),
    (".webp",    NF_FA_FILE_IMAGE_O),
    (".psd",     NF_FA_FILE_IMAGE_O),
    (".cur",     NF_FA_FILE_IMAGE_O),
    (".raw",     NF_FA_FILE_IMAGE_O),
    (".svg",     NF_MD_SVG),

    // Audio
    (".mp3",     NF_FA_FILE_AUDIO_O),
    (".wav",     NF_FA_FILE_AUDIO_O),
    (".flac",    NF_FA_FILE_AUDIO_O),
    (".m4a",     NF_FA_FILE_AUDIO_O),
    (".wma",     NF_FA_FILE_AUDIO_O),
    (".aac",     NF_FA_FILE_AUDIO_O),
    (".ogg",     NF_FA_FILE_AUDIO_O),
    (".opus",    NF_FA_FILE_AUDIO_O),
    (".aiff",    NF_FA_FILE_AUDIO_O),

    // Video
    (".mp4",     NF_FA_FILE_VIDEO_O),
    (".avi",     NF_FA_FILE_VIDEO_O),
    (".mkv",     NF_FA_FILE_VIDEO_O),
    (".mov",     NF_FA_FILE_VIDEO_O),
    (".wmv",     NF_FA_FILE_VIDEO_O),
    (".webm",    NF_FA_FILE_VIDEO_O),
    (".flv",     NF_FA_FILE_VIDEO_O),
    (".mpg",     NF_FA_FILE_VIDEO_O),
    (".mpeg",    NF_FA_FILE_VIDEO_O),

    // Fonts
    (".ttf",     NF_FA_FONT),
    (".otf",     NF_FA_FONT),
    (".woff",    NF_FA_FONT),
    (".woff2",   NF_FA_FONT),
    (".eot",     NF_FA_FONT),
    (".ttc",     NF_FA_FONT),

    // Archives
    (".7z",      NF_OCT_FILE_ZIP),
    (".arj",     NF_OCT_FILE_ZIP),
    (".gz",      NF_OCT_FILE_ZIP),
    (".rar",     NF_OCT_FILE_ZIP),
    (".tar",     NF_OCT_FILE_ZIP),
    (".zip",     NF_OCT_FILE_ZIP),
    (".xz",      NF_OCT_FILE_ZIP),
    (".bz2",     NF_OCT_FILE_ZIP),
    (".tgz",     NF_OCT_FILE_ZIP),
    (".cab",     NF_OCT_FILE_ZIP),
    (".zst",     NF_OCT_FILE_ZIP),

    // Certificates / Keys
    (".cer",     NF_FA_CERTIFICATE),
    (".cert",    NF_FA_CERTIFICATE),
    (".crt",     NF_FA_CERTIFICATE),
    (".pfx",     NF_FA_CERTIFICATE),
    (".pem",     NF_FA_KEY),
    (".pub",     NF_FA_KEY),
    (".key",     NF_FA_KEY),
    (".asc",     NF_FA_KEY),
    (".gpg",     NF_FA_KEY),

    // Docker
    (".dockerfile", NF_DEV_DOCKER),
    (".dockerignore", NF_DEV_DOCKER),

    // Terraform / IaC
    (".tf",      NF_SETI_TERRAFORM),
    (".tfvars",  NF_SETI_TERRAFORM),
    (".bicep",   NF_SETI_BICEP),

    // Lock files
    (".lock",    NF_FA_LOCK),

    // Resource
    (".rc",      NF_SETI_CONFIG),
];


/// Default well-known directory name → icon mappings.
/// Keys are lowercase (matching Config lookup convention).
pub const DEFAULT_WELL_KNOWN_DIR_ICONS: &[(&str, char)] = &[
    // Version control / IDEs (DEVIATIONS noted)
    (".git",             NF_SETI_GIT),             // DEVIATION: nf-seti-git
    (".github",          NF_SETI_GITHUB),           // DEVIATION: nf-seti-github
    (".vscode",          NF_DEV_VSCODE),             // DEVIATION: nf-dev-vscode
    (".vscode-insiders", NF_DEV_VSCODE),
    ("node_modules",     NF_SETI_NPM),               // DEVIATION: nf-seti-npm

    // Config / Cloud provider
    (".config",          NF_SETI_CONFIG),
    (".cargo",           NF_CUSTOM_FOLDER_CONFIG),
    (".cache",           NF_MD_CACHED),
    (".docker",          NF_DEV_DOCKER),
    (".aws",             NF_DEV_AWS),
    (".azure",           NF_MD_MICROSOFT_AZURE),
    (".kube",            NF_MD_SHIP_WHEEL),

    // Source / Development
    ("src",              NF_OCT_TERMINAL),
    ("source",           NF_OCT_TERMINAL),
    ("development",      NF_OCT_TERMINAL),
    ("projects",         NF_SETI_PROJECT),

    // Documentation
    ("docs",             NF_OCT_REPO),
    ("doc",              NF_OCT_REPO),
    ("documents",        NF_OCT_REPO),

    // Build outputs
    ("bin",              NF_OCT_FILE_BINARY),
    ("build",            NF_COD_OUTPUT),
    ("dist",             NF_COD_OUTPUT),
    ("out",              NF_COD_OUTPUT),
    ("output",           NF_COD_OUTPUT),
    ("artifacts",        NF_COD_PACKAGE),

    // Testing
    ("test",             NF_MD_TEST_TUBE),
    ("tests",            NF_MD_TEST_TUBE),
    ("__tests__",        NF_MD_TEST_TUBE),
    ("spec",             NF_MD_TEST_TUBE),
    ("specs",            NF_MD_TEST_TUBE),
    ("benchmark",        NF_MD_TIMER),

    // Libraries / Packages
    ("lib",              NF_COD_FOLDER_LIBRARY),
    ("libs",             NF_COD_FOLDER_LIBRARY),
    ("packages",         NF_SETI_NPM),

    // Scripts
    ("scripts",          NF_SETI_SHELL),

    // Media / Images
    ("images",           NF_MD_FOLDER_IMAGE),
    ("img",              NF_MD_FOLDER_IMAGE),
    ("photos",           NF_MD_FOLDER_IMAGE),
    ("pictures",         NF_MD_FOLDER_IMAGE),
    ("assets",           NF_MD_FOLDER_IMAGE),
    ("videos",           NF_MD_MOVIE),
    ("movies",           NF_MD_MOVIE),
    ("media",            NF_OCT_FILE_MEDIA),
    ("music",            NF_MD_MUSIC_BOX_MULTIPLE),
    ("songs",            NF_MD_MUSIC_BOX_MULTIPLE),
    ("fonts",            NF_FA_FONT),

    // User directories
    ("downloads",        NF_MD_FOLDER_DOWNLOAD),
    ("desktop",          NF_MD_DESKTOP_CLASSIC),
    ("favorites",        NF_MD_FOLDER_STAR),
    ("contacts",         NF_MD_CONTACTS),
    ("onedrive",         NF_MD_MICROSOFT_ONEDRIVE),
    ("users",            NF_FA_USERS),
    ("windows",          NF_FA_WINDOWS),

    // Other
    ("apps",             NF_MD_APPS),
    ("applications",     NF_MD_APPS),
    ("demo",             NF_COD_PREVIEW),
    ("samples",          NF_COD_PREVIEW),
    ("shortcuts",        NF_COD_FILE_SYMLINK_DIR),
    ("links",            NF_COD_FILE_SYMLINK_DIR),
    ("github",           NF_FA_GITHUB_ALT),
];
```

### `file_attribute_map.rs` — Attribute Precedence

```rust
/// Attribute precedence order for icon/color resolution (PSHERC0TA).
///
/// This order determines which file attribute "wins" when multiple attributes
/// are present on a file. Used by Config::get_display_style_for_file().
///
/// NOTE: This is different from FILE_ATTRIBUTE_MAP in file_info.rs (RHSATECP0),
/// which controls the display column order. Both arrays contain the same 9
/// attributes but in different sequences.
pub const ATTRIBUTE_PRECEDENCE: &[(u32, char)] = &[
    (FILE_ATTRIBUTE_REPARSE_POINT, 'P'),   // Priority 1 (highest) — identity-altering
    (FILE_ATTRIBUTE_SYSTEM,        'S'),   // Priority 2 — OS-critical
    (FILE_ATTRIBUTE_HIDDEN,        'H'),   // Priority 3 — intentionally invisible
    (FILE_ATTRIBUTE_ENCRYPTED,     'E'),   // Priority 4 — access-restricting
    (FILE_ATTRIBUTE_READONLY,      'R'),   // Priority 5 — access-restricting
    (FILE_ATTRIBUTE_COMPRESSED,    'C'),   // Priority 6 — informational
    (FILE_ATTRIBUTE_SPARSE_FILE,   '0'),   // Priority 7 — rare
    (FILE_ATTRIBUTE_TEMPORARY,     'T'),   // Priority 8 — ephemeral
    (FILE_ATTRIBUTE_ARCHIVE,       'A'),   // Priority 9 (lowest) — near-universal noise
];
```

**Purpose**: Separate module for attribute precedence, distinct from `file_info::FILE_ATTRIBUTE_MAP` (RHSATECP0 order for display columns). Both arrays contain the same 9 attributes but in different sequences. The precedence array is used exclusively by `Config::get_display_style_for_file()`.

---

## New Types — `nerd_font_detector.rs`

### Detection Enums

```rust
/// How icon display was requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconActivation {
    /// Determined by auto-detection
    Auto,
    /// /Icons CLI flag or RCDIR=Icons
    ForceOn,
    /// /Icons- CLI flag or RCDIR=Icons-
    ForceOff,
}


/// Result of the Nerd Font detection probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionResult {
    /// Nerd Font confirmed (canary glyph found or WezTerm env)
    Detected,
    /// No Nerd Font found
    NotDetected,
    /// Detection failed or inconclusive — default to OFF
    Inconclusive,
}
```

### FontProber Trait

```rust
/// Abstraction over GDI-dependent Nerd Font probing operations.
///
/// RCDir production code uses DefaultFontProber; tests use MockFontProber.
/// Replaces TCDir's protected virtual methods on CNerdFontDetector.
pub trait FontProber {
    /// Probe the current console font for a specific canary glyph.
    /// Returns Ok(true) if the glyph is present, Ok(false) if missing.
    fn probe_console_font_for_glyph(
        &self,
        console_handle: HANDLE,
        canary: char,
    ) -> Result<bool, AppError>;

    /// Check whether any Nerd Font is installed system-wide via font enumeration.
    fn is_nerd_font_installed(&self) -> Result<bool, AppError>;
}
```

### Detection Function

```rust
/// Run the layered Nerd Font detection chain.
///
/// Detection order (identical to TCDir):
///   1. WezTerm environment → Detected
///   2. ConPTY terminal detected → skip GDI canary (unreliable), fall to font enum
///   3. Classic conhost — GDI canary probe U+E5FF
///   4. System font enumeration for NF-pattern font names
///   5. Fallback → Inconclusive (treated as OFF)
pub fn detect(
    console_handle: HANDLE,
    env_provider: &dyn EnvironmentProvider,
    prober: &dyn FontProber,
) -> DetectionResult {
    // ...
}
```

---

## Extended Types — `Config`

### New Fields

```rust
pub struct Config {
    // --- Existing fields (unchanged) ---
    pub attributes:                [u16; Attribute::COUNT],
    pub attribute_sources:         [AttributeSource; Attribute::COUNT],
    pub extension_colors:          HashMap<String, u16>,
    pub extension_sources:         HashMap<String, AttributeSource>,
    pub file_attr_colors:          HashMap<u32, FileAttrStyle>,
    pub wide_listing:              Option<bool>,
    pub bare_listing:              Option<bool>,
    pub recurse:                   Option<bool>,
    pub perf_timer:                Option<bool>,
    pub multi_threaded:            Option<bool>,
    pub show_owner:                Option<bool>,
    pub show_streams:              Option<bool>,
    pub last_parse_result:         ValidationResult,

    // --- New icon fields ---
    pub extension_icons:           HashMap<String, char>,
    pub extension_icon_sources:    HashMap<String, AttributeSource>,
    pub well_known_dir_icons:      HashMap<String, char>,
    pub well_known_dir_icon_sources: HashMap<String, AttributeSource>,
    pub file_attr_icons:           HashMap<u32, char>,
    pub icons:                     Option<bool>,

    // Type fallback icons
    pub icon_directory_default:    char,           // '\u{E5FF}' NF_CUSTOM_FOLDER
    pub icon_file_default:         char,           // '\u{F15B}' NF_FA_FILE
    pub icon_symlink:              char,           // '\u{EAED}' NF_COD_FILE_SYMLINK_DIR
    pub icon_junction:             char,           // '\u{F08E}' NF_FA_EXTERNAL_LINK

    // Cloud status NF glyphs (used when icons are active)
    pub icon_cloud_only:           char,           // '\u{F0163}' NF_MD_CLOUD_OUTLINE
    pub icon_locally_available:    char,           // '\u{F0160}' NF_MD_CLOUD_CHECK
    pub icon_always_local:         char,           // '\u{F0403}' NF_MD_PIN
}
```

### FileDisplayStyle (Return Type)

```rust
/// Resolved display style for a single file entry.
/// Returned by Config::get_display_style_for_file().
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileDisplayStyle {
    /// Resolved color attribute (Windows console WORD)
    pub text_attr:        u16,
    /// Resolved icon glyph (None = no icon configured at any level)
    pub icon_code_point:  Option<char>,
    /// true if icon was explicitly suppressed (user typed ",")
    pub icon_suppressed:  bool,
}
```

**Purpose**: Replaces TCDir's `SFileDisplayStyle`. Both color and icon resolved in a single precedence walk. `icon_suppressed` distinguishes "no icon configured" from "icon explicitly removed by user override" — both display no icon, but the suppressed flag prevents fall-through to lower precedence levels.

### New Methods

```rust
impl Config {
    /// Resolve both color and icon for a file in a single precedence walk.
    ///
    /// Precedence (highest → lowest):
    ///   1. File attribute (PSHERC0TA order from ATTRIBUTE_PRECEDENCE)
    ///   2. Well-known directory name (icon only)
    ///   3. File extension
    ///   4. Type fallback (directory/file/symlink/junction)
    ///
    /// Color locks at the first matching level.
    /// Icon evaluation continues to lower levels if the winning color level
    /// has no icon configured (FR-020).
    pub fn get_display_style_for_file(
        &self,
        file_attributes: u32,
        file_name: &OsStr,
    ) -> FileDisplayStyle { /* ... */ }

    /// Get the NF glyph for a cloud status (when icons are active).
    pub fn get_cloud_status_icon(&self, status: CloudStatus) -> char { /* ... */ }

    // Internal initialization
    fn initialize_extension_icons(&mut self) { /* ... */ }
    fn initialize_well_known_dir_icons(&mut self) { /* ... */ }

    // Internal override processing (called during RCDIR env var parsing)
    fn process_extension_icon_override(
        &mut self,
        extension: &str,
        icon: char,
        suppressed: bool,
    ) { /* ... */ }

    fn process_well_known_dir_icon_override(
        &mut self,
        dir_name: &str,
        icon: char,
        suppressed: bool,
    ) { /* ... */ }

    fn process_file_attribute_icon_override(
        &mut self,
        attribute_flag: u32,
        icon: char,
    ) { /* ... */ }

    /// Parse an icon value from the RCDIR env var comma syntax.
    /// - Empty string → icon suppressed
    /// - Single BMP char → literal glyph
    /// - "U+XXXX" (4–6 hex digits) → code point
    fn parse_icon_value(icon_str: &str) -> Result<(Option<char>, bool), String> { /* ... */ }
}
```

### Modified Methods (Existing)

| Method | Change |
|--------|--------|
| `initialize_with_provider()` | Call `initialize_extension_icons()` and `initialize_well_known_dir_icons()` |
| `process_color_override_entry()` | Split value on first comma; left = color, right = icon |
| `get_text_attr_for_file()` | Delegates to `get_display_style_for_file().text_attr` for backward compat |
| `is_switch_name()` (internal) | Recognize "Icons" and "Icons-" |
| `process_switch_override()` (internal) | Handle Icons/Icons- → `self.icons = Some(bool)` |

---

## Extended Types — `CommandLine`

### New Field

```rust
pub struct CommandLine {
    // --- Existing fields (unchanged) ---
    pub recurse:          bool,
    // ... all existing fields ...
    pub debug:            bool,

    // --- New ---
    pub icons:            Option<bool>,  // None = not specified, Some(true) = /Icons, Some(false) = /Icons-
}
```

### Modified Methods

| Method | Change |
|--------|--------|
| `handle_long_switch()` | Match "Icons" → `Some(true)`, "Icons-" → `Some(false)` |
| `default()` | Initialize `icons: None` |

---

## Relationships

```
CommandLine  ──parses──>  icons: Option<bool>  (/Icons, /Icons-)

Config       ──reads───>  RCDIR env var ──extends──> icon maps + icons switch
             ──owns───>   extension_icons, well_known_dir_icons, file_attr_icons
             ──exposes──> get_display_style_for_file() → FileDisplayStyle

NerdFontDetector
  detect()   ──uses──>  &dyn EnvironmentProvider (env var detection)
             ──uses──>  &dyn FontProber (GDI operations)
             ──returns──> DetectionResult

Displayers   ──receive──> icons_active: bool (plain boolean)
             ──call───>   config.get_display_style_for_file()
             ──emit───>   icon glyph + space via console buffer
```

---

## State Flow

```
1. CommandLine::parse_from()
   └── Sets icons = Some(true/false) if /Icons or /Icons- present

2. Config::initialize_with_provider()
   ├── initialize_extension_colors()         (existing — colors)
   ├── initialize_extension_icons()          (new — default icons)
   ├── initialize_well_known_dir_icons()     (new — default dir icons)
   ├── initialize_file_attr_colors()         (existing — colors)
   └── apply_user_color_overrides()          (extended — parses color AND icon)

3. main.rs / lib.rs run flow
   ├── CLI check: if cmd.icons.is_some() → use it (ForceOn / ForceOff)
   ├── Env var check: if config.icons.is_some() → use it
   └── Auto-detect: nerd_font_detector::detect() → DetectionResult
   └── Set icons_active: bool = resolved value

4. Displayers (per-file)
   ├── style = config.get_display_style_for_file(attrs, name)
   ├── if icons_active && style.icon_code_point.is_some() && !style.icon_suppressed:
   │   ├── console.push(style.icon_code_point.unwrap())
   │   └── console.push(' ')
   ├── elif icons_active && style.icon_suppressed:
   │   └── console.push_str("  ")   // 2 spaces to maintain column alignment (FR-007)
   └── console.printf_attr(style.text_attr, filename)
```

---

## Validation Rules

| Entity | Rule | Error Handling |
|--------|------|---------------|
| Icon code point (U+XXXX) | 4–6 hex digits, range 0x0001–0x10FFFF, not D800–DFFF | `ErrorInfo` with underline on invalid hex |
| Icon literal glyph | Single BMP `char` | `ErrorInfo` if multi-char or invalid |
| Icon comma syntax | At most one comma per entry | `ErrorInfo` if multiple commas |
| Duplicate key | First-write-wins for both color and icon | `ErrorInfo` on duplicate, value preserved from first |
| Icons / Icons- switch | Mutually exclusive (first wins in env var) | `ErrorInfo` on conflicting switch |

---

## Key Design Differences from TCDir (C++)

| Aspect | TCDir (C++) | RCDir (Rust) |
|--------|-------------|--------------|
| Unicode storage | `char32_t` | `char` (Unicode scalar value, functionally identical) |
| UTF-16 encoding | `WideCharPair` + `CodePointToWideChars()` | `char::encode_utf16(&mut [u16; 2])` — no custom type needed |
| Icon map key type | `wstring` | `String` (lowercase, with leading dot) |
| File attribute map | `DWORD` → `char32_t` | `u32` → `char` |
| Testability | Protected virtual methods + derivation | `FontProber` trait + `&dyn FontProber` injection |
| Error handling | `HRESULT` + EHM macros | `Result<T, AppError>` + `?` operator |
| Display style | `SFileDisplayStyle { m_wTextAttr, m_iconCodePoint, m_fIconSuppressed }` | `FileDisplayStyle { text_attr, icon_code_point: Option<char>, icon_suppressed }` |
| Suppressed icon | `m_iconCodePoint == 0 && m_fIconSuppressed == true` | `icon_code_point == None && icon_suppressed == true` |

---

## No External Contracts

This feature has no REST APIs, IPC interfaces, or external service integrations. All contracts are internal Rust module interfaces:

- `EnvironmentProvider` trait — existing injection interface for env var access
- `FontProber` trait — new injection interface for GDI operations
- `ResultsDisplayer` trait — existing interface for display output

No new external crates are required. Only the `windows` crate gains one additional feature flag (`Win32_Graphics_Gdi`).
