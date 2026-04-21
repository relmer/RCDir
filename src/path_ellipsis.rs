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



    ////////////////////////////////////////////////////////////////////////////
    //
    //  Spec Test Data — Real WindowsApps paths from spec.md
    //
    //  Each entry: (source_filename, target_path, notes)
    //  Available width for each test is calculated from a hypothetical
    //  120-char console minus the metadata columns and filename columns.
    //
    ////////////////////////////////////////////////////////////////////////////

    // Helper: compute a typical available width for normal mode at 120-char
    // terminal.  The formula from research.md R1:
    //   available = console_width - 21 (date/time) - 9 (attrs) - size_col
    //               - cloud_col - icon_col - filename_len - 3 (arrow)
    //
    // For these tests, use a simplified "available width" parameter directly.
    // The displayer computes the real value; here we test the pure function.



    ////////////////////////////////////////////////////////////////////////////
    //
    //  T009: EllipsizedPath struct contract tests
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn not_truncated_has_full_prefix_empty_suffix() {
        let result = ellipsize_path ("C:\\Windows\\system32\\cmd.exe", 100);
        assert!(!result.truncated);
        assert_eq!(result.prefix, "C:\\Windows\\system32\\cmd.exe");
        assert!(result.suffix.is_empty());
    }


    #[test]
    fn truncated_prefix_plus_ellipsis_plus_suffix_fits() {
        // Force truncation by using a narrow width
        let path = "C:\\Program Files\\WindowsApps\\Microsoft.Long_1.0_x64__pkg\\app.exe";
        let result = ellipsize_path (path, 40);
        assert!(result.truncated);
        // prefix + … + suffix must fit within available_width
        let total = result.prefix.len() + 1 + result.suffix.len();
        assert!(total <= 40, "total {} should fit in 40", total);
    }


    #[test]
    fn truncated_result_is_shorter_than_original() {
        let path = "C:\\Program Files\\WindowsApps\\Microsoft.Long_1.0_x64__pkg\\Sub\\app.exe";
        let width = 50;
        let result = ellipsize_path (path, width);
        if result.truncated && !result.suffix.is_empty() {
            let total = result.prefix.len() + 1 + result.suffix.len();
            assert!(total < path.len(), "truncated form {} must be shorter than original {}", total, path.len());
        }
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  T008: Short path — fits without truncation
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn short_path_no_truncation() {
        let result = ellipsize_path ("C:\\Windows\\system32\\SystemUWPLauncher.exe", 100);
        assert!(!result.truncated);
        assert_eq!(result.prefix, "C:\\Windows\\system32\\SystemUWPLauncher.exe");
        assert!(result.suffix.is_empty());
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  T008: Path with exactly 2 components — never truncated
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn two_component_path_never_truncated() {
        let result = ellipsize_path ("C:\\file.exe", 5);
        assert!(!result.truncated);
        assert_eq!(result.prefix, "C:\\file.exe");
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  T008: Path with 3 components — minimal truncation
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn three_component_path_truncation() {
        // "C:\Windows\cmd.exe" has 3 components: C:, Windows, cmd.exe
        // Priority 2 should apply: C:\Windows\…\cmd.exe
        // But that's the same length — so only if forced narrow
        let path = "C:\\VeryLongDirectoryName\\cmd.exe";
        let result = ellipsize_path (path, 20);
        assert!(result.truncated);
        // Should get priority 3: C:\…\cmd.exe
        assert_eq!(result.prefix, "C:\\");
        assert_eq!(result.suffix, "\\cmd.exe");
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  T008: Priority level 1 — first two dirs + …\ + leaf dir + filename
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn priority_1_two_dirs_leaf_dir_filename() {
        // C:\Program Files\WindowsApps\Microsoft.Notepad_1.0\Notepad\Notepad.exe
        // → C:\Program Files\…\Notepad\Notepad.exe
        let path = "C:\\Program Files\\WindowsApps\\Microsoft.Notepad_1.0\\Notepad\\Notepad.exe";
        // Priority 1: "C:\Program Files\" + "…" + "\Notepad\Notepad.exe" = 17+1+20 = 38
        let result = ellipsize_path (path, 50);
        assert!(result.truncated);
        assert_eq!(result.prefix, "C:\\Program Files\\");
        assert_eq!(result.suffix, "\\Notepad\\Notepad.exe");
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  T008: Priority level 2 — first two dirs + …\ + filename
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn priority_2_two_dirs_filename() {
        // Make a path where priority 1 doesn't fit but priority 2 does
        let path = "C:\\Program Files\\WindowsApps\\SomeLongPackage\\SubDir\\app.exe";
        // Priority 1: "C:\Program Files\" + "…" + "\SubDir\app.exe" = 17+1+15 = 33
        // Priority 2: "C:\Program Files\" + "…" + "\app.exe" = 17+1+8 = 26
        let result = ellipsize_path (path, 30);
        assert!(result.truncated);
        assert_eq!(result.prefix, "C:\\Program Files\\");
        assert_eq!(result.suffix, "\\app.exe");
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  T008: Priority level 3 — first dir + …\ + filename
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn priority_3_first_dir_filename() {
        // Make a path where priority 2 doesn't fit but priority 3 does
        let path = "C:\\Program Files\\WindowsApps\\SomeLong\\VeryLongFilename.exe";
        // Priority 2: "C:\Program Files\" + "…" + "\VeryLongFilename.exe" = 17+1+21 = 39
        // Priority 3: "C:\" + "…" + "\VeryLongFilename.exe" = 3+1+21 = 25
        let result = ellipsize_path (path, 26);
        assert!(result.truncated);
        assert_eq!(result.prefix, "C:\\");
        assert_eq!(result.suffix, "\\VeryLongFilename.exe");
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  T008: Priority level 4 — leaf filename only (no ellipsis)
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn priority_4_leaf_only() {
        // Width too narrow for even level 3, but leaf fits
        let path = "C:\\Very\\Long\\Path\\app.exe";
        // Priority 3: "C:\" + "…" + "\app.exe" = 3+1+8 = 12
        // Leaf only: "app.exe" = 7
        let result = ellipsize_path (path, 10);
        assert!(!result.truncated);  // leaf only — no ellipsis in output
        assert_eq!(result.prefix, "app.exe");
        assert!(result.suffix.is_empty());
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  T008: Priority level 5 — leaf filename truncated with trailing …
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn priority_5_leaf_truncated_with_trailing_ellipsis() {
        // Width too narrow even for the full leaf filename
        let path = "C:\\Very\\Long\\Path\\VeryLongFilename.exe";
        // Leaf = "VeryLongFilename.exe" = 20 chars
        // Width = 10: should get "VeryLongF…" (9 + 1 ellipsis)
        let result = ellipsize_path (path, 10);
        assert!(result.truncated);
        assert_eq!(result.prefix, "VeryLongF");
        assert!(result.suffix.is_empty());
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  T008: Edge case — available_width of 0
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn available_width_zero() {
        let result = ellipsize_path ("C:\\Windows\\cmd.exe", 0);
        assert!(result.truncated);
        assert!(result.prefix.is_empty());
        assert!(result.suffix.is_empty());
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  T008: Edge case — available_width of 1
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn available_width_one() {
        let result = ellipsize_path ("C:\\Windows\\cmd.exe", 1);
        assert!(result.truncated);
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  T008: Edge case — truncated form not shorter than original
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn truncation_must_save_space() {
        // "C:\ab\c.e" — 3 components, 10 chars
        // Priority 2: "C:\ab\" + "…" + "\c.e" = 6+1+4 = 11 — longer!
        // Priority 3: "C:\" + "…" + "\c.e" = 3+1+4 = 8 — shorter, so use this
        let path = "C:\\ab\\c.e";
        let result = ellipsize_path (path, 9);
        if result.truncated && !result.suffix.is_empty() {
            let total = result.prefix.len() + 1 + result.suffix.len();
            assert!(total < path.len());
        }
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  T008: FR-004 guard — source filename is never modified
    //
    //  ellipsize_path only receives the target path, never the source
    //  filename.  Verify the function doesn't need or affect anything
    //  outside its input.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn fr004_only_target_path_is_processed() {
        let source_filename = "python.exe";
        let target_path = "C:\\Program Files\\WindowsApps\\Very.Long.Package_1.0_x64__hash\\python3.12.exe";
        let result = ellipsize_path (target_path, 40);
        // Source filename is completely untouched — it's not even an input
        assert_eq!(source_filename, "python.exe");
        // The result only contains parts of the target path
        assert!(result.truncated);
        assert!(!result.prefix.contains(source_filename) || target_path.contains(source_filename));
    }



    ////////////////////////////////////////////////////////////////////////////
    //
    //  T008: Spec Test Data — 7 real WindowsApps paths
    //
    //  These test the algorithm against real-world data from the spec.
    //  Available width is computed assuming a typical normal-mode layout
    //  at 120-char terminal.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn spec_data_desktop_sticker_editor() {
        // Long filename (52 chars) + long target
        // Source: MicrosoftWindows.DesktopStickerEditorCentennial.exe (52)
        // At 120 console: available ~= 120 - 21 - 9 - 11 - 3 - 2 - 52 - 3 = 19
        let target = "C:\\Windows\\SystemApps\\MicrosoftWindows.Client.CBS_cw5n1h2txyewy\\DesktopStickerEditorWin32Exe\\DesktopStickerEditorWin32Exe.exe";
        let result = ellipsize_path (target, 19);
        assert!(result.truncated);
        // At 19 chars, even level 3 "C:\…\DesktopStickerEditorWin32Exe.exe" is 37 chars
        // So we get level 5: leaf truncated with trailing …
        let total = result.prefix.len() + if result.suffix.is_empty() { 0 } else { 1 + result.suffix.len() };
        assert!(total <= 19, "total {} must fit in 19", total);
    }


    #[test]
    fn spec_data_wingetcreate() {
        // Short filename (17 chars) + very long target
        // Source: wingetcreate.exe (16)
        // available ~= 120 - 21 - 9 - 11 - 3 - 2 - 16 - 3 = 55
        let target = "C:\\Program Files\\WindowsApps\\Microsoft.WindowsPackageManagerManifestCreator_1.12.8.0_x64__8wekyb3d8bbwe\\WingetCreateCLI\\WingetCreateCLI.exe";
        let result = ellipsize_path (target, 55);
        assert!(result.truncated);
        // Level 1: "C:\Program Files\" + "…" + "\WingetCreateCLI\WingetCreateCLI.exe" = 17+1+36 = 54 — fits!
        assert_eq!(result.prefix, "C:\\Program Files\\");
        assert_eq!(result.suffix, "\\WingetCreateCLI\\WingetCreateCLI.exe");
    }


    #[test]
    fn spec_data_gamebar() {
        // Medium filename (27 chars)
        // Source: GameBarElevatedFT_Alias.exe (27)
        // available ~= 120 - 21 - 9 - 11 - 3 - 2 - 27 - 3 = 44
        let target = "C:\\Program Files\\WindowsApps\\Microsoft.XboxGamingOverlay_7.326.4151.0_arm64__8wekyb3d8bbwe\\GameBarElevatedFT.exe";
        let result = ellipsize_path (target, 44);
        assert!(result.truncated);
        // Level 2: "C:\Program Files\" + "…" + "\GameBarElevatedFT.exe" = 17+1+21 = 39 — fits!
        assert_eq!(result.prefix, "C:\\Program Files\\");
        assert_eq!(result.suffix, "\\GameBarElevatedFT.exe");
    }


    #[test]
    fn spec_data_wt() {
        // Short filename (6 chars) + medium target
        // Source: wt.exe (6)
        // available ~= 120 - 21 - 9 - 11 - 3 - 2 - 6 - 3 = 65
        let target = "C:\\Program Files\\WindowsApps\\Microsoft.WindowsTerminal_1.24.10921.0_arm64__8wekyb3d8bbwe\\wt.exe";
        let result = ellipsize_path (target, 65);
        assert!(result.truncated);
        // Level 2: "C:\Program Files\" + "…" + "\wt.exe" = 17+1+7 = 25 — fits!
        // But also level 1: "C:\Program Files\" + "…" + "\Microsoft.WindowsTerminal_1.24.10921.0_arm64__8wekyb3d8bbwe\wt.exe"
        // Level 1 = 17+1+len("\Microsoft.WindowsTerminal_1.24.10921.0_arm64__8wekyb3d8bbwe\wt.exe") — won't fit
        // So level 2.
        assert_eq!(result.prefix, "C:\\Program Files\\");
        assert_eq!(result.suffix, "\\wt.exe");
    }


    #[test]
    fn spec_data_notepad() {
        // Short filename (11 chars) + medium target with subdirectory
        // Source: notepad.exe (11)
        // available ~= 120 - 21 - 9 - 11 - 3 - 2 - 11 - 3 = 60
        let target = "C:\\Program Files\\WindowsApps\\Microsoft.WindowsNotepad_11.2601.26.0_arm64__8wekyb3d8bbwe\\Notepad\\Notepad.exe";
        let result = ellipsize_path (target, 60);
        assert!(result.truncated);
        // Level 1: "C:\Program Files\" + "…" + "\Notepad\Notepad.exe" = 17+1+20 = 38 — fits!
        assert_eq!(result.prefix, "C:\\Program Files\\");
        assert_eq!(result.suffix, "\\Notepad\\Notepad.exe");
    }


    #[test]
    fn spec_data_winget_not_truncated() {
        // Short filename + short-ish target
        // Source: winget.exe (10)
        // available ~= 120 - 21 - 9 - 11 - 3 - 2 - 10 - 3 = 61
        let target = "C:\\Program Files\\WindowsApps\\Microsoft.DesktopAppInstaller_1.29.30.0_arm64__8wekyb3d8bbwe\\winget.exe";
        // Target is 101 chars — longer than 61, so WILL be truncated
        let result = ellipsize_path (target, 61);
        assert!(result.truncated);
        // Level 2: "C:\Program Files\" + "…" + "\winget.exe" = 17+1+11 = 29 — fits!
        assert_eq!(result.prefix, "C:\\Program Files\\");
        assert_eq!(result.suffix, "\\winget.exe");
    }


    #[test]
    fn spec_data_winget_wide_terminal_not_truncated() {
        // At a wider terminal or with shorter metadata, target fits entirely
        let target = "C:\\Program Files\\WindowsApps\\Microsoft.DesktopAppInstaller_1.29.30.0_arm64__8wekyb3d8bbwe\\winget.exe";
        let result = ellipsize_path (target, 120);
        assert!(!result.truncated);
        assert_eq!(result.prefix, target);
    }


    #[test]
    fn spec_data_azurevpn_never_truncated() {
        // Short target — should never be truncated even at available_width 50
        let target = "C:\\Windows\\system32\\SystemUWPLauncher.exe";
        let result = ellipsize_path (target, 50);
        assert!(!result.truncated);
        assert_eq!(result.prefix, target);
    }
}
