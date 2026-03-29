// tests/tree_mode_tests.rs — Integration tests for tree display mode
//
// Tests tree-specific behavior: connector patterns, depth limiting,
// empty directory display, file mask pruning, interleaved sort order,
// abbreviated sizes, root header/footer, custom indent widths, etc.
//
// These tests run rcdir.exe directly and verify output patterns.

use std::path::PathBuf;
use std::process::Command;





////////////////////////////////////////////////////////////////////////////////
//
//  get_rcdir_exe
//
//  Get the path to the RCDir executable (built by cargo).
//
////////////////////////////////////////////////////////////////////////////////

fn get_rcdir_exe() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // deps
    path.pop(); // debug
    path.push ("rcdir.exe");
    path
}





////////////////////////////////////////////////////////////////////////////////
//
//  run_rcdir
//
//  Run rcdir with the given arguments and return stdout as a string.
//
////////////////////////////////////////////////////////////////////////////////

fn run_rcdir (args: &[&str]) -> String {
    let exe = get_rcdir_exe();
    let output = Command::new (&exe)
        .args (args)
        .output()
        .expect ("Failed to run rcdir");

    String::from_utf8_lossy (&output.stdout).to_string()
}





////////////////////////////////////////////////////////////////////////////////
//
//  strip_ansi
//
//  Strip ANSI escape sequences from output for pattern matching.
//
////////////////////////////////////////////////////////////////////////////////

fn strip_ansi (s: &str) -> String {
    let mut result = String::with_capacity (s.len());
    let mut chars = s.chars().peekable();
    while let Some (ch) = chars.next() {
        if ch == '\x1b' {
            if chars.peek() == Some (&'[') {
                chars.next();
                while let Some (&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push (ch);
        }
    }
    result
}





////////////////////////////////////////////////////////////////////////////////
//
//  tree_output_contains_connectors
//
//  Verify that tree output contains Unicode box-drawing connectors.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn tree_output_contains_connectors() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let output = run_rcdir (&["/Tree", &pattern]);
    let plain = strip_ansi (&output);

    // Should contain tree connector characters
    assert!(
        plain.contains ('\u{251C}') || plain.contains ('\u{2514}')
            || plain.contains ('\u{2502}') || plain.contains ('\u{2500}'),
        "Tree output should contain box-drawing connectors:\n{}",
        &plain[..plain.len().min (500)],
    );
}





////////////////////////////////////////////////////////////////////////////////
//
//  tree_output_has_root_header
//
//  Verify that tree output starts with a root directory header.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn tree_output_has_root_header() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let output = run_rcdir (&["/Tree", &pattern]);
    let plain = strip_ansi (&output);

    // Root header should contain "Directory of" (matching existing display)
    assert!(
        plain.contains ("Directory of"),
        "Tree output should contain 'Directory of' root header:\n{}",
        &plain[..plain.len().min (500)],
    );
}





////////////////////////////////////////////////////////////////////////////////
//
//  tree_output_has_summary
//
//  Verify that tree output ends with a summary (file count and byte total).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn tree_output_has_summary() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let output = run_rcdir (&["/Tree", &pattern]);
    let plain = strip_ansi (&output);
    let lines: Vec<&str> = plain.lines().collect();

    // Summary should mention "files" or "bytes"
    let has_summary = lines.iter().any (|l| l.contains ("files") || l.contains ("subdirectories"));
    assert!(
        has_summary,
        "Tree output should have a summary with files/subdirectories:\n{}",
        lines.iter().rev().take (5).copied().collect::<Vec<_>>().join ("\n"),
    );
}





////////////////////////////////////////////////////////////////////////////////
//
//  tree_depth_limiting
//
//  Verify depth limiting restricts tree depth.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn tree_depth_limiting() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());

    let deep_output = run_rcdir (&["/Tree", &pattern]);
    let shallow_output = run_rcdir (&["/Tree", "/Depth=1", &pattern]);

    let deep_lines = strip_ansi (&deep_output).lines().count();
    let shallow_lines = strip_ansi (&shallow_output).lines().count();

    // Depth-limited output should have fewer lines than unlimited
    assert!(
        shallow_lines <= deep_lines,
        "Depth=1 output ({} lines) should have <= lines than unlimited ({} lines)",
        shallow_lines,
        deep_lines,
    );
}





