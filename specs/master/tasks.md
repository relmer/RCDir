# Tasks: RCDir Full Application (C++ to Rust Port)

**Input**: Design documents from `/specs/002-full-app-spec/` and `/specs/master/`
**Prerequisites**: plan.md ✓, spec.md ✓, research.md ✓, data-model.md ✓, contracts/cli-contract.md ✓, quickstart.md ✓

**Tests**: Each module task includes writing `#[cfg(test)]` unit tests within the same source file. Tests are part of the task, not separate tasks. Output parity integration tests are in Phase 19.

**Organization**: Tasks grouped by user story in priority order. US-13 (Performance Timer) is first per plan.md mandate.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (e.g., US1, US13)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Project initialization — pop stashed scaffold, add real dependencies, establish module structure

- [X] T001 Pop cargo stash and update Cargo.toml with `windows` v0.62 feature flags (Win32_Storage_FileSystem, Win32_Storage_CloudFilters, Win32_System_Console, Win32_System_IO, Win32_System_Time, Win32_Security, Win32_Security_Authorization, Win32_Globalization, Win32_NetworkManagement_WNet) and `widestring = "1"` per research R-02/R-12 in Cargo.toml
- [X] T002 Create src/lib.rs with all module declarations (one `mod` per source file from plan.md: ehm, ansi_codes, color, environment_provider, console, command_line, config, file_info, directory_info, drive_info, mask_grouper, listing_totals, perf_timer, file_comparator, directory_lister, multi_threaded_lister, work_queue, results_displayer, cloud_status, streams, owner) and a `pub fn run() -> Result<(), AppError>` stub
  📖 Port from: `TCDir.cpp` (wmain structure), `pch.h` (module/include inventory)
- [X] T003 Update src/main.rs to call `rcdir::run()`, map `Err` to `eprintln!` + `process::exit(1)` per spec A.14 exit codes
  📖 Port from: `Main.cpp`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can begin

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Error Handling

- [X] T004 Implement `AppError` enum (`Win32`, `Io`, `InvalidArg`, `PathNotFound`) with `Display`, `Error`, and `From<windows::core::Error>`, `From<std::io::Error>` impls in src/ehm.rs — replaces TCDir's HRESULT + EHM macro pattern with Rust's `?` operator + `From` trait conversions
  📖 New Rust-idiomatic error type (no direct TCDir equivalent — TCDir uses raw HRESULT everywhere; `Ehm.h` macros are replaced by `?`)

### Color Infrastructure

- [X] T005 [P] Implement ANSI SGR escape code constants — Windows WORD (4-bit fg+bg) → ANSI code lookup table, +60 for bright variants, reset sequence `\x1b[0m`, SGR format `\x1b[{fg};{bg}m` — in src/ansi_codes.rs
  📖 Port from: `AnsiCodes.h`
- [X] T006 [P] Implement `Color` enum (16 values: Black, DarkBlue, DarkGreen, DarkCyan, DarkRed, DarkMagenta, DarkYellow, Gray, DarkGray, Blue, Green, Cyan, Red, Magenta, Yellow, White) with name→WORD and WORD→name bidirectional mapping in src/color.rs
  📖 Port from: `Color.h` (color constants + name mapping)
- [X] T007 [P] Implement `parse_color_spec()` — parse `"FgColor on BgColor"` strings into Windows WORD (4-bit fg | 4-bit bg<<4), case-insensitive color name matching per A.18 — in src/color.rs
  📖 Port from: `Config.cpp` → `CConfig::ParseColorValue()`

### Environment Abstraction

- [X] T008 [P] Implement `EnvironmentProvider` trait (`fn get_env_var(&self, name: &str) -> Option<String>`) and `DefaultEnvironmentProvider` (wraps `std::env::var`) in src/environment_provider.rs
  📖 Port from: `EnvironmentProviderBase.h`, `EnvironmentProvider.h`, `EnvironmentProvider.cpp`

### Console Output

- [X] T009 Implement `Console` struct initialization — GetStdHandle(STD_OUTPUT_HANDLE), GetConsoleMode for redirect detection, SetConsoleMode for ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleScreenBufferInfo for console width, 10 MB buffer pre-allocation — in src/console.rs
  📖 Port from: `Console.h` (class declaration), `Console.cpp` → `CConsole::CConsole()` constructor
- [X] T010 Implement `Console::set_color()` — emit ANSI SGR sequence via ansi_codes lookup, track `prev_attr: Option<u16>` for color elision (skip if unchanged), handle reset — in src/console.rs
  📖 Port from: `Console.cpp` → `CConsole::SetColor()`
- [X] T011 Implement `Console` basic output methods — `putchar(attr, ch)`, `puts(attr_idx, text)` (named color + newline), `printf(attr, text)` (formatted text with color) — in src/console.rs
  📖 Port from: `Console.cpp` → `PutChar()`, `Puts()`, `Printf()`
- [X] T012 Implement `Console::color_printf()` — text with embedded `{MarkerName}` color markers (e.g., `{Error}`, `{Date}`, `{Size}`), resolve marker names to Attribute enum → WORD via Config lookup — in src/console.rs
  📖 Port from: `Console.cpp` → `CConsole::ColorPrintf()`
- [X] T013 Implement `Console::print_colorful_string()` — rainbow cycling text (cycle through all 16 colors, skipping background color per spec D.1.1), used for colorful help screen headers — in src/console.rs
  📖 Port from: `Console.cpp` → `CConsole::PrintColorfulString()`
- [X] T014 Implement `Console::flush()` — dual output path: WriteConsoleW with UTF-16 conversion for real console, WriteFile with UTF-8 for redirected output, append `\x1b[0m` reset before flush — in src/console.rs
  📖 Port from: `Console.cpp` → `CConsole::Flush()`

### Command-Line Parser

