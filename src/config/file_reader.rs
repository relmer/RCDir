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
