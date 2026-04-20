# Tasks: Symlink & Junction Target Display

**Input**: Design documents from `/specs/007-symlink-junction-targets/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md

**Tests**: Included — spec requires unit tests for buffer parsing (SC-006).

**Organization**: Tasks grouped by user story. Each story is independently testable.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story (US1, US2, US3, US4)

---

## Phase 1: Setup

**Purpose**: Module scaffolding, dependencies, FileInfo field addition

- [ ] T001 Add `reparse_target: String` field to `FileInfo` struct in src/file_info.rs
- [ ] T002 [P] Create `src/reparse_resolver.rs` module with reparse tag constants and module-level doc comment
- [ ] T003 [P] Register `reparse_resolver` module in src/lib.rs
- [ ] T004 Verify `windows` crate features include `Win32_Storage_FileSystem` and `Win32_System_IO` in Cargo.toml (add if missing)

**Checkpoint**: New module exists, compiles, FileInfo has reparse_target field

---

## Phase 2: Foundational (Buffer Parsing — Pure Functions)

**Purpose**: Implement all reparse buffer parsing as pure functions with no I/O. These are the testable core that all user stories depend on.

**⚠️ CRITICAL**: No display or integration work can begin until parsers are complete and tested.

- [ ] T005 Implement `strip_device_prefix(path: &str) -> String` in src/reparse_resolver.rs — strips `\??\` prefix from paths (FR-005)
- [ ] T006 [P] Define `REPARSE_DATA_BUFFER` header struct and mount point / symlink / AppExecLink sub-structures as `#[repr(C)]` types in src/reparse_resolver.rs (per data-model.md)
- [ ] T007 Implement `parse_junction_buffer(buffer: &[u8]) -> String` in src/reparse_resolver.rs — extract PrintName (preferred) or SubstituteName with prefix stripping (FR-001, FR-004, FR-005)
- [ ] T008 Implement `parse_symlink_buffer(buffer: &[u8]) -> String` in src/reparse_resolver.rs — extract PrintName (preferred) or SubstituteName; strip prefix only for absolute symlinks (FR-002, FR-004)
- [ ] T009 Implement `parse_app_exec_link_buffer(buffer: &[u8]) -> String` in src/reparse_resolver.rs — parse version-3 buffer, extract third NUL-terminated UTF-16 string (FR-002a)
- [ ] T010 [P] Add unit tests for `strip_device_prefix` in src/reparse_resolver.rs — prefix removal, UNC paths, empty strings, no-prefix paths (SC-006)
- [ ] T011 Add test helper `build_junction_buffer(print_name, substitute_name) -> Vec<u8>` in src/reparse_resolver.rs tests
- [ ] T012 Add unit tests for `parse_junction_buffer` in src/reparse_resolver.rs — PrintName extraction, SubstituteName fallback, prefix stripping, truncated buffer, empty names (SC-006)
- [ ] T013 Add test helper `build_symlink_buffer(print_name, substitute_name, flags) -> Vec<u8>` in src/reparse_resolver.rs tests
- [ ] T014 Add unit tests for `parse_symlink_buffer` in src/reparse_resolver.rs — absolute symlinks, relative symlinks (SYMLINK_FLAG_RELATIVE), conditional prefix stripping, truncated buffer, verify relative paths preserved as-stored without resolution (FR-004, SC-006)
- [ ] T015 Add test helper `build_app_exec_link_buffer(version, pkg_id, app_id, target_exe) -> Vec<u8>` in src/reparse_resolver.rs tests
- [ ] T016 Add unit tests for `parse_app_exec_link_buffer` in src/reparse_resolver.rs — version 3 parsing, version mismatch returns empty, truncated buffer, bounds checks (SC-006)

**Checkpoint**: All three parsers + strip_device_prefix implemented and tested with synthetic byte arrays. `cargo test` passes.

---

## Phase 3: User Story 1 — Normal Mode Target Display (Priority: P1) 🎯 MVP

**Goal**: Show `→ target` after symlinks, junctions, and AppExecLinks in normal mode listings.

**Independent Test**: Run `rcdir` in a directory with a junction and verify `→ target` appears.

### Implementation for User Story 1

- [ ] T017 [US1] Implement `resolve_reparse_target(dir_path: &Path, file_info: &FileInfo) -> String` in src/reparse_resolver.rs — Win32 I/O wrapper: check attribute flag, check tag, build full path from `dir_path` + `file_info.file_name`, CreateFileW + DeviceIoControl, dispatch to parser (FR-001, FR-002, FR-002a, FR-011, FR-014)
- [ ] T018 [US1] Call `resolve_reparse_target()` in `add_match_to_list()` in src/directory_lister.rs — store result in `file_info.reparse_target` (Research Decision 2)
- [ ] T019 [US1] Call `resolve_reparse_target()` in multi-threaded enumeration path in src/multi_threaded_lister.rs — same integration as T018
- [ ] T020 [US1] Append `→ target` display in src/results_displayer/normal.rs — if `reparse_target` is non-empty: print ` → ` with Information color, then target with filename color (FR-003, FR-006, FR-007, FR-009)
- [ ] T021 [US1] Verify wide mode and bare mode do NOT display targets in src/results_displayer/wide.rs and src/results_displayer/bare.rs — confirm no changes needed (FR-009)

