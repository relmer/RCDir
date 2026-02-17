# Quickstart: Implementing Nerd Font File & Folder Icons (Rust)

**Feature**: 003-file-icons | **Date**: 2026-02-16

This guide walks through the implementation in dependency order. Each section can be implemented and tested independently.

---

## Prerequisites

- Branch: `003-file-icons`
- All existing tests pass (`cargo test`)
- Rust stable toolchain 1.85+ (edition 2024)
- VS Code with existing tasks, PowerShell terminal
- **Coding conventions**: [.github/copilot-instructions.md](../../.github/copilot-instructions.md) — authoritative reference for formatting, spacing, function headers, and all coding standards

---

## Implementation Order

```
Phase 1: Foundation (no visible output yet)
  1. icon_mapping.rs              — NF constants, default tables, lookup helpers
  2. file_attribute_map.rs        — PSHERC0TA precedence array
  3. nerd_font_detector.rs        — FontProber trait + detection chain
  4. config.rs (extend)           — Icon maps, comma-syntax parsing, get_display_style_for_file()
  5. command_line.rs (extend)     — /Icons, /Icons- switches
  6. Cargo.toml                   — Add Win32_Graphics_Gdi feature

Phase 2: Display Integration
  7. results_displayer.rs         — Icon glyph emission in Normal/Wide/Bare
  8. cloud_status.rs              — NF glyph alternatives for cloud symbols
  9. console.rs                   — Minor: no changes needed (char pushes to String buffer)
  10. lib.rs / main.rs            — Detection chain orchestration, wire icons_active

Phase 3: Diagnostics & Polish
  11. usage.rs                    — /? help, /env docs, /config display for icons
  12. lib.rs                      — Add pub mod declarations for 3 new modules

Phase 4: Tests
  13. icon_mapping tests          — Table coverage, constant validity, lookup
  14. nerd_font_detector tests    — Detection chain with mock prober + env provider
  15. config tests (extend)       — RCDIR icon comma syntax, duplicates, precedence
  16. command_line tests (extend) — /Icons, /Icons- parsing
  17. results_displayer tests     — Icon display in all modes
  18. tests/output_parity.rs      — End-to-end icon-mode parity scenarios
```

---

## Phase 1: Foundation

### Step 1. `src/icon_mapping.rs` — New File

**Purpose**: All Nerd Font constants and static default tables in one module.

#### 1a. Named Constants

Define all `NF_*` constants as `pub const char` values. See [data-model.md](data-model.md) for the complete list. Key guidelines:

- Group by NF prefix: Custom, Seti, Dev, Fae, Oct, Fa, Md, Cod
- Use `'\u{XXXX}'` syntax for all code points
- Comment DEVIATION entries
- Alphabetical within each group

```rust
// Example (see data-model.md for full list):
pub const NF_CUSTOM_FOLDER: char = '\u{E5FF}';
pub const NF_DEV_RUST:      char = '\u{E7A8}';
pub const NF_MD_PIN:         char = '\u{F0403}';  // supplementary plane
```

#### 1b. Default Extension Table

```rust
pub const DEFAULT_EXTENSION_ICONS: &[(&str, char)] = &[
    (".c",   NF_MD_LANGUAGE_C),
    (".cpp", NF_MD_LANGUAGE_CPP),
    // ... ~180 entries — see data-model.md for complete list
];
```

#### 1c. Default Well-Known Directory Table

```rust
pub const DEFAULT_WELL_KNOWN_DIR_ICONS: &[(&str, char)] = &[
    (".git",         NF_SETI_GIT),
    ("node_modules", NF_SETI_NPM),
    // ... ~65 entries — see data-model.md for complete list
];
```

#### 1d. Tests

```rust
#[cfg(test)]
mod tests {
    // - All NF_* constants are valid Unicode scalar values (implied by char type)
    // - DEFAULT_EXTENSION_ICONS has no duplicate keys
    // - DEFAULT_WELL_KNOWN_DIR_ICONS has no duplicate keys
    // - All keys are lowercase
    // - All extension keys start with '.'
    // - Table lengths match expected counts
}
```

**Checkpoint**: `cargo test` passes. New module compiles.

---

### Step 2. `src/file_attribute_map.rs` — New File

**Purpose**: Attribute precedence array for icon/color resolution.

```rust
use windows::Win32::Storage::FileSystem::*;

pub const ATTRIBUTE_PRECEDENCE: &[(u32, char)] = &[
    (FILE_ATTRIBUTE_REPARSE_POINT, 'P'),
    (FILE_ATTRIBUTE_SYSTEM,        'S'),
    (FILE_ATTRIBUTE_HIDDEN,        'H'),
    (FILE_ATTRIBUTE_ENCRYPTED,     'E'),
    (FILE_ATTRIBUTE_READONLY,      'R'),
    (FILE_ATTRIBUTE_COMPRESSED,    'C'),
    (FILE_ATTRIBUTE_SPARSE_FILE,   '0'),
    (FILE_ATTRIBUTE_TEMPORARY,     'T'),
    (FILE_ATTRIBUTE_ARCHIVE,       'A'),
];
```