- [X] T015 Implement `CommandLine` struct with all fields per data model E-06 (recurse, attrs_required, attrs_excluded, sort_order, sort_direction, sort_preference, masks, wide_listing, bare_listing, perf_timer, multi_threaded, show_env_help, show_config, show_help, switch_prefix, time_field, show_owner, show_streams, debug) and `Default` impl in src/command_line.rs
  📖 Port from: `CommandLine.h` (class declaration + enums)
- [X] T016 Implement `CommandLine::parse_from()` skeleton — iterate args, detect `/` or `-` prefix (set `switch_prefix`), route to short vs long switch handler based on length and second char per R-01 parsing rules — in src/command_line.rs
  📖 Port from: `CommandLine.cpp` → `CCommandLine::Parse()` outer loop
- [X] T017 Implement boolean switch handlers — S (recurse), W (wide), B (bare), P (perf_timer), M/M- (multi_threaded toggle), ? (show_help), each with trailing `-` disable support — in src/command_line.rs
  📖 Port from: `CommandLine.cpp` → `CCommandLine::Parse()` switch cases (S/W/B/P/M/?)
- [X] T018 Implement `/O` sort order handler — optional colon skip, optional `-` for reverse, read first key char (N/E/S/D), set sort_order + sort_direction + recompute sort_preference tiebreaker chain per A.6, case-insensitive per A.18 — in src/command_line.rs
  📖 Port from: `CommandLine.cpp` → `CCommandLine::Parse()` case 'O' block
- [X] T019 Implement `/A` attribute filter handler — optional colon skip, character-by-character loop, `-` toggles exclude mode for next char only then resets, map chars to Win32 attribute flags (D/H/S/R/A/T/E/C/P/0/X/I/B) plus cloud composites (O/L/V), double-`-` is error, case-insensitive per A.18 — in src/command_line.rs
  📖 Port from: `CommandLine.cpp` → `CCommandLine::Parse()` case 'A' block
- [X] T020 Implement `/T` time field handler — optional colon skip, read field char (C/A/W), case-insensitive, set `time_field` enum — in src/command_line.rs
  📖 Port from: `CommandLine.cpp` → `CCommandLine::Parse()` case 'T' block
- [X] T021 Implement long switch handler — case-insensitive match via lstrcmpiW or `eq_ignore_ascii_case` for `env`/`config`/`owner`/`streams`/`debug`, reject single-dash long (e.g., `-env` → error, `--env` and `/env` → valid) per A.18.2 — in src/command_line.rs
  📖 Port from: `CommandLine.cpp` → `CCommandLine::Parse()` long-switch detection block
- [X] T022 Implement positional argument (file mask) collection — args without switch prefix are masks, collect into `masks: Vec<OsString>` — in src/command_line.rs
  📖 Port from: `CommandLine.cpp` → `CCommandLine::Parse()` else-branch (non-switch args)
- [X] T023 Implement `CommandLine::apply_config_defaults()` — apply env var switch defaults from Config (wide_listing, bare_listing, recurse, perf_timer, multi_threaded, show_owner, show_streams), command-line switches override env var defaults — in src/command_line.rs
  📖 Port from: `CommandLine.cpp` → `CCommandLine::ApplyConfigDefaults()`

### Configuration

- [X] T024 Implement `Config` struct skeleton — Attribute enum (16 display item variants per A.2.4 + COUNT), AttributeSource enum, `attributes` array, `attribute_sources` array, extension_colors HashMap, file_attr_colors HashMap, switch default Option fields — in src/config.rs
  📖 Port from: `Config.h` (class declaration, EAttribute enum, EAttributeSource enum)
- [X] T025 Implement `Config` default display item colors — hardcode all 16 display item colors per spec A.3.1 (Background, Date, Time, AM/PM, Attrs, Size, DirSlash, Separator, Error, Header, etc.) — in src/config.rs
  📖 Port from: `Config.cpp` → `CConfig::CConfig()` constructor (s_defaultAttributes table)
- [X] T026 [P] Implement `Config` default extension color map — hardcode all ~40 extension→color mappings per spec A.3.3 (grouped: source code, web, config, data, binary, media, compressed, documents) — in src/config.rs
  📖 Port from: `Config.cpp` → `CConfig::CConfig()` (extension color initialization block)
- [X] T027 [P] Implement `Config` default file attribute colors — hardcode attribute flag → color mappings per spec A.3.2 (Hidden, System, Encrypted, Compressed, etc.) with priority ordering — in src/config.rs
  📖 Port from: `Config.cpp` → `CConfig::CConfig()` (file attribute color initialization block)
- [X] T028 Implement `Config::get_text_attr_for_file()` — resolve color priority: check file attribute colors first (in priority order), then extension color, then default FileName color — in src/config.rs
  📖 Port from: `Config.cpp` → `CConfig::GetTextAttrForFile()`

**Checkpoint**: Foundation ready — all modules compile, Console can output colored text, CommandLine parses all switches, Config provides default colors

---

## Phase 3: User Story 13 — Performance Timing Information (Priority: P1) 🎯 First

**Goal**: Implement `/p` switch so elapsed time is measurable from day one per plan.md mandate

**Independent Test**: `cargo run -- /p` prints `RCDir time elapsed:  X.XX msec`

- [X] T029 [US13] Implement `PerfTimer` struct — wraps `std::time::Instant`, `new()` captures start time, `elapsed_ms() -> f64` returns milliseconds with fractional precision — in src/perf_timer.rs
  📖 Port from: `PerfTimer.h`, `PerfTimer.cpp` (QueryPerformanceCounter → Rust Instant)
- [X] T030 [US13] Wire PerfTimer into lib::run() — start at entry, print formatted output per spec A.11 (`"RCDir time elapsed:  {:.2} msec\n"`, two spaces after colon, 2 decimal places) if `command_line.perf_timer` is true — in src/lib.rs
  📖 Port from: `TCDir.cpp` → timer output at end of `wmain()`

**Checkpoint**: `rcdir /p` produces timer output; baseline for all future performance comparisons

---

## Phase 4: User Story 15 — Help Display (Priority: P1)

