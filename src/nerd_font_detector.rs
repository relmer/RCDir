// RCDir - Nerd Font Detector
// Layered detection strategy for Nerd Font presence.
//
// Port of: TCDirCore/NerdFontDetector.h, NerdFontDetector.cpp
// Five-step detection chain with trait-based GDI abstraction for testability.

use windows::Win32::Foundation::HANDLE;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateFontW, DeleteDC, DeleteObject, EnumFontFamiliesExW,
    GetGlyphIndicesW, SelectObject,
    CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_QUALITY, FF_MODERN,
    FIXED_PITCH, FONT_CHARSET, FONT_CLIP_PRECISION, FONT_OUTPUT_PRECISION,
    FONT_QUALITY, FW_NORMAL, GGI_MARK_NONEXISTING_GLYPHS, LOGFONTW,
    OUT_DEFAULT_PRECIS,
};
use windows::Win32::System::Console::{
    GetCurrentConsoleFontEx, CONSOLE_FONT_INFOEX,
};
use windows::core::PCWSTR;

use crate::ehm::AppError;
use crate::environment_provider::EnvironmentProvider;
use crate::icon_mapping::NF_CUSTOM_FOLDER;





////////////////////////////////////////////////////////////////////////////////
//
//  IconActivation
//
//  How icon display was requested.
//
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconActivation {
    /// Determined by auto-detection
    Auto,
    /// /Icons CLI flag or RCDIR=Icons
    ForceOn,
    /// /Icons- CLI flag or RCDIR=Icons-
    ForceOff,
}





////////////////////////////////////////////////////////////////////////////////
//
//  DetectionResult
//
//  Result of the Nerd Font detection probe.
//
//  Port of: EDetectionResult in TCDirCore/NerdFontDetector.h
//
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionResult {
    /// Nerd Font confirmed (canary glyph found or WezTerm env)
    Detected,
    /// No Nerd Font found
    NotDetected,
    /// Detection failed or inconclusive — default to OFF
    Inconclusive,
}





////////////////////////////////////////////////////////////////////////////////
//
//  FontProber trait
//
//  Abstraction over GDI-dependent Nerd Font probing operations.
//  RCDir production code uses DefaultFontProber; tests use MockFontProber.
//
//  Port of: protected virtual methods on CNerdFontDetector
//
////////////////////////////////////////////////////////////////////////////////

pub trait FontProber {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  probe_console_font_for_glyph
    //
    //  Probe the current console font for a specific canary glyph.
    //  Returns Ok(true) if the glyph is present, Ok(false) if missing.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn probe_console_font_for_glyph(
        &self,
        console_handle: HANDLE,
        canary: char,
    ) -> Result<bool, AppError>;


    ////////////////////////////////////////////////////////////////////////////
    //
    //  is_nerd_font_installed
    //
    //  Check whether any Nerd Font is installed system-wide via font
    //  enumeration.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn is_nerd_font_installed(&self) -> Result<bool, AppError>;
}





////////////////////////////////////////////////////////////////////////////////
//
//  DefaultFontProber
//
//  Production implementation using Win32 GDI APIs.
//
//  Port of: CNerdFontDetector::ProbeConsoleFontForGlyph,
//           CNerdFontDetector::IsNerdFontInstalled
//
////////////////////////////////////////////////////////////////////////////////

pub struct DefaultFontProber;





////////////////////////////////////////////////////////////////////////////////
//
//  impl FontProber for DefaultFontProber
//
//  GDI-based canary probe and system font enumeration.
//
////////////////////////////////////////////////////////////////////////////////

impl FontProber for DefaultFontProber {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  probe_console_font_for_glyph
    //
    //  Classic conhost canary probe:
    //    1. GetCurrentConsoleFontEx → get font face name
    //    2. CreateCompatibleDC(None)
    //    3. CreateFontW with face name
    //    4. SelectObject
    //    5. GetGlyphIndicesW with GGI_MARK_NONEXISTING_GLYPHS
    //    6. Cleanup
    //    7. Return Ok(glyph_index != 0xFFFF)
    //
    //  Port of: CNerdFontDetector::ProbeConsoleFontForGlyph
    //
    ////////////////////////////////////////////////////////////////////////////

