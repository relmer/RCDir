// src/profile_file_manager.rs — Read/write/backup profile files, marker block parsing

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::alias_types::*;
use crate::ehm::AppError;



const HEADER_MARKER: &str = "#  RCDir Aliases";
const FOOTER_MARKER: &str = "#  End RCDir Aliases";
const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];
const UTF16_LE_BOM: [u8; 2] = [0xFF, 0xFE];
const UTF16_BE_BOM: [u8; 2] = [0xFE, 0xFF];





////////////////////////////////////////////////////////////////////////////////
//
//  read_profile_file
//
//  Reads a UTF-8 profile file into lines.  Detects BOM and returns whether
//  one was present (for preservation on write-back).
//  Returns Err for UTF-16 files (per FR-071).
//
////////////////////////////////////////////////////////////////////////////////

pub fn read_profile_file (path: &Path) -> Result<(Vec<String>, bool), AppError> {
    let bytes = fs::read (path).map_err (|e| AppError::Io (e))?;

    // Check for UTF-16 BOM — refuse to modify
    if bytes.len() >= 2 {
        if bytes[0..2] == UTF16_LE_BOM || bytes[0..2] == UTF16_BE_BOM {
            return Err (AppError::InvalidArg (format! (
                "Profile file is UTF-16 encoded: {}\nConvert to UTF-8 before using alias commands.",
                path.display()
            )));
        }
    }

    // Check for UTF-8 BOM
    let (content, has_bom) = if bytes.len() >= 3 && bytes[0..3] == UTF8_BOM {
        (String::from_utf8_lossy (&bytes[3..]).to_string(), true)
    } else {
        (String::from_utf8_lossy (&bytes).to_string(), false)
    };

    let lines: Vec<String> = content.lines().map (|l| l.to_string()).collect();
    Ok ((lines, has_bom))
}





////////////////////////////////////////////////////////////////////////////////
//
//  find_alias_block
//
//  Scans lines for opening/closing marker comments.
//  Returns an AliasBlock with found=true if a complete block is detected.
//  Processes only the first block found (per clarification).
//
////////////////////////////////////////////////////////////////////////////////

pub fn find_alias_block (lines: &[String]) -> AliasBlock {
    let mut block = AliasBlock::default();

    //
    // Find opening marker
    //

    for i in 0..lines.len() {
        if lines[i].contains (HEADER_MARKER) {
            // Backtrack to find banner start (### lines)
            let mut start = i;
            while start > 0 {
                let prev = &lines[start - 1];
                if prev.is_empty() {
                    break;
                }
                if prev.contains ("####") || (prev.len() <= 2 && prev.starts_with ('#')) {
                    start -= 1;
                } else {
                    break;
                }
            }
            block.start_line = start;

            // Parse version from the marker line
            if let Some (pos) = lines[i].find ("rcdir v") {
                let ver_str = lines[i][pos + 7..].trim();
                block.version = ver_str.to_string();
            }

            // Find closing marker
            for j in (i + 1)..lines.len() {
                if lines[j].contains (FOOTER_MARKER) {
                    let mut end = j;
                    // Scan forward past trailing ### lines
                    while end + 1 < lines.len() && lines[end + 1].contains ("####") {
                        end += 1;
                    }
                    block.end_line = end;
                    block.found = true;
                    break;
                }
            }

            break;
        }
    }

    //
    // Parse function names and full function lines from the block
    //

    if block.found {
        for k in block.start_line..=block.end_line {
            let line = &lines[k];
            if line.starts_with ("function ") {
                block.function_lines.push (line.clone());

                if let Some (rest) = line.strip_prefix ("function ") {
                    if let Some (pos) = rest.find (' ') {
                        let name = rest[..pos].to_string();
                        if block.root_alias.is_empty() {
                            block.root_alias = name.clone();
                        }
                        block.alias_names.push (name);
                    }
                }
            }
        }
    }

    block
}





////////////////////////////////////////////////////////////////////////////////
//
//  write_profile_file
//
//  Creates a timestamped backup, then writes lines back preserving BOM.
//  Creates parent directories if needed (FR-072).
//
////////////////////////////////////////////////////////////////////////////////

pub fn write_profile_file (
    path:     &Path,
    lines:    &[String],
    has_bom:  bool,
) -> Result<(), AppError> {

    // Create parent directories if needed
    if let Some (parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all (parent).map_err (|e| AppError::Io (e))?;
        }
    }

    // Create backup if file exists
    if path.exists() {
        create_backup (path)?;
    }

    // Write content
    let mut file = fs::File::create (path).map_err (|e| AppError::Io (e))?;

    if has_bom {
        file.write_all (&UTF8_BOM).map_err (|e| AppError::Io (e))?;
    }

    let content = lines.join ("\r\n");
    file.write_all (content.as_bytes()).map_err (|e| AppError::Io (e))?;

    // Ensure trailing newline
    if !content.ends_with ('\n') {
        file.write_all (b"\r\n").map_err (|e| AppError::Io (e))?;
    }

    Ok(())
}





////////////////////////////////////////////////////////////////////////////////
//
//  create_backup
//
//  Creates a timestamped .bak copy of the given file (FR-070).
//
////////////////////////////////////////////////////////////////////////////////

pub fn create_backup (path: &Path) -> Result<(), AppError> {
    let timestamp = format_backup_timestamp();

    let filename = path.file_name()
        .and_then (|n| n.to_str())
        .unwrap_or ("profile.ps1");

    let backup_name = format! ("{}.{}.bak", filename, timestamp);
    let backup_path = path.with_file_name (backup_name);

    fs::copy (path, &backup_path).map_err (|e| AppError::Io (e))?;
    Ok(())
}





