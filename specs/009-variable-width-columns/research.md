# Research: Variable-Width Columns in Wide Mode

**Date**: 2026-04-20 | **Feature**: 009-variable-width-columns

## R1: Column Layout Algorithm — Reference Implementation Analysis

**Decision**: Port TCDir's 4-function decomposition directly: `compute_display_width`, `compute_column_layout`, `fit_columns`, `try_column_count`.

**Rationale**: TCDir's implementation is the reference. The algorithm is well-structured, separates concerns cleanly, and is already proven correct. The 4-function decomposition maps naturally to Rust's module system and enables unit testing with synthetic data.

**Alternatives considered**:
- GNU `ls` column-fitting (similar greedy top-down approach) — essentially the same algorithm; TCDir already adopted this pattern
- Optimal bin-packing — too complex for negligible benefit; column fitting is not the bottleneck

**TCDir function mapping → Rust**:

| TCDir Function | Rust Equivalent | Purpose |
|----------------|--------------|--------|
| `ComputeDisplayWidth()` | Inline in `wide.rs` render setup | Per-entry width: filename + brackets + icon + cloud (needs `FileInfo`, `Config`, icon state) |
| `ComputeMedianDisplayWidth()` | `compute_median()` | O(N) median via `select_nth_unstable()` |
| `ComputeColumnLayout()` | `compute_column_layout()` | Orchestrator: outlier detection + conditional truncation |
| `FitColumns()` | `fit_columns()` | Try column counts max→2, return first that fits |
| `TryColumnCount()` | `try_column_count()` | Per-column widths + gap distribution for a given count |
| `DisplayFile()` (truncation) | Inline in render loop | Right-truncate outlier names with `…` |

## R2: Outlier Truncation Threshold

**Decision**: `max(2 × median_display_width, 40)` — matching TCDir's implementation.

**Rationale**: The floor of 40 prevents aggressive truncation in directories of short filenames. A floor of 20 (original spec draft) would truncate common filenames like `Microsoft.PowerShell.Core.psd1` (34 chars) even when the median is only 10 — too aggressive.

**Key behavior**: Truncation is conditional — only applied when it produces more columns than the untruncated layout. This avoids losing filename information for no visual benefit.

## R3: Median Computation

**Decision**: Use `slice::select_nth_unstable()` for O(N) median computation (Rust stdlib equivalent of C++ `nth_element`).

**Rationale**: No allocation needed beyond the existing display widths vector. O(N) average case, O(N²) worst case but irrelevant for directory sizes.

**Alternatives considered**:
- Full sort + index — O(N log N), wasteful when only the median is needed
- Streaming median — unnecessarily complex for a one-shot computation

## R4: Gap Distribution & Safety Reserve

**Decision**: After computing per-column minimum widths, distribute `console_width - total_width - 1` leftover chars evenly across `(columns - 1)` inter-column gaps. The `-1` safety reserve prevents the last column's widest entry from pushing the cursor to the exact console edge, which would trigger a terminal line-wrap before the explicit newline.

**Rationale**: Matches TCDir exactly. The 1-char safety reserve is critical for correct terminal behavior on Windows.

**Distribution formula**:
- `extra_per_gap = leftover / (columns - 1)`
- `remainder = leftover % (columns - 1)`
- First `remainder` gaps get +1 additional char

## R5: Cloud Status Width Discrepancy

**Decision**: Use RCDir's existing cloud status width values (+3 non-icon, +4 icon mode) for per-entry width computation, NOT TCDir's +2.

**Rationale**: RCDir's `display_cloud_status_symbol()` renders differently from TCDir's — it uses leading/trailing spaces that produce 3 or 4 visual columns. Changing this would affect all display modes, which is out of scope for 009. The variable-width algorithm must use the actual rendered width. Output parity tests will catch any discrepancies.

**Follow-up**: If output parity tests reveal mismatches in wide mode cloud status rendering, file a separate fix.

## R6: Filename Truncation Method

**Decision**: Right-truncation with `…` (U+2026) — remove `(cch_name - cap) + 1` characters from the end of the display name, append `…`.

**Rationale**: Matches TCDir. Right-truncation is simpler than middle-truncation (which is used for link target paths in spec 008). For filenames, the left portion (beginning of the name) is the most recognizable part, so preserving it makes sense.

**Reuse**: The `ELLIPSIS` constant from `path_ellipsis.rs` can be reused. The truncation logic itself is different (right-truncate vs. middle-truncate) so no code reuse beyond the constant.

## R7: Module Organization

**Decision**: Create `src/results_displayer/column_layout.rs` as a new submodule containing all pure layout functions. Keep rendering logic in `wide.rs`.

**Rationale**:
- Pure functions (no Console, no I/O) → fully testable with synthetic data
- Clear separation: layout computation vs. rendering
- Follows existing pattern (e.g., `path_ellipsis.rs` is pure, displayers consume its output)
- `ColumnLayout` struct returned from layout functions consumed by `wide.rs` render loop

**Alternatives considered**:
- Inline everything in `wide.rs` — mixes layout logic with rendering, harder to test
- Separate crate — overkill for a single module
