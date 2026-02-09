// console.rs — Buffered console output with ANSI colors
//
// Port of: Console.h, Console.cpp
// Key design: All output accumulates in a pre-allocated 10 MB String buffer.
// Color changes are ANSI SGR sequences inline in the buffer.
// The entire buffer is flushed in one WriteConsoleW / WriteFile call.

use std::sync::Arc;

use windows::Win32::System::Console::{
    GetStdHandle, GetConsoleMode, SetConsoleMode, GetConsoleScreenBufferInfo,
    STD_OUTPUT_HANDLE, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
    CONSOLE_SCREEN_BUFFER_INFO,
};
use windows::Win32::Storage::FileSystem::WriteFile;

use crate::ansi_codes;
use crate::config::{Config, Attribute};
use crate::ehm::AppError;

/// Initial buffer capacity: 10 MB (matches TCDir's s_kcchInitialBufferSize)
const INITIAL_BUFFER_SIZE: usize = 10 * 1024 * 1024;

pub struct Console {
    buffer:        String,
    stdout_handle: windows::Win32::Foundation::HANDLE,
    is_redirected: bool,
    console_width: u32,
    config:        Arc<Config>,
    prev_attr:     Option<u16>,
}

impl Console {
    /// Initialize the console: get stdout handle, detect redirection,
    /// enable VT processing, query width, pre-allocate buffer.
    ///
    /// Port of: CConsole::Initialize + InitializeConsoleMode + InitializeConsoleWidth
    pub fn initialize(config: Arc<Config>) -> Result<Self, AppError> {
        let stdout_handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE)? };

        let mut is_redirected = true;
        let mut console_width = 80u32;

        // Try to get console mode — if it succeeds, we're not redirected
        let mut mode = windows::Win32::System::Console::CONSOLE_MODE(0);
        let mode_ok = unsafe { GetConsoleMode(stdout_handle, &mut mode) };
        if mode_ok.is_ok() {
            is_redirected = false;

            // Enable virtual terminal processing for ANSI escape sequences
            let new_mode = windows::Win32::System::Console::CONSOLE_MODE(
                mode.0 | ENABLE_VIRTUAL_TERMINAL_PROCESSING.0
            );
            let _ = unsafe { SetConsoleMode(stdout_handle, new_mode) };

            // Query console width
            let mut csbi = CONSOLE_SCREEN_BUFFER_INFO::default();
            let info_ok = unsafe { GetConsoleScreenBufferInfo(stdout_handle, &mut csbi) };
            if info_ok.is_ok() {
                console_width = (csbi.srWindow.Right - csbi.srWindow.Left + 1) as u32;
            }
        }

        Ok(Console {
            buffer: String::with_capacity(INITIAL_BUFFER_SIZE),
            stdout_handle,
            is_redirected,
            console_width,
            config,
            prev_attr: None,
        })
    }

    /// Get the console width in columns.
    pub fn width(&self) -> u32 {
        self.console_width
    }

    /// Get a reference to the config.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get a shared reference-counted pointer to the config.
    /// Use this when you need config data across mutable Console calls.
    pub fn config_arc(&self) -> Arc<Config> {
        Arc::clone(&self.config)
    }

    /// Emit ANSI SGR color sequence if the color has changed from the previous call.
    /// Color elision: skip if unchanged (major perf optimization).
    ///
    /// Port of: CConsole::SetColor
    pub fn set_color(&mut self, attr: u16) {
        if self.prev_attr == Some(attr) {
            return;
        }
        self.prev_attr = Some(attr);
        ansi_codes::write_sgr(&mut self.buffer, attr);
    }

    /// Write a single character with a specific color attribute.
    ///
    /// Port of: CConsole::Putchar
    pub fn putchar(&mut self, attr: u16, ch: char) {
        self.set_color(attr);
        self.buffer.push(ch);
    }

    /// Write a string with a named attribute, followed by a newline.
    /// Resets to Default color before the newline to prevent color bleeding.
    ///
    /// Port of: CConsole::Puts
    pub fn puts(&mut self, attr_idx: Attribute, text: &str) {
        let attr = self.config.attributes[attr_idx as usize];
        self.process_multiline_string(text, attr);

        // Reset to default color before final newline
        let default_attr = self.config.attributes[Attribute::Default as usize];
        self.set_color(default_attr);
        self.buffer.push('\n');
    }

    /// Write formatted text with a specific color attribute (no trailing newline).
    ///
    /// Port of: CConsole::Printf (WORD attr variant)
    pub fn printf(&mut self, attr: u16, text: &str) {
        self.process_multiline_string(text, attr);
    }

    /// Write formatted text with a named attribute (no trailing newline).
    pub fn printf_attr(&mut self, attr_idx: Attribute, text: &str) {
        let attr = self.config.attributes[attr_idx as usize];
        self.process_multiline_string(text, attr);
    }

    /// Write text with embedded {MarkerName} color markers.
    /// Markers switch the active color; text between markers uses the current color.
    /// No trailing newline.
    ///
    /// Port of: CConsole::ColorPrint / ColorPrintf
    pub fn color_printf(&mut self, text: &str) {
        let default_attr = self.config.attributes[Attribute::Default as usize];
        let mut current_attr = default_attr;
        let mut rest = text;

        while !rest.is_empty() {
            if let Some(brace_pos) = rest.find('{') {
                // Emit text before the marker
                if brace_pos > 0 {
                    self.process_multiline_string(&rest[..brace_pos], current_attr);
                }

                // Try to parse the marker
                let after_brace = &rest[brace_pos + 1..];
                if let Some(close_pos) = after_brace.find('}') {
                    let marker_name = &after_brace[..close_pos];
                    if let Some(attr_idx) = Attribute::from_name(marker_name) {
                        current_attr = self.config.attributes[attr_idx as usize];
                        rest = &after_brace[close_pos + 1..];
                    } else {
                        // Unknown marker — emit the '{' as literal
                        self.process_multiline_string("{", current_attr);
                        rest = after_brace;
                    }
                } else {
                    // Unclosed brace — emit remaining text
                    self.process_multiline_string(&rest[brace_pos..], current_attr);
                    break;
                }
            } else {
                // No more markers
                self.process_multiline_string(rest, current_attr);
                break;
            }
        }
    }

    /// Write text with embedded color markers, followed by a newline.
    /// Port of: CConsole::ColorPuts
    pub fn color_puts(&mut self, text: &str) {
        self.color_printf(text);

        let default_attr = self.config.attributes[Attribute::Default as usize];
        self.set_color(default_attr);
        self.buffer.push('\n');
    }

    /// Print a string, cycling through all 16 colors for each character.
    /// Skips any color that matches the background to keep text visible.
    ///
    /// Port of: CConsole::PrintColorfulString
    pub fn print_colorful_string(&mut self, text: &str) {
        use crate::color::{ALL_FOREGROUND_COLORS, COLOR_COUNT};

        let default_attr = self.config.attributes[Attribute::Default as usize];
        let background = (default_attr >> 4) & 0x0F;
        let mut idx = 0usize;

        for ch in text.chars() {
            let mut color = ALL_FOREGROUND_COLORS[idx % COLOR_COUNT];
            idx += 1;

            // Skip if it matches the background
            if color == background {
                color = ALL_FOREGROUND_COLORS[idx % COLOR_COUNT];
                idx += 1;
            }

            let attr = color | (background << 4);
            self.putchar(attr, ch);
        }
    }

    /// Flush the buffer to the OS.
    /// - Real console: WriteConsoleW with UTF-16 conversion
    /// - Redirected: WriteFile with UTF-8 encoding
    ///
    /// Appends reset sequence before flushing.
    ///
    /// Port of: CConsole::Flush
    pub fn flush(&mut self) -> Result<(), AppError> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        if !self.is_redirected {
            // Real console: convert to UTF-16 and use WriteConsoleW
            let wide: Vec<u16> = self.buffer.encode_utf16().collect();
            let mut written = 0u32;
            unsafe {
                windows::Win32::System::Console::WriteConsoleW(
                    self.stdout_handle,
                    &wide,
                    Some(&mut written),
                    None,
                )?;
            }
        } else {
            // Redirected: write UTF-8 bytes via WriteFile
            let bytes = self.buffer.as_bytes();
            let mut written = 0u32;
            unsafe {
                WriteFile(
                    self.stdout_handle,
                    Some(bytes),
                    Some(&mut written),
                    None,
                )?;
            }
        }

        self.buffer.clear();
        self.prev_attr = None;
        Ok(())
    }

    /// Append raw text to the buffer (no color change).
    /// Used internally and for separator lines.
    pub fn write_raw(&mut self, text: &str) {
        self.buffer.push_str(text);
    }

    /// Helper: process text with proper color handling for embedded newlines.
    /// Resets to default color before each newline, then restores the desired color.
    ///
    /// Port of: CConsole::ProcessMultiLineStringWithAttribute
    fn process_multiline_string(&mut self, text: &str, attr: u16) {
        let default_attr = self.config.attributes[Attribute::Default as usize];
        self.set_color(attr);

        let mut rest = text;
        while let Some(nl_pos) = rest.find('\n') {
            // Text before newline
            self.buffer.push_str(&rest[..nl_pos]);

            // Reset to default before newline
            self.set_color(default_attr);
            self.buffer.push('\n');

            // Restore color for next line
            self.set_color(attr);

            rest = &rest[nl_pos + 1..];
        }

        // Remaining text after last newline (or all text if no newlines)
        if !rest.is_empty() {
            self.buffer.push_str(rest);
        }
    }
}

impl Drop for Console {
    fn drop(&mut self) {
        // Append reset sequence and flush on drop
        self.buffer.push_str(ansi_codes::RESET_ALL);
        let _ = self.flush();
    }
}

#[cfg(test)]
mod tests {
    // Console tests require a real stdout handle (Win32 API calls),
    // so unit tests here are limited. Integration tests validate output parity.

    use super::*;

    #[test]
    fn initial_buffer_size_is_10mb() {
        assert_eq!(INITIAL_BUFFER_SIZE, 10 * 1024 * 1024);
    }
}
