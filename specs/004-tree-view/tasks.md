# Tasks: Tree View Display Mode

**Input**: Design documents from `/specs/004-tree-view/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md

**Tests**: Yes — TCDir has 58 tree-specific tests that must be ported (see R16 in research.md). Unit tests are written alongside implementation within each phase. Output parity tests are in the final phase.

**Organization**: Tasks grouped by user story. User stories map to spec.md priorities (P1 → P3).

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Exact file paths included in descriptions

## Phase 1: Setup

**Purpose**: Module registration and new file scaffolding

- [X] T001 Add `pub mod tree_connector_state;` to `src/lib.rs`
- [X] T002 [P] Create `SizeFormat` enum (`Default`, `Auto`, `Bytes`) in `src/command_line.rs`
- [X] T003 [P] Create empty `src/tree_connector_state.rs` with struct definition and method stubs per data-model.md entity 3
- [X] T004 [P] Create empty `src/results_displayer/tree.rs` with `TreeDisplayer` struct stub and `mod tree;` in `src/results_displayer/mod.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core constructs that ALL user stories depend on — tree connector state, switch parsing, interleaved sort, abbreviated sizes

**CRITICAL**: No user story work can begin until this phase is complete

### Tree Connector State (R1, entity 3)

- [X] T005 Implement `TreeConnectorState::new()`, `push()`, `pop()`, `depth()` in `src/tree_connector_state.rs`
- [X] T006 Implement `TreeConnectorState::get_prefix()` (generates `├── ` / `└── ` with ancestor continuation lines) in `src/tree_connector_state.rs`
- [X] T007 Implement `TreeConnectorState::get_stream_continuation()` in `src/tree_connector_state.rs`
- [X] T008 Add `#[cfg(test)] mod tests` with 17 unit tests ported from TCDir `TreeConnectorStateTests` in `src/tree_connector_state.rs`: `DefaultConstructor_Depth0`, `CustomIndent_StoredCorrectly`, `PrefixAtDepth0_EmptyString`, `PrefixAtDepth0_LastEntry_EmptyString`, `PrefixAtDepth1_MiddleEntry`, `PrefixAtDepth1_LastEntry`, `PrefixAtDepth2_MiddleEntry_AncestorHasSibling`, `PrefixAtDepth2_LastEntry_AncestorHasNoSibling`, `PrefixAtDepth3_MixedAncestors`, `PushPop_DepthTracking`, `Pop_AtDepth0_NoOp`, `StreamContinuation_Depth0_EmptyString`, `StreamContinuation_Depth1`, `StreamContinuation_Depth2_AncestorHasSibling`, `CustomIndent1_ShortPrefix`, `CustomIndent2_NarrowPrefix`, `CustomIndent8_WidePrefix`

### Switch Parsing (R2, R3, entity 1)

- [X] T009 Add `tree: bool`, `max_depth: i32`, `tree_indent: i32`, `size_format: SizeFormat` fields to `CommandLine` struct in `src/command_line.rs`
- [X] T010 Add parameterized long switch support to `handle_long_switch()`: split on `=` for key/value, consume next arg for space-separated form (R2) in `src/command_line.rs`
- [X] T011 Parse `--Tree`, `--Tree-`, `--Depth=N`, `--TreeIndent=N`, `--Size=Auto|Bytes` in `handle_long_switch()` in `src/command_line.rs`
- [X] T012 Implement `validate_switch_combinations()` for all conflict rules (Tree vs Wide/Bare/Recurse/Owner/Size=Bytes, Depth without Tree, TreeIndent without Tree, TreeIndent out of range, Depth ≤ 0) in `src/command_line.rs`
- [X] T013 Add 26 tree switch parsing unit tests ported from TCDir `CommandLineTests` in `src/command_line.rs`: `ParseTreeSwitchDoubleDash`, `ParseTreeSwitchSlash`, `ParseTreeDisableSwitchDoubleDash`, `ParseTreeSwitchCaseInsensitive`, `ParseTreeSwitchSingleDashFails`, `ParseDepthSingleDashFails`, `ParseDepthWithEquals`, `ParseDepthWithSpace`, `ParseTreeIndentWithEquals`, `ParseTreeWithWideFails`, `ParseTreeWithBareFails`, `ParseTreeWithRecurseFails`, `ParseTreeWithOwnerFails`, `ParseDepthWithoutTreeFails`, `ParseTreeIndentWithoutTreeFails`, `ParseTreeIndentOutOfRangeFails`, `ParseDepthZeroFails`, `ParseDepthNegativeFails`, `ParseTreeWithOwner_Fails_EvenWithIcons`, `ParseTreeWithDepthAndIndentSucceeds`, `ApplyConfigDefaults_Tree_TransfersToCommandLine`, `ApplyConfigDefaults_TreeWithDepth_TransfersToCommandLine`, `ApplyConfigDefaults_DepthWithoutTree_SilentlyIgnored`, `CLITreeDisable_OverridesEnvVarTree`, `CLIDepth_OverridesEnvVarDepth`, `ParseSizeDefaultResolvesToBytesWithoutTree`
- [X] T014 Add `--Size` non-tree unit tests in `src/command_line.rs`: `ParseSizeBytesWithoutTree`, `ParseSizeAutoWithoutTree`, `ParseSizeInvalidFails`, `ParseSizeCaseInsensitive`, `ParseSizeAutoWithTree`, `ParseSizeBytesWithTreeFails`, `ParseSizeDefaultResolvesToAutoWithTree`, `ApplyConfigDefaults_SizeAuto_TransfersToCommandLine`, `ApplyConfigDefaults_SizeBytes_NotOverriddenByCLI`

