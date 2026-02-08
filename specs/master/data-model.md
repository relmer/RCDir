# Phase 1: Data Model — RCDir (C++ to Rust Port)

**Date**: 2026-02-08 | **Branch**: `master`

---

## Entity Map

### E-01: `FileInfo` — Extended file entry

**Source**: `WIN32_FIND_DATAW` wrapper + stream data

| Field | Rust Type | Source | Notes |
|-------|-----------|--------|-------|
| `attributes` | `u32` | `dwFileAttributes` | Win32 file attribute flags |
| `creation_time` | `FILETIME` | `ftCreationTime` | |
| `last_access_time` | `FILETIME` | `ftLastAccessTime` | |
| `last_write_time` | `FILETIME` | `ftLastWriteTime` | |
| `file_size` | `u64` | `(nFileSizeHigh << 32) \| nFileSizeLow` | Combined 64-bit size |
| `reparse_tag` | `u32` | `dwReserved0` | Reparse point tag (for cloud detection) |
| `file_name` | `OsString` | `cFileName[260]` | Wide → OsString conversion |
| `streams` | `Vec<StreamInfo>` | Enumerated separately | Only populated if `--streams` |

**Validation**: `file_name` must not be `.` or `..` (filtered during enumeration).

**Relationships**: Owned by `DirectoryInfo::matches`.

---

### E-02: `StreamInfo` — NTFS alternate data stream

| Field | Rust Type | Source | Notes |
|-------|-----------|--------|-------|
| `name` | `String` | Stream name with `:$DATA` stripped | Display name only |
| `size` | `u64` | `LARGE_INTEGER` | Stream size in bytes |

**Relationships**: Owned by `FileInfo::streams`.

---

### E-03: `DirectoryInfo` — Directory tree node

| Field | Rust Type | Default | Notes |
|-------|-----------|---------|-------|
| `dir_path` | `PathBuf` | ctor | Directory being listed |
| `file_specs` | `Vec<PathBuf>` | ctor | Glob patterns to match (e.g., `*.cpp`) |
| `matches` | `Vec<FileInfo>` | `vec![]` | Files matching the spec |
| `largest_file_size` | `u64` | `0` | For column alignment |
| `longest_filename` | `usize` | `0` | For column alignment |
| `file_count` | `u32` | `0` | Matched file count |
| `subdir_count` | `u32` | `0` | Subdirectory count |
| `stream_count` | `u32` | `0` | Alternate data stream count |
| `bytes_used` | `u64` | `0` | Total file bytes |
| `stream_bytes_used` | `u64` | `0` | Total stream bytes |
| `status` | `DirStatus` | `Waiting` | MT enumeration state |
| `error` | `Option<AppError>` | `None` | Error if status is `Error` |
| `children` | `Vec<Arc<Mutex<DirectoryInfo>>>` | `vec![]` | Child dirs (recursive tree) |
| `condvar` | `Condvar` | `Condvar::new()` | Notified when status → Done/Error; consumer waits on this |

**State Machine**:
```
Waiting ──(worker picks up)──> InProgress ──(success)──> Done   → condvar.notify_one()
                                           ──(failure)──> Error  → condvar.notify_one()
```

**Consumer blocks on `condvar.wait_while(lock, |s| s.status < Done)`** — the main thread walks the tree depth-first, blocking on each node's condvar individually.

**Relationships**:
- **Owns** `Vec<FileInfo>` (owns `StreamInfo`)
- **Owns children** via `Arc<Mutex<DirectoryInfo>>` (recursive tree for MT)
- Referenced by `ResultsDisplayer` trait methods

---

### E-04: `DriveInfo` — Volume/drive information

