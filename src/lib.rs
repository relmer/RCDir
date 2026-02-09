// RCDir - Rust Technicolor Directory
// A fast, colorized directory listing tool for Windows

pub mod ehm;
pub mod ansi_codes;
pub mod color;
pub mod environment_provider;
pub mod console;
pub mod command_line;
pub mod config;
pub mod file_info;
pub mod directory_info;
pub mod drive_info;
pub mod mask_grouper;
pub mod listing_totals;
pub mod perf_timer;
pub mod file_comparator;
pub mod directory_lister;
pub mod multi_threaded_lister;
pub mod work_queue;
pub mod results_displayer;
pub mod cloud_status;
pub mod streams;
pub mod owner;

use ehm::AppError;

/// Main entry point for the library.
/// Called by main.rs; returns Result for clean error handling.
pub fn run() -> Result<(), AppError> {
    // Stub — will be wired up as user stories are implemented
    Ok(())
}
