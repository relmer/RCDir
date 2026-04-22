# Quickstart: Variable-Width Columns in Wide Mode

**Date**: 2026-04-20 | **Feature**: 009-variable-width-columns

## Implementation Order

### Phase 1: Column Layout Algorithm (`column_layout.rs`)

1. Create `src/results_displayer/column_layout.rs`
2. Define `ColumnLayout` struct
3. Implement `compute_display_width()` — per-entry width calculation
4. Implement `compute_median()` — O(N) median via `select_nth_unstable()`
5. Implement `try_column_count()` — per-column widths + gap distribution for a candidate count
6. Implement `fit_columns()` — try column counts from max feasible down to 1
7. Implement `compute_column_layout()` — orchestrator with conditional outlier truncation
8. Register module in `src/results_displayer/mod.rs`
9. Write unit tests for all pure functions with synthetic data

### Phase 2: Integrate into Wide Displayer (`wide.rs`)

1. Replace uniform-width column calculation with `compute_column_layout()` call
2. Build per-entry display widths vector (reusing existing width logic)
3. Update render loop to use per-column widths from `ColumnLayout`
4. Add outlier truncation at render time (right-truncate with `…`)
5. Pass `ellipsize` flag from `CommandLine` to layout computation

### Phase 3: Testing & Parity

1. Unit tests for `column_layout.rs` — synthetic data covering all edge cases
2. Output parity tests — compare `rcdir /W` vs `tcdir /W` on test directories
3. Verify edge cases: narrow terminal, single file, uniform filenames, empty directory

## Key Files

| File | Action | Purpose |
|------|--------|---------|
| `src/results_displayer/column_layout.rs` | **CREATE** | Pure layout algorithm |
| `src/results_displayer/mod.rs` | **MODIFY** | Register `column_layout` module |
| `src/results_displayer/wide.rs` | **MODIFY** | Use `ColumnLayout` for rendering |
| `tests/output_parity.rs` | **MODIFY** | Add wide-mode parity test cases |

## Critical Implementation Details

### Column-Major Index Formula

The existing column-major mapping in `wide.rs` (lines 228-243) is correct and must be preserved exactly:

```rust
let full_rows = if items_in_last_row != 0 { rows - 1 } else { rows };
let mut idx = row + (col * full_rows);
if col < items_in_last_row { idx += col; } else { idx += items_in_last_row; }
```

The same formula is used in `try_column_count()` to map entries to columns during width computation.

### Safety Reserve

`TryColumnCount` checks `total_width >= console_width` (strict). The `-1` in leftover calculation (`console_width - total_width - 1`) ensures the last column never pushes the cursor to the exact console edge.

### Conditional Truncation

Truncation is NOT automatic. The algorithm:
1. Computes layout without truncation
2. If outliers exist and ellipsis is enabled, computes layout with truncation
3. Uses truncated layout ONLY if it produces more columns
4. Otherwise uses the original untruncated layout

### Cloud Status Width

RCDir uses +3 (non-icon) or +4 (icon) for cloud status — different from TCDir's +2. This is a pre-existing rendering difference and must be preserved in the per-entry width computation.