    fn probe_console_font_for_glyph(
        &self,
        console_handle: HANDLE,
        canary: char,
    ) -> Result<bool, AppError> {
        unsafe {
            // Get current console font info
            let mut font_info = CONSOLE_FONT_INFOEX {
                cbSize: std::mem::size_of::<CONSOLE_FONT_INFOEX>() as u32,
                ..Default::default()
            };

            GetCurrentConsoleFontEx (console_handle, false, &mut font_info)
                .map_err (AppError::Win32)?;

            // Create a memory DC
            let hdc = CreateCompatibleDC (None);
            if hdc.is_invalid() {
                return Err (AppError::Win32 (windows::core::Error::from_thread()));
            }

            // Create a font matching the console font
            let font_height = -(font_info.dwFontSize.Y as i32);
            let hfont = CreateFontW (
                font_height,
                0, 0, 0,
                FW_NORMAL.0 as i32,
                0, 0, 0,
                FONT_CHARSET (DEFAULT_CHARSET.0),
                FONT_OUTPUT_PRECISION (OUT_DEFAULT_PRECIS.0),
                FONT_CLIP_PRECISION (CLIP_DEFAULT_PRECIS.0),
                FONT_QUALITY (DEFAULT_QUALITY.0),
                (FIXED_PITCH.0 | FF_MODERN.0) as u32,
                PCWSTR (font_info.FaceName.as_ptr()),
            );

            if hfont.is_invalid() {
                let _ = DeleteDC (hdc);
                return Err (AppError::Win32 (windows::core::Error::from_thread()));
            }

            let hfont_old = SelectObject (hdc, hfont.into());

            // Probe for the canary glyph
            let mut buf = [0u16; 2];
            let encoded = canary.encode_utf16 (&mut buf);
            let mut glyph_idx: u16 = 0;

            let result = GetGlyphIndicesW (
                hdc,
                PCWSTR (encoded.as_ptr()),
                1,
                &mut glyph_idx,
                GGI_MARK_NONEXISTING_GLYPHS,
            );

            let has_glyph = result != 0xFFFF_FFFF && glyph_idx != 0xFFFF;

            // Cleanup
            let _ = SelectObject (hdc, hfont_old);
            let _ = DeleteObject (hfont.into());
            let _ = DeleteDC (hdc);

            Ok (has_glyph)
        }
    }


    ////////////////////////////////////////////////////////////////////////////
    //
    //  is_nerd_font_installed
    //
    //  Enumerate system fonts and check if any font family name contains
    //  "Nerd Font" or ends with " NF"/" NFM"/" NFP" (case-insensitive).
    //
    //  Port of: CNerdFontDetector::IsNerdFontInstalled + EnumFontCallback
    //
    ////////////////////////////////////////////////////////////////////////////