#### Tests

```rust
#[cfg(test)]
mod tests {
    // - ATTRIBUTE_PRECEDENCE has exactly 9 entries
    // - Same flags as FILE_ATTRIBUTE_MAP (different order)
    // - No duplicate flags
    // - No duplicate chars
}
```

**Checkpoint**: `cargo test` passes.

---

### Step 3. `src/nerd_font_detector.rs` — New File

**Purpose**: Layered Nerd Font detection with injectable GDI operations.

#### 3a. Types

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconActivation { Auto, ForceOn, ForceOff }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionResult { Detected, NotDetected, Inconclusive }

pub trait FontProber {
    fn probe_console_font_for_glyph(&self, console_handle: HANDLE, canary: char) -> Result<bool, AppError>;
    fn is_nerd_font_installed(&self) -> Result<bool, AppError>;
}
```

#### 3b. DefaultFontProber

```rust
pub struct DefaultFontProber;

impl FontProber for DefaultFontProber {
    fn probe_console_font_for_glyph(&self, console_handle: HANDLE, canary: char) -> Result<bool, AppError> {
        // 1. GetCurrentConsoleFontEx → get font face name
        // 2. CreateCompatibleDC(None)
        // 3. CreateFontW with face name
        // 4. SelectObject
        // 5. GetGlyphIndicesW with GGI_MARK_NONEXISTING_GLYPHS
        // 6. Cleanup: SelectObject(old), DeleteObject(font), DeleteDC(dc)
        // 7. Return Ok(glyph_index != 0xFFFF)
    }

    fn is_nerd_font_installed(&self) -> Result<bool, AppError> {
        // 1. CreateCompatibleDC(None)
        // 2. EnumFontFamiliesExW with callback
        // 3. Callback checks for "Nerd Font" / "NerdFont" / "NF " in face name
        // 4. Cleanup: DeleteDC
        // 5. Return Ok(found)
    }
}
```

#### 3c. Detection Function

```rust
pub fn detect(
    console_handle: HANDLE,
    env_provider: &dyn EnvironmentProvider,
    prober: &dyn FontProber,
) -> DetectionResult {
    // Step 1: WezTerm check
    if is_wezterm(env_provider) { return DetectionResult::Detected; }

    // Step 2: ConPTY check
    let is_conpty = is_conpty_terminal(env_provider);

    // Step 3: Classic conhost GDI canary (skip if ConPTY)
    if !is_conpty {
        match prober.probe_console_font_for_glyph(console_handle, NF_CUSTOM_FOLDER) {
            Ok(true)  => return DetectionResult::Detected,
            Ok(false) => return DetectionResult::NotDetected,
            Err(_)    => {}  // fall through to font enum
        }
    }

    // Step 4: System font enumeration
    match prober.is_nerd_font_installed() {
        Ok(true)  => DetectionResult::Detected,
        Ok(false) => DetectionResult::NotDetected,
        Err(_)    => DetectionResult::Inconclusive,
    }
}

fn is_wezterm(env: &dyn EnvironmentProvider) -> bool {
    env.get_env_var("TERM_PROGRAM").as_deref() == Some("WezTerm")
}

fn is_conpty_terminal(env: &dyn EnvironmentProvider) -> bool {
    env.get_env_var("WT_SESSION").is_some()
    || env.get_env_var("TERM_PROGRAM").is_some()
    || env.get_env_var("ConEmuPID").is_some()
    || env.get_env_var("ALACRITTY_WINDOW_ID").is_some()
}
```

#### 3d. Tests

```rust
#[cfg(test)]
mod tests {
    struct MockFontProber { canary_result: Result<bool, AppError>, nf_installed: Result<bool, AppError> }
    impl FontProber for MockFontProber { /* return configured values */ }

