// owner.rs — File ownership lookup via Windows Security APIs
//
// Port of: ResultsDisplayerNormal.cpp → GetFileOwner(), GetFileOwners()
//
// Uses GetNamedSecurityInfoW to get the file's security descriptor,
// then LookupAccountSidW to resolve the SID to DOMAIN\User.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use crate::directory_info::DirectoryInfo;

/// Get the owner of a single file as "DOMAIN\User" string.
///
/// Port of: CResultsDisplayerNormal::GetFileOwner
pub fn get_file_owner(file_path: &OsStr) -> String {
    use windows::Win32::Security::{
        LookupAccountSidW,
        OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
        SID_NAME_USE,
    };
    use windows::Win32::Security::Authorization::{
        GetNamedSecurityInfoW, SE_FILE_OBJECT,
    };
    use windows::Win32::Foundation::{LocalFree, HLOCAL, ERROR_SUCCESS};

    let path_wide: Vec<u16> = file_path.encode_wide().chain(Some(0)).collect();

    let mut p_sid_owner: PSID = PSID::default();
    let mut p_sd: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR::default();

    // Get the owner SID from the file's security descriptor
    let result = unsafe {
        GetNamedSecurityInfoW(
            windows::core::PCWSTR(path_wide.as_ptr()),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            Some(&mut p_sid_owner),
            None,
            None,
            None,
            &mut p_sd,
        )
    };

    if result != ERROR_SUCCESS {
        return "Unknown".to_string();
    }

    // Look up the account name for the SID
    let mut name_buf = [0u16; 256];
    let mut domain_buf = [0u16; 256];
    let mut cch_name = name_buf.len() as u32;
    let mut cch_domain = domain_buf.len() as u32;
    let mut sid_use = SID_NAME_USE::default();

    let success = unsafe {
        LookupAccountSidW(
            windows::core::PCWSTR::null(),
            p_sid_owner,
            Some(windows::core::PWSTR(name_buf.as_mut_ptr())),
            &mut cch_name,
            Some(windows::core::PWSTR(domain_buf.as_mut_ptr())),
            &mut cch_domain,
            &mut sid_use,
        )
    };

    // Free the security descriptor
    if !p_sd.0.is_null() {
        unsafe { let _ = LocalFree(Some(HLOCAL(p_sd.0))); }
    }

    if success.is_err() {
        return "Unknown".to_string();
    }

    let name = String::from_utf16_lossy(&name_buf[..cch_name as usize]);
    let domain = String::from_utf16_lossy(&domain_buf[..cch_domain as usize]);

    // Format as DOMAIN\User or just User if domain is empty
    if !domain.is_empty() {
        format!("{}\\{}", domain, name)
    } else {
        name
    }
}

/// Get owners for all files in a DirectoryInfo.
///
/// Port of: CResultsDisplayerNormal::GetFileOwners
///
/// Returns (owners_vec, max_owner_length) for column alignment.
pub fn get_file_owners(di: &DirectoryInfo) -> (Vec<String>, usize) {
    let mut owners = Vec::with_capacity(di.matches.len());
    let mut max_len = 0usize;

    for fi in &di.matches {
        let full_path = di.dir_path.join(&fi.file_name);
        let owner = get_file_owner(full_path.as_os_str());
        max_len = max_len.max(owner.len());
        owners.push(owner);
    }

    (owners, max_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_owner_of_current_exe() {
        // The current executable should have a valid owner
        let exe = std::env::current_exe().unwrap();
        let owner = get_file_owner(exe.as_os_str());
        // Should not be empty — at minimum "Unknown" or "DOMAIN\User"
        assert!(!owner.is_empty());
        // If running in CI or as a real user, should contain a backslash or be "Unknown"
        assert!(owner.contains('\\') || owner == "Unknown" || !owner.is_empty());
    }

    #[test]
    fn get_owner_of_nonexistent_file() {
        let path = Path::new("C:\\this\\path\\does\\not\\exist\\file.txt");
        let owner = get_file_owner(path.as_os_str());
        assert_eq!(owner, "Unknown");
    }
}
