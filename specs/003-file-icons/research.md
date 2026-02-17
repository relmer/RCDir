# Research: Nerd Font File Icons for RCDir (Rust)

## Purpose

Resolve all technical unknowns for implementing the 003-file-icons feature in Rust. This covers Win32 GDI APIs via the `windows` crate, UTF-16 encoding, icon data structures, and the detection strategy.

---

## R1: Win32 GDI APIs via `windows` Crate

### Decision: Use `windows` crate 0.62 with `Win32_Graphics_Gdi` feature

**Rationale**: All required GDI APIs are available in the `windows` crate. Only one new Cargo.toml feature flag is needed — `Win32_Graphics_Gdi`. The existing `Win32_System_Console` feature already covers `GetCurrentConsoleFontEx` and `CONSOLE_FONT_INFOEX`.

**Alternatives considered**:
- Raw FFI bindings via `extern "system"` — rejected; `windows` crate already a dependency, provides type-safe wrappers
- `winapi` crate — rejected; `windows` crate is the newer Microsoft-maintained alternative and already used by RCDir

### API Mapping (TCDir C++ → RCDir Rust)

| C++ API | Rust equivalent (`windows::Win32::Graphics::Gdi`) | Notes |
|---------|----------------------------------------------------|-------|
| `GetCurrentConsoleFontEx` | `windows::Win32::System::Console::GetCurrentConsoleFontEx` | Already available (existing feature) |
| `CreateCompatibleDC(NULL)` | `CreateCompatibleDC(None)` | Takes `Option<HDC>` |
| `CreateFontW(...)` | `CreateFontW(...)` | `HFONT` implements `Into<HGDIOBJ>` |
| `SelectObject(hdc, hFont)` | `SelectObject(hdc, hfont)` | Returns previous `HGDIOBJ` |
| `GetGlyphIndicesW(hdc, &ch, 1, &idx, GGI_MARK_NONEXISTING_GLYPHS)` | `GetGlyphIndicesW(hdc, PCWSTR::from_raw(buf.as_ptr()), 1, &mut idx, GGI_MARK_NONEXISTING_GLYPHS)` | Returns `u32`; 0xFFFF = missing glyph |
| `DeleteObject(hFont)` | `DeleteObject(hfont)` | |
| `DeleteDC(hdc)` | `DeleteDC(hdc)` | |
| `EnumFontFamiliesExW(hdc, &lf, callback, lParam, 0)` | `EnumFontFamiliesExW(hdc, &lf, Some(callback_fn), LPARAM(ptr as isize), 0)` | Callback via `unsafe extern "system" fn` |

### Callback Pattern for `EnumFontFamiliesExW`

```rust
unsafe extern "system" fn enum_font_callback(
    logfont: *const LOGFONTW,
    _textmetric: *const TEXTMETRICW,
    _font_type: u32,
    lparam: LPARAM,
) -> i32 {
    let found = &mut *(lparam.0 as *mut bool);
    // Check logfont.lfFaceName for NF patterns
    // Set *found = true and return 0 to stop, or return 1 to continue
}
```

Context is passed via `LPARAM` by casting a `*mut bool` (or `*mut SomeStruct`) to `LPARAM(ptr as isize)`.

### Feature Flag Addition

Add to `Cargo.toml` `[dependencies.windows.features]`:
```toml
"Win32_Graphics_Gdi"
```

No other new features or crates required.

---

## R2: UTF-16 Encoding for Nerd Font Glyphs

### Decision: Use Rust's native `char::encode_utf16()` — no custom struct needed

**Rationale**: Rust `char` is a Unicode scalar value (0–0x10FFFF). The standard library method `char::encode_utf16(&mut [u16; 2])` handles both BMP (single u16) and supplementary plane (surrogate pair) cases. This replaces TCDir's `WideCharPair` / `CodePointToWideChars(char32_t)` entirely.

**Alternatives considered**:
- Port `WideCharPair` struct — rejected; Rust's built-in is simpler and compile-time verified
- Store glyphs as `&str` — rejected; need `u16` for `GetGlyphIndicesW` and console write buffer

### Usage Pattern

```rust
let glyph: char = '\u{E5FF}';  // nf-custom-folder (BMP)
let mut buf = [0u16; 2];
let encoded: &[u16] = glyph.encode_utf16(&mut buf);
// encoded.len() == 1 for BMP, 2 for supplementary

let glyph2: char = '\u{F0163}';  // nf-md-cloud_outline (supplementary plane)
let encoded2: &[u16] = glyph2.encode_utf16(&mut buf);
// encoded2.len() == 2 (surrogate pair)
```

### Console Buffer Integration

RCDir's `Console` struct accumulates output in a `String` buffer (UTF-8), which gets converted to UTF-16 at flush time via `encode_utf16()`. Nerd Font glyphs can be embedded directly as `char` values pushed to the `String` — they're valid UTF-8. No special encoding path needed in the hot loop.

```rust
// In console.rs — icon emission is just pushing a char to the String buffer
self.buffer.push(glyph);     // NF glyph (valid UTF-8)
self.buffer.push(' ');        // space separator
```

---

