# Tasks: PowerShell Alias Configuration

**Input**: Design documents from `/specs/005-powershell-aliases/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md

**Tests**: Included — the spec requires unit tests (Constitution Principle II: Testing Discipline).

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project scaffolding, new headers in Cargo.toml, new source files added to project

- [X] T001 Add `windows` crate features for `Win32_System_Diagnostics_ToolHelp` and `Win32_UI_Shell` to `Cargo.toml`
- [X] T002 Add new switch fields (`set_aliases`, `get_aliases`, `remove_aliases`, `what_if`) to `CommandLine` struct in `src/command_line.rs`
- [X] T003 Add long switch entries for `set-aliases`, `get-aliases`, `remove-aliases`, `whatif` to switch parsing in `src/command_line.rs`
- [X] T004 Add mutual exclusivity validation for alias switches and `--whatif` in switch validation logic in `src/command_line.rs`
- [X] T005 [P] Add data model structs and enums (`PowerShellVersion`, `ProfileScope`, `ProfileLocation`, `AliasDefinition`, `AliasConfig`, `AliasBlock`) to `src/alias_types.rs` and declare the module in `src/lib.rs`. All alias modules import shared types from here.
- [X] T006 [P] Create empty `src/profile_path_resolver.rs` with struct skeleton and module declaration in `src/lib.rs`
- [X] T007 [P] Create empty `src/profile_file_manager.rs` with struct skeleton and module declaration in `src/lib.rs`
- [X] T008 [P] Create empty `src/alias_block_generator.rs` with struct skeleton and module declaration in `src/lib.rs`
- [X] T009 [P] Create empty `src/tui_widgets.rs` with struct skeleton and module declaration in `src/lib.rs`
- [X] T010 [P] Create empty `src/alias_manager.rs` with module-level function signatures (`set_aliases`, `get_aliases`, `remove_aliases`, `run`) and module declaration in `src/lib.rs`
- [X] T011 Add unit tests for new command-line switches (parse, validate mutual exclusivity, `--whatif` without alias switch errors) in `src/command_line.rs` `#[cfg(test)]` module
- [X] T012 Build and verify all tests pass

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure modules that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### ProfilePathResolver (Layer 0)

- [X] T013 Implement parent process detection: `CreateToolhelp32Snapshot` → find parent PID → `OpenProcess` → `QueryFullProcessImageNameW` → extract exe name → return `PowerShellVersion` in `src/profile_path_resolver.rs`
- [X] T014 Implement profile path resolution: `SHGetKnownFolderPath(FOLDERID_Documents)` for per-user paths, parent process directory (`$PSHOME`) for all-users paths, build 4 `ProfileLocation` structs for detected PS version in `src/profile_path_resolver.rs`
- [X] T015 Implement admin privilege detection for AllUsers scopes (check write access or token elevation) in `src/profile_path_resolver.rs`
- [X] T016 [P] Add unit tests in `src/profile_path_resolver.rs` `#[cfg(test)]` module: test path construction for PS7+ and PS5.1, test admin detection, test Unknown parent handling

### AliasBlockGenerator (Layer 0)

- [X] T017 Implement `Generate()`: given `AliasConfig`, produce complete alias block string with opening/closing markers (FR-040), version comment (FR-044), root function (FR-042), and sub-alias functions (FR-043) in `src/alias_block_generator.rs`
- [X] T018 Implement rcdir invocation resolution: `GetModuleFileNameW` for exe path, `SearchPathW` to check PATH reachability, set `AliasConfig::rcdir_invocation` accordingly (FR-041) in `src/alias_manager.rs` (result passed to generator via `AliasConfig`)
- [X] T019 [P] Add unit tests in `src/alias_block_generator.rs` `#[cfg(test)]` module: test generated block format, marker comments, version stamp, root alias variations, sub-alias toggling, short-name vs full-path invocation

### ProfileFileManager (Layer 1)

- [X] T020 Implement `ReadProfileFile()`: read UTF-8 file (with/without BOM) into `Vec<String>` lines in `src/profile_file_manager.rs`
- [X] T021 Implement `FindAliasBlock()`: scan lines for opening/closing markers, return `AliasBlock` with line indices and parsed alias names in `src/profile_file_manager.rs`
- [X] T022 Implement `WriteProfileFile()`: create timestamped `.bak` backup (FR-070), write lines back preserving encoding, create parent directories if needed (FR-072) in `src/profile_file_manager.rs`
- [X] T023 Implement `ReplaceAliasBlock()`: remove lines[start..end], insert new block at same position in `src/profile_file_manager.rs`
- [X] T024 Implement `AppendAliasBlock()`: append block at end of file in `src/profile_file_manager.rs`
- [X] T025 Implement `RemoveAliasBlock()`: delete lines[start..end] inclusive (FR-054, FR-055) in `src/profile_file_manager.rs`
- [X] T026 [P] Add unit tests in `src/profile_file_manager.rs` `#[cfg(test)]` module: test read/write round-trip, BOM preservation, marker detection, backup creation, block replace/append/remove, error on locked file