**Goal**: Implement `/?` and `-?` usage screen so users can discover all switches

**Independent Test**: `cargo run -- /?` displays full help with all switches, attribute codes, cloud symbols, version, and architecture

- [X] T031 [US15] Implement help screen header — product name (colorful rainbow text), version number, architecture (x64/ARM64) via `cfg!(target_arch)`, copyright — in src/lib.rs
  📖 Port from: `Usage.cpp` → `CUsage::DisplayUsage()` header section, `Version.h`
- [X] T032 [US15] Implement help screen switch listing — all switches with descriptions, column-aligned, switch prefix–aware formatting (show `/` or `-` based on `switch_prefix`) — in src/lib.rs
  📖 Port from: `Usage.cpp` → `CUsage::DisplayUsage()` switch listing section
- [X] T033 [US15] Implement help screen attribute codes table — all attribute filter characters (D/H/S/R/A/T/E/C/P/0/X/I/B/O/L/V) with descriptions, cloud status symbols (○/◐/●) in their configured colors with meanings — in src/lib.rs
  📖 Port from: `Usage.cpp` → `CUsage::DisplayUsage()` attribute codes + cloud symbols sections, `UnicodeSymbols.h`
- [X] T034 [US15] Implement `--env` help screen — RCDIR environment variable static syntax reference: entry types (switches, display item colors, extension colors, attribute colors), format examples, color chart, valid switch names per D.2.1–D.2.3 — in src/lib.rs
  📖 Port from: `Usage.cpp` → `CUsage::DisplayEnvVarHelp()` (static reference sections only)
- [X] T035 [US15] Wire help and env-help into lib::run() — check `show_help` and `show_env_help` as early-exit paths, display usage screen on invalid switch (`Err(InvalidArg)` → usage + exit 1) — in src/lib.rs
  📖 Port from: `TCDir.cpp` → `wmain()` help/env-help early-exit paths

**Checkpoint**: `rcdir /?` and `rcdir --env` both display complete reference information; invalid switches show usage + exit 1

---

## Phase 5: User Story 1 — View Directory Contents with Visual Differentiation (Priority: P1)

**Goal**: Core directory listing with colorized output — date, time, size, attributes, color-coded filenames, headers, and footers

**Independent Test**: `cargo run` in any directory shows colored listing with volume header, file details, and summary footer matching TCDir output

### Data Structures

- [X] T036 [P] [US1] Implement `FileInfo` struct — fields per data model E-01 (attributes, creation_time, last_access_time, last_write_time, file_size, reparse_tag, file_name as OsString, streams vec), construct from WIN32_FIND_DATAW — in src/file_info.rs
  📖 Port from: `DirectoryInfo.h` → `FILEENTRY` struct (WIN32_FIND_DATAW field mapping)
- [X] T037 [P] [US1] Implement RAII `FindHandle` struct (wraps Win32 HANDLE, Drop calls FindClose) and `SafeHandle` struct (wraps HANDLE, Drop calls CloseHandle) — these are NOT interchangeable per research R-02 — in src/file_info.rs
  📖 Port from: `UniqueFindHandle.h` (FindHandleDeleter + UniqueFindHandle typedef)
- [X] T038 [P] [US1] Implement `FileAttributeMap` — static constant array mapping u32 attribute flags → display chars in fixed order (R/H/S/A/T/E/C/P/0 per A.4.2), function to build 9-char attribute display string from a file's attributes — in src/file_info.rs
  📖 Port from: `FileAttributeMap.h`
- [X] T039 [P] [US1] Implement `ListingTotals` struct — fields per data model E-08 (file_count, dir_count, file_bytes, stream_count, stream_bytes), Default impl, `add()` accumulator method — in src/listing_totals.rs
  📖 Port from: `ListingTotals.h`
- [X] T040 [P] [US1] Implement `DirectoryInfo` struct for single-directory mode — fields: dir_path (PathBuf), file_specs (Vec), matches (Vec<FileInfo>), largest_file_size, longest_filename, file_count, subdir_count, stream_count, bytes_used, stream_bytes_used — in src/directory_info.rs
  📖 Port from: `DirectoryInfo.h` → `CDirectoryInfo` class
- [X] T041 [P] [US1] Implement `MaskGroup` struct and `group_masks_by_directory()` — rules: pure masks grouped under CWD, directory-qualified grouped by dir (case-insensitive), trailing separator → `*`, empty masks → single group `[CWD, ["*"]]`, handle drive-letter paths — in src/mask_grouper.rs
  📖 Port from: `MaskGrouper.h`, `MaskGrouper.cpp`

### Drive Information

- [X] T042 [US1] Implement `DriveInfo` struct — GetVolumeInformationW (volume label, filesystem name), GetDriveTypeW (volume_type), GetDiskFreeSpaceExW (free bytes), volume_description() human-readable type mapping, is_unc/is_ntfs/is_refs computed properties — in src/drive_info.rs
  📖 Port from: `DriveInfo.h`, `DriveInfo.cpp` → `CDriveInfo::CDriveInfo()` constructor, `GetVolumeDescription()`
- [X] T043 [US1] Implement DriveInfo UNC path handling — detect UNC paths, WNetGetConnectionW for mapped drive remote names, format for header display — in src/drive_info.rs
  📖 Port from: `DriveInfo.cpp` → UNC detection + `WNetGetConnectionW` block

### Directory Enumeration

- [X] T044 [US1] Implement core enumeration loop — FindFirstFileW with constructed search path (dir_path + file_spec), FindNextFileW loop, skip `.`/`..` entries, construct FileInfo from WIN32_FIND_DATAW, track largest_file_size and longest_filename — in src/directory_lister.rs
  📖 Port from: `DirectoryLister.cpp` → `CDirectoryLister::EnumerateFiles()` inner loop
- [X] T045 [US1] Implement file spec matching logic — for each file spec in DirectoryInfo.file_specs, enumerate matching files, separate directories from files for counting, populate matches vec and counters (file_count, subdir_count, bytes_used) — in src/directory_lister.rs
  📖 Port from: `DirectoryLister.cpp` → `CDirectoryLister::EnumerateFiles()` outer file-spec loop + counter accumulation

