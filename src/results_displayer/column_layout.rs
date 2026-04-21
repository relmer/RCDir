// column_layout.rs — Variable-width column layout algorithm for wide mode
//
// Pure functions for computing per-column widths when entries have varying
// display widths.  Tries candidate column counts from maximum feasible down
// to 2, optionally truncates outlier-length entries (when ellipsis is enabled
// and doing so produces more columns), and distributes leftover space evenly
// across inter-column gaps.
//
// Port of: CResultsDisplayerWide::ComputeColumnLayout / FitColumns /
//          TryColumnCount / ComputeMedianDisplayWidth

use std::cmp::{max, min};





////////////////////////////////////////////////////////////////////////////////
//
//  ColumnLayout
//
//  Output of the layout algorithm.  Consumed by the render loop in wide.rs.
//
////////////////////////////////////////////////////////////////////////////////

pub struct ColumnLayout {
    /// Number of columns in the layout (≥ 1).
    pub columns:       usize,

    /// Number of rows in the layout (≥ 1).
    pub rows:          usize,

    /// Per-column display width including distributed gap space.
    /// Length == `columns`.
    pub column_widths: Vec<usize>,

    /// Outlier truncation cap in display chars.  0 = no truncation.
    pub trunc_cap:     usize,
}





////////////////////////////////////////////////////////////////////////////////
//
//  compute_median
//
//  Compute the median value of a slice using O(N) `select_nth_unstable`.
//  Returns 0 for empty input.
//
////////////////////////////////////////////////////////////////////////////////

pub fn compute_median (widths: &[usize]) -> usize {
    if widths.is_empty() {
        return 0;
    }

    let mut buf = widths.to_vec();
    let mid = buf.len() / 2;
    buf.select_nth_unstable (mid);
    buf[mid]
}





////////////////////////////////////////////////////////////////////////////////
//
//  try_column_count
//
//  Attempt to fit entries into `num_cols` columns.  Computes per-column
//  widths using column-major entry mapping, adds +1 base gap for non-last
//  columns, checks total fits (< console_width), and distributes leftover
//  space evenly across inter-column gaps with a 1-char safety reserve.
//
//  Returns `None` if the layout doesn't fit.
//
////////////////////////////////////////////////////////////////////////////////

pub fn try_column_count (widths: &[usize], console_width: usize, num_cols: usize) -> Option<ColumnLayout> {
    let num_entries       = widths.len();
    let num_rows          = num_entries.div_ceil (num_cols);
    let items_in_last_row = num_entries % num_cols;
    let full_cols         = if items_in_last_row != 0 { items_in_last_row } else { num_cols };
    let entries_in_full   = full_cols * num_rows;

    // Compute per-column widths using the same column-major mapping
    // as the render loop in wide.rs.  The first `full_cols` columns
    // have `num_rows` entries each; remaining columns have `num_rows - 1`.

    let mut col_widths = vec![0usize; num_cols];

    for (i, &entry_width) in widths.iter().enumerate() {
        let col = if i < entries_in_full {
            i / num_rows
        } else {
            full_cols + (i - entries_in_full) / (num_rows - 1)
        };

        // +1 base gap for all columns except the last
        let w = entry_width + if col < num_cols - 1 { 1 } else { 0 };

        if w > col_widths[col] {
            col_widths[col] = w;
        }
    }

    // Check if total fits.  Reserve 1 char so the last column's widest
    // entry doesn't push the cursor to the exact console edge and trigger
    // a terminal line-wrap before the explicit newline.

    let total_width: usize = col_widths.iter().sum();

    if total_width >= console_width {
        return None;
    }

    // Distribute leftover space evenly across inter-column gaps.
    // Keep 1 char undistributed to maintain the strict-less-than guarantee.

    let leftover = console_width - total_width - 1;

    if num_cols > 1 && leftover > 0 {
        let extra_per_gap = leftover / (num_cols - 1);
        let remainder     = leftover % (num_cols - 1);

        for (c, cw) in col_widths.iter_mut().enumerate().take (num_cols - 1) {
            *cw += extra_per_gap + if c < remainder { 1 } else { 0 };
        }
    }

    Some (ColumnLayout {
        columns:       num_cols,
        rows:          num_rows,
        column_widths: col_widths,
        trunc_cap:     0,
    })
}