**Checkpoint**: Normal mode shows `→ target` for junctions, symlinks, and AppExecLinks. Wide/bare modes unaffected. `cargo test` passes.

---

## Phase 4: User Story 2 — Tree Mode Target Display (Priority: P2)

**Goal**: Show `→ target` after symlinks and junctions in tree mode listings.

**Independent Test**: Run `rcdir --Tree` in a directory tree with a junction and verify `→ target` appears.

### Implementation for User Story 2

- [ ] T022 [US2] Append `→ target` display in src/results_displayer/tree.rs — same pattern as T020: if `reparse_target` non-empty, print arrow with Information color and target with filename color (FR-001, FR-002, FR-003, FR-006, FR-007)
- [ ] T023 [US2] Verify junctions and symlinks are not recursed into during tree walk — confirm existing reparse-point cycle guard behavior is preserved (FR-010)

**Checkpoint**: Tree mode shows `→ target` for links. Junctions/symlinks not expanded. `cargo test` passes.

---

## Phase 5: User Story 3 — Color-Coded Target Paths (Priority: P3)

**Goal**: Arrow uses Information color, target uses filename's resolved color.

**Independent Test**: Run `rcdir` on a junction and a file symlink; verify arrow color differs from target color.

### Implementation for User Story 3

- [ ] T024 [US3] Verify color implementation in normal.rs (T020) and tree.rs (T022) already uses correct attributes — Information for arrow, `text_attr` for target (FR-006, FR-007)
- [ ] T025 [P] [US3] Add unit test verifying Information color ANSI escape sequence appears before arrow character in mock console buffered output (SC-006)

**Checkpoint**: Colors verified correct. If T020/T022 already implemented colors correctly, this phase is validation only.

---

## Phase 6: User Story 4 — Internal Path Prefix Stripping (Priority: P3)

**Goal**: `\??\` device prefix stripped from junction targets.

**Independent Test**: Create a junction; verify displayed target doesn't start with `\??\`.

### Implementation for User Story 4

- [ ] T026 [US4] Verify `strip_device_prefix` is already called in `parse_junction_buffer` (T007) and `parse_symlink_buffer` (T008) for SubstituteName fallback — confirm FR-005 is satisfied (T012 already covers this test case)

**Checkpoint**: Prefix stripping verified. If T007/T008 already handle this, phase is validation only.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Final verification, edge cases, cleanup

- [ ] T028 Run `cargo clippy -- -D warnings` and fix any warnings
- [ ] T029 Run `cargo test` and verify all existing + new tests pass (SC-005)
- [ ] T030 Verify recursive mode (`-S`) shows targets but does not recurse into links (FR-010)
- [ ] T031 Verify access-denied edge case: unreadable reparse point displays filename without target, no error (FR-011)
- [ ] T032 Verify non-reparse files are completely unaffected — no performance regression (SC-003)
- [ ] T033 Verify no new command-line switches or config keys were introduced (FR-012)
- [ ] T034 Verify hardlink information is not resolved or displayed (FR-013)

**Checkpoint**: Feature complete, all tests pass, clippy clean.

---

## Dependencies

```
Phase 1 (Setup) → Phase 2 (Parsers) → Phase 3 (US1: Normal) → Phase 4 (US2: Tree)
                                          ↓
                                    Phase 5 (US3: Colors) — validation only
                                    Phase 6 (US4: Prefix) — validation only
                                          ↓
                                    Phase 7 (Polish)
```

- Phase 3 (US1) depends on Phase 2 completion
- Phase 4 (US2) depends on Phase 3 (reuses same resolver + display pattern)
- Phases 5–6 are validation of work already done in Phases 2–4
- Phase 7 depends on all prior phases

## Parallel Execution Opportunities

| Tasks | Why Parallel |
|-------|-------------|
| T002, T003 | Different files (reparse_resolver.rs vs lib.rs) |
| T006, T010 | Struct definitions vs strip_device_prefix tests (no overlap) |
| T011, T013, T015 | Test helpers for different buffer types (independent) |
| T024, T025 | Color validation vs color unit test (independent) |

## Implementation Strategy

- **MVP**: Phase 1 + Phase 2 + Phase 3 = normal mode shows targets (core feature)
- **Increment 1**: Phase 4 = tree mode
- **Increment 2**: Phases 5–6 = color and prefix validation
- **Ship**: Phase 7 = polish and verify

## Summary

| Metric | Value |
|--------|-------|
| Total tasks | 34 |
| Phase 1 (Setup) | 4 tasks |
| Phase 2 (Foundational) | 12 tasks |
| Phase 3 (US1: Normal) | 5 tasks |
| Phase 4 (US2: Tree) | 2 tasks |
| Phase 5 (US3: Colors) | 2 tasks |
| Phase 6 (US4: Prefix) | 1 task |
| Phase 7 (Polish) | 7 tasks |
| Parallel opportunities | 4 groups |
| MVP scope | Phases 1–3 (21 tasks) |