### Display Infrastructure

- [X] T046 [US1] Implement `ResultsDisplayer` trait — `display_results(&self, console, drive_info, dir_info, level)` and `display_recursive_summary(&self, console, dir_info, totals)`, plus `DirectoryLevel` enum (First, Subsequent, RecursiveFirst, RecursiveSubsequent) — in src/results_displayer.rs
  📖 Port from: `IResultsDisplayer.h` (interface + EDirectoryLevel enum)

### Normal Listing Format (NormalDisplayer)

- [X] T047 [US1] Implement NormalDisplayer volume header — `color_printf` with drive type description, volume label, serial number, and "Directory of {path}" line per A.4.5, conditional on DirectoryLevel — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerWithHeaderAndFooter.cpp` → `DisplayHeader()`
- [X] T048 [US1] Implement date/time formatting helpers — FILETIME → FileTimeToSystemTime → SystemTimeToTzSpecificLocalTime → GetDateFormatEx(DATE_SHORTDATE) + GetTimeFormatEx(TIME_NOSECONDS), select which FILETIME based on TimeField setting — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerNormal.cpp` → `FormatDate()`, `FormatTime()`
- [X] T049 [US1] Implement size formatting helper — u64 file size → GetNumberFormatEx for locale-aware thousands-separated string, right-aligned to `largest_file_size` column width, `<DIR>` tag for directories — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerNormal.cpp` → `FormatSize()`
- [X] T050 [US1] Implement attribute column formatting — build 9-char attribute display string from FileAttributeMap, colorize each attribute char using file attribute colors from Config — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerNormal.cpp` → attribute display section within `DisplayFileEntry()`
- [X] T051 [US1] Implement filename colorization — resolve file color via Config::get_text_attr_for_file (priority: file attribute color → extension color → default), apply color to filename output — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerNormal.cpp` → filename color resolution within `DisplayFileEntry()`
- [X] T052 [US1] Implement per-file line assembly — combine date + time + AM/PM + size + attributes + filename into single formatted line with correct column widths and spacing per A.4.1 — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerNormal.cpp` → `DisplayFileEntry()` full line layout
- [X] T053 [US1] Implement directory footer — "X File(s)  Y bytes" line with locale-formatted numbers, free space line "Z bytes free" with locale-formatted number — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerWithHeaderAndFooter.cpp` → `DisplayFooter()`
- [X] T054 [US1] Implement separator lines — horizontal line between directories, blank line rules per A.12 — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerWithHeaderAndFooter.cpp` → separator logic in `DisplayResults()`
- [X] T054a [US1] Implement empty-directory and no-match messages — "Directory is empty." when all specs are `*`, "No files matching '...' found." for specific file specs per D.4.2 — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerWithHeaderAndFooter.cpp` → `DisplayResults()` empty-directory branch

### Main Orchestration

- [X] T055 [US1] Wire single-directory listing loop into lib::run() — parse args, init Config with defaults, apply_config_defaults, group masks, for each MaskGroup: get DriveInfo, create DirectoryInfo, enumerate, sort (default order), display via NormalDisplayer, flush Console — in src/lib.rs
  📖 Port from: `TCDir.cpp` → `wmain()` main listing orchestration loop

**Checkpoint**: `rcdir` produces colorized directory listing matching TCDir for a single directory; `rcdir *.rs *.toml` groups masks correctly; colors match spec defaults

---

## Phase 6: User Story 2 — Sort Directory Listings (Priority: P1)

**Goal**: Sort files by name, extension, size, or date with ascending/descending control

**Independent Test**: `cargo run -- /o:s` sorts by size ascending; `cargo run -- /o:-d` sorts by date descending

- [X] T056 [US2] Implement `FileComparator` sort dispatch — closure-based sort_by dispatching on SortOrder enum, directories-first partitioning (dirs sorted separately from files, dirs always listed first) — in src/file_comparator.rs
  📖 Port from: `FileComparator.h`, `FileComparator.cpp` → `CFileComparator::Sort()`, `CompareEntries()`
- [X] T057 [US2] Implement name/extension comparison — lstrcmpiW via windows crate for locale-aware case-insensitive string comparison (not Rust .cmp()), extract extension from filename for extension sort — in src/file_comparator.rs
  📖 Port from: `FileComparator.cpp` → `CompareName()`, `CompareExtension()`
- [X] T058 [US2] Implement size/date comparison and tiebreaker chain — u64 compare for size, CompareFileTime for dates respecting TimeField selection, tiebreaker fallback order per A.6.2 (primary → name → date → extension → size), ascending/descending via SortDirection — in src/file_comparator.rs
  📖 Port from: `FileComparator.cpp` → `CompareSize()`, `CompareDate()`, tiebreaker chain in `CompareEntries()`
- [ ] T059 [US2] Wire sorting into directory_lister — call FileComparator::sort on DirectoryInfo.matches after enumeration, before passing to displayer — in src/directory_lister.rs
  📖 Port from: `DirectoryLister.cpp` → sort call after `EnumerateFiles()`

**Checkpoint**: All four sort keys work; reverse sort works; tiebreaker chain produces stable ordering matching TCDir; directories always sort first

---

## Phase 7: User Story 3 — Filter Files by Attributes (Priority: P1)

**Goal**: Filter directory listings by file attributes with include/exclude semantics

**Independent Test**: `cargo run -- /a:d` shows only directories; `cargo run -- /a:-hsd` excludes hidden, system, directories

- [ ] T060 [US3] Implement standard attribute filter logic — during enumeration, apply `(attrs & required) == required && (attrs & excluded) == 0` test, map standard chars to Win32 flags (D/H/S/R/A/T/E/C/P/0/X/I/B) per A.7.1 — in src/directory_lister.rs
  📖 Port from: `DirectoryLister.cpp` → attribute filter check within `EnumerateFiles()`, `Flag.h` (CFlag::IsSet/IsNotSet)
- [ ] T061 [US3] Implement cloud-composite attribute mapping — O maps to composite (FILE_ATTRIBUTE_OFFLINE | RECALL_ON_OPEN | RECALL_ON_DATA_ACCESS), L maps to unpinned locally-available check, V maps to pinned check — per A.7 and A.8 in src/directory_lister.rs
  📖 Port from: `DirectoryLister.cpp` → cloud attribute filter logic, `CommandLine.cpp` → 'A' case cloud composites

**Checkpoint**: Standard attribute filters work; cloud attribute filters work; `-` exclusion prefix works; double-`-` is an error

---

## Phase 8: User Story 4 — Recursive Directory Listing with Multi-Threading (Priority: P1)

**Goal**: Recursive `/s` traversal with streaming producer/consumer multi-threaded enumeration per A.15

**Independent Test**: `cargo run -- /s` recursively lists all subdirectories with progressive output; `cargo run -- /s /p` shows timing

### Tree Infrastructure

- [ ] T062 [US4] Extend `DirectoryInfo` for tree structure — add `DirStatus` enum (Waiting/InProgress/Done/Error per A.15.6), per-node `Mutex` + `Condvar`, `error: Option<AppError>`, `children: Vec<Arc<Mutex<DirectoryInfo>>>` — in src/directory_info.rs
  📖 Port from: `DirectoryInfo.h` → tree fields (m_status, m_hEvent, m_children, m_hrError)
- [ ] T063 [P] [US4] Implement `WorkQueue<T>` — thread-safe FIFO using `Mutex<VecDeque<T>>` + `Condvar`, `push()` adds item + notify_one, `pop()` waits until item available or done, `set_done()` sets flag + notify_all, `is_done()` check — in src/work_queue.rs
  📖 Port from: `WorkQueue.h`

### Worker Threads

- [ ] T064 [US4] Implement `MultiThreadedLister` struct — spawn N worker threads (`std::thread::available_parallelism()`, min 1), store `JoinHandle<()>` vec, `Arc<AtomicBool>` stop signal — in src/multi_threaded_lister.rs
  📖 Port from: `MultiThreadedLister.h`, `MultiThreadedLister.cpp` → `CMultiThreadedLister::CMultiThreadedLister()` + `Start()`
- [ ] T065 [US4] Implement worker thread enumeration loop — pop work item, lock node → set InProgress, enumerate matching files (reuse enumeration from T044/T045), update node counters — in src/multi_threaded_lister.rs
  📖 Port from: `MultiThreadedLister.cpp` → `WorkerThread()` main loop
- [ ] T066 [US4] Implement worker child directory discovery — during enumeration, for each subdirectory found: create child DirectoryInfo node, lock parent → add to children vec, push child onto work queue — in src/multi_threaded_lister.rs
  📖 Port from: `MultiThreadedLister.cpp` → `WorkerThread()` subdirectory discovery block
- [ ] T067 [US4] Implement worker completion signaling — after enumeration: lock node → set status Done or Error, store error if failed, `condvar.notify_one()` to wake consumer — in src/multi_threaded_lister.rs
  📖 Port from: `MultiThreadedLister.cpp` → `WorkerThread()` completion + SetEvent signaling
- [ ] T068 [US4] Implement shutdown sequence — set AtomicBool stop flag, call work_queue.set_done(), join all worker threads, consumer's WaitForNodeCompletion checks stop flag in CV predicate — in src/multi_threaded_lister.rs
  📖 Port from: `MultiThreadedLister.cpp` → `Stop()` + `~CMultiThreadedLister()`

### Streaming Consumer

- [ ] T069 [US4] Implement `WaitForNodeCompletion()` — acquire per-node mutex, `condvar.wait_while(lock, |state| state.status < Done && !stop_requested)`, return status — in src/directory_lister.rs
  📖 Port from: `DirectoryLister.cpp` → `WaitForDirectoryCompletion()` (WaitForSingleObject → Condvar)
- [ ] T070 [US4] Implement `print_directory_tree()` — recursive depth-first walk: wait for node completion, sort results, display results, accumulate totals into ListingTotals, recurse into children in discovery order (never skip or reorder) per A.15.5 — in src/directory_lister.rs
  📖 Port from: `DirectoryLister.cpp` → `PrintDirectoryTree()`
- [ ] T071 [US4] Implement per-node error handling in tree walk — if node status is Error, print error message and continue to next sibling (don't abort tree walk) per A.15.8 — in src/directory_lister.rs
  📖 Port from: `DirectoryLister.cpp` → error handling within `PrintDirectoryTree()`

### Summary and Wiring

- [ ] T072 [US4] Implement recursive summary display — "Total Files Listed" footer with accumulated ListingTotals (total files, total bytes, total dirs) with locale-formatted numbers — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerWithHeaderAndFooter.cpp` → `DisplayRecursiveSummary()`
- [ ] T073 [US4] Wire recursive listing path into lib::run() — if `/s`: create root DirectoryInfo, enqueue root, start MultiThreadedLister, call print_directory_tree, print recursive summary, stop workers, flush — in src/lib.rs
  📖 Port from: `TCDir.cpp` → `wmain()` recursive-mode branch

