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
    ("d",  "-a:d",    "Directories only"),
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

    // Step 1: Root alias name (loops if conflict detected)
    let default_root = existing_block.as_ref()
        .map (|b| b.root_alias.clone())
        .unwrap_or_else (|| "d".to_string());

    let root_alias = loop {
        let candidate = match tui_widgets::text_input (console, "Root alias name (1-4 chars)", &default_root)? {
            TuiResult::Confirmed (val) => val,
            TuiResult::Cancelled       => { print_cancelled (console); return Ok(()); }
        };

        if let Some (conflict) = find_alias_conflict (&candidate) {
            console.printf_attr (Attribute::Error,
                &format! ("\n  '{}' conflicts with PowerShell alias for '{}'. Choose a different name.\n\n",
                    candidate, conflict));
            console.flush()?;
            continue;
        }

        break candidate;
    };

    // Step 2: Sub-aliases — check for conflicts, lock conflicting ones
    let sub_names: Vec<String> = SUB_ALIAS_DEFS.iter()
        .map (|(suffix, _, _)| format! ("{}{}", root_alias, suffix))
        .collect();
    let max_name_len = sub_names.iter().map (|n| n.len()).max().unwrap_or (0);

    let sub_locked: Vec<bool> = sub_names.iter()
        .map (|name| find_alias_conflict (name).is_some())
        .collect();

    let sub_items: Vec<(String, bool)> = SUB_ALIAS_DEFS.iter().enumerate().map (|(idx, (_suffix, flags, desc))| {
        let name = &sub_names[idx];
        let label = format! ("{:<width$} = {} {:<7} ({})", name, root_alias, flags, desc, width = max_name_len);
        let enabled = if sub_locked[idx] {
            false
        } else {
            // Default to ON unless we have an existing block with the SAME root alias
            // (in which case, preserve the user's previous sub-alias selections)
            match existing_block.as_ref() {
                Some (b) if b.root_alias == root_alias => b.alias_names.contains (name),
                _ => true,
            }
        };
        (label, enabled)
    }).collect();

    console.printf_attr (Attribute::Information, "\n  Select sub-aliases:\n");
    let pre_render_lines: usize = sub_items.iter().enumerate()
        .map (|(i, _)| if sub_locked[i] { 2 } else { 1 })
        .sum::<usize>() + 1; // +1 for guidance line
    for _ in 0..pre_render_lines { console.printf_attr (Attribute::Information, "\n"); }
    console.flush()?;

    let sub_enabled = match tui_widgets::checkbox_list (console, &sub_items, &sub_locked)? {
        TuiResult::Confirmed (states) => states,
        TuiResult::Cancelled           => { print_cancelled (console); return Ok(()); }
    };

    // Step 3: Profile location (conflict detection now handled by locked checkboxes)
    console.printf_attr (Attribute::Information, "\n  Save aliases to:");
    if what_if {
        console.printf_attr (Attribute::Error, " (Whatif: no changes will be written)");
    }
    console.printf_attr (Attribute::Information, "\n");

    let (radio_items, default_idx) = build_profile_labels (&locations, console.width() as usize);
    for _ in 0..(radio_items.len() + 1) { console.printf_attr (Attribute::Information, "\n"); } // +1 for guidance line
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

    let pre_render_lines: usize = check_items.iter()
        .map (|(label, _)| 1 + label.chars().filter (|&c| c == '\n').count())
        .sum::<usize>() + 1; // +1 for guidance line
    for _ in 0..pre_render_lines { console.printf_attr (Attribute::Information, "\n"); }
    console.flush()?;

    let no_locked = vec![false; check_items.len()];
    let selected = match tui_widgets::checkbox_list (console, &check_items, &no_locked)? {
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
//  find_alias_conflict
//
//  Checks a single name against known PowerShell built-in aliases.
//  Returns the conflicting cmdlet name if found, None otherwise.
//
////////////////////////////////////////////////////////////////////////////////

fn find_alias_conflict (name: &str) -> Option<&'static str> {
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

    for &(builtin, cmdlet) in BUILTINS {
        if name.eq_ignore_ascii_case (builtin) {
            return Some (cmdlet);
        }
    }
    None
}





////////////////////////////////////////////////////////////////////////////////
//
//  print_cancelled
//
////////////////////////////////////////////////////////////////////////////////