////////////////////////////////////////////////////////////////////////////////
//
//  format_backup_timestamp
//
//  Returns a timestamp string in YYYY-MM-DD-HH-MM-SS format using
//  Windows GetLocalTime API.
//
////////////////////////////////////////////////////////////////////////////////

fn format_backup_timestamp() -> String {
    let st = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };

    format! (
        "{:04}-{:02}-{:02}-{:02}-{:02}-{:02}",
        st.wYear, st.wMonth, st.wDay,
        st.wHour, st.wMinute, st.wSecond,
    )
}





////////////////////////////////////////////////////////////////////////////////
//
//  replace_alias_block
//
//  Removes the existing block and inserts the new block at the same position.
//
////////////////////////////////////////////////////////////////////////////////

pub fn replace_alias_block (
    lines:     &mut Vec<String>,
    block:     &AliasBlock,
    new_block: &[String],
) {
    lines.splice (block.start_line..=block.end_line, new_block.iter().cloned());
}





////////////////////////////////////////////////////////////////////////////////
//
//  append_alias_block
//
//  Appends the block at the end of the file with a preceding blank line.
//
////////////////////////////////////////////////////////////////////////////////

pub fn append_alias_block (lines: &mut Vec<String>, new_block: &[String]) {
    // Ensure there's a blank line before the block
    if !lines.is_empty() && !lines.last().map_or (true, |l| l.is_empty()) {
        lines.push (String::new());
    }
    lines.extend (new_block.iter().cloned());
}





////////////////////////////////////////////////////////////////////////////////
//
//  remove_alias_block
//
//  Removes lines[start..=end] inclusive (FR-054, FR-055).
//
////////////////////////////////////////////////////////////////////////////////

pub fn remove_alias_block (lines: &mut Vec<String>, block: &AliasBlock) {
    lines.drain (block.start_line..=block.end_line);

    // Clean up trailing blank lines left behind
    while lines.len() > block.start_line
        && block.start_line < lines.len()
        && lines[block.start_line].is_empty()
    {
        lines.remove (block.start_line);
    }
}





#[cfg(test)]
mod tests {
    use super::*;



    ////////////////////////////////////////////////////////////////////////////
    //
    //  find_block_in_profile
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn find_block_in_profile() {
        let lines = vec![
            "# some existing content".to_string(),
            "".to_string(),
            "################################################################################".to_string(),
            "#".to_string(),
            "#  RCDir Aliases -- Generated by rcdir v5.1.1133".to_string(),
            "#".to_string(),
            "################################################################################".to_string(),
            "".to_string(),
            "function d   { rcdir @args    }".to_string(),
            "function dt  { d --tree @args }".to_string(),
            "".to_string(),
            "################################################################################".to_string(),
            "#  End RCDir Aliases".to_string(),
            "################################################################################".to_string(),
        ];

        let block = find_alias_block (&lines);
        assert! (block.found);
        assert_eq! (block.start_line, 2);
        assert_eq! (block.end_line, 13);
        assert_eq! (block.root_alias, "d");
        assert_eq! (block.alias_names, vec!["d", "dt"]);
        assert_eq! (block.function_lines.len(), 2);
        assert_eq! (block.version, "5.1.1133");
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  find_no_block
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn find_no_block() {
        let lines = vec![
            "# just a regular profile".to_string(),
            "Set-Alias ll Get-ChildItem".to_string(),
        ];

        let block = find_alias_block (&lines);
        assert! (!block.found);
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  replace_block
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn replace_block() {
        let mut lines = vec![
            "# before".to_string(),
            "################################################################################".to_string(),
            "#  RCDir Aliases -- Generated by rcdir v1.0".to_string(),
            "################################################################################".to_string(),
            "function d { rcdir @args }".to_string(),
            "################################################################################".to_string(),
            "#  End RCDir Aliases".to_string(),
            "################################################################################".to_string(),
            "# after".to_string(),
        ];

        let block = find_alias_block (&lines);
        assert! (block.found);

        let new_block = vec!["# NEW BLOCK".to_string()];
        replace_alias_block (&mut lines, &block, &new_block);

        assert_eq! (lines[0], "# before");
        assert_eq! (lines[1], "# NEW BLOCK");
        assert_eq! (lines[2], "# after");
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  append_block
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn append_block() {
        let mut lines = vec!["# existing".to_string()];
        let new_block = vec!["# block".to_string()];

        append_alias_block (&mut lines, &new_block);
        assert_eq! (lines.len(), 3); // existing + blank + block
        assert_eq! (lines[1], "");
        assert_eq! (lines[2], "# block");
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  remove_block
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn remove_block() {
        let mut lines = vec![
            "# before".to_string(),
            "################################################################################".to_string(),
            "#  RCDir Aliases -- Generated by rcdir v1.0".to_string(),
            "################################################################################".to_string(),
            "function d { rcdir @args }".to_string(),
            "################################################################################".to_string(),
            "#  End RCDir Aliases".to_string(),
            "################################################################################".to_string(),
            "# after".to_string(),
        ];

        let block = find_alias_block (&lines);
        remove_alias_block (&mut lines, &block);

        assert_eq! (lines.len(), 2);
        assert_eq! (lines[0], "# before");
        assert_eq! (lines[1], "# after");
    }
}