### TuiWidgets (Layer 2)

- [X] T027 Implement `TuiGuard` struct with `Drop` impl: constructor saves console mode and cursor visibility, sets raw input mode, flushes input buffer; `Drop` restores original state in `src/tui_widgets.rs`
- [X] T028 Implement `TextInput` widget: prompt with default value, accept 1-4 alphanumeric chars, Enter confirms, Escape cancels (FR-020, FR-021) in `src/tui_widgets.rs`
- [X] T029 Implement `CheckboxList` widget: render `[✓]`/`[ ]` items with `❯` focus indicator, arrow keys navigate, Space toggles, Enter confirms, Escape cancels (FR-012, FR-013, FR-022, FR-024) in `src/tui_widgets.rs`
- [X] T030 Implement `RadioButtonList` widget: render `(●)`/`( )` items with `❯` focus indicator, arrow keys navigate, Enter selects, Escape cancels (FR-011, FR-013, FR-025) in `src/tui_widgets.rs`
- [X] T031 Implement `ConfirmationPrompt` widget: display preview text, Y/N prompt, Enter confirms, Escape cancels (FR-029) in `src/tui_widgets.rs`
- [X] T032 [P] Add unit tests in `src/tui_widgets.rs` `#[cfg(test)]` module: test widget state transitions with simulated key input sequences, test Escape cancellation, test cursor visibility restore

- [X] T033 Build and verify all foundational tests pass

**Checkpoint**: Foundation ready — user story implementation can now begin

---

## Phase 3: User Story 1 — First-Time Alias Setup (Priority: P1) 🎯 MVP

**Goal**: User runs `rcdir --set-aliases`, completes the wizard, and working aliases appear in their chosen profile.

**Independent Test**: Run `rcdir --set-aliases` with no existing aliases. Complete wizard, reload profile, verify aliases work.

- [X] T034 [US1] Implement `set_aliases()` orchestration in `src/alias_manager.rs`: detect PS version → resolve paths → scan for existing blocks → run TUI wizard (root alias → sub-aliases → profile location → preview) → generate block → write to profile
- [X] T035 [US1] Wire TUI wizard steps: call `TextInput` for root alias, recalculate sub-alias names, call `CheckboxList` for sub-aliases, call `RadioButtonList` for profile location (with admin markers per FR-028), call `ConfirmationPrompt` for preview in `src/alias_manager.rs`
- [X] T036 [US1] Handle "session only" storage option (FR-027): if selected, output alias block to console with instructions to paste, skip file write in `src/alias_manager.rs`
- [X] T037 [US1] Handle existing alias detection (FR-030): if marker block found during scan, inform user with current aliases and offer to replace in `src/alias_manager.rs`
- [X] T038 [US1] Add dispatch in `main.rs`: if `set_aliases` is set, call `alias_manager::set_aliases()` and return before directory listing
- [X] T039 [US1] Add `--set-aliases` help text to `src/usage.rs`
- [X] T040 [P] [US1] Add unit tests in `src/alias_manager.rs` `#[cfg(test)]` module: test set_aliases flow end-to-end with mocked file system (new profile, existing profile with block, session-only mode)
- [X] T041 [US1] Build and verify all US1 tests pass

**Checkpoint**: User Story 1 complete — first-time setup works end-to-end

---

## Phase 4: User Story 2 — View Current Aliases (Priority: P1)

**Goal**: User runs `rcdir --get-aliases` and sees a formatted summary of all rcdir aliases in profile files.

**Independent Test**: Set up aliases in a profile, run `rcdir --get-aliases`, verify output shows alias names, mappings, and source locations.

- [X] T042 [US2] Implement `get_aliases()` in `src/alias_manager.rs`: detect PS version → resolve paths → scan all 4 profiles for marker blocks → format and display results grouped by profile (FR-060, FR-061, FR-062)
- [X] T043 [US2] Handle "no aliases found" case: display message suggesting `--set-aliases` (FR-062, spec US2 scenario 2) in `src/alias_manager.rs`
- [X] T044 [US2] Add dispatch in `main.rs`: if `get_aliases` is set, call `alias_manager::get_aliases()` and return
- [X] T045 [US2] Add `--get-aliases` help text to `src/usage.rs`
- [X] T046 [US2] Add get_aliases tests to `src/alias_manager.rs` `#[cfg(test)]` module: test with aliases in one profile, multiple profiles, no aliases found

**Checkpoint**: User Story 2 complete — users can inspect their alias state

---

## Phase 5: User Story 3 — Update Existing Aliases (Priority: P2)

