// path_ellipsis.rs — Middle-truncate long link target paths with ellipsis
//
// Pure function for truncating long paths using `…` (U+2026) to prevent
// line wrapping in normal and tree display modes.  Preserves first two
// directory components and leaf filename where possible, falling back
// gracefully to shorter forms.

/// Ellipsis character used for path truncation (U+2026 HORIZONTAL ELLIPSIS).
pub const ELLIPSIS: char = '\u{2026}';





////////////////////////////////////////////////////////////////////////////////
//
//  EllipsizedPath
//
//  Return type from `ellipsize_path()`.  Enables the displayer to render
//  prefix and suffix in the source file's color with the `…` character
//  in Default color.
//
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EllipsizedPath {
    /// Path text before the ellipsis.  Full path if not truncated.
    pub prefix:    String,

    /// Path text after the ellipsis.  Empty if not truncated.
    pub suffix:    String,

    /// `true` if the path was middle-truncated, `false` if shown in full.
    pub truncated: bool,
}





////////////////////////////////////////////////////////////////////////////////
//
//  ellipsize_path
//
//  Middle-truncate a target path to fit within `available_width` characters.
//
//  Algorithm (priority order — uses highest-priority form that fits):
//    1. Full path (no truncation needed)
//    2. first two dirs + `\…\` + leaf dir + filename
//    3. first two dirs + `\…\` + filename
//    4. first dir + `\…\` + filename
//    5. Leaf filename only (no prefix, no ellipsis)
//    6. Leaf filename truncated with trailing `…`
//
//  Paths with fewer than 3 components are never truncated.
//
////////////////////////////////////////////////////////////////////////////////

pub fn ellipsize_path (target_path: &str, available_width: usize) -> EllipsizedPath {
    // If the path fits, return it unchanged
    if target_path.len() <= available_width {
        return EllipsizedPath {
            prefix:    target_path.to_string(),
            suffix:    String::new(),
            truncated: false,
        };
    }

    // Split into components on backslash
    let components: Vec<&str> = target_path.split ('\\').collect();

    // Paths with fewer than 3 components — nothing to elide
    if components.len() < 3 {
        return EllipsizedPath {
            prefix:    target_path.to_string(),
            suffix:    String::new(),
            truncated: false,
        };
    }

    let leaf = components[components.len() - 1];

    // Priority 1: first two dirs + \…\ + leaf dir + filename
    // e.g. "C:\Program Files\…\Notepad\Notepad.exe"
    if components.len() >= 4 {
        let leaf_dir = components[components.len() - 2];
        let prefix = format! ("{}\\{}", components[0], components[1]);
        let suffix = format! ("{}\\{}", leaf_dir, leaf);
        // Total: prefix + \…\ + suffix = prefix.len() + 3 + suffix.len()
        let total = prefix.len() + 3 + suffix.len();
        if total <= available_width && total < target_path.len() {
            return EllipsizedPath {
                prefix:    format! ("{}\\", prefix),
                suffix:    format! ("\\{}", suffix),
                truncated: true,
            };
        }
    }

    // Priority 2: first two dirs + \…\ + filename
    // e.g. "C:\Program Files\…\Notepad.exe"
    if components.len() >= 3 {
        let prefix = format! ("{}\\{}", components[0], components[1]);
        let suffix = leaf;
        let total = prefix.len() + 3 + suffix.len();
        if total <= available_width && total < target_path.len() {
            return EllipsizedPath {
                prefix:    format! ("{}\\", prefix),
                suffix:    format! ("\\{}", suffix),
                truncated: true,
            };
        }
    }

    // Priority 3: first dir + \…\ + filename
    // e.g. "C:\…\Notepad.exe"
    {
        let prefix = components[0];
        let suffix = leaf;
        let total = prefix.len() + 3 + suffix.len();
        if total <= available_width && total < target_path.len() {
            return EllipsizedPath {
                prefix:    format! ("{}\\", prefix),
                suffix:    format! ("\\{}", suffix),
                truncated: true,
            };
        }
    }

    // Priority 4: Leaf filename only (no ellipsis)
    if leaf.len() <= available_width {
        return EllipsizedPath {
            prefix:    leaf.to_string(),
            suffix:    String::new(),
            truncated: false,
        };
    }

    // Priority 5: Leaf filename truncated with trailing …
    if available_width >= 2 {
        let truncated_leaf = &leaf[..available_width - 1];
        return EllipsizedPath {
            prefix:    truncated_leaf.to_string(),
            suffix:    String::new(),
            truncated: true,
        };
    }

    // Edge case: available_width is 0 or 1 — return what we can
    if available_width == 1 {
        return EllipsizedPath {
            prefix:    String::new(),
            suffix:    String::new(),
            truncated: true,
        };
    }

    // available_width == 0
    EllipsizedPath {
        prefix:    String::new(),
        suffix:    String::new(),
        truncated: true,
    }
}





#[cfg(test)]
mod tests {
    use super::*;
}
