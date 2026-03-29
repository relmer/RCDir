// src/alias_manager.rs — Top-level orchestrator for --set/--get/--remove-aliases

use std::path::PathBuf;

use crate::alias_types::*;
use crate::alias_block_generator;
use crate::config::Attribute;
use crate::command_line::CommandLine;
use crate::console::Console;
use crate::ehm::AppError;
use crate::profile_file_manager;
use crate::profile_path_resolver;
use crate::tui_widgets::{self, TuiResult};





////////////////////////////////////////////////////////////////////////////////
//
//  Default sub-alias suffix definitions.
//
////////////////////////////////////////////////////////////////////////////////

const SUB_ALIAS_DEFS: &[(&str, &str, &str)] = &[
    ("t",  "--tree",  "Tree view"),
    ("w",  "-w",      "Wide format"),
    ("d",  "/a:d",    "Directories only"),
    ("s",  "-s",      "Recursive"),
    ("sb", "-s -b",   "Recursive bare"),
];





////////////////////////////////////////////////////////////////////////////////
//
//  run
//
//  Dispatch to the appropriate alias sub-command based on parsed switches.
//
////////////////////////////////////////////////////////////////////////////////

pub fn run (cmd: &CommandLine, console: &mut Console) -> Result<(), AppError> {
    if cmd.set_aliases {
        set_aliases (console, cmd.what_if)?;
    } else if cmd.get_aliases {
        get_aliases (console)?;
    } else if cmd.remove_aliases {
        remove_aliases (console, cmd.what_if)?;
    }

    console.flush()?;
    Ok(())
}





////////////////////////////////////////////////////////////////////////////////
//
//  set_aliases
//
//  Interactive wizard for configuring PowerShell aliases.
//
////////////////////////////////////////////////////////////////////////////////