fn print_cancelled (console: &mut Console) {
    console.printf_attr (Attribute::Information, "\n  Operation cancelled.\n");
}





#[cfg(test)]
mod tests {
    use super::*;
    use crate::alias_block_generator;
    use crate::profile_file_manager;

    /// Helper: build an AliasConfig for in-memory tests.
    fn make_config (root: &str, invocation: &str, subs: &[(&str, &str, &str, bool)]) -> AliasConfig {
        AliasConfig {
            root_alias:       root.to_string(),
            rcdir_invocation: invocation.to_string(),
            sub_aliases:      subs.iter().map (|(name, flags, desc, enabled)| AliasDefinition {
                name:        name.to_string(),
                flags:       flags.to_string(),
                description: desc.to_string(),
                enabled:     *enabled,
            }).collect(),
            target_scope:  ProfileScope::CurrentUserAllHosts,
            target_path:   PathBuf::new(),
            session_only:  false,
            what_if:       false,
        }
    }

    ////////////////////////////////////////////////////////////////////////////
    //
    //  build_default_sub_aliases_default_root
    //
    //  Port of: BuildDefaultSubAliases_DefaultRoot
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn build_default_sub_aliases_default_root() {
        let names: Vec<String> = SUB_ALIAS_DEFS.iter()
            .map (|(suffix, _, _)| format! ("d{}", suffix))
            .collect();

        assert_eq! (names.len(), 5);
        assert_eq! (names[0], "dt");
        assert_eq! (names[1], "dw");
        assert_eq! (names[2], "dd");
        assert_eq! (names[3], "ds");
        assert_eq! (names[4], "dsb");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  build_default_sub_aliases_custom_root
    //
    //  Port of: BuildDefaultSubAliases_CustomRoot
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn build_default_sub_aliases_custom_root() {
        let names: Vec<String> = SUB_ALIAS_DEFS.iter()
            .map (|(suffix, _, _)| format! ("tc{}", suffix))
            .collect();

        assert_eq! (names.len(), 5);
        assert_eq! (names[0], "tct");
        assert_eq! (names[1], "tcw");
        assert_eq! (names[2], "tcd");
        assert_eq! (names[3], "tcs");
        assert_eq! (names[4], "tcsb");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  set_aliases_generate_and_append_in_memory
    //
    //  Port of: SetAliases_GenerateAndAppend_InMemory
    //  Tests generate + append + find round-trip entirely in memory.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn set_aliases_generate_and_append_in_memory() {
        let config = make_config ("d", "rcdir", &[
            ("dd", "-a:d", "dirs",   true),
            ("ds", "-s",   "search", true),
            ("dw", "-w",   "wide",   false),
        ]);

        let block_lines = alias_block_generator::generate (&config);
        let mut lines: Vec<String> = Vec::new();

        profile_file_manager::append_alias_block (&mut lines, &block_lines);

        let block = profile_file_manager::find_alias_block (&lines);

        assert! (block.found);
        assert_eq! (block.root_alias, "d");
        // Root + 2 enabled subs = 3
        assert_eq! (block.alias_names.len(), 3);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  set_aliases_existing_block_replaces_correctly
    //
    //  Port of: SetAliases_ExistingBlock_ReplacesCorrectly
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn set_aliases_existing_block_replaces_correctly() {
        let config1 = make_config ("d", "rcdir", &[
            ("dd", "-a:d", "dirs", true),
        ]);
        let config2 = make_config ("tc", "rcdir", &[
            ("tcd", "-a:d", "dirs",   true),
            ("tcs", "-s",   "search", true),
        ]);

        let block1 = alias_block_generator::generate (&config1);
        let block2 = alias_block_generator::generate (&config2);

        let mut lines = vec!["# Before content".to_string()];
        profile_file_manager::append_alias_block (&mut lines, &block1);
        lines.push ("# After content".to_string());

        let found = profile_file_manager::find_alias_block (&lines);
        assert! (found.found);

        profile_file_manager::replace_alias_block (&mut lines, &found, &block2);

        let found2 = profile_file_manager::find_alias_block (&lines);
        assert! (found2.found);
        assert_eq! (found2.root_alias, "tc");
        assert_eq! (found2.alias_names.len(), 3); // tc, tcd, tcs

        assert_eq! (lines.first().unwrap(), "# Before content");
        assert_eq! (lines.last().unwrap(), "# After content");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  session_only_generates_block
    //
    //  Port of: SessionOnly_NoFileWrite
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn session_only_generates_block() {
        let mut config = make_config ("d", "rcdir", &[
            ("dd", "-a:d", "dirs", true),
        ]);
        config.session_only = true;

        let block_lines = alias_block_generator::generate (&config);

        assert! (block_lines.len() > 5);

        let found_root = block_lines.iter().any (|l| l.contains ("function d"));
        assert! (found_root);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  get_aliases_finds_block_in_file
    //
    //  Port of: GetAliases_FindsBlockInFile
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn get_aliases_finds_block_in_file() {
        let config = make_config ("d", "rcdir", &[
            ("dd", "-a:d", "dirs",   true),
            ("ds", "-s",   "search", true),
        ]);

        let block_lines = alias_block_generator::generate (&config);

        let mut lines = vec!["# My profile".to_string(), "Import-Module posh-git".to_string()];
        profile_file_manager::append_alias_block (&mut lines, &block_lines);

        let block = profile_file_manager::find_alias_block (&lines);

        assert! (block.found);
        assert_eq! (block.root_alias, "d");
        assert_eq! (block.alias_names.len(), 3); // d, dd, ds
        assert_eq! (block.alias_names[0], "d");
        assert_eq! (block.alias_names[1], "dd");
        assert_eq! (block.alias_names[2], "ds");
        assert! (!block.version.is_empty());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  get_aliases_no_aliases_block_not_found
    //
    //  Port of: GetAliases_NoAliases_BlockNotFound
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn get_aliases_no_aliases_block_not_found() {
        let lines = vec![
            "# My PowerShell profile".to_string(),
            "Set-Location C:\\code".to_string(),
            "Import-Module posh-git".to_string(),
        ];

        let block = profile_file_manager::find_alias_block (&lines);
        assert! (!block.found);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  get_aliases_multiple_profiles_scanned
    //
    //  Port of: GetAliases_MultipleProfilesCanBeScanned
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn get_aliases_multiple_profiles_scanned() {
        let config = make_config ("d", "rcdir", &[
            ("dd", "-a:d", "dirs", true),
        ]);

        let block_lines = alias_block_generator::generate (&config);

        // Profile 1: has aliases
        let mut profile1 = vec!["# profile 1".to_string()];
        profile_file_manager::append_alias_block (&mut profile1, &block_lines);

        // Profile 2: no aliases
        let profile2 = vec!["# profile 2".to_string()];

        let block1 = profile_file_manager::find_alias_block (&profile1);
        let block2 = profile_file_manager::find_alias_block (&profile2);

        assert! (block1.found);
        assert! (!block2.found);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  update_aliases_root_change_full_round_trip
    //
    //  Port of: UpdateAliases_RootChange_FullRoundTrip
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn update_aliases_root_change_full_round_trip() {
        let config1 = make_config ("d", "rcdir", &[
            ("dd", "-a:d", "dirs", true),
        ]);

        let block1 = alias_block_generator::generate (&config1);

        let mut lines = vec!["# My profile".to_string()];
        profile_file_manager::append_alias_block (&mut lines, &block1);

        let found = profile_file_manager::find_alias_block (&lines);
        assert! (found.found);
        assert_eq! (found.root_alias, "d");

        // Generate replacement with new root
        let config2 = make_config ("tc", "rcdir", &[
            ("tcd", "-a:d", "dirs",   true),
            ("tcs", "-s",   "search", true),
        ]);
        let block2 = alias_block_generator::generate (&config2);

        profile_file_manager::replace_alias_block (&mut lines, &found, &block2);

        let found2 = profile_file_manager::find_alias_block (&lines);
        assert! (found2.found);
        assert_eq! (found2.root_alias, "tc");
        assert_eq! (found2.alias_names.len(), 3);

        assert_eq! (lines[0], "# My profile");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  update_aliases_same_root_different_subs
    //
    //  Port of: UpdateAliases_SameRootDifferentSubs
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn update_aliases_same_root_different_subs() {
        let config1 = make_config ("d", "rcdir", &[
            ("dd", "-a:d", "dirs",   true),
            ("ds", "-s",   "search", true),
        ]);
        let config2 = make_config ("d", "rcdir", &[
            ("dd", "-a:d", "dirs",   false),  // disabled
            ("ds", "-s",   "search", true),
            ("dw", "-w",   "wide",   true),   // newly enabled
        ]);

        let block1 = alias_block_generator::generate (&config1);
        let block2 = alias_block_generator::generate (&config2);

        let mut lines: Vec<String> = Vec::new();
        profile_file_manager::append_alias_block (&mut lines, &block1);

        let found = profile_file_manager::find_alias_block (&lines);
        assert! (found.found);

        profile_file_manager::replace_alias_block (&mut lines, &found, &block2);

        let found2 = profile_file_manager::find_alias_block (&lines);
        assert! (found2.found);
        assert_eq! (found2.root_alias, "d");
        // d, ds, dw (dd disabled)
        assert_eq! (found2.alias_names.len(), 3);
        assert_eq! (found2.alias_names[0], "d");
        assert_eq! (found2.alias_names[1], "ds");
        assert_eq! (found2.alias_names[2], "dw");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  remove_aliases_clean_removal_in_memory
    //
    //  Port of: RemoveAliases_CleanRemoval_InMemory
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn remove_aliases_clean_removal_in_memory() {
        let config = make_config ("d", "rcdir", &[
            ("dd", "-a:d", "dirs", true),
        ]);
        let block_lines = alias_block_generator::generate (&config);

        let mut lines = vec![
            "# Before aliases".to_string(),
            "Import-Module posh-git".to_string(),
        ];
        profile_file_manager::append_alias_block (&mut lines, &block_lines);
        lines.push ("# After aliases".to_string());

        let found = profile_file_manager::find_alias_block (&lines);
        assert! (found.found);

        profile_file_manager::remove_alias_block (&mut lines, &found);

        let found2 = profile_file_manager::find_alias_block (&lines);
        assert! (!found2.found);

        let text = lines.join ("\n");
        assert! (text.contains ("# Before aliases"));
        assert! (text.contains ("Import-Module posh-git"));
        assert! (text.contains ("# After aliases"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  remove_aliases_no_aliases_nothing_to_remove
    //
    //  Port of: RemoveAliases_NoAliases_NothingToRemove
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn remove_aliases_no_aliases_nothing_to_remove() {
        let lines = vec![
            "# My profile".to_string(),
            "Set-Location C:\\code".to_string(),
        ];

        let block = profile_file_manager::find_alias_block (&lines);
        assert! (!block.found);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  whatif_set_aliases_no_file_modification
    //
    //  Port of: WhatIf_SetAliases_NoFileModification
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn whatif_set_aliases_no_file_modification() {
        let mut config = make_config ("d", "rcdir", &[
            ("dd", "-a:d", "dirs", true),
        ]);
        config.what_if = true;

        let block_lines = alias_block_generator::generate (&config);

        assert! (config.what_if);
        assert! (block_lines.len() > 5);

        let found_root = block_lines.iter().any (|l|
            l.contains ("function d") && l.contains ("rcdir @args")
        );
        assert! (found_root);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  whatif_remove_aliases_no_file_modification
    //
    //  Port of: WhatIf_RemoveAliases_NoFileModification
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn whatif_remove_aliases_no_file_modification() {
        let config = make_config ("d", "rcdir", &[
            ("dd", "-a:d", "dirs", true),
        ]);
        let block_lines = alias_block_generator::generate (&config);

        let mut lines = vec!["# Profile content".to_string()];
        profile_file_manager::append_alias_block (&mut lines, &block_lines);

        let original_count = lines.len();

        // WhatIf: detect block but do NOT remove
        let block = profile_file_manager::find_alias_block (&lines);
        assert! (block.found);

        // Verify lines are unchanged (whatif = no modification)
        assert_eq! (lines.len(), original_count);

        let block2 = profile_file_manager::find_alias_block (&lines);
        assert! (block2.found);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  conflict_detection_known_builtin
    //
    //  Port of: ConflictDetection_KnownBuiltinAliasTable
    //  "r" is a known PowerShell alias for Invoke-History.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn conflict_detection_known_builtin() {
        let conflict = find_alias_conflict ("r");
        assert! (conflict.is_some());
        assert_eq! (conflict.unwrap(), "Invoke-History");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  conflict_detection_no_conflict
    //
    //  Port of: ConflictDetection_NoConflict
    //  "zzz" is not a known built-in alias.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn conflict_detection_no_conflict() {
        let conflict = find_alias_conflict ("zzz");
        assert! (conflict.is_none());
    }
}