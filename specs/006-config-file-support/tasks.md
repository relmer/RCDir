# Tasks: Config File Support

**Input**: Design documents from `/specs/006-config-file-support/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md

**Tests**: Included — the constitution requires test coverage for all production code (Principle II: Testing Discipline).

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Create new source files and add module declarations

- [X] T001 Create `src/config/file_reader.rs` with module stub and `ConfigFileError` enum
- [X] T002 Add `pub mod file_reader;` to `src/config/mod.rs`
- [X] T003 Build and verify no compilation errors

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure changes that all user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T004 Extend `AttributeSource` enum with `ConfigFile` value in `src/config/mod.rs`
- [X] T005 Extend `ErrorInfo` struct with `source_file_path: String` and `line_number: usize` fields in `src/config/mod.rs`
- [X] T006 Add source parameter to `process_color_override_entry` and thread through all downstream methods that write to source maps in `src/config/env_overrides.rs`
- [X] T007 Update `apply_user_color_overrides` to pass `AttributeSource::Environment` as source parameter in `src/config/env_overrides.rs`
- [X] T008 Add config file fields to `Config`: `config_file_path`, `config_file_loaded`, `config_file_parse_result` in `src/config/mod.rs`
- [X] T009 Add switch/parameter source tracking fields: `switch_sources`, `max_depth_source`, `tree_indent_source`, `size_format_source` in `src/config/mod.rs`
- [X] T010 Add public methods to `Config`: `load_config_file`, `validate_config_file`, `config_file_path`, `is_config_file_loaded` in `src/config/mod.rs`
- [X] T011 Update existing config tests to verify source parameter threading does not break current env var override tests in `src/config/mod.rs`
- [X] T012 Build and run all existing tests to verify no regressions

**Checkpoint**: Foundation ready — source tracking supports 3 values, ErrorInfo has line numbers, process_color_override_entry accepts source parameter

---

## Phase 3: User Story 1 — Basic Config File Loading (Priority: P1) 🎯 MVP

**Goal**: Users can place settings in `%USERPROFILE%\.rcdirconfig` and have them applied on every run

**Independent Test**: Create a config file with switches and color overrides, run rcdir, verify settings are applied

### Implementation

- [X] T013 Implement `check_and_strip_bom` in `src/config/file_reader.rs` — UTF-8 BOM stripped, UTF-16 LE/BE BOM rejected with error
- [X] T014 Implement `read_config_file` in `src/config/file_reader.rs` — `fs::read`, BOM check, UTF-8 conversion, line splitting
- [X] T015 Implement `Config::load_config_file` in `src/config/mod.rs` — resolve path via USERPROFILE env var, call `read_config_file`, then `process_config_lines`
- [X] T016 Implement `Config::process_config_lines` in `src/config/mod.rs` — trim whitespace, skip blanks/comments, strip inline comments, pass entries to `process_color_override_entry` with `ConfigFile` source, tag errors with line numbers
- [X] T017 Insert `load_config_file` call into `Config::initialize` between default initialization and `apply_user_color_overrides` in `src/config/mod.rs`
- [X] T018 [P] Write file reader unit tests in `src/config/file_reader.rs` — UTF-8 BOM stripped, UTF-16 BOM rejected, empty file, line splitting (CRLF, LF, CR), file not found returns `NotFound`
- [X] T019 Write config file loading unit tests in `src/config/mod.rs` — switches applied, color overrides applied, icon overrides applied, parameterized values applied
- [X] T020 Write comment and blank line unit tests in `src/config/mod.rs` — comment lines skipped, inline comments stripped, blank lines skipped, whitespace-only lines skipped
- [X] T021 Build and run all tests

**Checkpoint**: Config file loads and applies settings. No env var interaction tested yet.

---

## Phase 4: User Story 2 — Environment Variable Overrides Config File (Priority: P1)

**Goal**: RCDIR env var settings take precedence over config file for conflicting keys; non-conflicting settings merge

**Independent Test**: Set `.cpp=LightGreen` in config file and `.cpp=Yellow` in env var, verify Yellow wins

### Implementation

- [X] T022 [US2] Write precedence unit tests in `src/config/mod.rs` — env var overrides config file color, env var overrides config file switch, non-conflicting settings merge from both sources
- [X] T023 [US2] Write source tracking unit tests in `src/config/mod.rs` — verify `AttributeSource::ConfigFile` for config-only settings, `AttributeSource::Environment` for env-var-overridden settings
- [X] T024 [US2] Build and run all tests

**Checkpoint**: Precedence model verified. Config file + env var merge correctly.

---

## Phase 5: User Story 3 — Readable Multi-Line Format (Priority: P1)

**Goal**: Config file supports comments, blank lines, and per-line settings for readable organization

**Independent Test**: Create a config file with comment headers, grouped settings, inline comments — verify all parsed correctly

### Implementation

- [X] T025 [US3] Write inline comment edge case tests in `src/config/mod.rs` — setting with inline comment, setting with multiple # characters, comment-only lines with leading whitespace
- [X] T026 [US3] Write whitespace handling tests in `src/config/mod.rs` — leading/trailing whitespace trimmed, whitespace around = in key=value, tabs as whitespace
- [X] T027 [US3] Write duplicate setting tests in `src/config/mod.rs` — last occurrence wins within config file
- [X] T028 [US3] Build and run all tests

**Checkpoint**: All format rules validated. Stories 1-3 form a complete, testable MVP.

---

## Phase 6: User Story 4 — Config File Error Reporting (Priority: P2)

**Goal**: Parse errors include file path and line number; errors shown on every run; config file and env var errors grouped separately

**Independent Test**: Place an invalid color name in config file, verify error message shows file path and line number

### Implementation

- [X] T029 [US4] Implement `display_config_file_issues` in `src/usage.rs` — render config file errors with line numbers, using existing underline pattern, accept `show_hint: bool`
- [X] T030 [US4] Update end-of-run error display in `src/lib.rs` `finalize()` — call `display_config_file_issues` before `display_env_var_issues`, skip group if no errors
- [X] T031 [US4] Implement file-level I/O error reporting in `Config::load_config_file` — single `ErrorInfo` for open/read/encoding failures in `src/config/mod.rs`
- [X] T032 [US4] Write error reporting unit tests in `src/config/mod.rs` — invalid color name shows line number, malformed entry shows line number, valid lines still apply alongside errors
- [X] T033 [US4] Write error grouping tests in `src/config/mod.rs` — config file errors separate from env var errors, config file errors listed first, verify `show_hint=true` includes `(see --config for help)` text and `show_hint=false` omits it
- [X] T034 [US4] Write I/O error tests in `src/config/file_reader.rs` — file not found returns `NotFound`, permission error returns `IoError`
- [X] T035 [US4] Build and run all tests

**Checkpoint**: Error reporting complete with line numbers and grouped display.

---

## Phase 7: User Story 5 — Diagnostic Command Restructuring (Priority: P2)

**Goal**: `--config` shows config file diagnostics; `--settings` shows merged configuration tables with 3-source column; `--env` unchanged

**Independent Test**: Set up config file + env var, run each of `--config`, `--settings`, `--env` and verify expected output scope

### Implementation

- [X] T036 [US5] Add `show_settings: bool` to `CommandLine` in `src/command_line.rs`
- [X] T037 [US5] Add `"settings"` to long switch table in `handle_long_switch` in `src/command_line.rs`
- [X] T038 [US5] Add `"settings"` to informational switch validation (mutual exclusion) in `src/command_line.rs`
- [X] T039 [US5] Implement `display_config_file_help` in `src/usage.rs` — config file syntax reference, color/icon format reference, example config file, env var override note, file path with load status, config file parse errors
- [X] T040 [US5] Repurpose `--config` handler in `src/lib.rs` `process_info_switches()` — call `display_config_file_help` instead of `display_current_configuration`
- [X] T041 [US5] Rename `display_current_configuration` to `display_settings` in `src/usage.rs`
- [X] T042 [US5] Update `display_settings` source column to render three values: Default, Config file, Environment in `src/usage.rs`
- [X] T043 [US5] Add `--settings` handler in `src/lib.rs` `process_info_switches()` — call `display_settings` when `show_settings` is set
- [X] T044 [US5] Update `display_settings` to show both config file and env var errors at the bottom in `src/usage.rs`
- [X] T045 [US5] Update `display_settings` to show "No config file or RCDIR..." message when neither source set in `src/usage.rs`
- [X] T046 [US5] Update `--help` text in `display_usage` to reflect new `--config` description and new `--settings` command in `src/usage.rs`
- [X] T047 [US5] Write `--settings` switch parsing test in `src/command_line.rs`
- [X] T048 [US5] Build and run all tests

**Checkpoint**: All three diagnostic commands work correctly.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Final validation and edge case hardening

- [X] T049 [P] Write edge case tests in `src/config/mod.rs` — empty file, file with only comments, file with only blank lines, 20+ settings file (SC-002 coverage), USERPROFILE not set (silent skip)
- [X] T050 [P] Write BOM edge case tests in `src/config/file_reader.rs` — UTF-16 LE BOM rejected with clear error, UTF-16 BE BOM rejected with clear error
- [X] T051 Verify config file does not exist scenario — no error, no warning, defaults used
- [X] T052 Run quickstart.md validation — create config file, verify all commands work as documented
- [X] T053 Full build (`cargo build`, `cargo clippy -- -D warnings`) and run all tests (`cargo test`) on all configurations

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Phase 2 — the MVP
- **US2 (Phase 4)**: Depends on Phase 3 (needs loading to work to test precedence)
- **US3 (Phase 5)**: Depends on Phase 3 (needs loading to work to test format rules)
- **US4 (Phase 6)**: Depends on Phase 3 (needs loading to work to test error reporting)
- **US5 (Phase 7)**: Depends on Phase 6 (needs error model complete for display)
- **Polish (Phase 8)**: Depends on all prior phases

### Within Each Phase

- Tasks marked [P] can run in parallel
- Tests alongside implementation
- Build verification at end of each phase

### Parallel Opportunities

After Phase 2 (Foundational) completes:
- US2 (Phase 4) and US3 (Phase 5) can run in parallel after US1 (Phase 3) completes
- US4 (Phase 6) can run in parallel with US2/US3 after US1 completes

---

## Implementation Strategy

### MVP First (User Stories 1-3)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: User Story 1 — Basic Loading
4. Complete Phase 4: User Story 2 — Precedence
5. Complete Phase 5: User Story 3 — Format Rules
6. **STOP and VALIDATE**: Test end-to-end with a real `.rcdirconfig` file
7. Continue to Phase 6-8 for error reporting and diagnostics

### Incremental Delivery

- After Phase 3: Config file works, no error diagnostics yet
- After Phase 5: Full MVP — loading, precedence, format all validated
- After Phase 7: Complete feature — all diagnostics and commands
- After Phase 8: Production-ready — all edge cases validated
