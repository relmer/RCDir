# Data Model: Variable-Width Columns in Wide Mode

**Date**: 2026-04-20 | **Feature**: 009-variable-width-columns

## Entities

### ColumnLayout

Central output of the layout algorithm. Consumed by the render loop in `wide.rs`.

| Field | Type | Description |
|-------|------|-------------|
| `columns` | `usize` | Number of columns in the layout (≥1) |
| `rows` | `usize` | Number of rows in the layout (≥1) |
| `column_widths` | `Vec<usize>` | Per-column display width including gap distribution. Length = `columns`. |
| `trunc_cap` | `usize` | Outlier truncation cap in display chars. 0 = no truncation. |

**Invariants**:
- `column_widths.len() == columns`
- `column_widths.iter().sum() < console_width` (strict less-than for safety reserve)
- `rows * columns >= total_entries` (may have empty slots in last row)
- When `columns == 1`: `column_widths[0] == console_width`, `trunc_cap == 0`

### EntryDisplayWidth (conceptual — computed inline, not stored as a struct)

Per-entry display width computation. Not a struct — computed as `usize` by `compute_display_width()`.

| Component | Condition | Width |
|-----------|-----------|-------|
| Filename | Always | `filename.len()` |
| Directory brackets | `is_directory && !icons_active` | +2 |
| Icon + space | `icons_active && !icon_suppressed` | +2 |
| Cloud status | `in_sync_root` (icon mode) | +4 |
| Cloud status | `in_sync_root` (non-icon mode) | +3 |

## Relationships

```text
DirectoryInfo.matches[]  ──compute_display_width()──►  Vec<usize>  (display widths)
                                                            │
                                                            ▼
                                              compute_column_layout()
                                              (+ optional outlier truncation)
                                                            │
                                                            ▼
                                                      ColumnLayout
                                                            │
                                                            ▼
                                              wide.rs render loop
                                              (column-major grid output)
```

## State Transitions

N/A — the layout algorithm is stateless. `ColumnLayout` is computed once per directory and consumed immediately by the render loop.

## Validation Rules

- `console_width` must be ≥ 1 (enforced by Console API)
- `total_entries` must be ≥ 1 (caller checks for empty directory before invoking)
- `trunc_cap` is only set when truncation actually improves column count
- Column widths must never exceed `console_width` in total (strict less-than due to safety reserve)
