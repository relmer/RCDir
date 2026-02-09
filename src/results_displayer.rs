// results_displayer.rs — Display formatting for directory listings
//
// Port of: IResultsDisplayer.h, ResultsDisplayerNormal.h/.cpp, ResultsDisplayerBare.h/.cpp
//
// Stub — full implementation in US-3 and US-6.

use crate::directory_info::DirectoryInfo;
use crate::drive_info::DriveInfo;
use crate::listing_totals::ListingTotals;

/// Directory level for display formatting.
/// Port of: IResultsDisplayer::EDirectoryLevel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectoryLevel {
    Initial,
    Subdirectory,
}

/// Trait for displaying directory listing results.
/// Port of: IResultsDisplayer
pub trait ResultsDisplayer {
    fn display_results(&mut self, drive_info: &DriveInfo, dir_info: &DirectoryInfo, level: DirectoryLevel);
    fn display_recursive_summary(&mut self, dir_info: &DirectoryInfo, totals: &ListingTotals);
}
