# Data Model: Config File Support

## Entities

### Modified: `AttributeSource` (enum)

**Current**:
```rust
pub enum AttributeSource { Default, Environment }
```

**New**:
```rust
pub enum AttributeSource { Default, ConfigFile, Environment }
```

Used in: `attribute_sources`, `extension_sources`, `extension_icon_sources`, `well_known_dir_icon_sources`, `file_attr_colors`

**Impact**: All existing source-tracking parallel maps gain a third possible value. Display code in usage.rs must render three labels: "Default", "Config file", "Environment".

---

### Modified: `ErrorInfo` (struct)

**Current fields**:
| Field | Type | Description |
|-------|------|-------------|
| `message` | `String` | Error description |
| `entry` | `String` | Full "Key=Value" segment |
| `invalid_text` | `String` | Portion to underline |
| `invalid_text_offset` | `usize` | Position of invalid_text within entry |

**New fields**:
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `source_file_path` | `String` | `String::new()` | Config file path (empty for env var errors) |
| `line_number` | `usize` | `0` | 1-based line number (0 for env var errors) |

**Grouping rule**: Config file errors stored in `config_file_parse_result`; env var errors in `last_parse_result`. Display code queries each separately.

---

### Modified: `Config` (struct)

**New public fields**:

| Field | Type | Description |
|-------|------|-------------|
| `config_file_path` | `String` | Resolved path to `.rcdirconfig` (empty if not found) |
| `config_file_loaded` | `bool` | Whether config file was successfully loaded |
| `config_file_parse_result` | `ValidationResult` | Errors from config file parsing (separate from env var errors) |

**New source tracking fields** (public):

| Field | Type | Description |
|-------|------|-------------|
| `switch_sources` | `[AttributeSource; SWITCH_COUNT]` | Source tracking for boolean switches. `SWITCH_COUNT = 9`. Indexed in order: wide_listing, bare_listing, recurse, perf_timer, multi_threaded, show_owner, show_streams, icons, tree |
| `max_depth_source` | `AttributeSource` | Source tracking for the Depth parameter |
| `tree_indent_source` | `AttributeSource` | Source tracking for the TreeIndent parameter |
| `size_format_source` | `AttributeSource` | Source tracking for the Size parameter |

**New public methods**:

| Method | Signature | Description |
|--------|-----------|-------------|
| `load_config_file` | `fn load_config_file(&mut self)` | Resolve `.rcdirconfig` path from USERPROFILE, read file, parse lines, apply settings with `ConfigFile` source |
| `validate_config_file` | `fn validate_config_file(&self) -> &ValidationResult` | Return reference to config file parse errors |
| `config_file_path` | `fn config_file_path(&self) -> &str` | Return resolved config file path |
| `is_config_file_loaded` | `fn is_config_file_loaded(&self) -> bool` | Whether config file was found and loaded |

**Modified methods** (in `env_overrides.rs`):

| Method | Change |
|--------|--------|
| `apply_user_color_overrides` | Add `source: AttributeSource` parameter so the same method handles both config file and env var entries |
| `process_color_override_entry` | Add `source: AttributeSource` parameter; all source-map writes use this parameter instead of hardcoded `Environment` |
| All methods that write to source maps | Thread the `source` parameter through the call chain |

**New private methods**:

| Method | Signature | Description |
|--------|-----------|-------------|
| `process_config_lines` | `fn process_config_lines(&mut self, lines: &[String])` | Line-by-line processing: trim, skip blanks/comments, strip inline comments, pass entries to `process_color_override_entry` with `ConfigFile` source, tag errors with line numbers |

---

### New: `config/file_reader.rs` (module)

A focused module for reading config file bytes and converting to lines. No struct — just functions:

| Function | Signature | Description |
|----------|-----------|-------------|
| `read_config_file` | `fn read_config_file(path: &str) -> Result<Vec<String>, ConfigFileError>` | Read file via `fs::read`, check BOM, convert to UTF-8, split lines. Returns `Ok(lines)` on success. |
| `check_and_strip_bom` | `fn check_and_strip_bom(bytes: &mut Vec<u8>) -> Result<(), String>` | Detect/strip UTF-8 BOM; reject UTF-16 BOM with descriptive error. |

```rust
pub enum ConfigFileError {
    NotFound,                    // File doesn't exist — silently skip
    IoError(String),             // Other I/O error — single error line
    EncodingError(String),       // UTF-16 BOM or invalid UTF-8
}
```

**Testability**: Functions accept raw bytes or paths. Unit tests can call `check_and_strip_bom` directly with constructed byte vectors. Integration tests create temp files.

---

### Modified: `CommandLine` (struct)

**New field**:

| Field | Type | Description |
|-------|------|-------------|
| `show_settings` | `bool` | `--settings` switch — display merged configuration tables |

**Modified**: `handle_long_switch` adds `"settings"` to the bool_switches table mapping to `show_settings`.

---

### Modified: `usage.rs` (module)

**New functions**:

| Function | Description |
|----------|-------------|
| `display_config_file_help` | Config file syntax reference + color/icon format reference + example file + env var override note + file path & load status + errors (new `--config` output) |
| `display_config_file_issues` | Render config file errors with line numbers. Accepts `show_hint: bool` — when true, appends `(see --config for help)` |
| `display_settings` | Merged configuration tables with 3-source column (new `--settings` output). Shows "No config file or RCDIR..." when neither source set. |

**Modified functions**:

| Function | Change |
|----------|--------|
| `display_current_configuration` | Renamed → `display_settings`; source column renders three values |
| `display_env_var_issues` | Gains `show_hint: bool` parameter |

---

## State Transitions

```
Startup Flow:
  [Initialize defaults] → [load_config_file] → [apply_user_color_overrides] → [apply_config_defaults] → [parse CLI]
       Default               ConfigFile            Environment               (merge switches)          (CLI wins)

Error Accumulation:
  load_config_file → config_file_parse_result.errors (with line_number + source_file_path)
  apply_user_color_overrides → last_parse_result.errors (line_number=0, source_file_path=empty)

Display at end of run:
  1. Config file errors (if any) — header: "There are some problems with your config file (see --config for help):"
  2. Env var errors (if any) — header: "There are some problems with your RCDIR environment variable (see --env for help):"
```

## Validation Rules

- Config file path: resolved from `USERPROFILE` env var + `\.rcdirconfig`
- File not found: silent skip (FR-013), `config_file_loaded = false`
- File I/O error: single `ErrorInfo` in `config_file_parse_result`, `config_file_loaded = false`
- Per-line parsing: identical rules to env var entry parsing, plus line number tracking
- Duplicate settings: last occurrence wins (both within config file, and env var over config file)
