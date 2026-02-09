// ansi_codes.rs — ANSI escape sequence constants
//
// Port of: AnsiCodes.h
// Maps Windows console color indices (4-bit WORD) to ANSI SGR codes.

/// Reset all attributes sequence: ESC[0m
pub const RESET_ALL: &str = "\x1b[0m";

/// ANSI foreground color codes (normal: 30-37, bright: 90-97)
pub const FG_BLACK:   i32 = 30;
pub const FG_RED:     i32 = 31;
pub const FG_GREEN:   i32 = 32;
pub const FG_YELLOW:  i32 = 33;
pub const FG_BLUE:    i32 = 34;
pub const FG_MAGENTA: i32 = 35;
pub const FG_CYAN:    i32 = 36;
pub const FG_WHITE:   i32 = 37;

/// Background offset: background code = foreground code + 10
pub const BG_OFFSET: i32 = 10;

/// Bright offset: add 60 to base code for bright variant (90-97 fg, 100-107 bg)
pub const BRIGHT_OFFSET: i32 = 60;

/// Maps Windows console color index (0-7) to ANSI foreground color code.
/// Index: 0=Black, 1=Blue, 2=Green, 3=Cyan, 4=Red, 5=Magenta, 6=Yellow, 7=White
///
/// Note: Windows and ANSI use different color orderings:
///   Windows: 0=Black 1=Blue 2=Green 3=Cyan 4=Red 5=Magenta 6=Yellow 7=White
///   ANSI:    30=Black 31=Red 32=Green 33=Yellow 34=Blue 35=Magenta 36=Cyan 37=White
pub const CONSOLE_COLOR_TO_ANSI: [i32; 8] = [
    FG_BLACK,     // 0: Black   -> 30
    FG_BLUE,      // 1: Blue    -> 34
    FG_GREEN,     // 2: Green   -> 32
    FG_CYAN,      // 3: Cyan    -> 36
    FG_RED,       // 4: Red     -> 31
    FG_MAGENTA,   // 5: Magenta -> 35
    FG_YELLOW,    // 6: Yellow  -> 33
    FG_WHITE,     // 7: White   -> 37
];

/// Convert a Windows console WORD (4-bit fg | 4-bit bg<<4) to an ANSI SGR sequence
/// and write it into the provided buffer string.
///
/// Format: ESC[{fg};{bg}m
/// Example: word 0x0C (bright red on black) → "\x1b[91;40m"
pub fn write_sgr(buf: &mut String, attr: u16) {
    use std::fmt::Write;

    let fg_index = (attr & 0x0F) as usize;
    let bg_index = ((attr >> 4) & 0x0F) as usize;

    // Base color (low 3 bits) → ANSI code
    let mut fg_code = CONSOLE_COLOR_TO_ANSI[fg_index & 0x07];
    let mut bg_code = CONSOLE_COLOR_TO_ANSI[bg_index & 0x07] + BG_OFFSET;

    // Intensity bit (bit 3) → bright variant (+60)
    if fg_index & 0x08 != 0 {
        fg_code += BRIGHT_OFFSET;
    }
    if bg_index & 0x08 != 0 {
        bg_code += BRIGHT_OFFSET;
    }

    let _ = write!(buf, "\x1b[{};{}m", fg_code, bg_code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sgr_bright_red_on_black() {
        // Bright red fg (0x0C) on black bg (0x00) = WORD 0x000C
        let mut buf = String::new();
        write_sgr(&mut buf, 0x000C);
        assert_eq!(buf, "\x1b[91;40m");
    }

    #[test]
    fn sgr_white_on_blue() {
        // White fg (0x0F) on blue bg (0x10) = WORD 0x001F
        let mut buf = String::new();
        write_sgr(&mut buf, 0x001F);
        // White=0x0F: base 7 (White=37) + bright (+60) = 97
        // Blue bg=0x01: base 1 (Blue=34) + 10 = 44
        assert_eq!(buf, "\x1b[97;44m");
    }

    #[test]
    fn sgr_default_grey_on_black() {
        // LightGrey fg (0x07) on black bg (0x00) = WORD 0x0007
        let mut buf = String::new();
        write_sgr(&mut buf, 0x0007);
        assert_eq!(buf, "\x1b[37;40m");
    }

    #[test]
    fn sgr_yellow_on_dark_red() {
        // Yellow fg (0x0E) on DarkRed bg (0x40) = WORD 0x004E
        let mut buf = String::new();
        write_sgr(&mut buf, 0x004E);
        // Yellow=0x0E: base 6 (Yellow=33) + bright (+60) = 93
        // DarkRed bg=0x04: base 4 (Red=31) + 10 = 41
        assert_eq!(buf, "\x1b[93;41m");
    }

    #[test]
    fn console_color_table_length() {
        assert_eq!(CONSOLE_COLOR_TO_ANSI.len(), 8);
    }
}