////////////////////////////////////////////////////////////////////////////////
//
//  fit_columns
//
//  Try column counts from maximum feasible down to 2, returning the first
//  (highest column count) layout that fits.  Falls back to single-column
//  output if nothing fits.
//
////////////////////////////////////////////////////////////////////////////////

pub fn fit_columns (widths: &[usize], console_width: usize) -> ColumnLayout {
    let num_entries = widths.len();
    let max_cols    = min (num_entries, console_width / 2);

    for n_cols in (2..=max_cols).rev() {
        if let Some (layout) = try_column_count (widths, console_width, n_cols) {
            return layout;
        }
    }

    // Single-column fallback
    ColumnLayout {
        columns:       1,
        rows:          num_entries,
        column_widths: vec![console_width],
        trunc_cap:     0,
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  compute_column_layout
//
//  Top-level orchestrator.  Handles trivial cases, computes the outlier
//  threshold, optionally truncates, and returns the best layout.
//
//  Port of: CResultsDisplayerWide::ComputeColumnLayout
//
////////////////////////////////////////////////////////////////////////////////

pub fn compute_column_layout (widths: &[usize], console_width: usize, ellipsize: bool) -> ColumnLayout {

    // Trivial cases: 0 or 1 entries

    if widths.len() <= 1 {
        return ColumnLayout {
            columns:       1,
            rows:          widths.len(),
            column_widths: vec![console_width],
            trunc_cap:     0,
        };
    }

    // Build effective widths, applying outlier truncation if enabled.
    // Only use truncation if it actually produces more columns than
    // the un-truncated layout — otherwise it hurts readability for no gain.

    if ellipsize {
        let median = compute_median (widths);
        let cap    = max (2 * median, 40);

        let has_outliers = widths.iter().any (|&w| w > cap);

        if has_outliers {
            // Compute layout without truncation first
            let clean_layout = fit_columns (widths, console_width);

            // Compute layout with truncation
            let effective: Vec<usize> = widths.iter().map (|&w| min (w, cap)).collect();
            let mut trunc_layout = fit_columns (&effective, console_width);

            // Only use truncation if it produces more columns
            if trunc_layout.columns > clean_layout.columns {
                trunc_layout.trunc_cap = cap;
                return trunc_layout;
            } else {
                return clean_layout;
            }
        }
    }

    // No truncation needed — fit with original widths
    fit_columns (widths, console_width)
}





////////////////////////////////////////////////////////////////////////////////
//
//  Unit tests
//
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;





    ////////////////////////////////////////////////////////////////////////////
    //
    //  compute_median tests (T007)
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn median_empty_returns_zero() {
        assert_eq! (compute_median (&[]), 0);
    }

    #[test]
    fn median_single_element() {
        assert_eq! (compute_median (&[42]), 42);
    }

    #[test]
    fn median_odd_count() {
        assert_eq! (compute_median (&[3, 1, 2]), 2);
    }

    #[test]
    fn median_even_count() {
        // For even count, we take the upper-middle element (len/2)
        let result = compute_median (&[10, 20, 30, 40]);
        assert_eq! (result, 30);
    }

    #[test]
    fn median_all_same() {
        assert_eq! (compute_median (&[7, 7, 7, 7, 7]), 7);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  try_column_count tests (T008)
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn try_two_entries_two_columns() {
        let widths = vec![10, 10];
        let layout = try_column_count (&widths, 80, 2).unwrap();
        assert_eq! (layout.columns, 2);
        assert_eq! (layout.rows, 1);
    }

    #[test]
    fn try_ten_entries_three_columns() {
        let widths = vec![8; 10];
        let layout = try_column_count (&widths, 80, 3).unwrap();
        assert_eq! (layout.columns, 3);
        assert_eq! (layout.rows, 4);
    }

    #[test]
    fn try_layout_doesnt_fit_returns_none() {
        let widths = vec![40, 40, 40];
        assert! (try_column_count (&widths, 80, 3).is_none());
    }

    #[test]
    fn try_variable_widths_total_respects_safety_reserve() {
        // Col 0 gets entries [5, 5], col 1 gets entries [20, 20].
        // After gap distribution the total must equal console_width - 1.
        let widths = vec![5, 5, 20, 20];
        let layout = try_column_count (&widths, 80, 2).unwrap();
        assert_eq! (layout.columns, 2);
        let total: usize = layout.column_widths.iter().sum();
        assert_eq! (total, 79);
    }

    #[test]
    fn try_gap_distribution() {
        // 2 entries of width 10, console 80.  Col 0 needs 10+1=11 (base gap),
        // col 1 needs 10.  Total = 21.  Leftover = 80 - 21 - 1 = 58.
        // Extra per gap = 58 / 1 = 58.  Col 0 gets 11 + 58 = 69.
        let widths = vec![10, 10];
        let layout = try_column_count (&widths, 80, 2).unwrap();
        let total: usize = layout.column_widths.iter().sum();
        assert! (total < 80, "total {} should be < 80", total);
        assert_eq! (total, 79); // 80 - 1 safety reserve
    }

    #[test]
    fn try_column_major_ordering_10_entries_3_cols() {
        // 10 entries, 3 cols → 4 rows, items_in_last_row=1
        // Col 0: entries 0,1,2,3 (4 items)
        // Col 1: entries 4,5,6 (3 items)
        // Col 2: entries 7,8,9 (3 items)
        //
        // Verify by making each entry's width unique and checking
        // which column gets which max width.
        let widths: Vec<usize> = (0..10).map (|i| 10 + i).collect();
        // Entry 0=10, 1=11, 2=12, 3=13, 4=14, 5=15, 6=16, 7=17, 8=18, 9=19
        let layout = try_column_count (&widths, 200, 3).unwrap();
        assert_eq! (layout.columns, 3);
        assert_eq! (layout.rows, 4);

        // Total after distribution = 199 (200 - 1 safety)
        let total: usize = layout.column_widths.iter().sum();
        assert_eq! (total, 199);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  fit_columns tests (T009)
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn fit_uniform_widths() {
        let widths = vec![10; 20];
        let layout = fit_columns (&widths, 80);
        assert! (layout.columns >= 2);
        // Non-last columns should be consistent within 1 of each other
        if layout.columns > 2 {
            let non_last: Vec<usize> = layout.column_widths[..layout.columns - 1].to_vec();
            let min_w = *non_last.iter().min().unwrap();
            let max_w = *non_last.iter().max().unwrap();
            assert! (max_w - min_w <= 1, "non-last cols should be within 1 of each other");
        }
        let total: usize = layout.column_widths.iter().sum();
        assert! (total < 80);
    }

    #[test]
    fn fit_mixed_widths_variable_columns() {
        let mut widths = vec![8; 19];
        widths.push (30);
        let layout = fit_columns (&widths, 80);
        assert! (layout.columns >= 2, "should fit at least 2 columns");
    }

    #[test]
    fn fit_single_entry() {
        let widths = vec![50];
        let layout = fit_columns (&widths, 80);
        assert_eq! (layout.columns, 1);
        assert_eq! (layout.rows, 1);
    }

    #[test]
    fn fit_narrow_console_fallback() {
        let widths = vec![30, 30, 30];
        let layout = fit_columns (&widths, 40);
        assert_eq! (layout.columns, 1);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  compute_column_layout tests (T010)
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn layout_no_outliers_ellipsize_has_no_effect() {
        let widths = vec![10; 20];
        let with    = compute_column_layout (&widths, 120, true);
        let without = compute_column_layout (&widths, 120, false);
        assert_eq! (with.columns, without.columns);
        assert_eq! (with.trunc_cap, 0);
        assert_eq! (without.trunc_cap, 0);
    }

    #[test]
    fn layout_outliers_with_ellipsize_more_columns() {
        // 50 short entries (width 10) + 5 long entries (width 80)
        // Median ≈ 10, cap = max(20, 40) = 40
        let mut widths = vec![10; 50];
        widths.extend (vec![80; 5]);
        let layout = compute_column_layout (&widths, 120, true);
        assert! (layout.trunc_cap > 0, "should have truncation cap set");
        assert_eq! (layout.trunc_cap, 40);

        let no_trunc = compute_column_layout (&widths, 120, false);
        assert_eq! (no_trunc.trunc_cap, 0);
        assert! (layout.columns > no_trunc.columns,
            "truncated {} cols should be > non-truncated {} cols",
            layout.columns, no_trunc.columns);
    }

    #[test]
    fn layout_outliers_ellipsize_disabled() {
        let mut widths = vec![10; 50];
        widths.extend (vec![80; 5]);
        let layout = compute_column_layout (&widths, 120, false);
        assert_eq! (layout.trunc_cap, 0, "no truncation when ellipsize disabled");
    }

    #[test]
    fn layout_outliers_truncation_doesnt_help() {
        let widths = vec![5, 5, 50, 50, 50, 50, 50, 50, 50, 50];
        // median ≈ 50, cap = max(100, 40) = 100 — no entries exceed 100
        let layout = compute_column_layout (&widths, 120, true);
        assert_eq! (layout.trunc_cap, 0);
    }

    #[test]
    fn layout_trivial_zero_entries() {
        let layout = compute_column_layout (&[], 80, true);
        assert_eq! (layout.columns, 1);
        assert_eq! (layout.rows, 0);
    }

    #[test]
    fn layout_trivial_one_entry() {
        let layout = compute_column_layout (&[50], 80, true);
        assert_eq! (layout.columns, 1);
        assert_eq! (layout.rows, 1);
        assert_eq! (layout.trunc_cap, 0);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  Display width scenario tests (T016)
    //
    //  Verify the layout algorithm handles varying per-entry display widths
    //  simulating icons, cloud status, and directory brackets.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn width_entries_with_icons_plus_2() {
        // Simulate 5 files with icon (+2): base 10 + 2 = 12
        let widths = vec![12; 5];
        let layout = fit_columns (&widths, 80);
        assert! (layout.columns >= 2);
    }

    #[test]
    fn width_suppressed_icons_still_plus_2() {
        // Suppressed icons still consume +2 (placeholder spaces).
        let widths = vec![12; 5];
        let layout = fit_columns (&widths, 80);
        assert! (layout.columns >= 2);
    }

    #[test]
    fn width_cloud_status_icon_mode_plus_4() {
        // Icon mode: base 10 + icon 2 + cloud 4 = 16
        let widths = vec![16; 5];
        let layout = fit_columns (&widths, 80);
        assert! (layout.columns >= 2);
    }

    #[test]
    fn width_cloud_status_non_icon_mode_plus_3() {
        // Non-icon mode: base 10 + cloud 3 = 13
        let widths = vec![13; 5];
        let layout = fit_columns (&widths, 80);
        assert! (layout.columns >= 2);
    }

    #[test]
    fn width_directory_brackets_plus_2() {
        // Dirs with brackets: base 8 + 2 = 10
        let widths = vec![10; 5];
        let layout = fit_columns (&widths, 80);
        assert! (layout.columns >= 2);
    }

    #[test]
    fn width_mixed_entries_widest_determines_column() {
        // Column 0 gets entries [10, 10, 10, 10] (4 entries)
        // Column 1 gets entries [10, 10, 30] (3 entries, one wide)
        // The wide entry (30) should determine column 1's width.
        let widths = vec![10, 10, 10, 10, 10, 10, 30];
        let layout = try_column_count (&widths, 80, 2).unwrap();
        assert_eq! (layout.columns, 2);
        assert! (layout.column_widths[1] >= 30,
            "col1 width {} should be >= 30", layout.column_widths[1]);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  Edge case tests (T018)
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn edge_narrow_console_40() {
        let widths = vec![15; 10];
        let layout = compute_column_layout (&widths, 40, true);
        // 15+1+15 = 31 < 40 → should fit 2 cols
        assert! (layout.columns >= 2, "should fit 2 cols in 40-wide console");
    }

    #[test]
    fn edge_single_entry_wider_than_console() {
        let widths = vec![100];
        let layout = compute_column_layout (&widths, 40, true);
        assert_eq! (layout.columns, 1);
        assert_eq! (layout.rows, 1);
    }

    #[test]
    fn edge_all_same_width_identical_to_uniform() {
        let widths = vec![20; 10];
        let layout = compute_column_layout (&widths, 100, true);
        if layout.columns > 2 {
            let non_last: Vec<usize> = layout.column_widths[..layout.columns - 1].to_vec();
            let min_w = *non_last.iter().min().unwrap();
            let max_w = *non_last.iter().max().unwrap();
            assert! (max_w - min_w <= 1, "uniform widths should give near-equal non-last cols");
        }
    }

    #[test]
    fn edge_exactly_two_entries() {
        let widths = vec![10, 20];
        let layout = compute_column_layout (&widths, 80, true);
        assert_eq! (layout.columns, 2);
        assert_eq! (layout.rows, 1);
    }

    #[test]
    fn edge_console_width_1() {
        let widths = vec![5, 10];
        let layout = compute_column_layout (&widths, 1, true);
        assert_eq! (layout.columns, 1);
    }
}