**Checkpoint**: Recursive listing produces correct depth-first output; output appears progressively (streaming); MT gives measurable speedup; error in one dir doesn't abort the rest

---

## Phase 9: User Story 5 — Wide Listing Format (Priority: P2)

**Goal**: Compact multi-column display with `/w` switch

**Independent Test**: `cargo run -- /w` displays files in multiple columns; directories show as `[dirname]`

- [ ] T074 [US5] Implement `WideDisplayer` column calculation — compute column count from console width and longest filename, column-major fill order (items flow down columns, not across rows) per A.4.3 — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerWide.h`, `ResultsDisplayerWide.cpp` → column count + layout logic
- [ ] T075 [US5] Implement `WideDisplayer` output — bracket directory names `[dirname]`, colorize filenames, pad to column width, handle last-row partial fill — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerWide.cpp` → `DisplayFileEntry()` + `DisplayResults()`

**Checkpoint**: Wide listing fills columns correctly; directory brackets display; column count adapts to terminal width

---

## Phase 10: User Story 6 — Bare Listing Format (Priority: P2)

**Goal**: Filenames-only output with `/b` for scripting and pipes

**Independent Test**: `cargo run -- /b` outputs one filename per line with no headers, footers, or decorations

- [ ] T076 [US6] Implement `BareDisplayer` — filenames only (one per line), no headers/footers/volume info/cloud symbols/summaries per A.4.4 — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerBare.h`, `ResultsDisplayerBare.cpp` → `DisplayFileEntry()`
- [ ] T077 [US6] Implement bare recursive output — full paths in recursive mode (`/s`), path constructed from DirectoryInfo.dir_path + filename — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerBare.cpp` → `DisplayResults()` recursive path construction

