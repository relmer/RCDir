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
