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

    if filename.eq_ignore_ascii_case ("pwsh.exe") {
        Ok (PowerShellVersion::PowerShell)
    } else if filename.eq_ignore_ascii_case ("powershell.exe") {
        Ok (PowerShellVersion::WindowsPowerShell)
    } else {
        Ok (PowerShellVersion::Unknown)
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

    let ps_subdir = match version {
        PowerShellVersion::PowerShell        => "PowerShell",
        PowerShellVersion::WindowsPowerShell => "WindowsPowerShell",
        PowerShellVersion::Unknown           => return Err (AppError::InvalidArg (
            "Cannot resolve profile paths for unknown PowerShell version.".into()
        )),
    };

    let user_dir = documents.join (ps_subdir);
    let admin_required = !is_writable (pshome);

    let locations = vec![
        ProfileLocation {
            scope:           ProfileScope::CurrentUserCurrentHost,
            variable_name:   ProfileScope::CurrentUserCurrentHost.variable_name().to_string(),
            resolved_path:   user_dir.join ("Microsoft.PowerShell_profile.ps1"),
            exists:          user_dir.join ("Microsoft.PowerShell_profile.ps1").exists(),
            requires_admin:  false,
            has_alias_block: false,
        },
        ProfileLocation {
            scope:           ProfileScope::CurrentUserAllHosts,
            variable_name:   ProfileScope::CurrentUserAllHosts.variable_name().to_string(),
            resolved_path:   user_dir.join ("profile.ps1"),
            exists:          user_dir.join ("profile.ps1").exists(),
            requires_admin:  false,
            has_alias_block: false,
        },
        ProfileLocation {
            scope:           ProfileScope::AllUsersCurrentHost,
            variable_name:   ProfileScope::AllUsersCurrentHost.variable_name().to_string(),
            resolved_path:   pshome.join ("Microsoft.PowerShell_profile.ps1"),
            exists:          pshome.join ("Microsoft.PowerShell_profile.ps1").exists(),
            requires_admin:  admin_required,
            has_alias_block: false,
        },
        ProfileLocation {
            scope:           ProfileScope::AllUsersAllHosts,
            variable_name:   ProfileScope::AllUsersAllHosts.variable_name().to_string(),
            resolved_path:   pshome.join ("profile.ps1"),
            exists:          pshome.join ("profile.ps1").exists(),
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

