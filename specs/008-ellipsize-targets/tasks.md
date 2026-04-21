# Tasks: Ellipsize Long Link Target Paths

**Input**: Design documents from `/specs/008-ellipsize-targets/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md

**Tests**: Included — spec requires new unit tests (SC-005) and output parity tests (release checklist).

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Create the feature branch and new module skeleton

- [X] T001 Create and checkout `008-ellipsize-targets` branch from `main` (version bump is handled automatically by `Build.ps1` / `IncrementVersion.ps1`)
- [X] T002 Create `src/path_ellipsis.rs` with `EllipsizedPath` struct (fields: `prefix: String`, `suffix: String`, `truncated: bool`) and a stub `ellipsize_path(target_path: &str, available_width: usize) -> EllipsizedPath` that returns the full path unchanged
- [X] T003 Register the new module by adding `pub mod path_ellipsis;` to `src/lib.rs`

---

## Phase 2: Foundational — Switch Infrastructure (Blocking)

**Purpose**: Wire up `--Ellipsize` / `--Ellipsize-` switch through the entire config pipeline so displayers can query it. MUST be complete before any user story phase.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T004 Add `pub ellipsize: Option<bool>` field to `Config` in `src/config/mod.rs`; bump `SWITCH_COUNT` from 9 to 10; add accessor to `SWITCH_MEMBER_ORDER`; initialize to `None` in `Config::new()`; add to `switch_sources` default array
- [X] T005 [P] Add `ellipsize`/`ellipsize-` entries to `SWITCH_MAPPINGS` in `src/config/env_overrides.rs`; add `"ellipsize" => Some(9)` to `switch_name_to_source_index`; update the error message string to include "Ellipsize"
- [X] T006 [P] Add `pub ellipsize: Option<bool>` field to `CommandLine` in `src/command_line.rs`; add `("ellipsize", ..., "ellipsize-", ...)` to the `bool_switches` table in `handle_long_switch`; add `"ellipsize"` to `is_recognized_long_switch`; add conditional merge in `apply_config_defaults` (default: `true` — `cmd.ellipsize.unwrap_or(true)`)
- [X] T007 Add `--Ellipsize` to help output in `src/usage.rs`: add `SwitchInfo` entry to `SWITCH_INFOS`; add line to `-?` help text; add to `--Settings` display output

**Checkpoint**: `cargo check` passes; `--Ellipsize` appears in `rcdir -?` output and `rcdir --Settings` output. Switch defaults to on.

---

## Phase 3: User Story 1 — Middle-Truncate Long Target Paths (Priority: P1) 🎯 MVP

**Goal**: In normal mode, long link target paths are middle-truncated with `…` (U+2026) to prevent line wrapping. The ellipsis renders in Default color for visual distinction.

**Independent Test**: Run `rcdir` in `%LOCALAPPDATA%\Microsoft\WindowsApps` at 120-char terminal width. Verify AppExecLink targets show truncated paths like `C:\Program Files\…\python3.12.exe` instead of wrapping.

### Tests for User Story 1 ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T008 [P] [US1] Write unit tests for `ellipsize_path()` in `src/path_ellipsis.rs` (inside `#[cfg(test)] mod tests`). Test cases MUST include all 7 real filename/target pairs from spec.md Test Data table at width 120, plus:
  - Short path that fits (no truncation)
  - Path with exactly 2 components (never truncated — nothing to elide)
  - Path with 3 components (minimal truncation case)
  - Priority level 1 form: first two dirs + `…\` + leaf dir + filename
  - Priority level 2 form: first two dirs + `…\` + filename
  - Priority level 3 form: first dir + `…\` + filename
  - Leaf-only fallback with trailing `…` when even level 3 doesn't fit
  - Edge case: `available_width` of 0 or 1
  - Edge case: truncated form is not shorter than original (should return original)
  - FR-004 guard: verify that only the target path is passed to `ellipsize_path()` — source filename is never modified by the truncation logic
- [X] T009 [P] [US1] Write unit test verifying `EllipsizedPath` struct fields: when `truncated` is false, `prefix` is full path and `suffix` is empty; when `truncated` is true, `prefix` + `…` + `suffix` fits within `available_width`

### Implementation for User Story 1

- [X] T010 [US1] Implement `ellipsize_path()` algorithm in `src/path_ellipsis.rs` per research.md R2: split on `\`, try priority forms (first two dirs + leaf dir + filename → first two dirs + filename → first dir + filename → leaf-only with trailing `…`), return `EllipsizedPath` with prefix/suffix split. Ensure all T008/T009 tests pass.
- [X] T011 [US1] Integrate ellipsize into normal displayer in `src/results_displayer/normal.rs`: compute `available_width` using formula from research.md R1 (console width minus date/time/attributes/size/cloud/debug/owner/icon/filename/arrow columns); call `ellipsize_path()` when `cmd.ellipsize.unwrap_or(true)` is true; render with split colors — prefix in `text_attr`, `…` in `Attribute::Default`, suffix in `text_attr`

**Checkpoint**: `cargo test` passes. Normal-mode output shows truncated targets for long paths. Short paths are unaffected.

---

## Phase 4: User Story 2 — Ellipsize in Tree Mode (Priority: P2)

**Goal**: Tree mode also middle-truncates long target paths, accounting for tree connector prefix width in the available-width calculation.

**Independent Test**: Run `rcdir --Tree` in a tree containing a junction with a long target path. Verify the target is truncated and the line does not wrap.

### Tests for User Story 2 ⚠️

- [X] T012 [P] [US2] Write unit tests for tree-mode available-width calculation in `src/path_ellipsis.rs` (or as part of tree displayer tests): verify that tree prefix width (from `tree_state.get_prefix(is_last).len()` + indent) is subtracted from available width before calling `ellipsize_path()`

### Implementation for User Story 2

- [X] T013 [US2] Integrate ellipsize into tree displayer in `src/results_displayer/tree.rs`: compute `available_width` using the same formula as normal mode but additionally subtracting tree prefix width (depth × indent + connector chars); call `ellipsize_path()` when `cmd.ellipsize.unwrap_or(true)` is true; render with same split-color pattern as normal mode

**Checkpoint**: `cargo test` passes. Tree-mode output shows truncated targets for long paths at all depths.

---

## Phase 5: User Story 3 — Disable Truncation with --Ellipsize- (Priority: P3)

**Goal**: Users can opt out of truncation via `--Ellipsize-` switch, config file, or environment variable.

**Independent Test**: Run `rcdir --Ellipsize-` in a directory with long targets. Verify full paths are shown and lines wrap as before.

### Tests for User Story 3 ⚠️

- [X] T014 [P] [US3] Write unit tests verifying `ellipsize_path()` is NOT called (or is bypassed) when `cmd.ellipsize == Some(false)`: test that both normal and tree displayer code paths respect the switch. Also test config precedence: CLI `--Ellipsize` overrides config file `Ellipsize-`.

### Implementation for User Story 3

- [X] T015 [US3] Verify the `--Ellipsize-` code path in `src/results_displayer/normal.rs` and `src/results_displayer/tree.rs`: when `cmd.ellipsize == Some(false)`, skip the call to `ellipsize_path()` and display the full target path unchanged (existing behavior). This should already work from T011/T013 guard checks — verify and add explicit test coverage.

**Checkpoint**: `cargo test` passes. `rcdir --Ellipsize-` shows full untruncated paths.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Output parity tests, documentation updates, and release checklist items.

- [X] T016 [P] Add output parity test cases in `tests/output_parity.rs`: add test cases for ellipsize in normal mode (default on) and tree mode, comparing `rcdir` vs `tcdir` output. Tests must gracefully skip when `tcdir.exe` is not available.
- [X] T017 [P] Add output parity test case in `tests/output_parity.rs` for `--Ellipsize-` (disabled) to verify both tools produce identical full-path output.
- [X] T018 [P] Update `CHANGELOG.md` with the new version entry and feature description for ellipsize
- [X] T019 [P] Update `README.md` "What's New" table with a row for the ellipsize feature
- [ ] T020 [P] Update `TCDir/specs/sync-status.md` with spec 008 row (RCDir status, version) — confirm with user before modifying TCDir workspace
- [X] T021 Run `cargo clippy -- -D warnings` and fix any warnings
- [X] T022 Run `cargo test` and verify all tests pass (unit + output parity)
- [X] T023 Run quickstart.md validation: build with VS Code task "Build Debug (current arch)", then manually test `rcdir` in `%LOCALAPPDATA%\Microsoft\WindowsApps` to confirm truncation behavior

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Phase 2 — core truncation algorithm + normal mode
- **User Story 2 (Phase 4)**: Depends on Phase 2 + Phase 3 (reuses same `ellipsize_path()` function and rendering pattern)
- **User Story 3 (Phase 5)**: Depends on Phases 3 and 4 — T014/T015 test displayer code paths that must exist first
- **Polish (Phase 6)**: Depends on Phases 3–5 being complete

### User Story Dependencies

- **User Story 1 (P1)**: After Foundational — implements the core pure function and normal-mode integration
- **User Story 2 (P2)**: After US1 — reuses `ellipsize_path()` with different available-width calculation
- **User Story 3 (P3)**: After US1 and US2 — displayer guard tests require normal and tree integration to be complete

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Pure function before displayer integration
- Normal mode before tree mode (shared algorithm)

### Parallel Opportunities

- T005 and T006 can run in parallel (different files: `env_overrides.rs` vs `command_line.rs`)
- T008 and T009 can run in parallel (independent test cases in same module)
- T012 can run in parallel with T014 (different story tests)
- T016, T017, T018, T019, T020 can all run in parallel (different files)

---

## Parallel Example: Phase 2

```
# These can run in parallel (different files):
T005: SWITCH_MAPPINGS in src/config/env_overrides.rs
T006: CommandLine field in src/command_line.rs
T007: Usage/help in src/usage.rs (after T004 for Config field)
```

## Parallel Example: User Story 1

```
# Tests first (parallel — same file, independent test functions):
T008: Unit tests for ellipsize_path() algorithm
T009: Unit tests for EllipsizedPath struct contract