**Goal**: User runs `rcdir --set-aliases` again with a different root or sub-alias selection, and the existing block is cleanly replaced.

**Independent Test**: Set aliases with root `d`, run `--set-aliases` again with root `tc`, verify old block replaced with new one.

- [X] T047 [US3] Enhance `set_aliases()` flow: when existing block detected, show current aliases, pre-populate wizard defaults from existing config (root alias, enabled sub-aliases, current profile) in `src/alias_manager.rs`
- [X] T048 [US3] Implement block replacement path: use `ProfileFileManager::ReplaceAliasBlock()` instead of `AppendAliasBlock()` when existing block found in `src/alias_manager.rs`
- [X] T049 [US3] Add update tests to `src/alias_manager.rs` `#[cfg(test)]` module: test root change (d→tc), sub-alias toggle change, same-root different-subs

**Checkpoint**: User Story 3 complete — update flow works

---

## Phase 6: User Story 4 — Remove Aliases (Priority: P2)

**Goal**: User runs `rcdir --remove-aliases`, selects a profile, and the alias block is cleanly removed.

**Independent Test**: Have aliases in a profile, run `--remove-aliases`, verify block removed and rest of profile untouched.

- [X] T050 [US4] Implement `remove_aliases()` in `src/alias_manager.rs`: detect PS version → resolve paths → scan for blocks → if none found display message and exit (FR-053) → present checkbox list of profiles with aliases (FR-051, FR-052), unchecked by default (opt-in removal) → remove selected blocks via `ProfileFileManager::remove_alias_block()`
- [X] T051 [US4] Add dispatch in `main.rs`: if `remove_aliases` is set, call `alias_manager::remove_aliases()` and return
- [X] T052 [US4] Add `--remove-aliases` help text to `src/usage.rs`
- [X] T053 [US4] Add remove_aliases tests to `src/alias_manager.rs` `#[cfg(test)]` module: test successful removal, no-aliases-found case, profile content preservation

**Checkpoint**: User Story 4 complete — clean uninstall of aliases works

---

## Phase 7: User Story 5 — Dry Run with --whatif (Priority: P2)

**Goal**: User appends `--whatif` to `--set-aliases` or `--remove-aliases` and sees a preview without file modifications.

**Independent Test**: Run `--set-aliases --whatif`, complete wizard, verify no files modified and console shows preview.

- [X] T054 [US5] Add `--whatif` integration to `set_aliases()`: after wizard completes and block is generated, display block content and target path, print "What if: No changes were made" message, skip all file operations in `src/alias_manager.rs`
- [X] T055 [US5] Add `--whatif` integration to `remove_aliases()`: after profile selected, display lines that would be removed, print "What if: No changes were made" message, skip file operations in `src/alias_manager.rs`
- [X] T056 [US5] Add whatif tests to `src/alias_manager.rs` `#[cfg(test)]` module: test set-aliases --whatif produces output but no file changes, test remove-aliases --whatif produces output but no file changes

**Checkpoint**: User Story 5 complete — dry-run previews work accurately

---

## Phase 8: User Story 6 — Alias Conflict Detection (Priority: P3)

**Goal**: During setup, if chosen alias names conflict with existing PowerShell commands, the user is warned.

**Independent Test**: Choose root alias `r` (conflicts with `Invoke-History`), verify warning appears.

- [X] T057 [US6] Implement conflict scanning: given list of alias names, check `SearchPathW` for matching executables and check known PowerShell built-in alias list for matches (FR-074) in `src/alias_manager.rs`
- [X] T058 [US6] Integrate conflict warning into `set_aliases()` wizard: after root alias and sub-alias selection, run conflict check, display warnings with conflicting command identity, offer to override or choose different name in `src/alias_manager.rs`
- [X] T059 [US6] Add conflict detection tests to `src/alias_manager.rs` `#[cfg(test)]` module: test known conflict (built-in alias), test no-conflict path, test override confirmation

**Checkpoint**: User Story 6 complete — safety warnings prevent accidental breakage

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Final integration, edge case handling, documentation

- [X] T060 [P] Handle Ctrl+C / terminal close during wizard: ensure console mode and cursor visibility are restored via Drop guard (Rust RAII via Drop trait) in `src/tui_widgets.rs`
- [X] T061 [P] Handle file permission errors gracefully (FR-073): clear error message and clean exit in `src/profile_file_manager.rs`
- [X] T062 [P] Handle paths with spaces and special characters: ensure all path operations use proper quoting in generated PowerShell code in `src/alias_block_generator.rs`
- [X] T063 Verify all existing tests still pass (regression check)
- [X] T064 Run quickstart.md validation: build from clean, execute all three commands, verify output

---

## Phase 10: Output Parity Tests (TCDir ↔ RCDir)