### Config / Environment Variable Parsing (R3, R9, entity 2)

- [X] T015 Add `Attribute::TreeConnector` variant with DarkGrey default color to `Attribute` enum in `src/config/mod.rs`
- [X] T016 [P] Add `tree: Option<bool>`, `max_depth: Option<i32>`, `tree_indent: Option<i32>`, `size_format: Option<SizeFormat>` fields to `Config` struct in `src/config/mod.rs`
- [X] T017 Parse `Tree`/`Tree-` boolean switch, `Depth=N`/`TreeIndent=N` integer switches, and `Size=Auto|Bytes` enum switch from RCDIR env var in `src/config/env_overrides.rs`
- [X] T018 Wire config-to-CLI override for tree/depth/tree_indent/size_format in `apply_config_defaults()` in `src/command_line.rs`
- [X] T019 Add 7 tree config env var unit tests ported from TCDir `ConfigSwitchOverrideTests` in `src/config/mod.rs`: `EnvVar_Tree_SetsTreeTrue`, `EnvVar_TreeDisable_SetsTreeFalse`, `EnvVar_Depth_SetsMaxDepth`, `EnvVar_TreeIndent_SetsTreeIndent`, `EnvVar_TreeWithDepthAndIndent_ParsesAll`, `EnvVar_DepthInvalid_RecordsError`, `EnvVar_TreeIndentOutOfRange_RecordsError`
- [X] T020 [P] Add env var `Size` unit tests in `src/config/mod.rs`: `EnvVar_SizeAuto_SetsSizeFormat`, `EnvVar_SizeBytes_SetsSizeFormat`, `EnvVar_SizeInvalid_RecordsError`, `EnvVar_SizeCaseInsensitive`

### Interleaved Sort (R6, entity 7)

- [X] T021 Add `interleaved_sort` parameter to `sort_files()` in `src/file_comparator.rs` — when true, skip `is_dir` grouping in `SortKey` comparison
- [X] T022 Add 3 interleaved sort unit tests ported from TCDir `FileComparatorTests` in `src/file_comparator.rs`: `InterleavedSort_DirectoriesNotGroupedFirst`, `InterleavedSort_SortsByNameNotType`, `InterleavedSort_NonInterleavedGroupsDirsFirst`

### Abbreviated Size Formatter (R13, entity 8)

- [X] T023 Implement `format_abbreviated_size()` function (1024-based, 3 significant digits, 7-char fixed width) in `src/results_displayer/normal.rs` (or `src/results_displayer/common.rs`)
- [X] T024 Add 12 abbreviated size unit tests ported from TCDir `ResultsDisplayerTests` in the same file: `FormatAbbreviatedSize_Zero`, `FormatAbbreviatedSize_SmallBytes`, `FormatAbbreviatedSize_1000`, `FormatAbbreviatedSize_1KB`, `FormatAbbreviatedSize_FractionalKB`, `FormatAbbreviatedSize_TensKB`, `FormatAbbreviatedSize_HundredsKB`, `FormatAbbreviatedSize_1MB`, `FormatAbbreviatedSize_TensMB`, `FormatAbbreviatedSize_1GB`, `FormatAbbreviatedSize_FractionalGB`, `FormatAbbreviatedSize_1TB`