**Checkpoint**: Bare output is clean for piping; recursive bare shows full paths; no extra decoration

---

## Phase 11: User Story 7 — Time Field Selection (Priority: P2)

**Goal**: Select which timestamp to display and sort by with `/t:` switch

**Independent Test**: `cargo run -- /t:c` shows creation times; `cargo run -- /t:c /o:d` sorts by creation time

- [ ] T078 [US7] Implement time field selection plumbing — pass `time_field` from CommandLine through to NormalDisplayer (selects which FILETIME for date/time columns) and FileComparator (selects which FILETIME for date sort key) — in src/directory_lister.rs
  📖 Port from: `CommandLine.h` (ETimeField enum), `ResultsDisplayerNormal.cpp` + `FileComparator.cpp` (time field dispatch)

**Checkpoint**: All three time fields display correctly; sort-by-date respects selected time field

---

## Phase 12: User Story 8 — Cloud Sync Status Visualization (Priority: P2)

**Goal**: Display cloud sync status symbols (○/◐/●) for OneDrive/iCloud files

**Independent Test**: `cargo run` in a OneDrive folder shows cloud status symbols; `cargo run -- /a:o` filters to cloud-only

- [ ] T079 [US8] Implement cloud sync root detection — `CfGetSyncRootInfoByPath` returns success if path is under a cloud provider (OneDrive, iCloud), cache result per drive/root — in src/cloud_status.rs
  📖 Port from: `DirectoryLister.cpp` → `IsCloudSyncRoot()` / `CfGetSyncRootInfoByPath` call
- [ ] T080 [US8] Implement per-file cloud placeholder state — `CfGetPlaceholderStateFromAttributeTag(dwFileAttributes, dwReserved0)` → map state to symbol: cloud-only → ○, local → ◐, pinned → ● — in src/cloud_status.rs
  📖 Port from: `DirectoryLister.cpp` → `GetCloudPlaceholderState()`, `UnicodeSymbols.h` (cloud symbols)
- [ ] T081 [US8] Integrate cloud status into NormalDisplayer — display cloud symbol with configured color after attribute column, before filename; suppress in bare mode and when not in a cloud sync root — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerNormal.cpp` → cloud symbol display within `DisplayFileEntry()`

**Checkpoint**: Cloud symbols appear for OneDrive/iCloud folders; non-cloud folders show no symbols; bare mode suppresses symbols

---

## Phase 13: User Story 9 — Color Configuration via Environment Variable (Priority: P2)

**Goal**: Parse RCDIR environment variable for color overrides and switch defaults per A.5 grammar

**Independent Test**: Set `RCDIR=W;D=LightGreen;.rs=Cyan` → wide mode default, green dates, cyan .rs files

- [ ] T082 [US9] Implement RCDIR env var top-level parser — read via EnvironmentProvider, split on semicolons, classify each entry: switch name (with optional `-`), display item color (`key=ColorSpec`), extension color (`.ext=ColorSpec`), attribute color (`Attr:X=ColorSpec`) — in src/config.rs
  📖 Port from: `Config.cpp` → `CConfig::ParseEnvironmentVariable()` outer loop
- [ ] T083 [US9] Implement switch default parsing — recognize valid switch names (W/S/P/M/B/Owner/Streams) with optional trailing `-` to disable, reject entries with switch prefixes (`/`, `-`, `--`), set Config Option fields — in src/config.rs
  📖 Port from: `Config.cpp` → `CConfig::ParseEnvironmentVariable()` switch-name handling branch
- [ ] T084 [US9] Implement display item color override parsing — map key names to Attribute enum variants (D=Date, T=Time, etc. per A.2.4), call parse_color_spec for value, update attributes array and set source to Environment — in src/config.rs
  📖 Port from: `Config.cpp` → `CConfig::ParseEnvironmentVariable()` display-item color branch
- [ ] T085 [US9] Implement extension color override parsing — entries starting with `.`, call parse_color_spec for value, update extension_colors HashMap with case-insensitive extension key — in src/config.rs
  📖 Port from: `Config.cpp` → `CConfig::ParseEnvironmentVariable()` extension color branch
- [ ] T086 [US9] Implement file attribute color override parsing — entries starting with `Attr:` followed by attribute char (H/S/E/C etc.), call parse_color_spec for value, update file_attr_colors HashMap — in src/config.rs
  📖 Port from: `Config.cpp` → `CConfig::ParseEnvironmentVariable()` attribute color branch
- [ ] T087 [US9] Implement env var validation and error collection — accumulate `ErrorInfo` structs for each invalid entry (invalid color name, invalid attribute key, unknown switch, invalid prefix), store for later display — in src/config.rs
  📖 Port from: `Config.cpp` → `CConfig::ParseEnvironmentVariable()` error accumulation + ErrorInfo struct
- [ ] T088 [US9] Implement env var error display — at end of normal output, print each error with: the original RCDIR value, an underline annotation pointing to the exact invalid text position, and error description — in src/lib.rs
  📖 Port from: `Usage.cpp` → `CUsage::DisplayEnvVarIssues()` + `DisplayEnvVarCurrentValue()`
- [ ] T089 [US9] Implement `--env` dynamic content — current RCDIR value display (D.2.4), decoded settings display grouped by switches/items/attrs/extensions (D.2.5), color chart (D.2.2) — in src/lib.rs
  📖 Port from: `Usage.cpp` → `CUsage::DisplayEnvVarDecodedSettings()`, `DisplayEnvVarCurrentValue()`

**Checkpoint**: Color overrides work; switch defaults work; syntax errors display with underline annotations; `--env` shows complete reference

---

## Phase 14: User Story 10 — Configuration Display (Priority: P3)

**Goal**: Show current color configuration with `--config` switch

**Independent Test**: `cargo run -- --config` displays all colors with sources (Default vs Environment)

- [ ] T090 [US10] Implement `--config` display item listing — for each Attribute enum variant, show name, color swatch (colored sample text), hex WORD value, source (Default or Environment) — in src/lib.rs
  📖 Port from: `Usage.cpp` → `CUsage::DisplayCurrentConfiguration()`, `DisplayConfigurationTable()`, `DisplayAttributeConfiguration()`
- [ ] T091 [US10] Implement `--config` extension and attribute color listing — show all extension color overrides (source-tagged), all file attribute color overrides (source-tagged) — in src/lib.rs
  📖 Port from: `Usage.cpp` → `DisplayExtensionConfiguration()`, `DisplayFileAttributeConfiguration()`, `DisplayColorConfiguration()`

**Checkpoint**: Config display shows all settings; source tracking distinguishes defaults from env var overrides

---

## Phase 15: User Story 11 — File Ownership Display (Priority: P3)

**Goal**: Show file owner with `--owner` switch

**Independent Test**: `cargo run -- --owner` displays `DOMAIN\User` for each file

- [ ] T092 [US11] Implement file owner lookup — `GetNamedSecurityInfoW` to get security descriptor, `GetSecurityDescriptorOwner` to get SID, `LookupAccountSidW` to get `DOMAIN\User` string — in src/owner.rs
  📖 Port from: `DirectoryLister.cpp` → `GetFileOwner()` (security descriptor + SID lookup)
- [ ] T093 [US11] Integrate owner column into NormalDisplayer — display owner string between cloud status and filename, pad to consistent column width for alignment — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerNormal.cpp` → owner column within `DisplayFileEntry()`