fn set_aliases (console: &mut Console, what_if: bool) -> Result<(), AppError> {
    let version = detect_and_validate_ps (console)?;
    let parent_path = profile_path_resolver::get_parent_process_path_public()?;
    let mut locations = profile_path_resolver::resolve_profile_paths (version, &parent_path)?;
    let rcdir_invocation = profile_path_resolver::resolve_rcdir_invocation()?;

    // Scan for existing alias blocks
    let mut existing_block: Option<AliasBlock> = None;

    for loc in locations.iter_mut() {
        if !loc.exists { continue; }
        if let Ok ((lines, _)) = profile_file_manager::read_profile_file (&loc.resolved_path) {
            let block = profile_file_manager::find_alias_block (&lines);
            if block.found {
                loc.has_alias_block = true;
                if existing_block.is_none() {
                    existing_block = Some (block);
                }
            }
        }
    }

    // Header
    console.printf_attr (Attribute::InformationHighlight, "\n  RCDir Alias Setup");
    if what_if {
        console.printf_attr (Attribute::Error, "  (Whatif: preview only, no changes will be made)");
    }
    console.printf_attr (Attribute::Information, "\n\n");
    console.printf_attr (Attribute::Information,
        "  This wizard configures PowerShell aliases so you can invoke rcdir\n\
         \x20 with short commands (e.g., 'd' instead of 'rcdir'). Aliases will be\n\
         \x20 saved to your PowerShell profile and loaded automatically.\n\n");
    console.flush()?;

    // Step 1: Root alias name
    let default_root = existing_block.as_ref()
        .map (|b| b.root_alias.clone())
        .unwrap_or_else (|| "d".to_string());

    let root_alias = match tui_widgets::text_input (console, "Root alias name (1-4 chars)", &default_root)? {
        TuiResult::Confirmed (val) => val,
        TuiResult::Cancelled       => { print_cancelled (console); return Ok(()); }
    };

    // Step 2: Sub-aliases
    let sub_items: Vec<(String, bool)> = SUB_ALIAS_DEFS.iter().map (|(suffix, flags, desc)| {
        let name = format! ("{}{}", root_alias, suffix);
        let label = format! ("{}  = {} {:<7} ({})", name, root_alias, flags, desc);
        let enabled = existing_block.as_ref()
            .map (|b| b.alias_names.contains (&name))
            .unwrap_or (true);
        (label, enabled)
    }).collect();

    console.printf_attr (Attribute::Information, "\n  Select sub-aliases:\n");
    for _ in 0..sub_items.len() { console.printf_attr (Attribute::Information, "\n"); }
    console.flush()?;

    let sub_enabled = match tui_widgets::checkbox_list (console, &sub_items)? {
        TuiResult::Confirmed (states) => states,
        TuiResult::Cancelled           => { print_cancelled (console); return Ok(()); }
    };

    // Step 3: Conflict detection
    let mut all_names = vec![root_alias.clone()];
    for (i, (suffix, _, _)) in SUB_ALIAS_DEFS.iter().enumerate() {
        if sub_enabled[i] {
            all_names.push (format! ("{}{}", root_alias, suffix));
        }
    }
    check_alias_conflicts (console, &all_names)?;

    // Step 4: Profile location
    console.printf_attr (Attribute::Information, "\n  Save aliases to:");
    if what_if {
        console.printf_attr (Attribute::Error, " (Whatif: no changes will be written)");
    }
    console.printf_attr (Attribute::Information, "\n");

    let (radio_items, default_idx) = build_profile_labels (&locations, console.width() as usize);
    for _ in 0..radio_items.len() { console.printf_attr (Attribute::Information, "\n"); }
    console.flush()?;

    let selected_idx = match tui_widgets::radio_button_list (console, &radio_items, default_idx)? {
        TuiResult::Confirmed (idx) => idx,
        TuiResult::Cancelled        => { print_cancelled (console); return Ok(()); }
    };

    let session_only = selected_idx == locations.len();

    // Build alias config
    let sub_aliases: Vec<AliasDefinition> = SUB_ALIAS_DEFS.iter().enumerate().map (|(i, (suffix, flags, desc))| {
        AliasDefinition {
            name:        format! ("{}{}", root_alias, suffix),
            flags:       flags.to_string(),
            description: desc.to_string(),
            enabled:     sub_enabled[i],
        }
    }).collect();

    let target_path = if session_only { PathBuf::new() } else { locations[selected_idx].resolved_path.clone() };

    let config = AliasConfig {
        root_alias: root_alias.clone(),
        rcdir_invocation,
        sub_aliases,
        target_scope: if session_only { ProfileScope::CurrentUserAllHosts } else { locations[selected_idx].scope },
        target_path: target_path.clone(),
        session_only,
        what_if,
    };

    let block_lines = alias_block_generator::generate (&config);

    // Preview / WhatIf
    console.printf_attr (Attribute::Information, "\n\n");

    if what_if {
        if session_only {
            console.printf_attr (Attribute::Error, "  Whatif: The following alias block would be written to console.\n\n");
        } else {
            console.printf_attr (Attribute::Error,
                &format! ("  Whatif: The following alias block would be written to:\n  {}\n\n", target_path.display()));
        }
    }

    for line in &block_lines {
        console.printf_attr (Attribute::Default, &format! ("  {}\n", line));
    }
    console.printf_attr (Attribute::Information, "\n");
    console.flush()?;

    if what_if {
        console.printf_attr (Attribute::Error, "  Whatif: No changes were made.\n");
        return Ok(());
    }

    if session_only {
        console.printf_attr (Attribute::Information, "  Paste the block above into your PowerShell session to activate.\n");
        return Ok(());
    }

    // Confirm and write
    match tui_widgets::confirmation_prompt (console, "Apply these changes?")? {
        TuiResult::Confirmed (true) => {}
        _ => { print_cancelled (console); return Ok(()); }
    }

    let (mut lines, has_bom) = if target_path.exists() {
        profile_file_manager::read_profile_file (&target_path)?
    } else {
        (Vec::new(), false)
    };

    let existing = profile_file_manager::find_alias_block (&lines);
    if existing.found {
        profile_file_manager::replace_alias_block (&mut lines, &existing, &block_lines);
    } else {
        profile_file_manager::append_alias_block (&mut lines, &block_lines);
    }

    profile_file_manager::write_profile_file (&target_path, &lines, has_bom)?;

    console.printf_attr (Attribute::Information,
        &format! ("\n  Aliases written to: {}\n\n", target_path.display()));
    console.printf_attr (Attribute::Information,
        "  To activate, open a new PowerShell window or paste this command:\n");
    console.printf_attr (Attribute::InformationHighlight,
        &format! ("    . \"{}\"\n", target_path.display()));
    console.printf_attr (Attribute::Information,
        "    ^--- the dot is required; paste the entire line exactly as shown\n");

    Ok(())
}