**Checkpoint**: All foundational constructs built, tested, and passing. `cargo test` green. User story implementation can now begin.

---

## Phase 3: User Story 1 — Basic Tree Listing (Priority: P1) MVP

**Goal**: `rcdir --Tree` displays directory contents hierarchically with Unicode box-drawing connectors. All metadata columns present, consistent alignment.

**Independent Test**: Run `rcdir --Tree` in a directory with 2+ levels of nesting. Verify tree connectors display correctly and metadata columns are aligned.

### Implementation for User Story 1

- [X] T025 [US1] Implement `TreeDisplayer::new()` wrapping `NormalDisplayer` via composition in `src/results_displayer/tree.rs` (R10, entity 4)
- [X] T026 [US1] Implement `TreeDisplayer::display_single_entry()` — calls inner's column helpers, inserts tree prefix from `TreeConnectorState` before icon/filename in `src/results_displayer/tree.rs` (R1)
- [X] T027 [US1] Implement `TreeDisplayer::begin_directory()`, `save_directory_state()`, `restore_directory_state()`, `DirectoryDisplayState` struct in `src/results_displayer/tree.rs` (R12, entity 4)
- [X] T028 [US1] Implement `TreeDisplayer::display_tree_root_header()` and `display_tree_root_summary()` in `src/results_displayer/tree.rs` (R8)
- [X] T029 [US1] Implement `TreeDisplayer::into_console()` and `console_mut()` accessor methods in `src/results_displayer/tree.rs`
- [X] T030 [US1] Extract column-rendering helpers on `NormalDisplayer` as `pub(crate)` so `TreeDisplayer` can delegate to them in `src/results_displayer/normal.rs`
- [X] T031 [US1] Add `Tree(TreeDisplayer)` variant to `Displayer` enum and implement `ResultsDisplayer` trait delegation in `src/results_displayer/mod.rs`
- [X] T032 [US1] Implement `print_directory_tree_mode()` on `MultiThreadedLister` — main-thread depth-first tree walk calling `TreeDisplayer` public methods, with `TreeConnectorState` threaded through recursion in `src/multi_threaded_lister.rs` (R4)
- [X] T033 [US1] Implement `display_tree_entries()` on `MultiThreadedLister` — iterates entries at one level, determines `is_last` via look-ahead, calls `display_single_entry` in `src/multi_threaded_lister.rs`
- [X] T034 [US1] Route `--Tree` to MT lister path (always use MT even with `-M-`) in `src/multi_threaded_lister.rs` (R5)
- [X] T035 [US1] Pass `interleaved_sort = true` when `tree` is active in the sort call site in `src/multi_threaded_lister.rs` (R6)
- [X] T036 [US1] Wire `TreeDisplayer` creation in `src/lib.rs` — instantiate `Displayer::Tree(TreeDisplayer::new(...))` when `cmd.tree` is true
- [X] T037 [US1] Implement console flush before child directory recursion and after entry loop in `print_directory_tree_mode` in `src/multi_threaded_lister.rs` (R11)
- [X] T038 [US1] Integrate abbreviated size display — call `format_abbreviated_size()` when `size_format` resolves to `Auto` in the file size rendering path in `src/results_displayer/normal.rs` and `src/results_displayer/tree.rs` (R13)

**Checkpoint**: `rcdir --Tree` displays hierarchical output with connectors, metadata, interleaved sort, abbreviated sizes, and streaming output. Manually verify alignment.

---

## Phase 4: User Story 2 — Depth-Limited Tree Listing (Priority: P1)

**Goal**: `rcdir --Tree --Depth=N` limits tree recursion to N levels. Directories at the limit appear as entries but are not expanded.

**Independent Test**: Run `rcdir --Tree --Depth=1` in a 3+ level directory. Verify only one level of subdirectory contents is shown.

### Implementation for User Story 2

- [X] T039 [US2] Add depth check in `print_directory_tree_mode`: compare `TreeConnectorState::depth()` against `cmd.max_depth` before recursing into child directories in `src/multi_threaded_lister.rs`
- [X] T040 [US2] Ensure directories at the depth limit are displayed as entries (with `<DIR>` / abbreviated `<DIR>`) but their children are not expanded in `src/multi_threaded_lister.rs`

**Checkpoint**: `rcdir --Tree --Depth=2` shows exactly 2 levels. `--Depth` without `--Tree` produces error. `--Depth 0` and `--Depth -1` produce errors. All verified by unit tests from Phase 2.

---

