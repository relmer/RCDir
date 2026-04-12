// config/file_reader.rs — Read .rcdirconfig file, handle BOM, split into lines
//
// Reads raw bytes from disk, checks for BOM (strips UTF-8 BOM, rejects
// UTF-16), converts to a UTF-8 String, and splits into lines.

use std::fs;
use std::io;
use std::path::Path;





////////////////////////////////////////////////////////////////////////////////
//
//  ConfigFileError
//
//  Distinguishes file-not-found (silent skip) from real I/O errors and
//  encoding errors.
//
////////////////////////////////////////////////////////////////////////////////

pub enum ConfigFileError {
    /// File does not exist — caller should silently skip.
    NotFound,

    /// Other I/O error (permissions, etc.) — caller should report.
    IoError (String),

    /// Unsupported encoding (UTF-16 BOM detected, or invalid UTF-8).
    EncodingError (String),
}





////////////////////////////////////////////////////////////////////////////////
//
//  UTF-8 / UTF-16 BOM constants
//
////////////////////////////////////////////////////////////////////////////////

const UTF8_BOM:     [u8; 3] = [0xEF, 0xBB, 0xBF];
const UTF16_LE_BOM: [u8; 2] = [0xFF, 0xFE];
const UTF16_BE_BOM: [u8; 2] = [0xFE, 0xFF];





////////////////////////////////////////////////////////////////////////////////
//
//  check_and_strip_bom
//
//  Detect and strip UTF-8 BOM from raw bytes.  Reject UTF-16 BOMs with
//  a descriptive error message.
//
////////////////////////////////////////////////////////////////////////////////

pub fn check_and_strip_bom (bytes: &mut Vec<u8>) -> Result<(), String> {
    if bytes.len() >= 3 && bytes[..3] == UTF8_BOM {
        bytes.drain (..3);
        return Ok (());
    }

    if bytes.len() >= 2 && bytes[..2] == UTF16_LE_BOM {
        return Err ("Unsupported encoding: UTF-16 LE (config file must be UTF-8)".into());
    }

    if bytes.len() >= 2 && bytes[..2] == UTF16_BE_BOM {
        return Err ("Unsupported encoding: UTF-16 BE (config file must be UTF-8)".into());
    }

    Ok (())
}





////////////////////////////////////////////////////////////////////////////////
//
//  read_config_file
//
//  Read a config file from disk and return its lines.  Handles BOM
//  stripping and UTF-8 validation.
//
////////////////////////////////////////////////////////////////////////////////

pub fn read_config_file (path: &str) -> Result<Vec<String>, ConfigFileError> {
    let file_path = Path::new (path);

    let mut bytes = match fs::read (file_path) {
        Ok (b) => b,
        Err (e) if e.kind() == io::ErrorKind::NotFound => {
            return Err (ConfigFileError::NotFound);
        }
        Err (e) => {
            return Err (ConfigFileError::IoError (
                format! ("Cannot open config file: {}", e)
            ));
        }
    };

    // Empty file is valid — no lines
    if bytes.is_empty() {
        return Ok (Vec::new());
    }

    check_and_strip_bom (&mut bytes)
        .map_err (ConfigFileError::EncodingError)?;

    let content = String::from_utf8 (bytes)
        .map_err (|_| ConfigFileError::EncodingError (
            "Failed to convert config file from UTF-8".into()
        ))?;

    let lines: Vec<String> = content.lines().map (|l| l.to_string()).collect();
    Ok (lines)
}





////////////////////////////////////////////////////////////////////////////////
//
//  Tests
//
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;



    #[test]
    fn bom_utf8_stripped() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF, b'h', b'e', b'l', b'l', b'o'];
        assert! (check_and_strip_bom (&mut bytes).is_ok());
        assert_eq! (bytes, b"hello");
    }



    #[test]
    fn bom_utf16_le_rejected() {
        let mut bytes = vec![0xFF, 0xFE, b'h', b'i'];
        let result = check_and_strip_bom (&mut bytes);
        assert! (result.is_err());
        assert! (result.unwrap_err().contains ("UTF-16 LE"));
    }



    #[test]
    fn bom_utf16_be_rejected() {
        let mut bytes = vec![0xFE, 0xFF, b'h', b'i'];
        let result = check_and_strip_bom (&mut bytes);
        assert! (result.is_err());
        assert! (result.unwrap_err().contains ("UTF-16 BE"));
    }



    #[test]
    fn bom_none_passes() {
        let mut bytes = vec![b'h', b'e', b'l', b'l', b'o'];
        assert! (check_and_strip_bom (&mut bytes).is_ok());
        assert_eq! (bytes, b"hello");
    }



    #[test]
    fn empty_bytes_passes() {
        let mut bytes: Vec<u8> = vec![];
        assert! (check_and_strip_bom (&mut bytes).is_ok());
    }



    #[test]
    fn read_nonexistent_file_returns_not_found() {
        let result = read_config_file ("C:\\nonexistent\\path\\.rcdirconfig");
        assert! (matches! (result, Err (ConfigFileError::NotFound)));
    }
}