    fn is_nerd_font_installed(&self) -> Result<bool, AppError> {
        unsafe {
            let hdc = CreateCompatibleDC (None);
            if hdc.is_invalid() {
                return Err (AppError::Win32 (windows::core::Error::from_thread()));
            }

            let lf = LOGFONTW {
                lfCharSet: DEFAULT_CHARSET,
                ..Default::default()
            };

            let mut found = false;

            EnumFontFamiliesExW (
                hdc,
                &lf,
                Some (enum_font_callback),
                windows::Win32::Foundation::LPARAM (&mut found as *mut bool as isize),
                0,
            );

            let _ = DeleteDC (hdc);

            Ok (found)
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  enum_font_callback
//
//  Callback for EnumFontFamiliesExW.  Checks if the font family name
//  contains "nerd font" or ends with " nf"/" nfm"/" nfp" (case-insensitive).
//
//  Port of: EnumFontCallback in TCDirCore/NerdFontDetector.cpp
//
////////////////////////////////////////////////////////////////////////////////

unsafe extern "system" fn enum_font_callback(
    plf: *const LOGFONTW,
    _textmetric: *const windows::Win32::Graphics::Gdi::TEXTMETRICW,
    _font_type: u32,
    lparam: windows::Win32::Foundation::LPARAM,
) -> i32 {
    let found = unsafe { &mut *(lparam.0 as *mut bool) };
    let lf = unsafe { &*plf };

    // Extract font name from lfFaceName (null-terminated WCHAR array)
    let name_len = lf.lfFaceName.iter().position (|&c| c == 0).unwrap_or (lf.lfFaceName.len());
    let name: String = String::from_utf16_lossy (&lf.lfFaceName[..name_len]);
    let name_lower = name.to_ascii_lowercase();

    if name_lower.contains ("nerd font") {
        *found = true;
        return 0; // Stop enumeration
    }

    if name_lower.ends_with (" nf")
        || name_lower.ends_with (" nfm")
        || name_lower.ends_with (" nfp")
    {
        *found = true;
        return 0; // Stop enumeration
    }

    1 // Continue enumeration
}





////////////////////////////////////////////////////////////////////////////////
//
//  detect
//
//  Run the layered Nerd Font detection chain.
//
//  Detection order (identical to TCDir):
//    1. WezTerm environment → Detected
//    2. ConPTY terminal detected → skip GDI canary, go to font enum
//    3. Classic conhost — GDI canary probe U+E5FF
//    4. System font enumeration
//    5. Fallback → Inconclusive (treated as OFF)
//
//  Port of: CNerdFontDetector::Detect
//
////////////////////////////////////////////////////////////////////////////////

pub fn detect(
    console_handle: HANDLE,
    env_provider: &dyn EnvironmentProvider,
    prober: &dyn FontProber,
) -> DetectionResult {
    // Step 1: WezTerm always bundles NF Symbols as a fallback font
    if is_wezterm (env_provider) {
        return DetectionResult::Detected;
    }

    // Step 2: ConPTY — can't use GDI canary, skip to font enumeration
    if is_conpty_terminal (env_provider) {
        return match prober.is_nerd_font_installed() {
            Ok (true)  => DetectionResult::Detected,
            Ok (false) => DetectionResult::NotDetected,
            Err (_)    => DetectionResult::Inconclusive,
        };
    }

    // Step 3: Classic conhost — GDI canary probe
    match prober.probe_console_font_for_glyph (console_handle, NF_CUSTOM_FOLDER) {
        Ok (true)  => return DetectionResult::Detected,
        Ok (false) => return DetectionResult::NotDetected,
        Err (_)    => {} // Fall through to font enumeration
    }

    // Step 4: Canary probe failed — fall back to system font enumeration
    match prober.is_nerd_font_installed() {
        Ok (true)  => DetectionResult::Detected,
        Ok (false) => DetectionResult::NotDetected,
        Err (_)    => DetectionResult::Inconclusive,
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  is_wezterm
//
//  Check if TERM_PROGRAM is set to "WezTerm" (case-insensitive).
//
//  Port of: CNerdFontDetector::IsWezTerm
//
////////////////////////////////////////////////////////////////////////////////

fn is_wezterm(env: &dyn EnvironmentProvider) -> bool {
    env.get_env_var ("TERM_PROGRAM")
        .is_some_and (|v| v.eq_ignore_ascii_case ("WezTerm"))
}





////////////////////////////////////////////////////////////////////////////////
//
//  is_conpty_terminal
//
//  Check if running under ConPTY (Windows Terminal, VS Code, etc.).
//
//  Port of: CNerdFontDetector::IsConPtyTerminal
//
////////////////////////////////////////////////////////////////////////////////

fn is_conpty_terminal(env: &dyn EnvironmentProvider) -> bool {
    const CONPTY_ENV_VARS: &[&str] = &[
        "WT_SESSION",          // Windows Terminal
        "TERM_PROGRAM",        // VS Code, Hyper, etc.
        "ConEmuPID",           // ConEmu
        "ALACRITTY_WINDOW_ID", // Alacritty
    ];

    for var in CONPTY_ENV_VARS {
        if let Some (v) = env.get_env_var (var)
            && !v.is_empty()
        {
            return true;
        }
    }

    false
}





////////////////////////////////////////////////////////////////////////////////
//
//  Unit Tests
//
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment_provider::MockEnvironmentProvider;





    ////////////////////////////////////////////////////////////////////////////
    //
    //  MockFontProber
    //
    //  Configurable test double for FontProber trait.
    //
    ////////////////////////////////////////////////////////////////////////////

    struct MockFontProber {
        canary_result:   Result<bool, AppError>,
        nf_installed:    Result<bool, AppError>,
    }

    impl MockFontProber {
        fn new(canary: Result<bool, AppError>, installed: Result<bool, AppError>) -> Self {
            MockFontProber {
                canary_result:  canary,
                nf_installed:   installed,
            }
        }
    }

    impl FontProber for MockFontProber {
        fn probe_console_font_for_glyph(
            &self,
            _console_handle: HANDLE,
            _canary: char,
        ) -> Result<bool, AppError> {
            match &self.canary_result {
                Ok (v) => Ok (*v),
                Err (_) => Err (AppError::Win32 (windows::core::Error::from_thread())),
            }
        }

        fn is_nerd_font_installed(&self) -> Result<bool, AppError> {
            match &self.nf_installed {
                Ok (v) => Ok (*v),
                Err (_) => Err (AppError::Win32 (windows::core::Error::from_thread())),
            }
        }
    }

    fn make_env(vars: &[(&str, &str)]) -> MockEnvironmentProvider {
        let mut mock = MockEnvironmentProvider::new();
        for &(k, v) in vars {
            mock.set (k, v);
        }
        mock
    }

    fn dummy_handle() -> HANDLE {
        HANDLE (std::ptr::null_mut())
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_wezterm_detected
    //
    //  WezTerm environment → Detected (prober never called).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_wezterm_detected() {
        let env = make_env (&[("TERM_PROGRAM", "WezTerm")]);
        let prober = MockFontProber::new (Ok (false), Ok (false));
        assert_eq! (detect (dummy_handle(), &env, &prober), DetectionResult::Detected);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_conpty_nf_installed
    //
    //  ConPTY terminal + NF installed → Detected.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_conpty_nf_installed() {
        let env = make_env (&[("WT_SESSION", "12345")]);
        let prober = MockFontProber::new (Ok (false), Ok (true));
        assert_eq! (detect (dummy_handle(), &env, &prober), DetectionResult::Detected);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_conpty_nf_not_installed
    //
    //  ConPTY terminal + NF not installed → NotDetected.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_conpty_nf_not_installed() {
        let env = make_env (&[("WT_SESSION", "12345")]);
        let prober = MockFontProber::new (Ok (false), Ok (false));
        assert_eq! (detect (dummy_handle(), &env, &prober), DetectionResult::NotDetected);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_classic_conhost_canary_hit
    //
    //  Classic conhost + canary glyph found → Detected.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_classic_conhost_canary_hit() {
        let env = make_env (&[]);
        let prober = MockFontProber::new (Ok (true), Ok (false));
        assert_eq! (detect (dummy_handle(), &env, &prober), DetectionResult::Detected);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_classic_conhost_canary_miss
    //
    //  Classic conhost + canary glyph not found → NotDetected.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_classic_conhost_canary_miss() {
        let env = make_env (&[]);
        let prober = MockFontProber::new (Ok (false), Ok (false));
        assert_eq! (detect (dummy_handle(), &env, &prober), DetectionResult::NotDetected);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_canary_fails_font_enum_succeeds
    //
    //  Canary probe fails, font enumeration finds NF → Detected.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_canary_fails_font_enum_succeeds() {
        let env = make_env (&[]);
        let prober = MockFontProber::new (
            Err (AppError::Win32 (windows::core::Error::from_thread())),
            Ok (true),
        );
        assert_eq! (detect (dummy_handle(), &env, &prober), DetectionResult::Detected);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_all_probes_fail
    //
    //  All probes fail → Inconclusive.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_all_probes_fail() {
        let env = make_env (&[]);
        let prober = MockFontProber::new (
            Err (AppError::Win32 (windows::core::Error::from_thread())),
            Err (AppError::Win32 (windows::core::Error::from_thread())),
        );
        assert_eq! (detect (dummy_handle(), &env, &prober), DetectionResult::Inconclusive);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_is_wezterm_true
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_is_wezterm_true() {
        let env = make_env (&[("TERM_PROGRAM", "WezTerm")]);
        assert! (is_wezterm (&env));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_is_wezterm_false
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_is_wezterm_false() {
        let env = make_env (&[]);
        assert! (!is_wezterm (&env));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_is_wezterm_case_insensitive
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_is_wezterm_case_insensitive() {
        let env = make_env (&[("TERM_PROGRAM", "wezterm")]);
        assert! (is_wezterm (&env));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_is_conpty_wt_session
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_is_conpty_wt_session() {
        let env = make_env (&[("WT_SESSION", "abc")]);
        assert! (is_conpty_terminal (&env));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_is_conpty_none
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_is_conpty_none() {
        let env = make_env (&[]);
        assert! (!is_conpty_terminal (&env));
    }
}