## R3: Icon Data Structure Design

### Decision: `HashMap<String, char>` for extension/dir lookups, seeded from static arrays

**Rationale**: Mirrors TCDir's architecture (`unordered_map<wstring, char32_t>` → Rust `HashMap<String, char>`). Static arrays with `(extension, char)` tuples are iterated at init time to populate the hash maps. This separates the default data (compile-time) from the mutable runtime state (user overrides applied on top).

**Alternatives considered**:
- `phf` crate (perfect hash at compile time) — rejected; need mutability for user overrides, and adds an external crate
- `BTreeMap` — rejected; O(log n) vs O(1) lookup, no ordering needed
- Flat sorted array with binary search — rejected; need to merge user overrides, hash map is simpler

### Data Layout

| TCDir (C++) | RCDir (Rust) |
|-------------|-------------|
| `NfIcon::CustomFolder` (constexpr char32_t) | `const NF_CUSTOM_FOLDER: char = '\u{E5FF}'` |
| `SIconMappingEntry { LPCWSTR, char32_t }` | `(&str, char)` tuple in a `const` array |
| `unordered_map<wstring, char32_t>` | `HashMap<String, char>` (lowercase keys) |
| `unordered_map<DWORD, char32_t>` | `HashMap<u32, char>` (file attribute flag → icon) |

### Key Differences from TCDir

1. **No `WideCharPair`** — Rust `char` handles all Unicode natively
2. **`String` keys** instead of `wstring` — RCDir already uses `String` for extension keys in `Config::extension_colors`
3. **Attribute precedence** uses a `const` slice `&[(u32, &str)]` instead of a separate struct — the PSHERC0TA order is a fixed compile-time array

---

## R4: Nerd Font Detection Strategy

### Decision: Port TCDir's 5-step layered detection with trait-based testability

**Rationale**: The detection chain is proven in TCDir and the spec requires identical behavior. Using a trait instead of virtual methods enables unit testing without subclassing.

**Alternatives considered**:
- Simplified 2-step (env var only + default off) — rejected; spec requires auto-detection
- Terminal-specific escape sequence probing — rejected; no reliable cross-terminal protocol exists for glyph capability queries

### Detection Trait

```rust
pub trait FontProber {
    fn probe_console_font_for_glyph(&self, console_handle: HANDLE, canary: char) -> Result<bool>;
    fn is_nerd_font_installed(&self) -> Result<bool>;
}
```

- `DefaultFontProber` — real GDI implementation
- `MockFontProber` (cfg(test)) — configurable test responses
- The `detect()` function takes `&dyn FontProber` + `&dyn EnvironmentProvider` for full testability

### Detection Chain (identical to TCDir)

| Step | Condition | Rust Implementation |
|------|-----------|-------------------|
| 1 | WezTerm (`TERM_PROGRAM=WezTerm`) | `env_provider.get_env_var("TERM_PROGRAM") == Some("WezTerm")` |
| 2 | ConPTY detected | Check `WT_SESSION`, `TERM_PROGRAM`, `ConEmuPID`, `ALACRITTY_WINDOW_ID` |
| 3 | Classic conhost — canary probe | `prober.probe_console_font_for_glyph(handle, '\u{E5FF}')` |
| 4 | Font enumeration | `prober.is_nerd_font_installed()` |
| 5 | Fallback | `DetectionResult::Inconclusive` → treated as OFF |

### Canary Code Point

U+E5FF (`nf-custom-folder`) — identical to TCDir. This is in the Seti-UI range, specific to Nerd Fonts v3, absent from all standard Windows fonts.

> **Note**: The spec says U+E5FA but TCDir code uses U+E5FF. TCDir's code is authoritative — use U+E5FF.

---

## R5: Config Extension — Comma-Syntax Parsing

### Decision: Extend existing `ParseOverrideValue` logic to split on first comma

**Rationale**: TCDir's approach — split the value on the **first comma**, left side is color, right side is icon — is simple and backward compatible. No comma means color-only (existing behavior preserved). The existing `Config` parsing pipeline already handles `key=value` dispatch; we add an icon field to the result.

### Parse Flow

1. `process_color_override_entry(entry)` — existing, splits on `=`
2. **New**: after extracting the value, split on first `,`
   - No comma → color only (backward compatible)
   - Comma present → left = color part, right = icon part
3. `parse_icon_value(icon_str)` — **new function**:
   - Empty string → icon suppressed
   - Length 1 → literal BMP glyph (`char::from_u32()`, reject surrogates)
   - Starts with `U+` → parse hex code point (4–6 digits, range 0x0001–0x10FFFF)

### New Config Fields

```rust
pub extension_icons:          HashMap<String, char>,
pub extension_icon_sources:   HashMap<String, AttributeSource>,
pub well_known_dir_icons:     HashMap<String, char>,
pub well_known_dir_icon_sources: HashMap<String, AttributeSource>,
pub file_attr_icons:          HashMap<u32, char>,
pub icons:                    Option<bool>,  // env var Icons/Icons- switch
pub icon_directory_default:   char,          // '\u{E5FF}' nf-custom-folder
pub icon_file_default:        char,          // '\u{F15B}' nf-fa-file
pub icon_symlink:             char,          // '\u{EAED}' nf-cod-file_symlink_directory
pub icon_junction:            char,          // '\u{F08E}' nf-fa-external_link
pub icon_cloud_only:          char,          // '\u{F0163}' nf-md-cloud_outline
pub icon_locally_available:   char,          // '\u{F0160}' nf-md-cloud_check
pub icon_always_local:        char,          // '\u{F0403}' nf-md-pin
```