**Purpose**: Verify alias command output matches TCDir using test mode (FR-090) and reference output files

- [X] T065 Implement `RCDIR_ALIAS_TEST_INPUTS` env var support in TUI wizard: when set, skip interactive input and parse semicolon-delimited predetermined answers (FR-090) in `src/tui_widgets.rs` and `src/alias_manager.rs`
- [X] T066 Ensure test mode is not shown in `--help` or usage output (FR-092)
- [X] T067 [P] Capture TCDir reference output files for parity scenarios (user-supplied): `--set-aliases --whatif` with default inputs, `--get-aliases` with no aliases, `--get-aliases` with aliases present, `--remove-aliases --whatif`; stored in `tests/fixtures/alias_parity/`
- [X] T068 [P] Add parity test `parity_set_aliases_whatif` in `tests/output_parity.rs`: run `rcdir --set-aliases --whatif` with `RCDIR_ALIAS_TEST_INPUTS=d;all;CurrentUserAllHosts;y`, compare output against reference file, filtering tool name (`rcdir`/`tcdir`) and version strings
- [X] T069 [P] Add parity test `parity_get_aliases_no_aliases` in `tests/output_parity.rs`: run `rcdir --get-aliases`, compare against reference file for "no aliases found" scenario
- [X] T070 [P] Add parity test `parity_remove_aliases_whatif` in `tests/output_parity.rs`: run `rcdir --remove-aliases --whatif` with `RCDIR_ALIAS_TEST_INPUTS`, compare against reference file
- [X] T071 Build and verify all parity tests pass

---

## Phase 11: Test Scaffolding Cleanup

**Purpose**: Remove all test-only code before feature completion. The feature is NOT done until this phase passes.

**⚠️ CRITICAL**: This phase is a hard gate — the feature branch MUST NOT be merged until all test scaffolding is removed.

- [X] T072 Remove `RCDIR_ALIAS_TEST_INPUTS` env var support from `src/tui_widgets.rs` and `src/alias_manager.rs` (revert T065)
- [X] T073 Remove FR-090/FR-091/FR-092 test mode requirements from spec (mark as completed/removed in this tasks file)
- [X] T074 Verify `cargo build --release` produces no references to `RCDIR_ALIAS_TEST_INPUTS` (grep the binary)
- [X] T075 Remove parity test fixture files from `tests/fixtures/alias_parity/` and parity test functions (T068–T070) from `tests/output_parity.rs`
- [X] T076 Build and verify all remaining tests still pass (no regressions from cleanup)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies — start immediately
- **Phase 2 (Foundational)**: Depends on Phase 1 — BLOCKS all user stories
- **Phase 3 (US1)**: Depends on Phase 2 — MVP delivery
- **Phase 4 (US2)**: Depends on Phase 2 — can run in parallel with US1
- **Phase 5 (US3)**: Depends on Phase 3 (extends SetAliases)
- **Phase 6 (US4)**: Depends on Phase 2 — can run in parallel with US1/US2
- **Phase 7 (US5)**: Depends on Phase 3 and Phase 6 (modifies both flows)
- **Phase 8 (US6)**: Depends on Phase 3 (extends SetAliases wizard)
- **Phase 9 (Polish)**: Depends on all user stories being complete
- **Phase 10 (Parity)**: Depends on Phase 9 — T065 (test mode) can start after Phase 3; T067–T071 after all user stories
- **Phase 11 (Cleanup)**: Depends on Phase 10 — HARD GATE: must complete before merge

### User Story Dependencies

```
Phase 2 (Foundation) ──→ US1 (Set Aliases) ──→ US3 (Update Existing)
                     │                     └──→ US5 (WhatIf, set path)
                     │                     └──→ US6 (Conflict Detection)
                     ├──→ US2 (Get Aliases)     [independent of US1]
                     └──→ US4 (Remove Aliases) ──→ US5 (WhatIf, remove path)
```

### Parallel Opportunities

- **Phase 1**: T005–T010 all [P] — file skeletons can be created in parallel
- **Phase 2**: T016, T019, T026, T032 — test files can be created in parallel with implementation
- **US1 + US2 + US4**: Can proceed in parallel after Phase 2
- **US3 + US5 + US6**: Sequential after US1, but US5 (remove path) can overlap with US3/US6

---

## Implementation Strategy

**MVP**: Phase 1 + Phase 2 + Phase 3 (User Story 1) = working `--set-aliases` wizard

**Incremental delivery**:
1. MVP: `--set-aliases` (new profile, first-time setup)
2. Add `--get-aliases` for visibility (US2)
3. Add `--remove-aliases` for clean uninstall (US4)
4. Add update support to `--set-aliases` (US3)
5. Add `--whatif` preview to both set and remove (US5)
6. Add conflict detection for safety (US6)
