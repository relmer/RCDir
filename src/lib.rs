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
pub mod icon_mapping;
pub mod nerd_font_detector;
pub mod file_attribute_map;





use std::ffi::OsStr;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use std::sync::Arc;

use ehm::AppError;





////////////////////////////////////////////////////////////////////////////////
//
//  run
//
//  Main entry point for the library.
//  Called by main.rs; returns Result for clean error handling.
//
////////////////////////////////////////////////////////////////////////////////

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

    // Resolve icon activation (CLI → env var → auto-detect)
    let icons_active = resolve_icons (&cmd, &cfg);

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

    ////////////////////////////////////////////////////////////////////////////////

    let cmd = Arc::new(cmd);

    // Default mask: "*" if none specified
    let masks = if cmd.masks.is_empty() {
        vec![std::ffi::OsString::from("*")]
    } else {
        cmd.masks.clone()
    };

    // Group masks by their target directories
    let groups = mask_grouper::group_masks_by_directory(&masks);

    let mut totals = listing_totals::ListingTotals::default();

    for group in &groups {
        let (dir_path, file_specs) = group;

        // Validate directory exists
        if !dir_path.exists() || !dir_path.is_dir() {
            console.color_printf(&format!(
                "{{Error}}Error:   {{InformationHighlight}}{}{{Error}} does not exist\n",
                dir_path.display(),
            ));
            continue;
        }

        console.puts(config::Attribute::Default, "");

        let drive_info = match drive_info::DriveInfo::new(dir_path) {
            Ok(di) => di,
            Err(_) => {
                console.color_printf(&format!(
                    "{{Error}}Error:   Unable to get drive info for {{InformationHighlight}}{}\n",
                    dir_path.display(),
                ));
                continue;
            }
        };

        // Create the displayer for this listing (bare > wide > normal priority)
        let mut displayer = results_displayer::Displayer::new(
            console,
            Arc::clone(&cmd),
            Arc::clone(&cfg),
            icons_active,
        );

        if cmd.multi_threaded && cmd.recurse {
            ////////////////////////////////////////////////////////////////////////////////
            let mut mt_lister = multi_threaded_lister::MultiThreadedLister::new(
                Arc::clone(&cmd),
                Arc::clone(&cfg),
            );

            mt_lister.process(
                &drive_info,
                dir_path,
                file_specs,
                &mut displayer,
                &mut totals,
            );

            // Build a summary DirectoryInfo for the recursive summary display
            let spec_strings: Vec<String> = file_specs.iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect();
            let summary_di = directory_info::DirectoryInfo::new_multi(dir_path.clone(), spec_strings);

            use results_displayer::ResultsDisplayer;
            displayer.display_recursive_summary(&summary_di, &totals);

            mt_lister.stop_workers();
        } else {
            ////////////////////////////////////////////////////////////////////////////////
            for file_spec in file_specs {
                let spec_str = file_spec.to_string_lossy().to_string();
                let mut di = directory_info::DirectoryInfo::new(dir_path.clone(), spec_str);

                // Enumerate matching files
                directory_lister::collect_matching_files(
                    dir_path,
                    file_spec.as_os_str(),
                    &mut di,
                    &cmd,
                    &mut totals,
                    &cfg,
                );

                // Track directory count in totals
                totals.directory_count += di.subdirectory_count;

                // Sort results
                file_comparator::sort_files(&mut di.matches, &cmd);

                // Display results
                use results_displayer::{ResultsDisplayer, DirectoryLevel};
                displayer.display_results(&drive_info, &di, DirectoryLevel::Initial);

                // Recurse into subdirectories if /S switch (single-threaded)
                if cmd.recurse {
                    recurse_into_subdirectories(
                        &drive_info,
                        dir_path,
                        file_spec.as_os_str(),
                        &cmd,
                        &cfg,
                        &mut totals,
                        &mut displayer,
                    );

                    // Show recursive summary at the end of the initial directory
                    displayer.display_recursive_summary(&di, &totals);
                }
            }
        }

        // Recover the console from the displayer
        console = displayer.into_console();
    }

    // Display any RCDIR env var parsing errors at end of output
    // Port of: TCDir.cpp → DisplayEnvVarIssues at end of wmain()
    usage::display_env_var_issues(&mut console, cmd.switch_prefix, true);
    console.flush()?;

    // Performance timer output — spec A.11: "RCDir time elapsed:  X.XX msec\n"
    if cmd.perf_timer {
        timer.stop();
        console.printf_attr(config::Attribute::Default, &format!("RCDir time elapsed:  {:.2} msec\n", timer.elapsed_ms()));
        console.flush()?;
    }

    Ok(())
}





