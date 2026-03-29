// console.rs — Buffered console output with ANSI colors
//
// Port of: Console.h, Console.cpp
// Key design: All output accumulates in a pre-allocated 10 MB String buffer.
// Color changes are ANSI SGR (Select Graphic Rendition) sequences inline in
// the buffer.
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





////////////////////////////////////////////////////////////////////////////////
//
//  impl Console
//
//  Console output with ANSI color support and buffered writes.
//
////////////////////////////////////////////////////////////////////////////////

impl Console {
    ////////////////////////////////////////////////////////////////////////////
    //
    //  initialize
    //
    //  Initialize the console: get stdout handle, detect redirection,
    //  enable VT processing, query width, pre-allocate buffer.
    //
    //  Port of: CConsole::Initialize + InitializeConsoleMode +
    //  InitializeConsoleWidth
    //
    ////////////////////////////////////////////////////////////////////////////

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





    ////////////////////////////////////////////////////////////////////////////
    //
    //  width
    //
    //  Get the console width in columns.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn width(&self) -> u32 {
        self.console_width
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  config
    //
    //  Get a reference to the config.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn config(&self) -> &Config {
        &self.config
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  config_arc
    //
    //  Get a shared reference-counted pointer to the config.
    //  Use this when you need config data across mutable Console calls.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn config_arc(&self) -> Arc<Config> {
        Arc::clone(&self.config)
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  set_color
    //
    //  Emit ANSI SGR color sequence if the color has changed from the
    //  previous call.  Color elision: skip if unchanged (major perf
    //  optimization).
    //
    //  Port of: CConsole::SetColor
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn set_color(&mut self, attr: u16) {
        if self.prev_attr == Some(attr) {
            return;
        }
        self.prev_attr = Some(attr);
        ansi_codes::write_sgr(&mut self.buffer, attr);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  putchar
    //
    //  Write a single character with a specific color attribute.
    //
    //  Port of: CConsole::Putchar
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn putchar(&mut self, attr: u16, ch: char) {
        self.set_color(attr);
        self.buffer.push(ch);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  puts
    //
    //  Write a string with a named attribute, followed by a newline.
    //  Resets to Default color before the newline to prevent color bleeding.
    //
    //  Port of: CConsole::Puts
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn puts(&mut self, attr_idx: Attribute, text: &str) {
        let attr = self.config.attributes[attr_idx as usize];
        self.process_multiline_string(text, attr);

        // Reset to default color before final newline
        let default_attr = self.config.attributes[Attribute::Default as usize];
        self.set_color(default_attr);
        self.buffer.push('\n');
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  printf
    //
    //  Write formatted text with a specific color attribute
    //  (no trailing newline).
    //
    //  Port of: CConsole::Printf (WORD attr variant)
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn printf(&mut self, attr: u16, text: &str) {
        self.process_multiline_string(text, attr);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  printf_attr
    //
    //  Write formatted text with a named attribute (no trailing newline).
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn printf_attr(&mut self, attr_idx: Attribute, text: &str) {
        let attr = self.config.attributes[attr_idx as usize];
        self.process_multiline_string(text, attr);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  writef
    //
    //  Write pre-formatted arguments directly into the output buffer with
    //  a specific color attribute.  Avoids the intermediate String
    //  allocation that `printf(attr, &format!(...))` would require.
    //
    //  IMPORTANT: The caller guarantees the formatted text contains NO
    //  embedded newlines.  Use `writef_line` for text that needs a
    //  trailing newline with proper color-reset semantics.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn writef(&mut self, attr: u16, args: std::fmt::Arguments<'_>) {
        self.set_color (attr);
        std::fmt::Write::write_fmt (&mut self.buffer, args).unwrap();
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  writef_attr
    //
    //  Write pre-formatted arguments directly into the output buffer with
    //  a named attribute.  Same zero-allocation semantics as `writef`.
    //
    //  IMPORTANT: The caller guarantees the formatted text contains NO
    //  embedded newlines.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn writef_attr(&mut self, attr_idx: Attribute, args: std::fmt::Arguments<'_>) {
        let attr = self.config.attributes[attr_idx as usize];
        self.set_color (attr);
        std::fmt::Write::write_fmt (&mut self.buffer, args).unwrap();
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  writef_line
    //
    //  Write pre-formatted arguments into the output buffer followed by a
    //  newline.  Resets to default color before the newline to prevent
    //  color bleeding (matching `process_multiline_string` semantics).
    //
    //  IMPORTANT: The caller guarantees the formatted text contains NO
    //  embedded newlines.  The trailing newline is handled here.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn writef_line(&mut self, attr: u16, args: std::fmt::Arguments<'_>) {
        self.set_color (attr);
        std::fmt::Write::write_fmt (&mut self.buffer, args).unwrap();

        let default_attr = self.config.attributes[Attribute::Default as usize];
        self.set_color (default_attr);
        self.buffer.push ('\n');

        // Restore attr after the newline to match process_multiline_string
        // semantics (ensures color-elision state is identical for the next
        // printf/writef call).
        self.set_color (attr);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_printf
    //
    //  Write text with embedded {MarkerName} color markers.  Markers
    //  switch the active color; text between markers uses the current
    //  color.  No trailing newline.
    //
    //  Port of: CConsole::ColorPrint / ColorPrintf
    //
    //  Matches TCDir's ColorPrint algorithm exactly:
    //  - Always calls process_multiline_string for text before each
    //    marker, even when that text is empty (emits SetColor for the
    //    default attr).
    //  - This ensures the same color-reset sequence as TCDir at the
    //    start of each ColorPrintf call.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn color_printf(&mut self, text: &str) {
        let default_attr = self.config.attributes[Attribute::Default as usize];
        let mut current_attr = default_attr;
        let mut chunk_start = 0;

        while chunk_start < text.len() {
            // Find the next potential marker
            let chunk_end = text[chunk_start..].find('{').map(|p| chunk_start + p);

            // Emit text before the marker (or all remaining text if no marker found)
            let chunk = match chunk_end {
                Some(end) => &text[chunk_start..end],
                None      => &text[chunk_start..],
            };
            self.process_multiline_string(chunk, current_attr);

            // If no marker found, we're done
            let Some(brace_pos) = chunk_end else { break };

            // Try to parse the marker
            let after_brace = &text[brace_pos + 1..];
            if let Some(close_pos) = after_brace.find('}') {
                let marker_name = &after_brace[..close_pos];
                if let Some(attr_idx) = Attribute::from_name(marker_name) {
                    current_attr = self.config.attributes[attr_idx as usize];
                    chunk_start = brace_pos + 1 + close_pos + 1;
                } else {
                    // Unknown marker — emit the '{' as literal
                    self.process_multiline_string("{", current_attr);
                    chunk_start = brace_pos + 1;
                }
            } else {
                // Unclosed brace — emit '{' and remaining text
                self.process_multiline_string(&text[brace_pos..], current_attr);
                break;
            }
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_puts
    //
    //  Write text with embedded color markers, followed by a newline.
    //
    //  Port of: CConsole::ColorPuts
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn color_puts(&mut self, text: &str) {
        self.color_printf(text);

        let default_attr = self.config.attributes[Attribute::Default as usize];
        self.set_color(default_attr);
        self.buffer.push('\n');
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  print_colorful_string
    //
    //  Print a string, cycling through all 16 colors for each character.
    //  Skips any color that matches the background to keep text visible.
    //
    //  Port of: CConsole::PrintColorfulString
    //
    ////////////////////////////////////////////////////////////////////////////

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





    ////////////////////////////////////////////////////////////////////////////
    //
    //  flush
    //
    //  Flush the buffer to the OS.
    //  - Real console: WriteConsoleW with UTF-16 conversion
    //  - Redirected: WriteFile with UTF-8 encoding
    //
    //  Appends reset sequence before flushing.
    //
    //  Port of: CConsole::Flush
    //
    ////////////////////////////////////////////////////////////////////////////

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





    ////////////////////////////////////////////////////////////////////////////
    //
    //  write_raw
    //
    //  Append raw text to the buffer (no color change).
    //  Used internally and for separator lines.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn write_raw(&mut self, text: &str) {
        self.buffer.push_str(text);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  new_for_testing
    //
    //  Create a Console suitable for unit tests — no Win32 handle, no real
    //  output.  Buffer contents can be inspected via take_test_buffer().
    //
    ////////////////////////////////////////////////////////////////////////////

    #[cfg(test)]
    pub(crate) fn new_for_testing (config: Arc<Config>) -> Console {
        Console {
            buffer:        String::with_capacity (4096),
            stdout_handle: windows::Win32::Foundation::HANDLE(std::ptr::null_mut()),
            is_redirected: true,
            console_width: 120,
            config,
            prev_attr:     None,
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  take_test_buffer
    //
    //  Move the current buffer contents out, leaving the buffer empty.
    //  This prevents the Drop impl from trying to flush stale content.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[cfg(test)]
    pub(crate) fn take_test_buffer (&mut self) -> String {
        self.prev_attr = None;
        std::mem::take (&mut self.buffer)
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  process_multiline_string
    //
    //  Helper: process text with proper color handling for embedded
    //  newlines.  Resets to default color before each newline, then
    //  restores the desired color.
    //
    //  Port of: CConsole::ProcessMultiLineStringWithAttribute
    //
    ////////////////////////////////////////////////////////////////////////////

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





////////////////////////////////////////////////////////////////////////////////
//
//  impl Drop for Console
//
//  Append reset sequence and flush on drop.
//
////////////////////////////////////////////////////////////////////////////////

impl Drop for Console {
    fn drop(&mut self) {
        // Append reset sequence and flush on drop
        self.buffer.push_str(ansi_codes::RESET_ALL);
        let _ = self.flush();
    }
}





#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a Console for testing with default-initialized Config.
    fn make_test_console() -> Console {
        let mut cfg = Config::new();
        cfg.initialize (0x07); // LightGrey on Black default
        Console::new_for_testing (Arc::new (cfg))
    }

    /// Helper: strip all ANSI SGR escape sequences from text, leaving only
    /// visible characters.  Used to verify text content independently of
    /// color changes.
    fn strip_ansi (s: &str) -> String {
        let mut result = String::with_capacity (s.len());
        let mut chars = s.chars().peekable();
        while let Some (ch) = chars.next() {
            if ch == '\x1b' {
                // Skip ESC [ ... m sequence
                if chars.peek() == Some (&'[') {
                    chars.next(); // consume '['
                    while let Some (&c) = chars.peek() {
                        chars.next();
                        if c == 'm' { break; }
                    }
                }
            } else {
                result.push (ch);
            }
        }
        result
    }

    ////////////////////////////////////////////////////////////////////////////
    //
    //  initial_buffer_size_is_10mb
    //
    //  Verify initial buffer capacity is 10 MB.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn initial_buffer_size_is_10mb() {
        assert_eq!(INITIAL_BUFFER_SIZE, 10 * 1024 * 1024);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_puts_no_markers_outputs_entire_string
    //
    //  Port of: ColorPuts_NoMarkers_OutputsEntireString
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn color_puts_no_markers_outputs_entire_string() {
        let mut con = make_test_console();
        con.color_puts ("Hello, World!");
        let buf = con.take_test_buffer();
        let plain = strip_ansi (&buf);
        assert! (plain.contains ("Hello, World!"));
        assert! (plain.ends_with ('\n'));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_puts_single_marker_switches_color
    //
    //  Port of: ColorPuts_SingleMarker_SwitchesColor
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn color_puts_single_marker_switches_color() {
        let mut con = make_test_console();
        con.color_puts ("Normal {InformationHighlight}Highlighted");
        let buf = con.take_test_buffer();
        let plain = strip_ansi (&buf);
        assert! (plain.contains ("Normal "));
        assert! (plain.contains ("Highlighted"));
        // The marker itself must not appear in output
        assert! (!plain.contains ("{InformationHighlight}"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_puts_multiple_markers_switches_correctly
    //
    //  Port of: ColorPuts_MultipleMarkers_SwitchesColorsCorrectly
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn color_puts_multiple_markers_switches_correctly() {
        let mut con = make_test_console();
        con.color_puts ("{InformationHighlight}-A{Information}  Displays files");
        let buf = con.take_test_buffer();
        let plain = strip_ansi (&buf);
        assert! (plain.contains ("-A"));
        assert! (plain.contains ("Displays files"));
        assert! (!plain.contains ("{InformationHighlight}"));
        assert! (!plain.contains ("{Information}"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_puts_all_attributes_parses_correctly
    //
    //  Port of: ColorPuts_AllAttributes_ParsesCorrectly
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn color_puts_all_attributes_parses_correctly() {
        let mut con = make_test_console();
        con.color_puts (
            "{Default}D\
             {Date}Dt\
             {Time}Tm\
             {FileAttributePresent}Fa\
             {FileAttributeNotPresent}Fn\
             {Size}Sz\
             {Directory}Dr\
             {Information}In\
             {InformationHighlight}Hl\
             {SeparatorLine}Sl\
             {Error}Er\
             {Owner}Ow\
             {Stream}St\
             {CloudStatusCloudOnly}C1\
             {CloudStatusLocallyAvailable}C2\
             {CloudStatusAlwaysLocallyAvailable}C3"
        );
        let buf = con.take_test_buffer();
        let plain = strip_ansi (&buf);
        assert! (plain.contains ("DDtTmFaFnSzDrInHlSlErOwStC1C2C3"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_puts_cloud_status_markers
    //
    //  Port of: ColorPuts_CloudStatusMarkers_ApplyCorrectColors
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn color_puts_cloud_status_markers() {
        let mut con = make_test_console();
        con.color_puts (
            "Cloud: {CloudStatusCloudOnly}\u{25CB}{Information} \
             {CloudStatusLocallyAvailable}\u{25D0}{Information} \
             {CloudStatusAlwaysLocallyAvailable}\u{25CF}{Information}"
        );
        let buf = con.take_test_buffer();
        let plain = strip_ansi (&buf);
        assert! (plain.contains ("\u{25CB}"));
        assert! (plain.contains ("\u{25D0}"));
        assert! (plain.contains ("\u{25CF}"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_puts_marker_at_start
    //
    //  Port of: ColorPuts_MarkerAtStart_ParsesCorrectly
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn color_puts_marker_at_start() {
        let mut con = make_test_console();
        con.color_puts ("{InformationHighlight}Highlighted text");
        let buf = con.take_test_buffer();
        let plain = strip_ansi (&buf);
        assert! (plain.contains ("Highlighted text"));
        assert! (!plain.contains ("{InformationHighlight}"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_puts_marker_at_end
    //
    //  Port of: ColorPuts_MarkerAtEnd_ParsesCorrectly
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn color_puts_marker_at_end() {
        let mut con = make_test_console();
        con.color_puts ("Text then marker{Default}");
        let buf = con.take_test_buffer();
        let plain = strip_ansi (&buf);
        assert! (plain.contains ("Text then marker"));
        assert! (!plain.contains ("{Default}"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_puts_consecutive_markers
    //
    //  Port of: ColorPuts_ConsecutiveMarkers_ParsesCorrectly
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn color_puts_consecutive_markers() {
        let mut con = make_test_console();
        con.color_puts ("{Information}{InformationHighlight}Text");
        let buf = con.take_test_buffer();
        let plain = strip_ansi (&buf);
        assert! (plain.contains ("Text"));
        assert! (!plain.contains ("{Information}"));
        assert! (!plain.contains ("{InformationHighlight}"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_puts_empty_string_no_output
    //
    //  Port of: ColorPuts_EmptyString_NoOutput
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn color_puts_empty_string_no_output() {
        let mut con = make_test_console();
        con.color_puts ("");
        let buf = con.take_test_buffer();
        let plain = strip_ansi (&buf);
        // Only a newline from color_puts
        assert_eq! (plain.trim(), "");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_puts_only_markers_no_visible_output
    //
    //  Port of: ColorPuts_OnlyMarkers_NoVisibleOutput
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn color_puts_only_markers_no_visible_output() {
        let mut con = make_test_console();
        con.color_puts ("{Information}{InformationHighlight}{Default}");
        let buf = con.take_test_buffer();
        let plain = strip_ansi (&buf);
        assert_eq! (plain.trim(), "");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_puts_multiline_with_markers
    //
    //  Port of: ColorPuts_MultilineWithMarkers_HandlesNewlines
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn color_puts_multiline_with_markers() {
        let mut con = make_test_console();
        con.color_puts (
            "Line 1 {InformationHighlight}highlighted\nLine 2 {Information}normal"
        );
        let buf = con.take_test_buffer();
        let plain = strip_ansi (&buf);
        assert! (plain.contains ("Line 1 "));
        assert! (plain.contains ("highlighted"));
        assert! (plain.contains ("Line 2 "));
        assert! (plain.contains ("normal"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_printf_formats_and_processes_markers
    //
    //  Port of: ColorPrintf_FormatsAndProcessesMarkers
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn color_printf_formats_and_processes_markers() {
        let mut con = make_test_console();
        con.color_printf (&format! ("{{InformationHighlight}}{}{{Information}} = {}", "Value", 42));
        let buf = con.take_test_buffer();
        let plain = strip_ansi (&buf);
        assert! (plain.contains ("Value"));
        assert! (plain.contains ("= 42"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_printf_no_markers_formats_correctly
    //
    //  Port of: ColorPrintf_NoMarkers_FormatsCorrectly
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn color_printf_no_markers_formats_correctly() {
        let mut con = make_test_console();
        con.color_printf (&format! ("Simple format: {}", 123));
        let buf = con.take_test_buffer();
        let plain = strip_ansi (&buf);
        assert! (plain.contains ("Simple format: 123"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_puts_unclosed_brace_emits_literal
    //
    //  Port of: ColorPuts_UnclosedBrace_AssertsInDebug
    //  In Rust, unclosed braces emit remaining text as literal (no panic).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn color_puts_unclosed_brace_emits_literal() {
        let mut con = make_test_console();
        con.color_puts ("Text with {Information unclosed");
        let buf = con.take_test_buffer();
        let plain = strip_ansi (&buf);
        // Unclosed brace: the remaining text is emitted as literal
        assert! (plain.contains ("Text with "));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_puts_unknown_marker_emits_literal
    //
    //  Port of: ColorPuts_UnknownMarkerName_AssertsInDebug
    //  In Rust, unknown markers emit '{' as literal and continue parsing.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn color_puts_unknown_marker_emits_literal() {
        let mut con = make_test_console();
        con.color_puts ("Text with {UnknownMarker} here");
        let buf = con.take_test_buffer();
        let plain = strip_ansi (&buf);
        assert! (plain.contains ("Text with "));
        assert! (plain.contains ("{UnknownMarker}"));
        assert! (plain.contains ("here"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_puts_valid_marker_does_not_panic
    //
    //  Port of: ColorPuts_ValidMarker_DoesNotAssert
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn color_puts_valid_marker_does_not_panic() {
        let mut con = make_test_console();
        con.color_puts ("Text with {InformationHighlight}valid{Information} marker");
        let buf = con.take_test_buffer();
        let plain = strip_ansi (&buf);
        assert! (plain.contains ("Text with "));
        assert! (plain.contains ("valid"));
        assert! (plain.contains (" marker"));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  color_printf_switch_style_usage
    //
    //  Port of: ColorPuts_SwitchStyleUsage_FormatsCorrectly
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn color_printf_switch_style_usage() {
        let mut con = make_test_console();
        con.color_puts (
            "  {InformationHighlight}-A{Information}          Displays files with specified attributes."
        );
        let buf = con.take_test_buffer();
        let plain = strip_ansi (&buf);
        assert! (plain.contains ("-A"));
        assert! (plain.contains ("Displays files with specified attributes."));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  process_multiline_string_resets_color_at_newlines
    //
    //  Verify that process_multiline_string resets to default color before
    //  each embedded newline and restores the desired color after.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn process_multiline_string_resets_color_at_newlines() {
        let mut con = make_test_console();
        // Use a non-default color (bright red = 0x0C)
        con.process_multiline_string ("line1\nline2", 0x0C);
        let buf = con.take_test_buffer();

        // Should contain at least two color sequences (one for bright red, one for reset/default)
        let sgr_count = buf.matches ("\x1b[").count();
        assert! (sgr_count >= 3, "Expected at least 3 SGR sequences (color, reset at newline, restore), got {}", sgr_count);
        let plain = strip_ansi (&buf);
        assert_eq! (plain, "line1\nline2");
    }
}