    // - WezTerm detected → Detected (prober never called)
    // - ConPTY + NF installed → Detected (canary skipped)
    // - ConPTY + NF not installed → NotDetected
    // - Classic conhost + canary hit → Detected
    // - Classic conhost + canary miss → NotDetected
    // - All probes fail → Inconclusive
    // - is_wezterm with/without env var
    // - is_conpty_terminal with various env combos
}
```

**Checkpoint**: `cargo test` passes. Detection logic fully unit-tested.

---

### Step 4. `src/config.rs` — Extend

#### 4a. Add New Fields to `Config` Struct

Add icon-related fields after existing fields. See data-model.md for complete list.

```rust
// In Config struct:
pub extension_icons:           HashMap<String, char>,
pub extension_icon_sources:    HashMap<String, AttributeSource>,
pub well_known_dir_icons:      HashMap<String, char>,
pub well_known_dir_icon_sources: HashMap<String, AttributeSource>,
pub file_attr_icons:           HashMap<u32, char>,
pub icons:                     Option<bool>,
pub icon_directory_default:    char,
pub icon_file_default:         char,
pub icon_symlink:              char,
pub icon_junction:             char,
pub icon_cloud_only:           char,
pub icon_locally_available:    char,
pub icon_always_local:         char,
```

#### 4b. Initialize in `new()` and `initialize_with_provider()`

```rust
// In new():
extension_icons:           HashMap::new(),
extension_icon_sources:    HashMap::new(),
well_known_dir_icons:      HashMap::new(),
well_known_dir_icon_sources: HashMap::new(),
file_attr_icons:           HashMap::new(),
icons:                     None,
icon_directory_default:    NF_CUSTOM_FOLDER,
icon_file_default:         NF_FA_FILE,
icon_symlink:              NF_COD_FILE_SYMLINK_DIR,
icon_junction:             NF_FA_EXTERNAL_LINK,
icon_cloud_only:           NF_MD_CLOUD_OUTLINE,
icon_locally_available:    NF_MD_CLOUD_CHECK,
icon_always_local:         NF_MD_PIN,

// In initialize_with_provider():
self.initialize_extension_icons();
self.initialize_well_known_dir_icons();
```

#### 4c. Add `FileDisplayStyle` and `get_display_style_for_file()`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileDisplayStyle {
    pub text_attr:       u16,
    pub icon_code_point: Option<char>,
    pub icon_suppressed: bool,
}

pub fn get_display_style_for_file(&self, file_attributes: u32, file_name: &OsStr) -> FileDisplayStyle {
    // Walk ATTRIBUTE_PRECEDENCE for color + icon
    // Then check well-known dirs (icon only if not yet resolved)
    // Then check extension (color if not yet resolved, icon if not yet resolved)
    // Then type fallback
    // Return FileDisplayStyle
}
```

#### 4d. Extend `process_color_override_entry()` for Comma Syntax

Split value on first comma. Left = color part (existing logic). Right = icon part (new `parse_icon_value()`).

#### 4e. Add `parse_icon_value()`

```rust
fn parse_icon_value(icon_str: &str) -> Result<(Option<char>, bool), String> {
    if icon_str.is_empty() {
        return Ok((None, true));  // suppressed
    }
    if let Some(hex) = icon_str.strip_prefix("U+").or_else(|| icon_str.strip_prefix("u+")) {
        // Parse hex, validate range, return char
    }
    // Single char literal
    let mut chars = icon_str.chars();
    let ch = chars.next().ok_or("empty icon value")?;
    if chars.next().is_some() {
        return Err("icon value must be a single character or U+XXXX".into());
    }
    Ok((Some(ch), false))
}
```

#### 4f. Handle `Icons`/`Icons-` in Switch Processing

Extend `is_switch_name()` and `process_switch_override()` to recognize `Icons`/`Icons-`.

#### 4g. Tests

```rust
// - Comma syntax: ".rs=Yellow,U+E7A8" → color=Yellow, icon='\u{E7A8}'
// - Color only: ".rs=Yellow" → color=Yellow, icon unchanged
// - Icon suppressed: ".rs=Yellow," → color=Yellow, icon_suppressed
// - Icon only: ".rs=,U+E7A8" → default color, icon='\u{E7A8}'
// - Invalid hex: ".rs=Yellow,U+ZZZZ" → error
// - Surrogate range: ".rs=Yellow,U+D800" → error
// - Multi-char icon: ".rs=Yellow,AB" → error
// - get_display_style_for_file() precedence tests
// - Icons/Icons- switch parsing
// - Duplicate Icons/Icons- → first wins + error
```

**Checkpoint**: `cargo test` passes. Config fully extended.

---

### Step 5. `src/command_line.rs` — Extend

#### 5a. Add Field

```rust
pub icons: Option<bool>,  // None, Some(true), Some(false)
```

#### 5b. Default

```rust
icons: None,
```

#### 5c. Handle in `handle_long_switch()`

```rust
"icons" => {
    if self.icons.is_none() {
        self.icons = Some(true);
    }
}
"icons-" => {
    if self.icons.is_none() {
        self.icons = Some(false);
    }
}
```

#### 5d. Tests

```rust
// - /Icons → icons = Some(true)
// - /Icons- → icons = Some(false)
// - No switch → icons = None
// - /Icons /Icons- → first wins (Some(true))
// - Case insensitive: /ICONS, /icons
```

**Checkpoint**: `cargo test` passes.

---

### Step 6. `Cargo.toml` — Add Feature