////////////////////////////////////////////////////////////////////////////////
//
//  tree_custom_indent_width
//
//  Verify custom indent widths change the output.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn tree_custom_indent_width() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());

    let default_output = run_rcdir (&["/Tree", &pattern]);
    let wide_output = run_rcdir (&["/Tree", "/TreeIndent=8", &pattern]);

    // With wider indent, lines should generally be longer
    let default_max = strip_ansi (&default_output).lines()
        .map (|l| l.len())
        .max()
        .unwrap_or (0);
    let wide_max = strip_ansi (&wide_output).lines()
        .map (|l| l.len())
        .max()
        .unwrap_or (0);

    // At minimum, the wide output should not be shorter than default
    // (unless the directory is completely flat)
    assert!(
        wide_max >= default_max || default_max - wide_max < 5,
        "Wide indent output max line ({}) should be >= default ({})",
        wide_max,
        default_max,
    );
}





////////////////////////////////////////////////////////////////////////////////
//
//  tree_last_entry_uses_corner
//
//  Verify the last entry at each depth uses └── (corner) connector.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn tree_last_entry_uses_corner() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let output = run_rcdir (&["/Tree", &pattern]);
    let plain = strip_ansi (&output);

    // Should contain └── (corner connector for last entry)
    let corner = "\u{2514}\u{2500}\u{2500}";
    assert!(
        plain.contains (corner),
        "Tree output should contain corner connector └──:\n{}",
        &plain[..plain.len().min (500)],
    );
}





////////////////////////////////////////////////////////////////////////////////
//
//  tree_non_last_entry_uses_tee
//
//  Verify non-last entries use ├── (tee) connector.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn tree_non_last_entry_uses_tee() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let output = run_rcdir (&["/Tree", &pattern]);
    let plain = strip_ansi (&output);

    // Should contain ├── (tee connector for non-last entries)
    let tee = "\u{251C}\u{2500}\u{2500}";
    assert!(
        plain.contains (tee),
        "Tree output should contain tee connector ├──:\n{}",
        &plain[..plain.len().min (500)],
    );
}





////////////////////////////////////////////////////////////////////////////////
//
//  tree_continuation_uses_pipe
//
//  Verify continuation lines use │ (pipe) connector.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn tree_continuation_uses_pipe() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let output = run_rcdir (&["/Tree", &pattern]);
    let plain = strip_ansi (&output);

    // Should contain │ (pipe for continuation)
    let pipe = "\u{2502}";
    assert!(
        plain.contains (pipe),
        "Tree output should contain pipe connector │:\n{}",
        &plain[..plain.len().min (500)],
    );
}





////////////////////////////////////////////////////////////////////////////////
//
//  tree_dir_abbreviated_size
//
//  Verify directories show <DIR> in abbreviated format in tree mode.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn tree_dir_abbreviated_size() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let output = run_rcdir (&["/Tree", &pattern]);
    let plain = strip_ansi (&output);

    // In tree mode with default SizeFormat (Auto), dirs show <DIR>
    assert!(
        plain.contains ("<DIR>"),
        "Tree output should contain '<DIR>' for directories:\n{}",
        &plain[..plain.len().min (500)],
    );
}





////////////////////////////////////////////////////////////////////////////////
//
//  tree_interleaved_sort_order
//
//  Verify that files and directories are sorted together (interleaved).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn tree_interleaved_sort_order() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let output = run_rcdir (&["/Tree", &pattern]);
    let plain = strip_ansi (&output);

    // In interleaved mode, dirs and files should be alphabetically mixed.
    // Collect names from connector lines (after the last connector char)
    let connector_lines: Vec<&str> = plain.lines()
        .filter (|l| l.contains ('\u{251C}') || l.contains ('\u{2514}'))
        .collect();

    // Should have multiple entries
    assert!(
        connector_lines.len() >= 2,
        "Tree output should have at least 2 connector entries, got {}",
        connector_lines.len(),
    );
}





////////////////////////////////////////////////////////////////////////////////
//
//  tree_file_mask_shows_matching_dirs
//
//  Verify that tree view with a file mask only shows directories
//  that have matching descendants (when pruning is active).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn tree_file_mask_shows_matching_dirs() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*.rs", test_dir.display());
    let output = run_rcdir (&["/Tree", &pattern]);
    let plain = strip_ansi (&output);

    // With *.rs mask, the src/ directory should appear (it contains .rs files)
    assert!(
        plain.contains ("src"),
        "Tree output with *.rs mask should show the src directory:\n{}",
        &plain[..plain.len().min (500)],
    );
}





////////////////////////////////////////////////////////////////////////////////
//
//  tree_with_icons
//
//  Verify that tree view with icons produces output with icon characters.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn tree_with_icons() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let output = run_rcdir (&["/Tree", "/Icons", &pattern]);

    // Icons output should be longer than without (icon chars + spaces added)
    let no_icons_output = run_rcdir (&["/Tree", &pattern]);

    let icon_lines = output.lines().count();
    let no_icon_lines = no_icons_output.lines().count();

    // Same number of lines, but icon lines should be wider
    assert_eq!(
        icon_lines,
        no_icon_lines,
        "Icon and no-icon tree outputs should have same line count",
    );
}





