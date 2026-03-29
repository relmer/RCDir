// src/profile_path_resolver.rs — PS version detection + profile path resolution

use std::path::PathBuf;

use windows::Win32::System::Diagnostics::ToolHelp::*;
use windows::Win32::System::Threading::*;
use windows::Win32::UI::Shell::*;
use windows::core::PWSTR;

use crate::alias_types::*;
use crate::ehm::AppError;





////////////////////////////////////////////////////////////////////////////////
//
//  detect_powershell_version
//
//  Inspects the parent process to determine if we're running inside
//  PowerShell 7+ (pwsh.exe) or Windows PowerShell 5.1 (powershell.exe).
//
////////////////////////////////////////////////////////////////////////////////

pub fn detect_powershell_version() -> Result<PowerShellVersion, AppError> {
    let parent_path = get_parent_process_path()?;

    let filename = parent_path
        .file_name()
        .and_then (|n| n.to_str())
        .unwrap_or ("");

    Ok (map_image_name_to_version (filename))
}





////////////////////////////////////////////////////////////////////////////////
//
//  map_image_name_to_version
//
//  Pure function: convert an executable filename to a PowerShellVersion.
//  Port of: CProfilePathResolver::MapImageNameToVersion
//
////////////////////////////////////////////////////////////////////////////////

