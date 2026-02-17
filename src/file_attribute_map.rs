// RCDir - File Attribute Precedence Map
// Attribute precedence order (PSHERC0TA) for icon/color resolution.
//
// This is DIFFERENT from FILE_ATTRIBUTE_MAP in file_info.rs (RHSATECP0),
// which controls the display column order.  Both arrays contain the same
// 9 attributes but in different sequences.

use crate::file_info::{
    FILE_ATTRIBUTE_REPARSE_POINT,
    FILE_ATTRIBUTE_SYSTEM,
    FILE_ATTRIBUTE_HIDDEN,
    FILE_ATTRIBUTE_ENCRYPTED,
    FILE_ATTRIBUTE_READONLY,
    FILE_ATTRIBUTE_COMPRESSED,
    FILE_ATTRIBUTE_SPARSE_FILE,
    FILE_ATTRIBUTE_TEMPORARY,
    FILE_ATTRIBUTE_ARCHIVE,
};





////////////////////////////////////////////////////////////////////////////////
//
//  ATTRIBUTE_PRECEDENCE
//
//  Attribute precedence order for icon/color resolution (PSHERC0TA).
//  Used by Config::get_display_style_for_file().
//
//  Port of: g_rgAttributePrecedenceOrder[] in TCDirCore/IconMapping.cpp
//
////////////////////////////////////////////////////////////////////////////////

pub const ATTRIBUTE_PRECEDENCE: &[(u32, char)] = &[
    (FILE_ATTRIBUTE_REPARSE_POINT, 'P'),   // Priority 1 (highest) — identity-altering
    (FILE_ATTRIBUTE_SYSTEM,        'S'),   // Priority 2 — OS-critical
    (FILE_ATTRIBUTE_HIDDEN,        'H'),   // Priority 3 — intentionally invisible
    (FILE_ATTRIBUTE_ENCRYPTED,     'E'),   // Priority 4 — access-restricting
    (FILE_ATTRIBUTE_READONLY,      'R'),   // Priority 5 — access-restricting
    (FILE_ATTRIBUTE_COMPRESSED,    'C'),   // Priority 6 — informational
    (FILE_ATTRIBUTE_SPARSE_FILE,   '0'),   // Priority 7 — rare
    (FILE_ATTRIBUTE_TEMPORARY,     'T'),   // Priority 8 — ephemeral
    (FILE_ATTRIBUTE_ARCHIVE,       'A'),   // Priority 9 (lowest) — near-universal noise
];





////////////////////////////////////////////////////////////////////////////////
//
//  Unit Tests
//
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_info::FILE_ATTRIBUTE_MAP;
    use std::collections::HashSet;





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_attribute_precedence_count
    //
    //  Exactly 9 entries in the precedence array.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_attribute_precedence_count() {
        assert_eq! (ATTRIBUTE_PRECEDENCE.len(), 9);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_same_flags_as_display_map
    //
    //  Precedence array contains the same flags as FILE_ATTRIBUTE_MAP.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_same_flags_as_display_map() {
        let prec_flags: HashSet<u32> = ATTRIBUTE_PRECEDENCE.iter().map (|&(f, _)| f).collect();
        let disp_flags: HashSet<u32> = FILE_ATTRIBUTE_MAP.iter().map (|&(f, _)| f).collect();
        assert_eq! (prec_flags, disp_flags);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_no_duplicate_flags
    //
    //  No duplicate attribute flags in the precedence array.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_no_duplicate_flags() {
        let mut seen = HashSet::new();
        for &(flag, _) in ATTRIBUTE_PRECEDENCE {
            assert! (seen.insert (flag), "Duplicate flag: 0x{:08X}", flag);
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  test_no_duplicate_chars
    //
    //  No duplicate display characters in the precedence array.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn test_no_duplicate_chars() {
        let mut seen = HashSet::new();
        for &(_, ch) in ATTRIBUTE_PRECEDENCE {
            assert! (seen.insert (ch), "Duplicate char: '{}'", ch);
        }
    }
}
