// ehm.rs — Error handling module
//
// Replaces TCDir's HRESULT + EHM macro pattern (CHR, CBR, CWRA, etc.)
// with Rust's Result<T, AppError> + ? operator + From trait conversions.

use std::fmt;
use std::path::PathBuf;





/// Unified error type for RCDir.
/// Maps to TCDir's HRESULT failure codes.
#[derive(Debug)]
pub enum AppError {
    /// Win32 API error (wraps windows::core::Error)
    Win32(windows::core::Error),

    /// Standard I/O error
    Io(std::io::Error),

    /// Invalid command-line argument (triggers usage display + exit 1)
    InvalidArg(String),

    /// Path does not exist
    PathNotFound(PathBuf),
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl fmt::Display for AppError
//
//  Formats AppError variants for display output.
//
////////////////////////////////////////////////////////////////////////////////

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Win32(e) => write!(f, "{}", e),
            AppError::Io(e) => write!(f, "{}", e),
            AppError::InvalidArg(msg) => write!(f, "{}", msg),
            AppError::PathNotFound(path) => {
                write!(f, "Error:   {} does not exist", path.display())
            }
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl std::error::Error for AppError
//
//  Returns the underlying error source, if any.
//
////////////////////////////////////////////////////////////////////////////////

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Win32(e) => Some(e),
            AppError::Io(e) => Some(e),
            _ => None,
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl From<windows::core::Error> for AppError
//
//  Converts a Win32 error into AppError::Win32.
//
////////////////////////////////////////////////////////////////////////////////

impl From<windows::core::Error> for AppError {
    fn from(e: windows::core::Error) -> Self {
        AppError::Win32(e)
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl From<std::io::Error> for AppError
//
//  Converts a standard I/O error into AppError::Io.
//
////////////////////////////////////////////////////////////////////////////////

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}





#[cfg(test)]
mod tests {
    use super::*;

    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_invalid_arg
    //
    //  Verifies display output for InvalidArg error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn display_invalid_arg() {
        let e = AppError::InvalidArg("bad switch".into());
        assert_eq!(format!("{}", e), "bad switch");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_path_not_found
    //
    //  Verifies display output for PathNotFound error.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn display_path_not_found() {
        let e = AppError::PathNotFound(PathBuf::from(r"C:\NoSuchDir"));
        assert_eq!(format!("{}", e), r"Error:   C:\NoSuchDir does not exist");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  from_io_error
    //
    //  Verifies conversion from std::io::Error to AppError::Io.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let app_err: AppError = io_err.into();
        assert!(matches!(app_err, AppError::Io(_)));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  from_win32_error
    //
    //  Verifies conversion from windows::core::Error to AppError::Win32.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn from_win32_error() {
        let win_err = windows::core::Error::from_hresult(windows::core::HRESULT(0x80070002u32 as i32));
        let app_err: AppError = win_err.into();
        assert!(matches!(app_err, AppError::Win32(_)));
    }
}