Add `"Win32_Graphics_Gdi"` to the windows crate features list.

**Checkpoint**: `cargo check` passes.

---

## Phase 2: Display Integration

### Step 7. `src/results_displayer.rs` — Extend

#### Normal Mode

In `display_file_results()` for normal mode, before printing the filename:

```rust
if icons_active {
    match style.icon_code_point {
        Some(glyph) if !style.icon_suppressed => {
            console.printf_attr (style.text_attr, &format!("{} ", glyph));
        }
        _ if style.icon_suppressed => {
            console.print ("  ");  // 2 spaces for alignment (FR-007)
        }
        _ => {
            console.print ("  ");  // no icon at any level → 2 spaces
        }
    }
}
```

#### Wide Mode

- Same icon emission before filename
- Suppress bracket column when icons active (FR-013)
- Use `config.get_cloud_status_icon()` for cloud status when icons active (FR-014)

#### Bare Mode

- Icon emission before path (same pattern)

### Step 8. `src/cloud_status.rs` — Extend

Add method or modify `symbol()` to return NF glyph when icons are active:

```rust
pub fn nf_symbol(self, config: &Config) -> char {
    match self {
        CloudStatus::None      => ' ',
        CloudStatus::CloudOnly => config.icon_cloud_only,
        CloudStatus::Local     => config.icon_locally_available,
        CloudStatus::Pinned    => config.icon_always_local,
    }
}
```

### Step 9. `src/console.rs` — No Changes Needed

NF glyphs are valid `char` values that push directly into the `String` buffer. The existing `WriteConsoleW` flush path handles them correctly via `encode_utf16()`.

### Step 10. `src/lib.rs` / `src/main.rs` — Wire Detection

```rust
// In the main flow, after Config and CommandLine are initialized:
let icons_active = resolve_icons(
    &cmd,
    &config,
    console_handle,
    &DefaultEnvironmentProvider,
    &DefaultFontProber,
);

fn resolve_icons(
    cmd: &CommandLine,
    config: &Config,
    handle: HANDLE,
    env: &dyn EnvironmentProvider,
    prober: &dyn FontProber,
) -> bool {
    // 1. CLI override
    if let Some(v) = cmd.icons { return v; }
    // 2. Env var override
    if let Some(v) = config.icons { return v; }
    // 3. Auto-detect
    matches!(nerd_font_detector::detect(handle, env, prober), DetectionResult::Detected)
}
```

Pass `icons_active` to displayers (add parameter to `display_file_results()` or store on displayer struct).

**Checkpoint**: `cargo build` succeeds. Icons visible when Nerd Font is present.

---

## Phase 3: Diagnostics & Polish

### Step 11. `src/usage.rs` — Extend

- Add `/Icons` and `/Icons-` to the switch help text in `show_help()`
- Add comma syntax documentation to `show_env_help()`
- Add icon table display to `show_config()`

### Step 12. `src/lib.rs` — Module Declarations

```rust
pub mod icon_mapping;
pub mod nerd_font_detector;
pub mod file_attribute_map;
```

**Checkpoint**: `cargo clippy` clean. `cargo test` passes.

---

## Phase 4: Tests

### Step 13–16. Unit Tests

Each module has inline `#[cfg(test)]` tests. Key coverage:

| Module | Tests |
|--------|-------|
| `icon_mapping` | Constants valid, no duplicate keys, keys lowercase, extension keys have dots |
| `file_attribute_map` | 9 entries, same flags as FILE_ATTRIBUTE_MAP, no duplicates |
| `nerd_font_detector` | All 5 detection steps with mock prober, WezTerm/ConPTY env combos |
| `config` | Comma syntax (all variants), precedence resolution, Icons switch, duplicates, errors |
| `command_line` | /Icons, /Icons-, case insensitivity, first-wins |

### Step 17. `tests/output_parity.rs` — Extend

Add integration test scenarios that verify:

- Icons off → output is byte-identical to pre-feature baseline
- Icons on → icon glyphs appear before filenames
- Suppressed icons → 2-space placeholder
- Wide mode → brackets suppressed, NF cloud symbols
- Bare mode → icon + space + path

---

## Verification Checklist

After all phases:

- [ ] `cargo build` succeeds (debug)
- [ ] `cargo build --release` succeeds
- [ ] `cargo test` — all tests pass
- [ ] `cargo clippy` — no warnings
- [ ] Icons appear with Nerd Font terminal
- [ ] Icons do not appear without Nerd Font
- [ ] `/Icons` forces icons on
- [ ] `/Icons-` forces icons off
- [ ] RCDIR comma syntax works for extensions, dirs, attributes
- [ ] Output is byte-identical when icons are off (SC-002)
- [ ] Performance within 5% on 1000+ file directory (SC-004)
