// src/tui_widgets.rs — Interactive TUI components for alias configuration
//
// Provides: TuiGuard (RAII console mode), TextInput, CheckboxList,
// RadioButtonList, ConfirmationPrompt.

use windows::Win32::System::Console::*;

use crate::console::Console;
use crate::ehm::AppError;





////////////////////////////////////////////////////////////////////////////////
//
//  TuiGuard
//
//  RAII guard that saves console mode + cursor visibility on creation,
//  sets raw input mode, and restores everything on Drop.
//
////////////////////////////////////////////////////////////////////////////////

pub struct TuiGuard {
    stdin_handle:    windows::Win32::Foundation::HANDLE,
    original_mode:   CONSOLE_MODE,
    cursor_was_visible: bool,
}

impl TuiGuard {
    pub fn new() -> Result<Self, AppError> {
        let stdin = unsafe { GetStdHandle (STD_INPUT_HANDLE) }
            .map_err (|e| AppError::Win32 (e))?;

        // Save original mode
        let mut original_mode = CONSOLE_MODE::default();
        unsafe { GetConsoleMode (stdin, &mut original_mode) }
            .map_err (|e| AppError::Win32 (e))?;

        // Save cursor visibility
        let stdout = unsafe { GetStdHandle (STD_OUTPUT_HANDLE) }
            .map_err (|e| AppError::Win32 (e))?;
        let mut cursor_info = CONSOLE_CURSOR_INFO::default();
        unsafe { GetConsoleCursorInfo (stdout, &mut cursor_info) }
            .map_err (|e| AppError::Win32 (e))?;
        let cursor_was_visible = cursor_info.bVisible.as_bool();

        // Set raw input mode
        let raw_mode = ENABLE_EXTENDED_FLAGS | ENABLE_WINDOW_INPUT;
        unsafe { SetConsoleMode (stdin, raw_mode) }
            .map_err (|e| AppError::Win32 (e))?;

        // Flush input buffer
        unsafe { FlushConsoleInputBuffer (stdin) }
            .map_err (|e| AppError::Win32 (e))?;

        // Hide cursor
        let hidden_cursor = CONSOLE_CURSOR_INFO { dwSize: cursor_info.dwSize, bVisible: false.into() };
        unsafe { SetConsoleCursorInfo (stdout, &hidden_cursor) }
            .map_err (|e| AppError::Win32 (e))?;

        Ok (TuiGuard { stdin_handle: stdin, original_mode, cursor_was_visible })
    }

    pub fn read_key (&self) -> Result<KeyEvent, AppError> {
        let mut buf = [INPUT_RECORD::default(); 1];
        let mut count = 0u32;

        loop {
            unsafe { ReadConsoleInputW (self.stdin_handle, &mut buf, &mut count) }
                .map_err (|e| AppError::Win32 (e))?;

            if count > 0 {
                let rec = &buf[0];
                if rec.EventType == KEY_EVENT as u16 {
                    let key = unsafe { rec.Event.KeyEvent };
                    if key.bKeyDown.as_bool() {
                        let vk = key.wVirtualKeyCode;
                        let ch = unsafe { key.uChar.UnicodeChar };
                        let ctrl = key.dwControlKeyState;

                        // Detect Ctrl+C
                        if ch == 3 && (ctrl & 0x0008 != 0  // LEFT_CTRL_PRESSED
                            || ctrl & 0x0004 != 0)          // RIGHT_CTRL_PRESSED
                        {
                            return Ok (KeyEvent::CtrlC);
                        }

                        return Ok (match vk {
                            0x26 => KeyEvent::Up,       // VK_UP
                            0x28 => KeyEvent::Down,     // VK_DOWN
                            0x20 => KeyEvent::Space,    // VK_SPACE
                            0x0D => KeyEvent::Enter,    // VK_RETURN
                            0x1B => KeyEvent::Escape,   // VK_ESCAPE
                            0x08 => KeyEvent::Backspace,// VK_BACK
                            _    => {
                                if ch > 0 {
                                    KeyEvent::Char (char::from_u32 (ch as u32).unwrap_or ('\0'))
                                } else {
                                    KeyEvent::Other
                                }
                            }
                        });
                    }
                }
            }
        }
    }
}