## Phase 5: User Story 3 — Tree View with Metadata Columns (Priority: P1)

**Goal**: All metadata columns (date, time, attributes, size, cloud status, icons) render correctly at every tree nesting level with consistent alignment.

**Independent Test**: Run `rcdir --Tree --Icons` and verify icon+filename alignment across all nesting levels. Verify tree connector color.

### Implementation for User Story 3

- [X] T041 [US3] Emit `Attribute::TreeConnector` color via `Console` around tree prefix characters in `TreeDisplayer::display_single_entry()` in `src/results_displayer/tree.rs` (R9)
- [X] T042 [US3] Handle `<DIR>` display in abbreviated size mode — render `" <DIR>   "` (7-char fixed width) when `size_format == Auto` in `src/results_displayer/normal.rs` (entity 8)
- [X] T043 [US3] Verify icon positioning: tree connectors prepend before icon glyph (not between icon and filename) in `src/results_displayer/tree.rs` (R1)

**Checkpoint**: `rcdir --Tree --Icons` shows icons in correct position. Tree connectors colored DarkGrey by default. `<DIR>` entries correctly formatted in abbreviated mode.

---

## Phase 6: User Story 4 — Tree View with Alternate Data Streams (Priority: P2)

**Goal**: `rcdir --Tree --Streams` displays stream entries with `│` vertical continuation instead of tree connectors.

**Independent Test**: Run `rcdir --Tree --Streams` on a directory with files that have alternate data streams. Verify stream lines use `│` continuation.

### Implementation for User Story 4

- [X] T044 [US4] Implement `TreeDisplayer::display_file_streams_with_tree_prefix()` — prepend `│   ` continuation prefix to each stream line in `src/results_displayer/tree.rs`
- [X] T045 [US4] Call `display_file_streams_with_tree_prefix()` after each file entry that has streams in `display_tree_entries()` in `src/multi_threaded_lister.rs`

**Checkpoint**: `rcdir --Tree --Streams` shows correctly indented stream entries with vertical continuation.

---

## Phase 7: User Story 5 — Incompatible Switch Detection (Priority: P2)

**Goal**: Clear errors for all `/Tree` switch conflicts.

**Independent Test**: Run `rcdir --Tree -W`, `--Tree -B`, `--Tree -S`, `--Tree --Owner`, `--Tree --Size=Bytes` and verify each produces a specific error message.

### Implementation for User Story 5

- [X] T046 [US5] Add error messages for each conflict case in `validate_switch_combinations()` matching TCDir output format in `src/command_line.rs`

**Checkpoint**: All 5 conflict scenarios produce clear, specific error messages. No partial output. All verified by unit tests from Phase 2 (T013).

---

## Phase 8: User Story 6 — Tree View Environment Variable Configuration (Priority: P3)

**Goal**: `RCDIR=Tree;Depth=3` enables tree mode and depth limiting via environment variable. CLI flags override env var defaults.

**Independent Test**: Set `RCDIR=Tree` and run `rcdir` without flags. Verify tree mode activates. Set `RCDIR=Tree` and run `rcdir --Tree-`. Verify tree is disabled.

### Implementation for User Story 6

- [X] T047 [US6] Verify end-to-end env var → config → CLI flow for tree/depth/tree_indent/size_format (wiring was done in T017/T018, this task is integration verification) in `src/config/env_overrides.rs` and `src/command_line.rs`

**Checkpoint**: `RCDIR=Tree;Depth=2` works. `--Tree-` overrides env var. `Depth=N` without `Tree` silently ignored in env var. All verified by unit tests from Phase 2 (T019/T020).

---

## Phase 9: Cross-Cutting — Cycle Detection, Pruning, Usage, Fidelity

**Purpose**: Shared infrastructure (reparse point guard), tree-mode-specific pruning, help text, and output fidelity testing

### Reparse Point / Cycle Guard (R7)

- [X] T048 Add `FILE_ATTRIBUTE_REPARSE_POINT` check before recursing into child directories in the worker thread function in `src/multi_threaded_lister.rs` — if reparse point, list directory but do not expand children; show `[→ target]` indicator (R7). This protects both `--Tree` and `-S` modes.

### Thread-Safe Empty Subdirectory Pruning (R14, entities 5–6)

