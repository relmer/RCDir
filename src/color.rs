// color.rs — Color types, name mapping, and color spec parsing
//
// Port of: Color.h (EForeColor/EBackColor enums) + Config.cpp (ParseColorName/ParseColorSpec)
//
// Windows console uses a 4-bit color model:
//   Foreground: bits 0-3 of WORD (FOREGROUND_BLUE=1, GREEN=2, RED=4, INTENSITY=8)
//   Background: bits 4-7 of WORD (BACKGROUND_BLUE=16, GREEN=32, RED=64, INTENSITY=128)

use crate::ehm::AppError;

// ── Foreground color constants (bits 0-3) ─────────────────────────────────────

pub const FC_BLACK:         u16 = 0x00;
pub const FC_BLUE:          u16 = 0x01;  // FOREGROUND_BLUE
pub const FC_GREEN:         u16 = 0x02;  // FOREGROUND_GREEN
pub const FC_CYAN:          u16 = 0x03;  // BLUE | GREEN
pub const FC_RED:           u16 = 0x04;  // FOREGROUND_RED
pub const FC_MAGENTA:       u16 = 0x05;  // RED | BLUE
pub const FC_BROWN:         u16 = 0x06;  // RED | GREEN (dark yellow)
pub const FC_LIGHT_GREY:    u16 = 0x07;  // RED | GREEN | BLUE
pub const FC_DARK_GREY:     u16 = 0x08;  // INTENSITY
pub const FC_LIGHT_BLUE:    u16 = 0x09;  // INTENSITY | BLUE
pub const FC_LIGHT_GREEN:   u16 = 0x0A;  // INTENSITY | GREEN
pub const FC_LIGHT_CYAN:    u16 = 0x0B;  // INTENSITY | CYAN
pub const FC_LIGHT_RED:     u16 = 0x0C;  // INTENSITY | RED
pub const FC_LIGHT_MAGENTA: u16 = 0x0D;  // INTENSITY | MAGENTA
pub const FC_YELLOW:        u16 = 0x0E;  // INTENSITY | BROWN
pub const FC_WHITE:         u16 = 0x0F;  // INTENSITY | LIGHT_GREY
pub const FC_MASK:          u16 = 0x0F;

// ── Background color constants (bits 4-7) ─────────────────────────────────────

pub const BC_BLACK:         u16 = 0x00;
pub const BC_BLUE:          u16 = 0x10;
pub const BC_GREEN:         u16 = 0x20;
pub const BC_CYAN:          u16 = 0x30;
pub const BC_RED:           u16 = 0x40;
pub const BC_MAGENTA:       u16 = 0x50;
pub const BC_BROWN:         u16 = 0x60;
pub const BC_LIGHT_GREY:    u16 = 0x70;
pub const BC_DARK_GREY:     u16 = 0x80;
pub const BC_LIGHT_BLUE:    u16 = 0x90;
pub const BC_LIGHT_GREEN:   u16 = 0xA0;
pub const BC_LIGHT_CYAN:    u16 = 0xB0;
pub const BC_LIGHT_RED:     u16 = 0xC0;
pub const BC_LIGHT_MAGENTA: u16 = 0xD0;
pub const BC_YELLOW:        u16 = 0xE0;
pub const BC_WHITE:         u16 = 0xF0;
pub const BC_MASK:          u16 = 0xF0;

// ── Color name ↔ value mapping ────────────────────────────────────────────────

struct ColorMapping {
    name: &'static str,
    fore: u16,
    back: u16,
}

static COLOR_MAP: &[ColorMapping] = &[
    ColorMapping { name: "Black",        fore: FC_BLACK,         back: BC_BLACK         },
    ColorMapping { name: "Blue",         fore: FC_BLUE,          back: BC_BLUE          },
    ColorMapping { name: "Green",        fore: FC_GREEN,         back: BC_GREEN         },
    ColorMapping { name: "Cyan",         fore: FC_CYAN,          back: BC_CYAN          },
    ColorMapping { name: "Red",          fore: FC_RED,           back: BC_RED           },
    ColorMapping { name: "Magenta",      fore: FC_MAGENTA,       back: BC_MAGENTA       },
    ColorMapping { name: "Brown",        fore: FC_BROWN,         back: BC_BROWN         },
    ColorMapping { name: "LightGrey",    fore: FC_LIGHT_GREY,    back: BC_LIGHT_GREY    },
    ColorMapping { name: "DarkGrey",     fore: FC_DARK_GREY,     back: BC_DARK_GREY     },
    ColorMapping { name: "LightBlue",    fore: FC_LIGHT_BLUE,    back: BC_LIGHT_BLUE    },
    ColorMapping { name: "LightGreen",   fore: FC_LIGHT_GREEN,   back: BC_LIGHT_GREEN   },
    ColorMapping { name: "LightCyan",    fore: FC_LIGHT_CYAN,    back: BC_LIGHT_CYAN    },
    ColorMapping { name: "LightRed",     fore: FC_LIGHT_RED,     back: BC_LIGHT_RED     },
    ColorMapping { name: "LightMagenta", fore: FC_LIGHT_MAGENTA, back: BC_LIGHT_MAGENTA },
    ColorMapping { name: "Yellow",       fore: FC_YELLOW,        back: BC_YELLOW        },
    ColorMapping { name: "White",        fore: FC_WHITE,         back: BC_WHITE         },
];

/// Parse a single color name (case-insensitive) into its WORD value.
/// If `is_background` is true, returns the background-shifted value.
///
/// Port of: Config.cpp → CConfig::ParseColorName()
pub fn parse_color_name(name: &str, is_background: bool) -> Result<u16, AppError> {
    for mapping in COLOR_MAP {
        if mapping.name.eq_ignore_ascii_case(name) {
            return Ok(if is_background { mapping.back } else { mapping.fore });
        }
    }
    Err(AppError::InvalidArg(format!("Invalid color name: {}", name)))
}

