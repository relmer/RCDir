# Research: Config File Support

## R-001: File I/O Approach

**Decision**: Use `std::fs::read()` to read the entire file as raw bytes, then check for BOM, convert to UTF-8 string, and split into lines. This matches the existing pattern in `profile_file_manager.rs`.

**Rationale**: `std::fs::read()` returns `Vec<u8>`, which allows BOM detection before string conversion. The existing `profile_file_manager::read_profile_file` already demonstrates this exact pattern (read bytes, check BOM, split lines). Reusing the same approach keeps the codebase consistent. For a sub-1KB config file, reading the entire file into memory is trivially fast.

**Error handling**: `fs::read()` returns `io::Error` which maps cleanly to `AppError::Io` via the existing `?` operator pattern. File-not-found (`ErrorKind::NotFound`) is silently skipped per FR-013; other errors produce a single file-level `ErrorInfo`.

**Alternatives considered**:
- `std::fs::read_to_string()`: Doesn't allow BOM detection before conversion. Would silently include BOM bytes in the string.
- `BufReader` line-by-line: Unnecessary complexity for a small file; harder to detect BOM.

## R-002: BOM Handling

**Decision**: Reuse the BOM detection pattern from `profile_file_manager.rs` — check first 3 bytes for UTF-8 BOM (`EF BB BF`) and skip if present. Reject UTF-16 BOMs (`FF FE` / `FE FF`) with an error since config files are UTF-8 only.

**Rationale**: `profile_file_manager.rs` already implements this exact check. The pattern is proven and tested. UTF-16 rejection adds safety without complexity.

## R-003: Comment Character Safety

**Decision**: `#` is safe for comment lines and inline comments. No disambiguation needed.

**Rationale**: The existing color parser accepts only named colors (Black, Blue, LightGreen, etc.) — no hex color codes. Icon code points use the `U+XXXX` prefix format, not `#`. There is no valid setting value that contains `#`, so stripping everything from `#` onward is unambiguous.

## R-004: Source Tracking Architecture

**Decision**: Extend the existing `AttributeSource` enum from 2 to 3 values: `Default`, `ConfigFile`, `Environment`. The config file applies first (producing `ConfigFile` source markers), then the env var applies on top (overwriting to `Environment` where it conflicts).

**Rationale**: The codebase already tracks sources via `AttributeSource` and parallel `extension_sources` / `attribute_sources` maps. Adding a third enum value is the minimal change. The `process_color_override_entry` function already updates source data on every call — it just needs to accept a source parameter.

**Key insight**: `apply_user_color_overrides()` in `env_overrides.rs` currently hardcodes `AttributeSource::Environment` when writing source maps. Refactor to accept a source parameter so the same method chain handles both config file and env var entries.

## R-005: Config File Path Resolution

**Decision**: Use `std::env::var("USERPROFILE")` to resolve `%USERPROFILE%`, then append `\.rcdirconfig`. Use the `EnvironmentProvider` trait for testability.

**Rationale**: The `EnvironmentProvider` trait already abstracts env var access and has a `MockEnvironmentProvider` for testing. Using it for USERPROFILE resolution naturally integrates with the existing test infrastructure. `USERPROFILE` is always set on Windows.

**Alternatives considered**:
- `dirs::home_dir()` crate: Adds an external dependency for something trivially solved with an env var.
- Hardcoded path: Fragile, breaks on non-standard installs.

## R-006: Parsing Architecture — Reuse Strategy

**Decision**: Reuse `process_color_override_entry()` directly for each config file line. The config file parser's job is only: read file → split lines → strip comments → pass each line to the existing entry processor.

**Rationale**: The spec requires identical syntax between config file entries and env var entries. The existing `process_color_override_entry()` already handles all entry types (switches, colors, icons, parameterized values). Reusing it avoids duplicating parsing logic and guarantees syntax parity.

**Line number tracking**: A new `process_config_lines()` method iterates lines, tracks line numbers, and after each `process_color_override_entry` call, tags any newly appended errors in `config_file_parse_result.errors` with the file path and 1-based line number.

## R-007: Error Model Extension

**Decision**: Extend `ErrorInfo` with two fields: `source_file_path` (String, empty for env var errors) and `line_number` (usize, 0 for env var errors). Config file errors go to a separate `config_file_parse_result: ValidationResult`; env var errors remain in `last_parse_result`. Display functions query their respective result object.

**Rationale**: Separate error containers simplifies grouping in display code — each display function queries one result object. Display functions accept a `show_hint` parameter: when `true` (normal listing runs), error headers include `(see --config for help)` or `(see --env for help)`; when `false` (inside `--settings`), the hint is omitted.

## R-008: Initialization Order

**Decision**: Config file is loaded first, then env var overrides, then CLI overrides. Specifically:
1. `Config::initialize()` → load built-in defaults, extension/icon maps
2. `Config::load_config_file()` → NEW: read `.rcdirconfig`, process entries with `AttributeSource::ConfigFile`
3. `Config::apply_user_color_overrides()` → existing: read RCDIR env var, process entries with `AttributeSource::Environment`
4. `CommandLine::apply_config_defaults()` → merge switch states into CLI defaults
5. `CommandLine::parse_from()` → CLI flags override everything

**Rationale**: The existing flow is initialize → apply_user_color_overrides → CLI. Inserting config file loading between initialize and apply_user_color_overrides naturally produces the correct precedence (config < env var < CLI).

## R-009: Switch Source Tracking

**Decision**: Add a parallel source-tracking array for switch overrides, similar to `attribute_sources`. Each `Option<bool>` switch in Config gets a corresponding `AttributeSource` entry. Parameterized values (Depth, TreeIndent, Size) also get source tracking.

**Rationale**: The `--settings` command needs to show the source of each switch/parameter setting. Without source tracking, there's no way to distinguish "set by config file" from "set by env var" for switches.
