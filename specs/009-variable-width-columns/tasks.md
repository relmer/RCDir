# Tasks: Variable-Width Columns in Wide Mode

**Input**: Design documents from `/specs/009-variable-width-columns/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Create the new module and register it

- [X] T001 Create `src/results_displayer/column_layout.rs` with module header, `ColumnLayout` struct definition (fields: `columns: usize`, `rows: usize`, `column_widths: Vec<usize>`, `trunc_cap: usize`), and `use` imports
- [X] T002 Register `column_layout` module in `src/results_displayer/mod.rs` with `mod column_layout;` and `pub use self::column_layout::ColumnLayout;`

**Checkpoint**: Project compiles with empty new module (`cargo check`)

---

## Phase 2: Foundational (Core Algorithm)

**Purpose**: Implement the pure layout algorithm functions in `column_layout.rs`. These MUST be complete before any user story integration.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T003 [P] Implement `compute_median(widths: &[usize]) -> usize`
- [X] T004 [P] Implement `try_column_count(widths: &[usize], console_width: usize, num_cols: usize) -> Option<ColumnLayout>`
- [X] T005 Implement `fit_columns(widths: &[usize], console_width: usize) -> ColumnLayout`
- [X] T006 Implement `compute_column_layout(widths: &[usize], console_width: usize, ellipsize: bool) -> ColumnLayout`
- [X] T007 Add `#[cfg(test)] mod tests` with unit tests for `compute_median()`
- [X] T008 Add unit tests for `try_column_count()` including column-major ordering verification
- [X] T009 Add unit tests for `fit_columns()`
- [X] T010 Add unit tests for `compute_column_layout()`

**Checkpoint**: All pure algorithm functions implemented and tested. `cargo test` passes. `cargo clippy -- -D warnings` clean.

---

## Phase 3: User Story 1 — Better Space Utilization in Wide Mode (Priority: P1) 🎯 MVP

**Goal**: Replace uniform-width column calculation with variable-width layout in `display_wide_file_results()`.

**Independent Test**: Run `rcdir /W` on a directory with mixed-length filenames and verify more columns than before.

### Implementation for User Story 1

- [X] T011 [US1] Build per-entry display widths vector in `display_wide_file_results()`
- [X] T012 [US1] Replace uniform-width column calculation with `compute_column_layout()` call
- [X] T013 [US1] Update render loop to use per-column widths from `ColumnLayout`
- [X] T014 [US1] Add outlier truncation at render time with `ELLIPSIS` char

**Checkpoint**: `rcdir /W` on mixed-length directories shows variable-width columns. `rcdir /W` on uniform-length directories produces identical output to before. Column-major ordering is preserved.

---

## Phase 4: User Story 3 — Correct Width Accounting for Icons and Cloud Status (Priority: P2)

**Goal**: Per-entry display width correctly accounts for icon and cloud status presence/absence on each entry individually.

**Independent Test**: Run `rcdir /W /I` and verify icon-bearing entries display without truncation or misalignment.

### Implementation for User Story 3

- [X] T016 [US3] Add unit tests for display width scenarios (icons, cloud status, brackets, mixed)

**Checkpoint**: Width accounting verified by unit tests. The per-entry width vector built in T011 already handles these cases — this phase confirms correctness.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Output parity verification and edge case hardening

- [ ] T017 [P] Add output parity test cases for wide mode in `tests/output_parity.rs`
- [ ] T018 [P] Add edge case unit tests in `src/results_displayer/column_layout.rs`
- [ ] T019 Run `cargo clippy -- -D warnings` and `cargo test` — verify zero errors before final commit

**Checkpoint**: All tests pass. Output parity verified. Clippy clean.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Phase 2 — core integration
- **User Story 3 (Phase 4)**: Depends on Phase 2 — can run in parallel with Phase 3
- **Polish (Phase 5)**: Depends on Phase 3 completion (needs working binary for parity tests)

Note: User Story 2 (column-major ordering) is verified by T008 in the foundational phase — no separate phase needed.

### Within Foundational Phase

- T003 and T004 can run in parallel (independent pure functions)
- T005 depends on T004 (calls `try_column_count`)
- T006 depends on T003 + T005 (calls `compute_median` + `fit_columns`)
- T007–T010 can run after their corresponding implementation tasks

### Parallel Opportunities

```text
After Phase 1:
  ├── T003 (compute_median)         ─┐
  └── T004 (try_column_count)       ─┤
                                     ├── T005 (fit_columns)
                                     └── T006 (compute_column_layout)
                                          │
After Phase 2:                            │
  ├── T011-T014 (US1: integration)  ◄─────┘
  └── T016 (US3: width tests)       ◄─────┘

After Phase 3:
  ├── T017 (parity tests)
  └── T018 (edge case tests)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T002)
2. Complete Phase 2: Foundational algorithm (T003-T010)
3. Complete Phase 3: User Story 1 integration (T011-T014)
4. **STOP and VALIDATE**: Run `rcdir /W` manually on mixed-length directories
5. Build with `scripts/Build.ps1`, run `cargo test`, run `cargo clippy -- -D warnings`

### Full Delivery

1. MVP (Phases 1-3) — variable-width columns working
2. Phase 4 — width accounting verified by tests
3. Phase 5 — output parity + edge cases + final cleanup