**Checkpoint**: Owner column displays correct `DOMAIN\User`; column alignment is preserved

---

## Phase 16: User Story 12 — Alternate Data Streams Display (Priority: P3)

**Goal**: Show NTFS alternate data streams with `--streams` switch

**Independent Test**: `cargo run -- --streams` on a file with ADS shows stream names and sizes below each file

- [ ] T094 [US12] Implement stream enumeration — `FindFirstStreamW`/`FindNextStreamW` loop, construct StreamInfo (strip `:$DATA` suffix from name, extract size), only on NTFS/ReFS volumes — in src/streams.rs
  📖 Port from: `DirectoryLister.cpp` → `EnumerateStreams()` (FindFirstStreamW/FindNextStreamW loop)
- [ ] T095 [US12] Integrate streams into NormalDisplayer — display each stream below parent file (indented, showing stream name and locale-formatted size per A.9.2), accumulate stream counts/bytes in ListingTotals — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerNormal.cpp` → stream display within `DisplayFileEntry()`

**Checkpoint**: Streams display below files; stream totals included in summaries; only on NTFS/ReFS volumes

---

## Phase 17: User Story 14 — Multi-Threading Control (Priority: P3)

**Goal**: Toggle multi-threading with `/m` and `/m-` switches

**Independent Test**: `cargo run -- /s /m-` runs single-threaded recursive listing

- [ ] T096 [US14] Implement single-threaded fallback path — when `multi_threaded == false`, enumerate directories sequentially on main thread: BFS or DFS enumeration without worker threads, same output ordering and totals — in src/directory_lister.rs
  📖 Port from: `DirectoryLister.cpp` → `ListDirectoryRecursive()` single-threaded path

**Checkpoint**: `/m-` produces identical output to multi-threaded mode; single-threaded is measurably slower on large trees

---

## Phase 18: User Story 16 — Debug Attribute Display (Priority: P4)

**Goal**: Show raw hex attributes with `--debug` switch (debug builds only)

**Independent Test**: `cargo run -- --debug` (debug build) shows `[XXXXXXXX:YY]` before each filename

- [ ] T097 [US16] Implement debug attribute display — `[{:08X}:{:02X}]` format showing file attributes (8 hex digits) and cloud placeholder state (2 hex digits), gated behind `#[cfg(debug_assertions)]`, positioned after cloud status and before owner/filename per A.17 — in src/results_displayer.rs
  📖 Port from: `ResultsDisplayerNormal.cpp` → debug attribute display block (`#ifdef _DEBUG`)

**Checkpoint**: Debug builds show hex attributes; release builds have no debug output; format matches A.17

---

## Phase 19: Polish & Cross-Cutting Concerns

**Purpose**: Output parity validation, lint cleanup, final quality checks