- [X] T049 Add `parent: Option<Weak<...>>`, `descendant_match_found: AtomicBool`, `subtree_complete: AtomicBool` fields to `DirectoryInfo` in `src/directory_info.rs`
- [X] T050 Add `tree_pruning_active: bool` field to `MultiThreadedLister` in `src/multi_threaded_lister.rs`
- [X] T051 Implement `propagate_descendant_match()` — walk up parent chain setting `descendant_match_found` and notifying `Condvar` in `src/multi_threaded_lister.rs`
- [X] T052 Implement `try_signal_parent_subtree_complete()` — check all children complete, set parent complete, recurse to grandparent in `src/multi_threaded_lister.rs`
- [X] T053 Wire producer-side: after enumeration, call `propagate_descendant_match` (if file_count > 0) and `try_signal_parent_subtree_complete` (if leaf) in `src/multi_threaded_lister.rs`
- [X] T054 Wire parent back-reference: set `parent` `Weak` ref during child `DirectoryInfo` creation when `tree_pruning_active` in `src/multi_threaded_lister.rs`
- [X] T055 Implement `wait_for_tree_visibility()` — block on `Condvar` until `descendant_match_found` or `subtree_complete` in `src/multi_threaded_lister.rs`
- [X] T056 Integrate pruning look-ahead in `display_tree_entries()` — call `wait_for_tree_visibility()` for directory entries, skip invisible dirs, adjust `is_last` in `src/multi_threaded_lister.rs`

### Usage Help Text

- [X] T057 [P] Document `--Tree`, `--Depth=N`, `--TreeIndent=N`, `--Size=Auto|Bytes` switches in help output in `src/usage.rs`

### Output Parity Tests (R15)

- [X] T058 [P] Add 12 tree-mode output parity tests to `tests/output_parity.rs`: `parity_tree_basic`, `parity_tree_depth_limited`, `parity_tree_custom_indent`, `parity_tree_with_icons`, `parity_tree_with_streams`, `parity_tree_file_mask`, `parity_tree_size_auto`, `parity_size_auto_non_tree`, `parity_size_bytes_explicit`, `parity_tree_sort_by_size`, `parity_tree_time_created`, `parity_tree_attr_filter`
- [X] T059 [P] Create `scripts/CompareOutput.ps1` — ad-hoc cross-tool comparison script per R15 / data-model entity 10

### Tree Integration / Scenario Tests (R16)

- [X] T060 [P] Create `tests/tree_mode_tests.rs` with 18 tree-specific integration tests covering: connector patterns at multiple depths, depth limiting behavior, empty directory display, file mask pruning, access-denied inline error in tree mode (FR-018), reparse point `[→ target]` indicator (FR-022), interleaved sort order verification, stream continuation lines, `<DIR>` abbreviated display, root-only header/footer, and custom indent widths

### Final Verification

- [ ] T061 Run `cargo test` — all unit + integration tests pass
- [ ] T062 Run `cargo clippy -- -D warnings` — no warnings
- [ ] T063 Run `scripts/CompareOutput.ps1` against multiple directories with various tree arguments — verify byte-for-byte match between `rcdir` and `tcdir`
- [ ] T064 Run quickstart.md verification commands (quick verification section + error cases)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies — start immediately
- **Phase 2 (Foundational)**: Depends on Phase 1 — BLOCKS all user stories
- **Phases 3–8 (User Stories)**: All depend on Phase 2 completion
  - US1 (Phase 3): Must complete first — provides the core tree infrastructure
  - US2 (Phase 4): Depends on US1 — adds depth check to US1's `print_directory_tree_mode`
  - US3 (Phase 5): Depends on US1 — adds color/formatting to US1's display path
  - US4 (Phase 6): Depends on US1 — adds stream handling to US1's display loop
  - US5 (Phase 7): No implementation dependency on US1 (validation code done in Phase 2), but verifies as part of complete feature
  - US6 (Phase 8): No implementation dependency (env var parsing done in Phase 2), integration verification only
- **Phase 9 (Cross-Cutting)**: Depends on US1 completion at minimum; parity tests depend on all user stories

### Within Phase 2 (Foundational)

```
T005–T008 (TreeConnectorState)  ─┐
T009–T014 (Switch Parsing)      ─┤─ can run in parallel (different files)
T015–T020 (Config/Env)          ─┤
T021–T022 (Interleaved Sort)    ─┤
T023–T024 (Abbreviated Sizes)   ─┘
```

### Within Phase 3 (US1)

```
T030 (extract NormalDisplayer helpers) ─┐
                                        ├─> T025–T029 (TreeDisplayer struct)
T031 (Displayer enum variant)          ─┤
                                        ├─> T032–T037 (MT lister tree walk)
T036 (wire in lib.rs)                  ─┤
                                        └─> T038 (abbreviated size integration)
```