---

## R6: Display Style Resolution

### Decision: New `FileDisplayStyle` struct returned by `Config::get_display_style_for_file()`

**Rationale**: TCDir returns `SFileDisplayStyle { m_wTextAttr, m_iconCodePoint, m_fIconSuppressed }`. We mirror this with a Rust struct. The existing `get_text_attr_for_file()` is refactored to become the color half of a unified resolution function.

### Precedence Chain (identical to TCDir / spec FR-009, FR-010)

| Priority | Level | Color source | Icon source |
|----------|-------|-------------|-------------|
| 1 (highest) | File attribute (PSHERC0TA order) | `file_attr_colors` | `file_attr_icons` (falls through if absent) |
| 2 | Well-known directory name | — | `well_known_dir_icons` |
| 3 | File extension | `extension_colors` | `extension_icons` |
| 4 (lowest) | File type fallback | `attributes[Attribute::Directory]` or default | `icon_directory_default` / `icon_file_default` / `icon_symlink` / `icon_junction` |

**Key behavior**: Color locks at the first matching level. Icon evaluation continues to lower levels if the winning color level has no icon configured (FR-020).

---

## R7: Attribute Precedence Reorder

### Decision: New `file_attribute_map.rs` module with PSHERC0TA-order const array

**Rationale**: The spec mandates a new attribute precedence order (PSHERC0TA) that differs from the existing display column order (RHSATECP0). This is used by `get_display_style_for_file()` to determine which attribute wins for both color and icon. A dedicated module keeps the mapping isolated and testable.

### Precedence Order

```rust
pub const ATTRIBUTE_PRECEDENCE: &[(u32, &str)] = &[
    (FILE_ATTRIBUTE_REPARSE_POINT, "P"),   // 1 — identity-altering
    (FILE_ATTRIBUTE_SYSTEM,        "S"),   // 2 — OS-critical
    (FILE_ATTRIBUTE_HIDDEN,        "H"),   // 3 — intentionally invisible
    (FILE_ATTRIBUTE_ENCRYPTED,     "E"),   // 4 — access-restricting
    (FILE_ATTRIBUTE_READONLY,      "R"),   // 5 — access-restricting
    (FILE_ATTRIBUTE_COMPRESSED,    "C"),   // 6 — informational
    (FILE_ATTRIBUTE_SPARSE_FILE,   "0"),   // 7 — rare
    (FILE_ATTRIBUTE_TEMPORARY,     "T"),   // 8 — ephemeral
    (FILE_ATTRIBUTE_ARCHIVE,       "A"),   // 9 — near-universal noise
];
```

**Note**: This is a **behavioral change** from the existing RCDir attribute color resolution. The changelog must document this.

---

## R8: Performance Considerations

### Decision: Zero per-file allocations for icon lookup

**Rationale**: SC-004 requires <5% overhead. Icon lookup is an O(1) HashMap lookup returning a `char` (Copy type). No heap allocation per file. The NF detection runs once at startup.

### Hot Path Analysis

1. `get_display_style_for_file()` — 1-3 HashMap lookups per file (attribute → extension → fallback)
2. Icon glyph emission — `buffer.push(char)` + `buffer.push(' ')` — two byte pushes to pre-allocated String
3. No `String` construction per icon — `char` is Copy, pushed directly

### Startup Cost

- `NerdFontDetector::detect()` — one-time GDI probe (~1ms) or font enumeration (~5ms)
- `IconMapping::initialize()` — seed ~240 entries into 2 HashMaps (~0.1ms)
- Both negligible compared to directory enumeration

---

## Summary of Decisions

| # | Topic | Decision | Key Rationale |
|---|-------|----------|---------------|
| R1 | Win32 GDI APIs | `windows` crate + `Win32_Graphics_Gdi` feature | Already a dependency; type-safe wrappers |
| R2 | UTF-16 encoding | `char::encode_utf16()` — no custom struct | Rust handles natively; simpler than C++ |
| R3 | Icon data structures | `HashMap<String, char>` seeded from const arrays | Mirrors TCDir; O(1) lookup; mutable for overrides |
| R4 | NF detection | 5-step layered chain with `FontProber` trait | Identical to TCDir; trait enables testing |
| R5 | Comma-syntax parsing | Split on first comma in existing parse pipeline | Backward compatible; minimal code change |
| R6 | Display style | `FileDisplayStyle` struct from unified resolver | Mirrors TCDir's `SFileDisplayStyle` |
| R7 | Attribute precedence | PSHERC0TA const array in dedicated module | Spec-mandated; isolated and testable |
| R8 | Performance | Zero per-file allocations; one-time startup cost | <5% overhead target (SC-004) |
