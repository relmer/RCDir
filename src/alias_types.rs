// src/alias_types.rs — Shared data model for PowerShell alias configuration

use std::path::PathBuf;





////////////////////////////////////////////////////////////////////////////////
//
//  PowerShellVersion
//
//  Detected PowerShell version of the calling shell.
//
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerShellVersion {
    PowerShell,         // 7+ (pwsh.exe) — profile dir: PowerShell\
    WindowsPowerShell,  // 5.1 (powershell.exe) — profile dir: WindowsPowerShell\
    Unknown,            // Parent process is neither pwsh.exe nor powershell.exe
}





////////////////////////////////////////////////////////////////////////////////
//
//  ProfileScope
//
//  One of the four standard PowerShell profile scopes.
//
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileScope {
    CurrentUserCurrentHost,
    CurrentUserAllHosts,
    AllUsersCurrentHost,
    AllUsersAllHosts,
}

impl ProfileScope {
    pub fn variable_name (&self) -> &'static str {
        match self {
            ProfileScope::CurrentUserCurrentHost => "$PROFILE.CurrentUserCurrentHost",
            ProfileScope::CurrentUserAllHosts    => "$PROFILE.CurrentUserAllHosts",
            ProfileScope::AllUsersCurrentHost    => "$PROFILE.AllUsersCurrentHost",
            ProfileScope::AllUsersAllHosts       => "$PROFILE.AllUsersAllHosts",
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  ProfileLocation
//
//  A resolved profile file path with metadata.
//
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct ProfileLocation {
    pub scope:           ProfileScope,
    pub variable_name:   String,
    pub resolved_path:   PathBuf,
    pub exists:          bool,
    pub requires_admin:  bool,
    pub has_alias_block: bool,
}





////////////////////////////////////////////////////////////////////////////////
//
//  AliasDefinition
//
//  A single alias (root or sub) to be generated.
//
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct AliasDefinition {
    pub name:        String,
    pub flags:       String,
    pub description: String,
    pub enabled:     bool,
}





////////////////////////////////////////////////////////////////////////////////
//
//  AliasConfig
//
//  The complete user configuration from the TUI wizard.
//
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct AliasConfig {
    pub root_alias:        String,
    pub rcdir_invocation:  String,
    pub sub_aliases:       Vec<AliasDefinition>,
    pub target_scope:      ProfileScope,
    pub target_path:       PathBuf,
    pub session_only:      bool,
    pub what_if:           bool,
}





////////////////////////////////////////////////////////////////////////////////
//
//  AliasBlock
//
//  A parsed alias block found in an existing profile file.
//
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Default)]
pub struct AliasBlock {
    pub start_line:     usize,
    pub end_line:       usize,
    pub root_alias:     String,
    pub alias_names:    Vec<String>,
    pub function_lines: Vec<String>,
    pub version:        String,
    pub found:          bool,
}