impl Drop for TuiGuard {
    fn drop (&mut self) {
        // Restore console mode
        let _ = unsafe { SetConsoleMode (self.stdin_handle, self.original_mode) };

        // Restore cursor visibility
        if let Ok (stdout) = unsafe { GetStdHandle (STD_OUTPUT_HANDLE) } {
            let cursor_info = CONSOLE_CURSOR_INFO { dwSize: 25, bVisible: self.cursor_was_visible.into() };
            let _ = unsafe { SetConsoleCursorInfo (stdout, &cursor_info) };
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  KeyEvent
//
//  Simplified key event enum for TUI consumption.
//
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq)]
pub enum KeyEvent {
    Up,
    Down,
    Space,
    Enter,
    Escape,
    Backspace,
    CtrlC,
    Char(char),
    Other,
}





////////////////////////////////////////////////////////////////////////////////
//
//  TuiResult
//
//  Result of a TUI widget interaction.
//
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum TuiResult<T> {
    Confirmed(T),
    Cancelled,
}





////////////////////////////////////////////////////////////////////////////////
//
//  text_input
//
//  Prompts for text input with a default value.
//  Accepts 1-4 alphanumeric characters.  Enter confirms, Escape cancels.
//  (FR-020, FR-021)
//
////////////////////////////////////////////////////////////////////////////////

pub fn text_input (
    console:      &mut Console,
    prompt:       &str,
    default:      &str,
) -> Result<TuiResult<String>, AppError> {

    // Check for test mode
    if let Some (val) = get_test_input() {
        return Ok (TuiResult::Confirmed (if val.is_empty() { default.to_string() } else { val }));
    }

    let guard = TuiGuard::new()?;
    let mut value = String::new();

    console.printf_attr (crate::config::Attribute::Information, &format! ("  {} [{}]: ", prompt, default));
    console.flush()?;

    loop {
        match guard.read_key()? {
            KeyEvent::Enter => {
                console.printf_attr (crate::config::Attribute::Information, "\n");
                console.flush()?;
                let result = if value.is_empty() { default.to_string() } else { value };
                return Ok (TuiResult::Confirmed (result));
            }
            KeyEvent::Escape | KeyEvent::CtrlC => {
                console.printf_attr (crate::config::Attribute::Information, "\n");
                console.flush()?;
                return Ok (TuiResult::Cancelled);
            }
            KeyEvent::Backspace => {
                if !value.is_empty() {
                    value.pop();
                    // Erase character: move back, space, move back
                    console.write_raw ("\x08 \x08");
                    console.flush()?;
                }
            }
            KeyEvent::Char (ch) => {
                if ch.is_alphanumeric() && value.len() < 4 {
                    value.push (ch);
                    console.write_raw (&ch.to_string());
                    console.flush()?;
                }
            }
            _ => {}
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  checkbox_list
//
//  Displays a checkbox list with [✓]/[ ] markers.
//  Space toggles, Enter confirms, Escape cancels.
//  (FR-012, FR-013, FR-022, FR-024)
//
////////////////////////////////////////////////////////////////////////////////

pub fn checkbox_list (
    console:  &mut Console,
    items:    &[(String, bool)],
) -> Result<TuiResult<Vec<bool>>, AppError> {

    // Check for test mode
    if let Some (val) = get_test_input() {
        let states = if val == "all" {
            vec![true; items.len()]
        } else {
            items.iter().map (|(_, checked)| *checked).collect()
        };
        return Ok (TuiResult::Confirmed (states));
    }

    let guard = TuiGuard::new()?;
    let mut selected: Vec<bool> = items.iter().map (|(_, s)| *s).collect();
    let mut cursor = 0usize;

    render_checkbox_list (console, items, &selected, cursor)?;

    loop {
        match guard.read_key()? {
            KeyEvent::Up    => { if cursor > 0 { cursor -= 1; } }
            KeyEvent::Down  => { if cursor < items.len() - 1 { cursor += 1; } }
            KeyEvent::Space => { selected[cursor] = !selected[cursor]; }
            KeyEvent::Enter => {
                console.printf_attr (crate::config::Attribute::Information, "\n");
                console.flush()?;
                return Ok (TuiResult::Confirmed (selected));
            }
            KeyEvent::Escape | KeyEvent::CtrlC => {
                console.printf_attr (crate::config::Attribute::Information, "\n");
                console.flush()?;
                return Ok (TuiResult::Cancelled);
            }
            _ => continue,
        }

        render_checkbox_list (console, items, &selected, cursor)?;
    }
}

fn render_checkbox_list (
    console:  &mut Console,
    items:    &[(String, bool)],
    selected: &[bool],
    cursor:   usize,
) -> Result<(), AppError> {
    // Move cursor up to overwrite previous render
    if items.len() > 0 {
        console.write_raw (&format! ("\x1b[{}A", items.len()));
    }

    for (i, (label, _)) in items.iter().enumerate() {
        let focus = if i == cursor { "❯" } else { " " };
        let check = if selected[i] { "✓" } else { " " };
        console.write_raw (&format! ("\x1b[2K  {} [{}] {}\n", focus, check, label));
    }

    console.flush()?;
    Ok(())
}





////////////////////////////////////////////////////////////////////////////////
//
//  radio_button_list
//
//  Displays a radio button list with (●)/( ) markers.
//  Arrow keys navigate, Enter selects, Escape cancels.
//  (FR-011, FR-013, FR-025)
//
////////////////////////////////////////////////////////////////////////////////

pub fn radio_button_list (
    console:  &mut Console,
    items:    &[String],
    default:  usize,
) -> Result<TuiResult<usize>, AppError> {

    // Check for test mode
    if let Some (val) = get_test_input() {
        // Parse by name match or use default
        for (i, item) in items.iter().enumerate() {
            if item.contains (&val) {
                return Ok (TuiResult::Confirmed (i));
            }
        }
        return Ok (TuiResult::Confirmed (default));
    }

    let guard = TuiGuard::new()?;
    let mut cursor = default;

    render_radio_list (console, items, cursor)?;

    loop {
        match guard.read_key()? {
            KeyEvent::Up    => { if cursor > 0 { cursor -= 1; } }
            KeyEvent::Down  => { if cursor < items.len() - 1 { cursor += 1; } }
            KeyEvent::Enter => {
                console.printf_attr (crate::config::Attribute::Information, "\n");
                console.flush()?;
                return Ok (TuiResult::Confirmed (cursor));
            }
            KeyEvent::Escape | KeyEvent::CtrlC => {
                console.printf_attr (crate::config::Attribute::Information, "\n");
                console.flush()?;
                return Ok (TuiResult::Cancelled);
            }
            _ => continue,
        }

        render_radio_list (console, items, cursor)?;
    }
}

fn render_radio_list (
    console: &mut Console,
    items:   &[String],
    cursor:  usize,
) -> Result<(), AppError> {
    if items.len() > 0 {
        console.write_raw (&format! ("\x1b[{}A", items.len()));
    }

    for (i, label) in items.iter().enumerate() {
        let focus = if i == cursor { "❯" } else { " " };
        let radio = if i == cursor { "●" } else { " " };
        console.write_raw (&format! ("\x1b[2K  {} ({}) {}\n", focus, radio, label));
    }

    console.flush()?;
    Ok(())
}





////////////////////////////////////////////////////////////////////////////////
//
//  confirmation_prompt
//
//  Displays a Y/N confirmation prompt.  Enter or Y confirms, N or Escape
//  cancels.  (FR-029)
//
////////////////////////////////////////////////////////////////////////////////

pub fn confirmation_prompt (
    console: &mut Console,
    prompt:  &str,
) -> Result<TuiResult<bool>, AppError> {

    // Check for test mode
    if let Some (val) = get_test_input() {
        let confirmed = val.eq_ignore_ascii_case ("y") || val.eq_ignore_ascii_case ("yes");
        return Ok (TuiResult::Confirmed (confirmed));
    }

    let guard = TuiGuard::new()?;

    console.printf_attr (crate::config::Attribute::Information, &format! ("  {} [Y/n]: ", prompt));
    console.flush()?;

    loop {
        match guard.read_key()? {
            KeyEvent::Enter | KeyEvent::Char ('y') | KeyEvent::Char ('Y') => {
                console.printf_attr (crate::config::Attribute::Information, "y\n");
                console.flush()?;
                return Ok (TuiResult::Confirmed (true));
            }
            KeyEvent::Char ('n') | KeyEvent::Char ('N') | KeyEvent::Escape | KeyEvent::CtrlC => {
                console.printf_attr (crate::config::Attribute::Information, "n\n");
                console.flush()?;
                return Ok (TuiResult::Cancelled);
            }
            _ => {}
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  Test input support (FR-090)
//
//  When RCDIR_ALIAS_TEST_INPUTS is set, each widget call consumes the next
//  semicolon-delimited value instead of reading interactive input.
//
////////////////////////////////////////////////////////////////////////////////

use std::sync::Mutex;
use std::sync::OnceLock;

static TEST_INPUTS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
static TEST_INITIALIZED: OnceLock<bool> = OnceLock::new();

fn init_test_inputs() {
    TEST_INITIALIZED.get_or_init (|| {
        if let Ok (val) = std::env::var ("RCDIR_ALIAS_TEST_INPUTS") {
            let inputs: Vec<String> = val.split (';').map (|s| s.to_string()).collect();
            let _ = TEST_INPUTS.set (Mutex::new (inputs));
        }
        true
    });
}

fn get_test_input() -> Option<String> {
    init_test_inputs();

    TEST_INPUTS.get().and_then (|mutex| {
        let mut inputs = mutex.lock().ok()?;
        if inputs.is_empty() {
            None
        } else {
            Some (inputs.remove (0))
        }
    })
}