/// Get the display name for a foreground color WORD value.
/// Returns None if the value doesn't match any known color.
pub fn color_name_from_fg(value: u16) -> Option<&'static str> {
    let fg = value & FC_MASK;
    for mapping in COLOR_MAP {
        if mapping.fore == fg {
            return Some(mapping.name);
        }
    }
    None
}

/// Parse a color specification string in the format: "FgColor [on BgColor]"
/// Case-insensitive matching per spec A.18.
///
/// Examples: "Yellow", "LightCyan on Blue", "Red on Black"
///
/// Port of: Config.cpp → CConfig::ParseColorSpec()
pub fn parse_color_spec(spec: &str) -> Result<u16, AppError> {
    // Search for " on " separator (case-insensitive)
    let lower = spec.to_ascii_lowercase();
    let on_pos = lower.find(" on ");

    if let Some(pos) = on_pos {
        let fore_str = spec[..pos].trim();
        let back_str = spec[pos + 4..].trim();

        let fore = parse_color_name(fore_str, false)?;
        let back = if back_str.is_empty() {
            0
        } else {
            parse_color_name(back_str, true).unwrap_or(0)
        };

        Ok(fore | back)
    } else {
        let fore_str = spec.trim();
        parse_color_name(fore_str, false)
    }
}

/// Total number of foreground colors (16: 0x00..0x0F)
pub const COLOR_COUNT: usize = 16;

/// Array of all 16 foreground color values in index order (for rainbow cycling)
pub const ALL_FOREGROUND_COLORS: [u16; COLOR_COUNT] = [
    FC_BLACK, FC_BLUE, FC_GREEN, FC_CYAN,
    FC_RED, FC_MAGENTA, FC_BROWN, FC_LIGHT_GREY,
    FC_DARK_GREY, FC_LIGHT_BLUE, FC_LIGHT_GREEN, FC_LIGHT_CYAN,
    FC_LIGHT_RED, FC_LIGHT_MAGENTA, FC_YELLOW, FC_WHITE,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_foreground_colors() {
        assert_eq!(parse_color_name("Black", false).unwrap(), FC_BLACK);
        assert_eq!(parse_color_name("Yellow", false).unwrap(), FC_YELLOW);
        assert_eq!(parse_color_name("White", false).unwrap(), FC_WHITE);
        assert_eq!(parse_color_name("LightRed", false).unwrap(), FC_LIGHT_RED);
    }

    #[test]
    fn parse_background_colors() {
        assert_eq!(parse_color_name("Black", true).unwrap(), BC_BLACK);
        assert_eq!(parse_color_name("Blue", true).unwrap(), BC_BLUE);
        assert_eq!(parse_color_name("White", true).unwrap(), BC_WHITE);
    }

    #[test]
    fn parse_case_insensitive() {
        assert_eq!(parse_color_name("yellow", false).unwrap(), FC_YELLOW);
        assert_eq!(parse_color_name("YELLOW", false).unwrap(), FC_YELLOW);
        assert_eq!(parse_color_name("lightred", false).unwrap(), FC_LIGHT_RED);
        assert_eq!(parse_color_name("LIGHTRED", false).unwrap(), FC_LIGHT_RED);
    }

    #[test]
    fn parse_invalid_color() {
        assert!(parse_color_name("Purple", false).is_err());
        assert!(parse_color_name("", false).is_err());
    }

    #[test]
    fn parse_spec_foreground_only() {
        assert_eq!(parse_color_spec("Yellow").unwrap(), FC_YELLOW);
        assert_eq!(parse_color_spec("  LightRed  ").unwrap(), FC_LIGHT_RED);
    }

    #[test]
    fn parse_spec_foreground_and_background() {
        assert_eq!(parse_color_spec("LightCyan on Blue").unwrap(), FC_LIGHT_CYAN | BC_BLUE);
        assert_eq!(parse_color_spec("Red on Black").unwrap(), FC_RED | BC_BLACK);
        assert_eq!(parse_color_spec("White on DarkGrey").unwrap(), FC_WHITE | BC_DARK_GREY);
    }

    #[test]
    fn parse_spec_case_insensitive() {
        assert_eq!(parse_color_spec("lightcyan ON blue").unwrap(), FC_LIGHT_CYAN | BC_BLUE);
    }

    #[test]
    fn color_name_roundtrip() {
        assert_eq!(color_name_from_fg(FC_YELLOW), Some("Yellow"));
        assert_eq!(color_name_from_fg(FC_BLACK), Some("Black"));
        assert_eq!(color_name_from_fg(FC_WHITE), Some("White"));
    }

    #[test]
    fn foreground_constants_match_windows() {
        // Verify bit patterns match Windows FOREGROUND_* constants
        assert_eq!(FC_BLUE, 0x01);   // FOREGROUND_BLUE
        assert_eq!(FC_GREEN, 0x02);  // FOREGROUND_GREEN
        assert_eq!(FC_RED, 0x04);    // FOREGROUND_RED
        assert_eq!(FC_DARK_GREY, 0x08); // FOREGROUND_INTENSITY
    }

    #[test]
    fn background_constants_are_shifted() {
        // Background = foreground << 4
        assert_eq!(BC_BLUE, FC_BLUE << 4);
        assert_eq!(BC_GREEN, FC_GREEN << 4);
        assert_eq!(BC_RED, FC_RED << 4);
        assert_eq!(BC_WHITE, FC_WHITE << 4);
    }
}