////////////////////////////////////////////////////////////////////////////////
//
//  get_aliases
//
//  Non-interactive display of currently configured aliases.
//
////////////////////////////////////////////////////////////////////////////////

fn get_aliases (console: &mut Console) -> Result<(), AppError> {
    let version = detect_and_validate_ps (console)?;
    let parent_path = profile_path_resolver::get_parent_process_path_public()?;
    let locations = profile_path_resolver::resolve_profile_paths (version, &parent_path)?;

    console.printf_attr (Attribute::Information, "\n");

    let mut found_any = false;

    for loc in &locations {
        if !loc.exists { continue; }
        let (lines, _) = match profile_file_manager::read_profile_file (&loc.resolved_path) {
            Ok (result) => result,
            Err (_)     => continue,
        };

        let block = profile_file_manager::find_alias_block (&lines);
        if !block.found { continue; }

        if found_any {
            console.printf_attr (Attribute::Information, "\n");
        }

        console.printf_attr (Attribute::InformationHighlight,
            &format! ("  {}  ({})\n", loc.variable_name, loc.resolved_path.display()));

        if !block.version.is_empty() {
            console.printf_attr (Attribute::Information,
                &format! ("  Generated by rcdir v{}\n", block.version));
        }

        console.printf_attr (Attribute::Information, "\n");

        for func_line in &block.function_lines {
            console.printf_attr (Attribute::Information, &format! ("    {}\n", func_line));
        }

        console.printf_attr (Attribute::Information, "\n");
        found_any = true;
    }

    if !found_any {
        console.printf_attr (Attribute::Information, "  No rcdir aliases found.\n");
        console.printf_attr (Attribute::Information, "  Run 'rcdir --set-aliases' to configure aliases.\n");
        console.printf_attr (Attribute::Information, "\n");
    }

    Ok(())
}





////////////////////////////////////////////////////////////////////////////////
//
//  remove_aliases
//
//  Interactive wizard for removing PowerShell aliases.
//
////////////////////////////////////////////////////////////////////////////////

fn remove_aliases (console: &mut Console, what_if: bool) -> Result<(), AppError> {
    let version = detect_and_validate_ps (console)?;
    let parent_path = profile_path_resolver::get_parent_process_path_public()?;
    let locations = profile_path_resolver::resolve_profile_paths (version, &parent_path)?;

    struct ProfileWithAliases {
        path:     PathBuf,
        var_name: String,
        block:    AliasBlock,
    }

    let mut profiles: Vec<ProfileWithAliases> = Vec::new();

    for loc in &locations {
        if !loc.exists { continue; }
        let (lines, _) = match profile_file_manager::read_profile_file (&loc.resolved_path) {
            Ok (result) => result,
            Err (_)     => continue,
        };
        let block = profile_file_manager::find_alias_block (&lines);
        if block.found {
            profiles.push (ProfileWithAliases {
                path:     loc.resolved_path.clone(),
                var_name: loc.variable_name.clone(),
                block,
            });
        }
    }

    if profiles.is_empty() {
        console.printf_attr (Attribute::Information, "\n  No rcdir aliases found.\n\n");
        return Ok(());
    }

    console.printf_attr (Attribute::Information, "\n\n  Remove aliases from:\n");

    let check_items: Vec<(String, bool)> = profiles.iter().map (|p| {
        let alias_list = p.block.alias_names.join (", ");
        let label = format! ("{} ({})\n        Found aliases: {}",
            p.var_name, p.path.display(), alias_list);
        (label, false)
    }).collect();

    for _ in 0..(check_items.len() * 3) { console.printf_attr (Attribute::Information, "\n"); }
    console.flush()?;

    let selected = match tui_widgets::checkbox_list (console, &check_items)? {
        TuiResult::Confirmed (states) => states,
        TuiResult::Cancelled           => { print_cancelled (console); return Ok(()); }
    };

    if !selected.iter().any (|&s| s) {
        console.printf_attr (Attribute::Information, "\n  No profiles selected for removal.\n");
        return Ok(());
    }

    for (i, profile) in profiles.iter().enumerate() {
        if !selected[i] { continue; }

        if what_if {
            console.printf_attr (Attribute::Error,
                &format! ("\n\n  Whatif: The following aliases would be removed from:\n  {}\n\n",
                    profile.path.display()));
            for name in &profile.block.alias_names {
                console.printf_attr (Attribute::Information, &format! ("    {}\n", name));
            }
        } else {
            let (mut lines, has_bom) = profile_file_manager::read_profile_file (&profile.path)?;
            let block = profile_file_manager::find_alias_block (&lines);
            if block.found {
                profile_file_manager::remove_alias_block (&mut lines, &block);
                profile_file_manager::write_profile_file (&profile.path, &lines, has_bom)?;
                console.printf_attr (Attribute::Information,
                    &format! ("\n  Aliases removed from: {}\n", profile.path.display()));
            }
        }
    }

    if what_if {
        console.printf_attr (Attribute::Error, "\n  Whatif: No changes were made.\n");
    }

    Ok(())
}