| Field | Rust Type | Default | Notes |
|-------|-----------|---------|-------|
| `root_path` | `PathBuf` | — | Drive root (e.g., `C:\`) |
| `unc_path` | `Option<PathBuf>` | `None` | UNC path if applicable |
| `volume_name` | `String` | `""` | Volume label from `GetVolumeInformationW` |
| `filesystem_name` | `String` | `""` | FS type (NTFS, ReFS, etc.) |
| `volume_type` | `u32` | `0` | `GetDriveType` return value |
| `remote_name` | `Option<String>` | `None` | Remote name for network drives |

**Computed Properties**:
- `volume_description() -> &str`: Human-readable type (maps `volume_type` → "a hard drive", etc.)
- `is_unc() -> bool`: `unc_path.is_some()`
- `is_ntfs() -> bool`: `filesystem_name == "NTFS"`
- `is_refs() -> bool`: `filesystem_name == "ReFS"`

**Relationships**: Constructed from directory path. Passed to `ResultsDisplayer::display_results`.

---

### E-05: `Config` — Color configuration and env var overrides

| Field | Rust Type | Default | Notes |
|-------|-----------|---------|-------|
| `attributes` | `[u16; Attribute::COUNT]` | Default color scheme | WORD per display attribute |
| `attribute_sources` | `[AttributeSource; Attribute::COUNT]` | All `Default` | Tracks origin |
| `extension_colors` | `HashMap<OsString, u16>` | Built-in defaults | Extension → WORD |
| `extension_sources` | `HashMap<OsString, AttributeSource>` | All `Default` | Extension → source |
| `file_attr_colors` | `HashMap<u32, FileAttrStyle>` | Built-in defaults | Win32 attr → style |
| `wide_listing` | `Option<bool>` | `None` | Env var default for `/W` |
| `bare_listing` | `Option<bool>` | `None` | Env var default for `/B` |
| `recurse` | `Option<bool>` | `None` | Env var default for `/S` |
| `perf_timer` | `Option<bool>` | `None` | Env var default for `/P` |
| `multi_threaded` | `Option<bool>` | `None` | Env var default for `/M` |
| `show_owner` | `Option<bool>` | `None` | Env var default for `--owner` |
| `show_streams` | `Option<bool>` | `None` | Env var default for `--streams` |

**Nested Types**:
- `Attribute` — enum with 16 variants + `COUNT` (see A.2.4 in spec)
- `AttributeSource` — `enum { Default, Environment }`
- `FileAttrStyle` — `struct { attr: u16, source: AttributeSource }`
- `ValidationResult` — `struct { errors: Vec<ErrorInfo> }`
- `ErrorInfo` — `struct { message: String, entry: String, invalid_text: String, invalid_text_offset: usize }`

**Key Methods**:
- `initialize(default_attr: u16)` — Set up defaults + parse `RCDIR` env var
- `get_text_attr_for_file(file_info: &FileInfo) -> u16` — Resolve color priority: extension → attr → default
- `validate_environment_variable() -> ValidationResult` — Parse and validate `RCDIR`
- `parse_color_spec(spec: &str) -> Result<u16, AppError>` — Parse `"LightRed on Black"` → WORD

**Relationships**:
- References `EnvironmentProvider` trait for env var access (testability)
- Uses `FileAttributeMap` constants for attribute ↔ char mapping
- Consumed by `Console` (for color lookup) and `CommandLine` (for defaults)

---

### E-06: `CommandLine` — Parsed CLI arguments

| Field | Rust Type | Default | Notes |
|-------|-----------|---------|-------|
| `recurse` | `bool` | `false` | `/S` |
| `attrs_required` | `u32` | `0` | `/A:xyz` required mask |
| `attrs_excluded` | `u32` | `0` | `/A:-xyz` excluded mask |
| `sort_order` | `SortOrder` | `Default` | `/O:x` primary sort |
| `sort_direction` | `SortDirection` | `Ascending` | `/O:-x` |
| `sort_preference` | `[SortOrder; 5]` | `[Default, Name, Date, Extension, Size]` | Tiebreaker chain |
| `masks` | `Vec<OsString>` | `vec![]` | Positional args |
| `wide_listing` | `bool` | `false` | `/W` |
| `bare_listing` | `bool` | `false` | `/B` |
| `perf_timer` | `bool` | `false` | `/P` |
| `multi_threaded` | `bool` | `true` | `/M` |
| `show_env_help` | `bool` | `false` | `--env` |
| `show_config` | `bool` | `false` | `--config` |
| `show_help` | `bool` | `false` | `/?` or `-?` |
| `switch_prefix` | `char` | `'-'` | Last-used prefix char |
| `time_field` | `TimeField` | `Written` | `/T:x` |
| `show_owner` | `bool` | `false` | `--owner` |
| `show_streams` | `bool` | `false` | `--streams` |
| `debug` | `bool` | `false` | `--debug` (debug builds only) |

**Nested Enums**:
- `SortOrder` — `{ Default, Name, Extension, Size, Date }`
- `SortDirection` — `{ Ascending, Descending }`
- `TimeField` — `{ Written, Creation, Access }`

**Key Methods**:
- `parse_from(args: impl Iterator) -> Result<Self, AppError>` — Parse argv
- `apply_config_defaults(&mut self, config: &Config)` — Apply env var switch defaults

**Relationships**: Consumed by `DirectoryLister`, `ResultsDisplayer`, `FileComparator`.

---

### E-07: `Console` — Buffered console output with colors

| Field | Rust Type | Default | Notes |
|-------|-----------|---------|-------|
| `buffer` | `String` | `String::with_capacity(10_485_760)` | 10 MB pre-allocated |
| `stdout_handle` | `HANDLE` | `GetStdHandle` | Raw console handle |
| `is_redirected` | `bool` | `true` | Initially assumed redirected |
| `console_width` | `u32` | `80` | From `GetConsoleScreenBufferInfo` |
| `config` | `Arc<Config>` | — | Color configuration reference |
| `prev_attr` | `Option<u16>` | `None` | Color elision tracking |

**Key Methods**:
- `initialize(config: Arc<Config>) -> Result<Self, AppError>`
- `putchar(attr: u16, ch: char)` — Single char with color
- `puts(attr_idx: Attribute, text: &str)` — Line with named color + newline
- `printf(attr: u16, text: &str)` — Formatted text with color
- `color_printf(text: &str)` — Text with `{MarkerName}` embedded color markers
- `print_colorful_string(text: &str)` — Rainbow cycling text
- `set_color(attr: u16)` — Emit ANSI SGR if color changed
- `flush() -> Result<(), AppError>` — Write buffer to OS (WriteConsoleW or WriteFile)
- `width() -> u32` — Console column count

**Relationships**: References `Config` (for color lookups). Used by all displayer implementations.

---

### E-08: `ListingTotals` — Accumulator for recursive summaries

| Field | Rust Type | Default | Notes |
|-------|-----------|---------|-------|
| `file_count` | `u32` | `0` | Total files |
| `dir_count` | `u32` | `0` | Total subdirectories |
| `file_bytes` | `u64` | `0` | Total file bytes |
| `stream_count` | `u32` | `0` | Total streams |
| `stream_bytes` | `u64` | `0` | Total stream bytes |

**Methods**: `add(&mut self, other: &ListingTotals)` — Accumulate.

**Relationships**: Built from `DirectoryInfo` nodes. Passed to `ResultsDisplayer::display_recursive_summary`.

---

### E-09: `DirStatus` — Enumeration state enum

```rust
enum DirStatus {
    Waiting,
    InProgress,
    Done,
    Error,
}
```

---

### E-10: `MaskGroup` — Grouped file masks by directory

| Field | Rust Type | Notes |
|-------|-----------|-------|
| `directory` | `PathBuf` | Absolute directory path |
| `file_specs` | `Vec<PathBuf>` | File masks for this directory |

**Key Function**: `group_masks_by_directory(masks: &[OsString]) -> Vec<MaskGroup>`

**Rules**:
1. Pure masks (no path) → grouped under CWD
2. Directory-qualified masks → grouped by directory (case-insensitive)
3. Trailing separator on mask → filespec becomes `*`
4. Empty masks → single group: `[CWD, ["*"]]`

---

### E-11: `FileAttributeMap` — Attribute display constants

Static constant array mapping `u32` → `char`:

| Constant | Display Char |
|----------|-------------|
| `FILE_ATTRIBUTE_READONLY` | `R` |
| `FILE_ATTRIBUTE_HIDDEN` | `H` |
| `FILE_ATTRIBUTE_SYSTEM` | `S` |
| `FILE_ATTRIBUTE_ARCHIVE` | `A` |
| `FILE_ATTRIBUTE_TEMPORARY` | `T` |
| `FILE_ATTRIBUTE_ENCRYPTED` | `E` |
| `FILE_ATTRIBUTE_COMPRESSED` | `C` |
| `FILE_ATTRIBUTE_REPARSE_POINT` | `P` |
| `FILE_ATTRIBUTE_SPARSE_FILE` | `0` |

Order is fixed (display order for attribute column).

---

## Trait Definitions

### T-01: `ResultsDisplayer` — Display mode interface

```rust
trait ResultsDisplayer {
    fn display_results(
        &self,
        console: &mut Console,
        drive_info: &DriveInfo,
        dir_info: &DirectoryInfo,
        level: DirectoryLevel,
    );

    fn display_recursive_summary(
        &self,
        console: &mut Console,
        dir_info: &DirectoryInfo,
        totals: &ListingTotals,
    );
}
```

**Implementations**:
- `NormalDisplayer` — Full listing (date, time, attrs, size, cloud, owner, name)
- `WideDisplayer` — Column-major compact (`[dirname]` / `filename`)
- `BareDisplayer` — Filenames only, no headers/footers

---

### T-02: `EnvironmentProvider` — Env var abstraction (for testability)

```rust
trait EnvironmentProvider {
    fn get_env_var(&self, name: &str) -> Option<String>;
}
```

**Implementations**:
- `DefaultEnvironmentProvider` — `std::env::var()`
- `MockEnvironmentProvider` — Returns preset values for unit tests

---

## Entity Relationship Diagram

```
CommandLine ──uses──> Config (apply_config_defaults)
    │
    ├──> DirectoryLister (owns enumeration loop)
    │       │
    │       ├──> MaskGrouper::group_masks_by_directory(masks)
    │       │       └── Vec<MaskGroup>
    │       │
    │       ├──> DriveInfo::new(path) ──(one per mask group)
    │       │
    │       ├──> DirectoryInfo (root)
    │       │       ├── Vec<FileInfo>
    │       │       │       └── Vec<StreamInfo>
    │       │       └── Vec<Arc<Mutex<DirectoryInfo>>> (children)
    │       │
    │       ├──> MultiThreadedLister (std::thread workers + work queue)
    │       │       └── populates DirectoryInfo tree concurrently
    │       │       └── consumer (main thread) prints as nodes complete
    │       │
    │       └──> FileComparator::sort(matches, sort_order, sort_direction, time_field)
    │
    └──> ResultsDisplayer (trait)
            ├── NormalDisplayer ──uses──> Console
            ├── WideDisplayer  ──uses──> Console
            └── BareDisplayer  ──uses──> Console
                                            └── Config (color lookup)
```

---

## Validation Rules

| Entity | Rule | Error |
|--------|------|-------|
| `CommandLine` | Unknown switch → `E_INVALIDARG` | Display usage + exit 1 |
| `CommandLine` | Empty `/O` or `/A` arg → `E_INVALIDARG` | Display usage + exit 1 |
| `CommandLine` | Double `-` in attribute spec → `E_INVALIDARG` | Display usage + exit 1 |
| `Config` | Invalid color name → `ErrorInfo` | Displayed at end of output |
| `Config` | Invalid switch prefix in env var → `ErrorInfo` | Displayed at end of output |
| `Config` | Unknown display attribute key → `ErrorInfo` | Displayed at end of output |
| `DirectoryInfo` | Path not found → status `Error` | `"Error: {path} does not exist"` |
| `FileInfo` | Filter: `(attrs & required) == required && (attrs & excluded) == 0` | File excluded from matches |