### Parallel Opportunities

- **Phase 2**: All 5 foundational workstreams (connector state, switch parsing, config, sort, size formatter) can run in parallel — they touch different files
- **Phase 3**: T025–T029 (TreeDisplayer methods) can be developed in parallel once T030 (helper extraction) is done
- **Phase 9**: T057 (usage), T058 (parity tests), T059 (comparison script) can run in parallel

---

## Parallel Example: Phase 2 Foundational

```
# Stream 1: Tree Connector State (src/tree_connector_state.rs)
T005: Implement TreeConnectorState methods
T006: Implement get_prefix()
T007: Implement get_stream_continuation()
T008: Add 17 unit tests

# Stream 2: Switch Parsing (src/command_line.rs)
T009: Add fields to CommandLine
T010: Add parameterized long switch support
T011: Parse tree switches
T012: Implement validate_switch_combinations()
T013: Add 26 tree parsing unit tests
T014: Add Size non-tree unit tests

# Stream 3: Config (src/config/)
T015: Add Attribute::TreeConnector (config/mod.rs)
T016: Add Option fields to Config (config/mod.rs)
T017: Parse tree entries from RCDIR env var (config/env_overrides.rs)
T018: Wire config-to-CLI override (command_line.rs)
T019: Add 7 config unit tests
T020: Add Size config unit tests

# Stream 4: Sort (src/file_comparator.rs)
T021: Add interleaved_sort parameter
T022: Add 3 interleaved sort unit tests

# Stream 5: Size Formatter (src/results_displayer/)
T023: Implement format_abbreviated_size()
T024: Add 12 size formatter unit tests
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (module registration)
2. Complete Phase 2: Foundational (connector state, parsing, config, sort, sizes — all tested)
3. Complete Phase 3: User Story 1 (basic tree with all metadata + streaming)
4. **STOP and VALIDATE**: `rcdir --Tree` works end-to-end, compare against `tcdir --Tree`
5. Continue with remaining stories

### Incremental Delivery

1. Setup + Foundational → All building blocks tested in isolation
2. Add US1 → `rcdir --Tree` works → Compare with TCDir (MVP!)
3. Add US2 → Depth limiting works → Test independently
4. Add US3 → Metadata columns + icons aligned → Test independently
5. Add US4 → Streams display correctly → Test independently
6. Add US5 + US6 → Error messages + env var config → Test independently
7. Phase 9 → Cycle guard + pruning + parity tests → Final verification

### Task Summary

| Phase | Tasks | Description |
|-------|-------|-------------|
| Phase 1: Setup | T001–T004 | Module registration, file scaffolding |
| Phase 2: Foundational | T005–T024 | Connector state, switch parsing, config, sort, sizes (all with tests) |
| Phase 3: US1 (P1) | T025–T038 | Basic tree listing — core feature |
| Phase 4: US2 (P1) | T039–T040 | Depth limiting |
| Phase 5: US3 (P1) | T041–T043 | Metadata columns, connector color, icons |
| Phase 6: US4 (P2) | T044–T045 | Streams with tree prefix |
| Phase 7: US5 (P2) | T046 | Switch conflict error messages |
| Phase 8: US6 (P3) | T047 | Env var integration verification |
| Phase 9: Polish | T048–T064 | Cycle guard, pruning, usage, parity tests, integration tests, final verification |
| **Total** | **64 tasks** | |

### Test Count by Category

| Category | Tests | Source |
|----------|-------|--------|
| TreeConnectorState unit tests | 17 | T008 |
| Command-line tree parsing | 26 | T013 |
| Size switch parsing | 9 | T014 |
| Config tree env var | 7 | T019 |
| Config Size env var | 4 | T020 |
| Interleaved sort | 3 | T022 |
| Abbreviated size formatter | 12 | T024 |
| Output parity (cross-tool) | 12 | T058 |
| Tree integration / scenario | 18 | T060 |
| **Total new tests** | **108** | |

---

## Notes

- [P] tasks = different files, no dependencies on incomplete tasks
- [USn] label maps task to specific user story for traceability
- Each user story should be independently testable after completion
- Commit after each task or logical group
- Run `cargo test` after every phase
- Run `cargo clippy -- -D warnings` before marking phase complete
- Use `scripts/CompareOutput.ps1` for ad-hoc fidelity checking during development