////////////////////////////////////////////////////////////////////////////////
//
//  recurse_into_subdirectories
//
//  Recurse into subdirectories applying the same file spec.
//  Port of: CDirectoryLister::RecurseIntoSubdirectories
//
////////////////////////////////////////////////////////////////////////////////

fn recurse_into_subdirectories(
    drive_info: &drive_info::DriveInfo,
    dir_path: &Path,
    file_spec: &OsStr,
    cmd: &Arc<command_line::CommandLine>,
    cfg: &Arc<config::Config>,
    totals: &mut listing_totals::ListingTotals,
    displayer: &mut results_displayer::Displayer,
) {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{FindFirstFileW, FindNextFileW, WIN32_FIND_DATAW};
    use crate::file_info::{FindHandle, FILE_ATTRIBUTE_DIRECTORY};
    use crate::results_displayer::{ResultsDisplayer, DirectoryLevel};

    // Search for all entries with "*" to find subdirectories
    let mut search_path = dir_path.to_path_buf();
    search_path.push("*");
    let search_wide: Vec<u16> = search_path.as_os_str().encode_wide().chain(Some(0)).collect();

    let mut wfd = WIN32_FIND_DATAW::default();
    let handle = unsafe { FindFirstFileW(windows::core::PCWSTR(search_wide.as_ptr()), &mut wfd) };
    let handle = match handle {
        Ok(h) if !h.is_invalid() => h,
        _ => return,
    };
    let _find_handle = FindHandle(handle);

    loop {
        // Check if this is a directory (not "." or "..")
        if (wfd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0 {
            let name_len = wfd.cFileName.iter().position(|&c| c == 0).unwrap_or(0);
            let name = std::ffi::OsString::from_wide(&wfd.cFileName[..name_len]);
            let name_str = name.to_string_lossy();

            if name_str != "." && name_str != ".." {
                let subdir_path = dir_path.join(&name);
                let spec_str = file_spec.to_string_lossy().to_string();
                let mut di = directory_info::DirectoryInfo::new(subdir_path.clone(), spec_str);

                // Enumerate matching files in subdirectory
                directory_lister::collect_matching_files(
                    &subdir_path,
                    file_spec,
                    &mut di,
                    cmd,
                    totals,
                    cfg,
                );

                totals.directory_count += di.subdirectory_count;

                // Sort results
                file_comparator::sort_files(&mut di.matches, cmd);

                // Display results (Subdirectory level — skips empty dirs)
                displayer.display_results(drive_info, &di, DirectoryLevel::Subdirectory);

                // Continue recursion depth-first
                recurse_into_subdirectories(
                    drive_info,
                    &subdir_path,
                    file_spec,
                    cmd,
                    cfg,
                    totals,
                    displayer,
                );
            }
        }

        let success = unsafe { FindNextFileW(handle, &mut wfd) };
        if success.is_err() {
            break;
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  resolve_icons
//
//  Determine whether icons should be active.  Priority cascade:
//    1. CLI flag (/Icons or /Icons-)  — always wins
//    2. RCDIR env var (Icons / Icons-) — second priority
//    3. Auto-detect via NerdFontDetector — third priority
//
//  Port of: CreateDisplayer() icon activation in TCDirCore/TCDir.cpp
//
////////////////////////////////////////////////////////////////////////////////

pub fn resolve_icons(cmd: &command_line::CommandLine, cfg: &config::Config) -> bool {
    // CLI flag always wins
    if let Some(cli_icons) = cmd.icons {
        return cli_icons;
    }

    // RCDIR env var Icons/Icons- switch
    if let Some(env_icons) = cfg.icons {
        return env_icons;
    }

    // Auto-detect: probe console font / enumerate system fonts
    let console_handle = unsafe {
        windows::Win32::System::Console::GetStdHandle (
            windows::Win32::System::Console::STD_OUTPUT_HANDLE,
        )
    };

    let console_handle = match console_handle {
        Ok(h) if !h.is_invalid() => h,
        _ => return false,
    };

    let prober = nerd_font_detector::DefaultFontProber;
    let env    = environment_provider::DefaultEnvironmentProvider;

    let result = nerd_font_detector::detect (console_handle, &env, &prober);

    matches!(result, nerd_font_detector::DetectionResult::Detected)
}