# Then implementation (sequential):
T010: Implement ellipsize_path() — make tests pass
T011: Integrate into normal displayer
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (branch + module skeleton)
2. Complete Phase 2: Foundational (switch infrastructure — 4 files)
3. Complete Phase 3: User Story 1 (pure function + normal mode)
4. **STOP and VALIDATE**: `cargo test`, `cargo clippy`, manual test in WindowsApps
5. Commit — this is a shippable MVP

### Incremental Delivery

1. Setup + Foundational → Switch wired, `rcdir -?` shows `--Ellipsize`
2. User Story 1 → Normal mode truncation works → Commit
3. User Story 2 → Tree mode truncation works → Commit
4. User Story 3 → `--Ellipsize-` opt-out verified → Commit
5. Polish → Parity tests, docs, clippy clean → Final commit

### Commit Points (per project rules)

- After Phase 2 (switch infrastructure complete)
- After Phase 3 (US1 MVP — normal mode working)
- After Phase 4 (US2 — tree mode working)
- After Phase 5 (US3 — opt-out verified)
- After Phase 6 (polish — docs, parity tests, clippy clean)

---

## Notes

- [P] tasks = different files, no dependencies on incomplete tasks
- [Story] label maps task to specific user story for traceability
- All unit tests use synthetic data — no system state (per copilot-instructions.md)
- Output parity tests are the allowed exception — they run real `rcdir`/`tcdir` binaries
- Build MUST use VS Code task "Build Debug (current arch)" or `scripts/Build.ps1` — never raw `cargo build`
- `cargo test`, `cargo check`, `cargo clippy` are fine to run directly
