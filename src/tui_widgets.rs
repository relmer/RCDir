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
//  show_cursor
//
//  Makes the console cursor visible.  Used during text_input so the user
//  can see the blinking caret.
//
////////////////////////////////////////////////////////////////////////////////

fn show_cursor() {
    if let Ok (stdout) = unsafe { GetStdHandle (STD_OUTPUT_HANDLE) } {
        let cursor_info = CONSOLE_CURSOR_INFO { dwSize: 25, bVisible: true.into() };
        let _ = unsafe { SetConsoleCursorInfo (stdout, &cursor_info) };
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

    let guard = TuiGuard::new()?;
    let mut value = String::new();

    // Show cursor for text entry (TuiGuard hides it by default)
    show_cursor();

    console.printf_attr (crate::config::Attribute::Information, &format! ("  {} [", prompt));
    console.printf_attr (crate::config::Attribute::InformationHighlight, default);
    console.printf_attr (crate::config::Attribute::Information, "]: ");
    console.flush()?;

    // Guidance line below prompt
    console.printf_attr (crate::config::Attribute::Information, "\n  (Enter=confirm, Esc=cancel)");
    // Move cursor back up to the input position
    console.write_raw ("\x1b[1A");
    // Position cursor at end of prompt line (after ]: )
    let cursor_col = 2 + prompt.len() + 2 + default.len() + 3;
    console.write_raw (&format! ("\x1b[{}G", cursor_col + 1));
    console.flush()?;

    loop {
        match guard.read_key()? {
            KeyEvent::Enter => {
                // Move to guidance line, clear it, then newline
                console.write_raw ("\n\x1b[2K");
                console.flush()?;
                let result = if value.is_empty() { default.to_string() } else { value };
                return Ok (TuiResult::Confirmed (result));
            }
            KeyEvent::Escape | KeyEvent::CtrlC => {
                console.write_raw ("\n\x1b[2K");
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
                    console.printf_attr (crate::config::Attribute::InformationHighlight, &ch.to_string());
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
    locked:   &[bool],
) -> Result<TuiResult<Vec<bool>>, AppError> {

    let guard = TuiGuard::new()?;
    let mut selected: Vec<bool> = items.iter().enumerate()
        .map (|(i, (_, s))| *s && !locked.get (i).copied().unwrap_or (false))
        .collect();
    let mut cursor = 0usize;

    render_checkbox_list (console, items, &selected, locked, cursor)?;

    loop {
        match guard.read_key()? {
            KeyEvent::Up    => { if cursor > 0 { cursor -= 1; } }
            KeyEvent::Down  => { if cursor < items.len() - 1 { cursor += 1; } }
            KeyEvent::Space => {
                if !locked.get (cursor).copied().unwrap_or (false) {
                    selected[cursor] = !selected[cursor];
                }
            }
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

        render_checkbox_list (console, items, &selected, locked, cursor)?;
    }
}

fn render_checkbox_list (
    console:  &mut Console,
    items:    &[(String, bool)],
    selected: &[bool],
    locked:   &[bool],
    cursor:   usize,
) -> Result<(), AppError> {
    // Count total display lines (including multi-line labels, locked warnings, and guidance)
    let total_lines: usize = items.iter().enumerate()
        .map (|(i, (label, _))| {
            let label_lines = 1 + label.chars().filter (|&c| c == '\n').count();
            let locked_extra = if locked.get (i).copied().unwrap_or (false) { 1 } else { 0 };
            label_lines + locked_extra
        })
        .sum::<usize>() + 1; // +1 for guidance line

    // Move cursor up to overwrite previous render
    if total_lines > 0 {
        console.write_raw (&format! ("\x1b[{}A", total_lines));
    }

    for (i, (label, _)) in items.iter().enumerate() {
        let is_locked = locked.get (i).copied().unwrap_or (false);

        console.write_raw ("\x1b[2K");
        if i == cursor {
            console.printf_attr (crate::config::Attribute::InformationHighlight, "  \u{276f} ");
        } else {
            console.printf_attr (crate::config::Attribute::Information, "    ");
        }
        console.printf_attr (crate::config::Attribute::Information, "[");
        if is_locked {
            console.printf_attr (crate::config::Attribute::Error, "x");
        } else if selected[i] {
            console.printf_attr (crate::config::Attribute::InformationHighlight, "\u{2713}");
        } else {
            console.printf_attr (crate::config::Attribute::Information, " ");
        }
        // Render label — clear each line for multi-line labels
        let label_parts: Vec<&str> = label.split ('\n').collect();
        for (j, part) in label_parts.iter().enumerate() {
            if j == 0 {
                console.printf_attr (crate::config::Attribute::Information, &format! ("] {}\n", part));
            } else {
                console.write_raw ("\x1b[2K");
                console.printf_attr (crate::config::Attribute::Information, &format! ("{}\n", part));
            }
        }

        if is_locked {
            console.write_raw ("\x1b[2K");
            console.printf_attr (crate::config::Attribute::Error, "        ^ conflicts with PowerShell built-in\n");
        }
    }

    // Guidance line
    console.write_raw ("\x1b[2K");
    console.printf_attr (crate::config::Attribute::Information, "  (Space=toggle, Enter=confirm, Esc=cancel)\n");

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
    let total_lines = items.len() + 1; // +1 for guidance line
    if total_lines > 0 {
        console.write_raw (&format! ("\x1b[{}A", total_lines));
    }

    for (i, label) in items.iter().enumerate() {
        console.write_raw ("\x1b[2K");
        if i == cursor {
            console.printf_attr (crate::config::Attribute::InformationHighlight, "  \u{276f} ");
        } else {
            console.printf_attr (crate::config::Attribute::Information, "    ");
        }
        console.printf_attr (crate::config::Attribute::Information, "(");
        if i == cursor {
            console.printf_attr (crate::config::Attribute::InformationHighlight, "\u{25cf}");
        } else {
            console.printf_attr (crate::config::Attribute::Information, " ");
        }
        console.printf_attr (crate::config::Attribute::Information, &format! (") {}\n", label));
    }

    // Guidance line
    console.write_raw ("\x1b[2K");
    console.printf_attr (crate::config::Attribute::Information, "  (Enter=select, Esc=cancel)\n");

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

    let guard = TuiGuard::new()?;
    show_cursor();

    console.printf_attr (crate::config::Attribute::Information, &format! ("  {} [", prompt));
    console.printf_attr (crate::config::Attribute::InformationHighlight, "Y/n");
    console.printf_attr (crate::config::Attribute::Information, "]: ");
    console.flush()?;

    loop {
        match guard.read_key()? {
            KeyEvent::Enter | KeyEvent::Char ('y') | KeyEvent::Char ('Y') => {
                console.printf_attr (crate::config::Attribute::InformationHighlight, "y");
                console.printf_attr (crate::config::Attribute::Information, "\n");
                console.flush()?;
                return Ok (TuiResult::Confirmed (true));
            }
            KeyEvent::Char ('n') | KeyEvent::Char ('N') | KeyEvent::Escape | KeyEvent::CtrlC => {
                console.printf_attr (crate::config::Attribute::InformationHighlight, "n");
                console.printf_attr (crate::config::Attribute::Information, "\n");
                console.flush()?;
                return Ok (TuiResult::Cancelled);
            }
            _ => {}
        }
    }
}


