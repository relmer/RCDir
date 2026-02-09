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
pub mod usage;

use std::sync::Arc;

use ehm::AppError;

/// Main entry point for the library.
/// Called by main.rs; returns Result for clean error handling.
pub fn run() -> Result<(), AppError> {
    // Start performance timer immediately
    let mut timer = perf_timer::PerfTimer::new();
    timer.start();

    // Parse command line
    let args: Vec<String> = std::env::args().collect();
    let cmd = command_line::CommandLine::parse_from(args.iter().skip(1))?;

    // Initialize configuration
    let mut cfg = config::Config::new();
    cfg.initialize(0x07); // default: LightGrey on Black

    // Apply config defaults from RCDIR env var to command line
    let mut cmd = cmd;
    cmd.apply_config_defaults(&cfg);

    // Wrap config in Arc for shared ownership with Console
    let cfg = Arc::new(cfg);

    // Initialize console
    let mut console = console::Console::initialize(Arc::clone(&cfg))?;

    // Help early exits — show requested help and return
    if cmd.show_help {
        usage::display_usage(&mut console, cmd.switch_prefix);
        console.flush()?;
        return Ok(());
    }

    if cmd.show_env_help {
        usage::display_env_var_help(&mut console, cmd.switch_prefix);
        console.flush()?;
        return Ok(());
    }

    if cmd.show_config {
        usage::display_current_configuration(&mut console, cmd.switch_prefix);
        console.flush()?;
        return Ok(());
    }

    // TODO: US-1 directory listing pipeline will go here

    // Performance timer output — spec A.11: "RCDir time elapsed:  X.XX msec\n"
    if cmd.perf_timer {
        timer.stop();
        console.printf_attr(config::Attribute::Default, &format!("RCDir time elapsed:  {:.2} msec\n", timer.elapsed_ms()));
        console.flush()?;
    }

    Ok(())
}