////////////////////////////////////////////////////////////////////////////////
//
//  detect_and_validate_ps
//
////////////////////////////////////////////////////////////////////////////////

fn detect_and_validate_ps (console: &mut Console) -> Result<PowerShellVersion, AppError> {
    let version = profile_path_resolver::detect_powershell_version()?;

    if version == PowerShellVersion::Unknown {
        console.printf_attr (Attribute::Error,
            "\n  This command must be run from PowerShell (pwsh.exe or powershell.exe).\n\n");
        return Err (AppError::InvalidArg ("Not running from PowerShell".into()));
    }

    Ok (version)
}





////////////////////////////////////////////////////////////////////////////////
//
//  build_profile_labels
//
////////////////////////////////////////////////////////////////////////////////

fn build_profile_labels (
    locations: &[ProfileLocation],
    _console_width: usize,
) -> (Vec<String>, usize) {
    let max_var_len = locations.iter()
        .map (|l| l.variable_name.len())
        .max()
        .unwrap_or (0)
        .max ("Current session only".len());

    let mut items = Vec::new();
    let mut default_idx = 0;

    for (i, loc) in locations.iter().enumerate() {
        let padded = format! ("{:<width$}", loc.variable_name, width = max_var_len);
        let mut suffix = String::new();
        if loc.requires_admin { suffix = " (requires admin)".to_string(); }
        if loc.has_alias_block { suffix = " [has aliases]".to_string(); }
        items.push (format! ("{}  ({}){}", padded, loc.resolved_path.display(), suffix));
        if loc.scope == ProfileScope::CurrentUserAllHosts { default_idx = i; }
    }

    items.push (format! ("{:<width$}  (not persisted)", "Current session only", width = max_var_len));
    (items, default_idx)
}





////////////////////////////////////////////////////////////////////////////////
//
//  check_alias_conflicts
//
////////////////////////////////////////////////////////////////////////////////

fn check_alias_conflicts (console: &mut Console, names: &[String]) -> Result<(), AppError> {
    const BUILTINS: &[(&str, &str)] = &[
        ("ac", "Add-Content"), ("cat", "Get-Content"), ("cd", "Set-Location"),
        ("cls", "Clear-Host"), ("cp", "Copy-Item"), ("del", "Remove-Item"),
        ("dir", "Get-ChildItem"), ("echo", "Write-Output"), ("gc", "Get-Content"),
        ("h", "Get-History"), ("ls", "Get-ChildItem"), ("man", "help"),
        ("md", "mkdir"), ("mv", "Move-Item"), ("ps", "Get-Process"),
        ("r", "Invoke-History"), ("rm", "Remove-Item"), ("sc", "Set-Content"),
        ("sl", "Set-Location"), ("sp", "Set-ItemProperty"), ("sv", "Set-Variable"),
        ("type", "Get-Content"),
    ];

    let mut any = false;
    for name in names {
        for &(builtin, cmdlet) in BUILTINS {
            if name.eq_ignore_ascii_case (builtin) {
                if !any { console.printf_attr (Attribute::Information, "\n"); any = true; }
                console.printf_attr (Attribute::Error,
                    &format! ("  Warning: '{}' conflicts with PowerShell alias for '{}'\n", name, cmdlet));
            }
        }
    }
    Ok(())
}





////////////////////////////////////////////////////////////////////////////////
//
//  print_cancelled
//
////////////////////////////////////////////////////////////////////////////////

fn print_cancelled (console: &mut Console) {
    console.printf_attr (Attribute::Information, "\n  Operation cancelled.\n");
}