- [ ] T098 [P] Create output parity integration test framework — build both rcdir and tcdir, capture output for representative scenarios, compare line-by-line — in tests/output_parity/mod.rs
- [ ] T099 [P] Add output parity test cases — single dir, recursive, sorted (/o:s, /o:-d), filtered (/a:d, /a:-hs), wide, bare, combined switches — in tests/output_parity/
- [ ] T100 Run `cargo clippy` and fix all warnings across all modules
- [ ] T101 Run quickstart.md validation — build both architectures (x64+ARM64), run tests, deploy to test path per quickstart.md
- [ ] T101a Validate performance criteria — benchmark typical directory (<1s per SC-001), large recursive listing (<10s per SC-002), verify MT gives 2x+ speedup over `/m-` (SC-010) using `/p` switch
- [ ] T102 Final review — verify byte-identical output for representative test cases against TCDir, check column alignment, number formatting, color accuracy

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — pop stash and configure
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **US-13 (Phase 3)**: Depends on Foundational — first story per plan.md mandate
- **US-15 (Phase 4)**: Depends on Foundational — needed for invalid-switch error handling
- **US-1 (Phase 5)**: Depends on Foundational — the core listing engine
- **US-2 (Phase 6)**: Depends on US-1 (sorts enumerated files)
- **US-3 (Phase 7)**: Depends on US-1 (filters during enumeration)
- **US-4 (Phase 8)**: Depends on US-1 + US-2 + US-3 (recursive needs full single-dir pipeline)
- **US-5 (Phase 9)**: Depends on US-1 (alternate displayer)
- **US-6 (Phase 10)**: Depends on US-1 (alternate displayer)
- **US-7 (Phase 11)**: Depends on US-1 + US-2 (time field affects display + sort)
- **US-8 (Phase 12)**: Depends on US-1 (cloud symbols in display)
- **US-9 (Phase 13)**: Depends on US-1 (color overrides affect display)
- **US-10 (Phase 14)**: Depends on US-9 (displays env var–sourced config)
- **US-11 (Phase 15)**: Depends on US-1 (owner column in NormalDisplayer)
- **US-12 (Phase 16)**: Depends on US-1 (streams below files in NormalDisplayer)
- **US-14 (Phase 17)**: Depends on US-4 (needs MT infrastructure to toggle)
- **US-16 (Phase 18)**: Depends on US-1 (debug display in NormalDisplayer)
- **Polish (Phase 19)**: Depends on all desired stories being complete

### User Story Independence

After Foundational completes, the following groups can proceed independently:

| Group | Stories | Prerequisite |
|-------|---------|-------------|
| Timer + Help | US-13, US-15 | Foundational only |
| Core Pipeline | US-1 → US-2 → US-3 → US-4 | Sequential chain |
| Display Variants | US-5, US-6 | US-1 |
| Extended Features | US-7, US-8, US-11, US-12, US-16 | US-1 |
| Configuration | US-9 → US-10 | US-1 |
| Threading Control | US-14 | US-4 |

### Parallel Opportunities Within Phases

**Phase 2** (Foundational): T005 + T006 + T007 + T008 can run in parallel (independent leaf modules); T026 + T027 can run in parallel (independent color tables)

**Phase 5** (US-1): T036–T041 can all run in parallel (independent struct definitions); T042+T043 sequential (DriveInfo base then UNC); T047–T054 are mostly sequential within NormalDisplayer

**Phase 8** (US-4): T062 + T063 can run in parallel (DirectoryInfo extension vs WorkQueue)

---

## Parallel Example: Phase 5 (US-1 Core Listing)

```
# Batch 1 — Launch all independent data structures together:
T036: FileInfo struct in src/file_info.rs
T037: FindHandle/SafeHandle RAII wrappers in src/file_info.rs
T038: FileAttributeMap constants in src/file_info.rs
T039: ListingTotals struct in src/listing_totals.rs
T040: DirectoryInfo struct in src/directory_info.rs
T041: MaskGrouper in src/mask_grouper.rs

# Batch 2 — Drive info (needs file_info types):
T042: DriveInfo base struct in src/drive_info.rs
T043: DriveInfo UNC handling in src/drive_info.rs

# Batch 3 — Enumeration (needs DirectoryInfo + FileInfo):
T044: Core enumeration loop in src/directory_lister.rs
T045: File spec matching in src/directory_lister.rs

# Batch 4 — Display infrastructure:
T046: ResultsDisplayer trait in src/results_displayer.rs

# Batch 5 — NormalDisplayer formatting (sequential):
T047: Volume header
T048: Date/time formatting helpers
T049: Size formatting helper
T050: Attribute column formatting
T051: Filename colorization
T052: Per-file line assembly
T053: Directory footer
T054: Separator lines

# Batch 6 — Wire it all together:
T055: lib::run() orchestration
```

---

## Implementation Strategy

### MVP First (Phases 1–5: Setup → Foundational → US-13 → US-15 → US-1)

1. Complete Phase 1: Setup (3 tasks)
2. Complete Phase 2: Foundational (25 tasks — CRITICAL, blocks all stories)
3. Complete Phase 3: US-13 Performance Timer (2 tasks)
4. Complete Phase 4: US-15 Help Display (5 tasks)
5. Complete Phase 5: US-1 Core Listing (20 tasks)
6. **STOP and VALIDATE**: Compare output against TCDir for single-directory listing
7. **MVP total: 55 tasks** → a working colored directory listing with timer, help, and full formatting

### Incremental Delivery (P1 → P2 → P3 → P4)

1. **P1 complete** (Phases 1–8, 77 tasks): Full-featured single + recursive listing with sort, filter, MT
2. **P2 complete** (Phases 9–13, 16 tasks): Wide/bare modes, time field, cloud status, color config
3. **P3 complete** (Phases 14–17, 6 tasks): Config display, owner, streams, MT control
4. **P4 complete** (Phase 18, 1 task): Debug attribute display
5. **Polish** (Phase 19, 5 tasks): Output parity validation, lint, cross-arch builds

### Port Fidelity Checkpoints

After each P1 story, run side-by-side comparison:
```powershell
tcdir /s /o:s | Out-File tcdir-output.txt
rcdir /s /o:s | Out-File rcdir-output.txt
Compare-Object (Get-Content tcdir-output.txt) (Get-Content rcdir-output.txt)
```

---

## Notes

- **[P]** tasks can run in parallel (different files, no dependencies on incomplete tasks in same phase)
- **[USn]** label maps each task to its user story for traceability
- TCDir source under `TCDir/TCDirCore/` is the authoritative reference — **never modify**
- Commit after each task or logical group
- Run `cargo clippy` frequently to catch issues early
- Total external crates: 2 (`windows` v0.62, `widestring`)