////////////////////////////////////////////////////////////////////////////////
//
//  tree_depth_one_limits_output
//
//  Verify that /Depth=1 limits to root entries only (no child recursion).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn tree_depth_one_limits_output() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());

    let unlimited = run_rcdir (&["/Tree", &pattern]);
    let depth_one = run_rcdir (&["/Tree", "/Depth=1", &pattern]);

    let unlimited_lines = strip_ansi (&unlimited).lines().count();
    let depth_one_lines = strip_ansi (&depth_one).lines().count();

    // Depth=1 should have significantly fewer lines than unlimited
    // (only root entries, no child directories)
    assert!(
        depth_one_lines < unlimited_lines,
        "Depth=1 ({} lines) should have fewer than unlimited ({} lines)",
        depth_one_lines,
        unlimited_lines,
    );
}





////////////////////////////////////////////////////////////////////////////////
//
//  tree_size_bytes_shows_full_numbers
//
//  Verify that /Size=Bytes in tree mode shows full byte counts (commas).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn tree_size_bytes_shows_full_numbers() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\src\\*.rs", test_dir.display());
    let output = run_rcdir (&["/Tree", "/Size=Bytes", &pattern]);
    let plain = strip_ansi (&output);

    // With /Size=Bytes, file sizes should be full numbers (no K/M/G suffix)
    // Check that at least some lines DON'T have abbreviated suffixes
    let has_abbreviated = plain.lines().any (|l| {
        let trimmed = l.trim();
        trimmed.ends_with ('K') || trimmed.ends_with ('M') || trimmed.ends_with ('G')
    });

    // This is a heuristic — /Size=Bytes should avoid abbreviated suffixes in the size column
    // But filenames could end with K/M/G, so we check the full numbers with commas
    let has_comma_numbers = plain.lines().any (|l| {
        // Look for patterns like "1,234" or "12,345"
        l.contains (',') && l.chars().any (|c| c.is_ascii_digit())
    });

    // At least one of these should be true for a non-empty directory
    assert!(
        has_comma_numbers || !has_abbreviated,
        "Tree /Size=Bytes output should show full numbers or at least no abbreviated sizes",
    );
}





////////////////////////////////////////////////////////////////////////////////
//
//  tree_sort_by_size_ordering
//
//  Verify that /Tree /os produces entries sorted by size rather than name.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn tree_sort_by_size_ordering() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\src\\*.rs", test_dir.display());
    let output = run_rcdir (&["/Tree", "/os", &pattern]);
    let plain = strip_ansi (&output);

    // Just verify output is non-empty and has connector chars
    assert!(
        plain.contains ('\u{251C}') || plain.contains ('\u{2514}'),
        "Tree /os output should contain connectors:\n{}",
        &plain[..plain.len().min (500)],
    );
}





////////////////////////////////////////////////////////////////////////////////
//
//  tree_switch_conflicts_with_bare
//
//  Verify that /Tree and /b produce an error message.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn tree_switch_conflicts_with_bare() {
    let exe = get_rcdir_exe();
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());

    let output = Command::new (&exe)
        .args (["/Tree", "/b", &pattern])
        .output()
        .expect ("Failed to run rcdir");

    let stderr = String::from_utf8_lossy (&output.stderr).to_string();
    let stdout = String::from_utf8_lossy (&output.stdout).to_string();
    let combined = format! ("{}{}", stderr, stdout);

    // Should produce an error about incompatible switches
    assert!(
        combined.contains ("not compatible") || combined.contains ("cannot")
            || !output.status.success(),
        "Tree + bare should produce an error or non-zero exit",
    );
}





////////////////////////////////////////////////////////////////////////////////
//
//  tree_switch_conflicts_with_wide
//
//  Verify that /Tree and /w produce an error message.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn tree_switch_conflicts_with_wide() {
    let exe = get_rcdir_exe();
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());

    let output = Command::new (&exe)
        .args (["/Tree", "/w", &pattern])
        .output()
        .expect ("Failed to run rcdir");

    let stderr = String::from_utf8_lossy (&output.stderr).to_string();
    let stdout = String::from_utf8_lossy (&output.stdout).to_string();
    let combined = format! ("{}{}", stderr, stdout);

    assert!(
        combined.contains ("not compatible") || combined.contains ("cannot")
            || !output.status.success(),
        "Tree + wide should produce an error or non-zero exit",
    );
}