pub fn map_image_name_to_version (filename: &str) -> PowerShellVersion {
    if filename.eq_ignore_ascii_case ("pwsh.exe") {
        PowerShellVersion::PowerShell
    } else if filename.eq_ignore_ascii_case ("powershell.exe") {
        PowerShellVersion::WindowsPowerShell
    } else {
        PowerShellVersion::Unknown
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  get_parent_process_path
//
//  Uses ToolHelp snapshot to find our parent PID, then queries the parent
//  process image path via QueryFullProcessImageNameW.
//
////////////////////////////////////////////////////////////////////////////////

fn get_parent_process_path() -> Result<PathBuf, AppError> {
    let current_pid = unsafe { windows::Win32::System::Threading::GetCurrentProcessId() };
    let parent_pid = find_parent_pid (current_pid)?;

    let process = unsafe {
        OpenProcess (PROCESS_QUERY_LIMITED_INFORMATION, false, parent_pid)
    }.map_err (|e| AppError::Win32 (e))?;

    let mut buf = vec![0u16; 1024];
    let mut len = buf.len() as u32;

    unsafe {
        QueryFullProcessImageNameW (
            process,
            PROCESS_NAME_FORMAT (0),
            PWSTR (buf.as_mut_ptr()),
            &mut len,
        )
    }.map_err (|e| AppError::Win32 (e))?;

    let path_str = String::from_utf16_lossy (&buf[..len as usize]);
    Ok (PathBuf::from (path_str))
}





////////////////////////////////////////////////////////////////////////////////
//
//  find_parent_pid
//
//  Walks the ToolHelp process snapshot to find the parent PID of the given PID.
//
////////////////////////////////////////////////////////////////////////////////

fn find_parent_pid (pid: u32) -> Result<u32, AppError> {
    let snapshot = unsafe {
        CreateToolhelp32Snapshot (TH32CS_SNAPPROCESS, 0)
    }.map_err (|e| AppError::Win32 (e))?;

    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };

    unsafe { Process32FirstW (snapshot, &mut entry) }
        .map_err (|e| AppError::Win32 (e))?;

    loop {
        if entry.th32ProcessID == pid {
            return Ok (entry.th32ParentProcessID);
        }

        match unsafe { Process32NextW (snapshot, &mut entry) } {
            Ok (()) => continue,
            Err (_) => break,
        }
    }

    Err (AppError::InvalidArg (
        "Could not determine parent process. Run this command from PowerShell.".into()
    ))
}





////////////////////////////////////////////////////////////////////////////////
//
//  resolve_profile_paths
//
//  Builds the 4 ProfileLocation structs for the detected PS version.
//  Uses SHGetKnownFolderPath for user Documents, parent process dir for PSHOME.
//
////////////////////////////////////////////////////////////////////////////////

pub fn resolve_profile_paths (
    version:     PowerShellVersion,
    parent_path: &std::path::Path,
) -> Result<Vec<ProfileLocation>, AppError> {

    let documents = get_documents_folder()?;
    let pshome = parent_path
        .parent()
        .unwrap_or (std::path::Path::new (""));

    let admin_required = !is_writable (pshome);

    let mut locations = build_profile_locations (&documents, pshome, version, admin_required)?;

    // Probe the filesystem for path existence — kept out of
    // build_profile_locations so that function stays pure for unit tests.
    for loc in &mut locations {
        loc.exists = loc.resolved_path.exists();
    }

    Ok (locations)
}





////////////////////////////////////////////////////////////////////////////////
//
//  build_profile_locations
//
//  Pure-logic path construction — no system calls, directly unit-testable.
//  Builds the four ProfileLocation structs from base directory paths.
//  Does NOT check path existence (caller does that).
//
//  Port of: CProfilePathResolver::BuildProfileLocations
//
////////////////////////////////////////////////////////////////////////////////

pub fn build_profile_locations (
    documents_dir:   &std::path::Path,
    pshome:          &std::path::Path,
    version:         PowerShellVersion,
    admin_required:  bool,
) -> Result<Vec<ProfileLocation>, AppError> {

    let ps_subdir = match version {
        PowerShellVersion::PowerShell        => "PowerShell",
        PowerShellVersion::WindowsPowerShell => "WindowsPowerShell",
        PowerShellVersion::Unknown           => return Err (AppError::InvalidArg (
            "Cannot resolve profile paths for unknown PowerShell version.".into()
        )),
    };

    let user_dir = documents_dir.join (ps_subdir);

    let locations = vec![
        ProfileLocation {
            scope:           ProfileScope::CurrentUserCurrentHost,
            variable_name:   ProfileScope::CurrentUserCurrentHost.variable_name().to_string(),
            resolved_path:   user_dir.join ("Microsoft.PowerShell_profile.ps1"),
            exists:          false,
            requires_admin:  false,
            has_alias_block: false,
        },
        ProfileLocation {
            scope:           ProfileScope::CurrentUserAllHosts,
            variable_name:   ProfileScope::CurrentUserAllHosts.variable_name().to_string(),
            resolved_path:   user_dir.join ("profile.ps1"),
            exists:          false,
            requires_admin:  false,
            has_alias_block: false,
        },
        ProfileLocation {
            scope:           ProfileScope::AllUsersCurrentHost,
            variable_name:   ProfileScope::AllUsersCurrentHost.variable_name().to_string(),
            resolved_path:   pshome.join ("Microsoft.PowerShell_profile.ps1"),
            exists:          false,
            requires_admin:  admin_required,
            has_alias_block: false,
        },
        ProfileLocation {
            scope:           ProfileScope::AllUsersAllHosts,
            variable_name:   ProfileScope::AllUsersAllHosts.variable_name().to_string(),
            resolved_path:   pshome.join ("profile.ps1"),
            exists:          false,
            requires_admin:  admin_required,
            has_alias_block: false,
        },
    ];

    Ok (locations)
}





////////////////////////////////////////////////////////////////////////////////
//
//  get_parent_process_path_for_resolution
//
//  Public wrapper: gets the parent process path for use by resolve_profile_paths
//  and for PSHOME determination.
//
////////////////////////////////////////////////////////////////////////////////

pub fn get_parent_process_path_public() -> Result<PathBuf, AppError> {
    get_parent_process_path()
}





////////////////////////////////////////////////////////////////////////////////
//
//  get_documents_folder
//
//  Uses SHGetKnownFolderPath to get the user's Documents folder,
//  correctly handling OneDrive KFM redirection.
//
////////////////////////////////////////////////////////////////////////////////

fn get_documents_folder() -> Result<PathBuf, AppError> {
    let path = unsafe {
        SHGetKnownFolderPath (
            &windows::Win32::UI::Shell::FOLDERID_Documents,
            KNOWN_FOLDER_FLAG::default(),
            None,
        )
    }.map_err (|e| AppError::Win32 (e))?;

    let path_str = unsafe { path.to_string() }
        .map_err (|_| AppError::InvalidArg ("Failed to convert Documents path".into()))?;

    Ok (PathBuf::from (path_str))
}





////////////////////////////////////////////////////////////////////////////////
//
//  is_writable
//
//  Checks if a directory is writable by attempting to get write access info.
//  Used to determine if AllUsers profile paths require admin.
//
////////////////////////////////////////////////////////////////////////////////

fn is_writable (path: &std::path::Path) -> bool {
    use std::fs;

    // Try to create a temp file — if it succeeds, the dir is writable
    let test_path = path.join (".rcdir_write_test");
    match fs::write (&test_path, b"") {
        Ok (()) => {
            let _ = fs::remove_file (&test_path);
            true
        }
        Err (_) => false,
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  resolve_rcdir_invocation
//
//  Determines how the generated alias should invoke rcdir:
//  - If on PATH, use "rcdir"
//  - If not on PATH, use the full path to the exe
//
////////////////////////////////////////////////////////////////////////////////

pub fn resolve_rcdir_invocation() -> Result<String, AppError> {
    let exe_path = std::env::current_exe()
        .map_err (|e| AppError::Io (e))?;

    let exe_name = exe_path
        .file_name()
        .and_then (|n| n.to_str())
        .unwrap_or ("rcdir.exe");

    // Check if exe_name is findable on PATH
    let mut found_buf = vec![0u16; 1024];
    let search_name: Vec<u16> = exe_name.encode_utf16().chain (std::iter::once (0)).collect();

    let found_len = unsafe {
        windows::Win32::Storage::FileSystem::SearchPathW (
            None,
            windows::core::PCWSTR (search_name.as_ptr()),
            None,
            Some (&mut found_buf),
            None,
        )
    };

    if found_len > 0 {
        let found_path = String::from_utf16_lossy (&found_buf[..found_len as usize]);
        let found_canonical = PathBuf::from (&found_path);
        let exe_canonical = exe_path.clone();

        // Compare paths case-insensitively (Windows)
        if found_canonical.to_string_lossy().eq_ignore_ascii_case (&exe_canonical.to_string_lossy()) {
            return Ok ("rcdir".to_string());
        }
    }

    // Not on PATH — use full path with quoting if needed
    let full = exe_path.to_string_lossy().to_string();
    if full.contains (' ') {
        Ok (format! ("& \"{}\"", full))
    } else {
        Ok (full)
    }
}





#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    ////////////////////////////////////////////////////////////////////////////
    //
    //  map_image_name_pwsh_returns_powershell
    //
    //  Port of: MapImageName_Pwsh_ReturnsPowerShell
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn map_image_name_pwsh_returns_powershell() {
        assert_eq! (map_image_name_to_version ("pwsh.exe"), PowerShellVersion::PowerShell);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  map_image_name_pwsh_upper_case_returns_powershell
    //
    //  Port of: MapImageName_PwshUpperCase_ReturnsPowerShell
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn map_image_name_pwsh_upper_case_returns_powershell() {
        assert_eq! (map_image_name_to_version ("PWSH.EXE"), PowerShellVersion::PowerShell);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  map_image_name_powershell_returns_windows_powershell
    //
    //  Port of: MapImageName_Powershell_ReturnsWindowsPowerShell
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn map_image_name_powershell_returns_windows_powershell() {
        assert_eq! (map_image_name_to_version ("powershell.exe"), PowerShellVersion::WindowsPowerShell);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  map_image_name_cmd_returns_unknown
    //
    //  Port of: MapImageName_Cmd_ReturnsUnknown
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn map_image_name_cmd_returns_unknown() {
        assert_eq! (map_image_name_to_version ("cmd.exe"), PowerShellVersion::Unknown);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  map_image_name_explorer_returns_unknown
    //
    //  Port of: MapImageName_Explorer_ReturnsUnknown
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn map_image_name_explorer_returns_unknown() {
        assert_eq! (map_image_name_to_version ("explorer.exe"), PowerShellVersion::Unknown);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  build_profile_locations_powershell_returns_four_paths
    //
    //  Port of: BuildProfileLocations_PowerShell_ReturnsFourPaths
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn build_profile_locations_powershell_returns_four_paths() {
        let docs  = Path::new (r"C:\Users\test\Documents");
        let pshome = Path::new (r"C:\Program Files\PowerShell\7");

        let locs = build_profile_locations (docs, pshome, PowerShellVersion::PowerShell, true).unwrap();

        assert_eq! (locs.len(), 4);
        for loc in &locs {
            let path_str = loc.resolved_path.to_string_lossy();
            assert! (path_str.contains (r"\PowerShell\"), "Path should contain \\PowerShell\\: {}", path_str);
            assert! (!path_str.contains (r"\WindowsPowerShell\"));
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  build_profile_locations_windows_powershell_uses_correct_dir
    //
    //  Port of: BuildProfileLocations_WindowsPowerShell_UsesCorrectDir
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn build_profile_locations_windows_powershell_uses_correct_dir() {
        let docs   = Path::new (r"C:\Users\test\Documents");
        let pshome = Path::new (r"C:\Windows\System32\WindowsPowerShell\v1.0");

        let locs = build_profile_locations (docs, pshome, PowerShellVersion::WindowsPowerShell, true).unwrap();

        assert_eq! (locs.len(), 4);
        let path0 = locs[0].resolved_path.to_string_lossy();
        let path1 = locs[1].resolved_path.to_string_lossy();
        assert! (path0.contains (r"\WindowsPowerShell\"));
        assert! (path1.contains (r"\WindowsPowerShell\"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  build_profile_locations_scopes_are_correct
    //
    //  Port of: BuildProfileLocations_ScopesAreCorrect
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn build_profile_locations_scopes_are_correct() {
        let docs   = Path::new (r"C:\Users\test\Documents");
        let pshome = Path::new (r"C:\Program Files\PowerShell\7");

        let locs = build_profile_locations (docs, pshome, PowerShellVersion::PowerShell, true).unwrap();

        assert_eq! (locs[0].scope, ProfileScope::CurrentUserCurrentHost);
        assert_eq! (locs[1].scope, ProfileScope::CurrentUserAllHosts);
        assert_eq! (locs[2].scope, ProfileScope::AllUsersCurrentHost);
        assert_eq! (locs[3].scope, ProfileScope::AllUsersAllHosts);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  build_profile_locations_all_users_requires_admin
    //
    //  Port of: BuildProfileLocations_AllUsersRequiresAdmin
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn build_profile_locations_all_users_requires_admin() {
        let docs   = Path::new (r"C:\Users\test\Documents");
        let pshome = Path::new (r"C:\Program Files\PowerShell\7");

        let locs = build_profile_locations (docs, pshome, PowerShellVersion::PowerShell, true).unwrap();

        assert! (!locs[0].requires_admin);
        assert! (!locs[1].requires_admin);
        assert! (locs[2].requires_admin);
        assert! (locs[3].requires_admin);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  build_profile_locations_variable_names_correct
    //
    //  Port of: BuildProfileLocations_VariableNamesCorrect
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn build_profile_locations_variable_names_correct() {
        let docs   = Path::new (r"C:\Users\test\Documents");
        let pshome = Path::new (r"C:\Program Files\PowerShell\7");

        let locs = build_profile_locations (docs, pshome, PowerShellVersion::PowerShell, true).unwrap();

        assert_eq! (locs[0].variable_name, "$PROFILE.CurrentUserCurrentHost");
        assert_eq! (locs[1].variable_name, "$PROFILE.CurrentUserAllHosts");
        assert_eq! (locs[2].variable_name, "$PROFILE.AllUsersCurrentHost");
        assert_eq! (locs[3].variable_name, "$PROFILE.AllUsersAllHosts");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  build_profile_locations_full_paths_correct_powershell
    //
    //  Port of: BuildProfileLocations_FullPathsCorrect_PowerShell
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn build_profile_locations_full_paths_correct_powershell() {
        let docs   = Path::new (r"C:\Users\test\Documents");
        let pshome = Path::new (r"C:\Program Files\PowerShell\7");

        let locs = build_profile_locations (docs, pshome, PowerShellVersion::PowerShell, true).unwrap();

        assert_eq! (locs[0].resolved_path.to_string_lossy(), r"C:\Users\test\Documents\PowerShell\Microsoft.PowerShell_profile.ps1");
        assert_eq! (locs[1].resolved_path.to_string_lossy(), r"C:\Users\test\Documents\PowerShell\profile.ps1");
        assert_eq! (locs[2].resolved_path.to_string_lossy(), r"C:\Program Files\PowerShell\7\Microsoft.PowerShell_profile.ps1");
        assert_eq! (locs[3].resolved_path.to_string_lossy(), r"C:\Program Files\PowerShell\7\profile.ps1");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  build_profile_locations_full_paths_correct_windows_powershell
    //
    //  Port of: BuildProfileLocations_FullPathsCorrect_WindowsPowerShell
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn build_profile_locations_full_paths_correct_windows_powershell() {
        let docs   = Path::new (r"C:\Users\test\Documents");
        let pshome = Path::new (r"C:\Windows\System32\WindowsPowerShell\v1.0");

        let locs = build_profile_locations (docs, pshome, PowerShellVersion::WindowsPowerShell, true).unwrap();

        assert_eq! (locs[0].resolved_path.to_string_lossy(), r"C:\Users\test\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1");
        assert_eq! (locs[1].resolved_path.to_string_lossy(), r"C:\Users\test\Documents\WindowsPowerShell\profile.ps1");
        assert_eq! (locs[2].resolved_path.to_string_lossy(), r"C:\Windows\System32\WindowsPowerShell\v1.0\Microsoft.PowerShell_profile.ps1");
        assert_eq! (locs[3].resolved_path.to_string_lossy(), r"C:\Windows\System32\WindowsPowerShell\v1.0\profile.ps1");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  build_profile_locations_exists_defaults_false
    //
    //  Port of: BuildProfileLocations_FExistsDefaultsFalse
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn build_profile_locations_exists_defaults_false() {
        let docs   = Path::new (r"C:\Fake\Documents");
        let pshome = Path::new (r"C:\Fake\PSHome");

        let locs = build_profile_locations (docs, pshome, PowerShellVersion::PowerShell, true).unwrap();

        for loc in &locs {
            assert! (!loc.exists, "exists should be false for fake paths: {}", loc.resolved_path.display());
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  build_profile_locations_onedrive_redirected_path
    //
    //  Port of: BuildProfileLocations_OneDriveRedirectedPath
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn build_profile_locations_onedrive_redirected_path() {
        let docs   = Path::new (r"C:\Users\test\OneDrive\Documents");
        let pshome = Path::new (r"C:\Program Files\PowerShell\7");

        let locs = build_profile_locations (docs, pshome, PowerShellVersion::PowerShell, true).unwrap();

        assert_eq! (locs[1].resolved_path.to_string_lossy(),
            r"C:\Users\test\OneDrive\Documents\PowerShell\profile.ps1");
    }
}
