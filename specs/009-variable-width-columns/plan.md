# Implementation Plan: Variable-Width Columns in Wide Mode

**Branch**: `009-variable-width-columns` | **Date**: 2026-04-20 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/009-variable-width-columns/spec.md`

## Summary

Replace the uniform-width column layout in `WideDisplayer` with a variable-width algorithm that computes per-column widths based on actual entry display widths. This enables better space utilization when a directory contains a mix of short and long filenames. The algorithm tries column counts from maximum feasible down to 2 (with single-column fallback), optionally truncates outlier-length filenames (when ellipsis is enabled and doing so produces more columns), and distributes leftover space evenly across inter-column gaps. Port of TCDir's `CResultsDisplayerWide::ComputeColumnLayout` / `FitColumns` / `TryColumnCount`. Per-entry display width computation stays inline in `wide.rs` (rather than a separate function in `column_layout.rs`) because it requires access to `FileInfo`, `Config`, and icon state.

## Technical Context

**Language/Version**: Rust stable (latest)
**Primary Dependencies**: `windows` crate (Console API), standard library
**Storage**: N/A
**Testing**: `cargo test` — unit tests with mock data, output parity tests with real binaries
**Target Platform**: Windows 10/11, x64 and ARM64
**Project Type**: CLI tool
**Performance Goals**: Column layout computation must be imperceptible (<1ms for typical directories)
**Constraints**: Output must be byte-identical to TCDir for the same inputs
**Scale/Scope**: Single module change (`src/results_displayer/wide.rs`) + new layout algorithm module

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Code Quality | ✅ PASS | Pure algorithm functions return `Result` where appropriate; no unwrap in production |
| II. Testing Discipline | ✅ PASS | Unit tests with synthetic data (no real FS); output parity tests for visual verification |
| III. UX Consistency | ✅ PASS | Same CLI switches; output parity target with TCDir; backward compatible for uniform-width dirs |
| IV. Performance | ✅ PASS | O(N × C) algorithm where N=entries, C=candidate column counts; negligible for typical dirs |
| V. Simplicity | ✅ PASS | Isolated to wide.rs + new column_layout module; no changes to other display modes |

All gates pass. No violations to justify.

## Project Structure

### Documentation (this feature)

```text
specs/009-variable-width-columns/
├── plan.md              # This file
├── research.md          # Phase 0: algorithm research & decisions
├── data-model.md        # Phase 1: data structures
├── quickstart.md        # Phase 1: implementation quickstart
└── tasks.md             # Phase 2 output (created by /speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── results_displayer/
│   ├── wide.rs              # MODIFY — refactor to use ColumnLayout
│   └── column_layout.rs     # NEW — pure layout algorithm (compute, fit, truncate)
├── command_line.rs           # READ ONLY — ellipsize flag
├── config/mod.rs             # READ ONLY — display style, icon suppression
└── path_ellipsis.rs          # READ ONLY — ELLIPSIS constant reuse

tests/
└── output_parity.rs          # MODIFY — add wide-mode parity test cases
```

**Structure Decision**: Extract the layout algorithm into a new `column_layout.rs` submodule under `results_displayer/`. This keeps the pure algorithm (testable with synthetic data) separate from the rendering logic (console I/O). The existing `wide.rs` is refactored to call the new module.

## Complexity Tracking

No constitution violations. Table intentionally left empty.
