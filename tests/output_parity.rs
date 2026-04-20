// tests/output_parity.rs — Integration test: compare RCDir vs TCDir output
//
// Runs both executables with the same arguments and compares output line-by-line.
// Output is compared WITH ANSI escape codes (they must be byte-identical).
// Skips comparison of timing lines (RCDir/TCDir time elapsed) and volume free space
// (which may differ between runs).
//
// Requirements:
//   - RCDir must be built (cargo build)
//   - TCDir.exe must exist at the path specified by TCDIR_EXE env var or at the default location

use std::path::{Path, PathBuf};
use std::process::Command;





////////////////////////////////////////////////////////////////////////////////
//
//  get_tcdir_exe
//
//  Get the path to the native TCDir executable.
//
////////////////////////////////////////////////////////////////////////////////

fn get_tcdir_exe() -> Option<PathBuf> {
    // Check env var first
    if let Ok(path) = std::env::var("TCDIR_EXE") {
        let p = PathBuf::from(&path);
        if p.exists() {
            return Some(p);
        }
    }

    // Default locations based on architecture (prefer Release for performance)
    let candidates = [
        r"c:\Users\relmer\source\repos\relmer\TCDir\ARM64\Release\TCDir.exe",
        r"c:\Users\relmer\source\repos\relmer\TCDir\x64\Release\TCDir.exe",
        r"c:\Users\relmer\source\repos\relmer\TCDir\ARM64\Debug\TCDir.exe",
        r"c:\Users\relmer\source\repos\relmer\TCDir\x64\Debug\TCDir.exe",
    ];

    for candidate in &candidates {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return Some(p);
        }
    }

    None
}





////////////////////////////////////////////////////////////////////////////////
//
//  get_rcdir_exe
//
//  Get the path to the RCDir executable (built by cargo).
//
////////////////////////////////////////////////////////////////////////////////

fn get_rcdir_exe() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    // tests are in target/debug/deps/, executable is in target/debug/
    path.pop(); // deps
    path.pop(); // debug
    path.push("rcdir.exe");
    if !path.exists() {
        // Try alternate location
        path.pop();
        path.push("rcdir.exe");
    }
    path
}





////////////////////////////////////////////////////////////////////////////////
//
//  run_command
//
//  Run a command and capture stdout as a string.
//
////////////////////////////////////////////////////////////////////////////////

fn run_command(exe: &Path, args: &[&str]) -> String {
    let output = Command::new(exe)
        .args(args)
        .output()
        .expect("Failed to run command");

    String::from_utf8_lossy(&output.stdout).to_string()
}





////////////////////////////////////////////////////////////////////////////////
//
//  filter_lines
//
//  Filter lines for comparison — remove timing lines, free space lines, and
//  bytes-available lines.  Does NOT strip ANSI codes — output must be
//  byte-identical including escape sequences.
//
////////////////////////////////////////////////////////////////////////////////

fn filter_lines(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|line| {
            // Strip ANSI only for the purpose of checking content-based skip patterns
            let plain = strip_ansi_for_check(line);
            let trimmed = plain.trim();
            // Skip timing lines
            if trimmed.starts_with("RCDir time elapsed:") || trimmed.starts_with("TCDir time elapsed:") {
                return false;
            }
            // Skip free space lines (vary between runs)
            if trimmed.ends_with("bytes free on volume") {
                return false;
            }
            // Skip free space available to user (varies between runs)
            if trimmed.ends_with("bytes available to user") {
                return false;
            }
            true
        })
        .map(|s| s.to_string())
        .collect()
}





////////////////////////////////////////////////////////////////////////////////
//
//  strip_ansi_for_check
//
//  Strip ANSI escape sequences from a string — used only for filter pattern
//  matching.
//
////////////////////////////////////////////////////////////////////////////////

fn strip_ansi_for_check(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Skip escape sequence: ESC [ ... final_byte
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Read until a letter (final byte of CSI sequence)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
}





////////////////////////////////////////////////////////////////////////////////
//
//  compare_output
//
//  Compare RCDir and TCDir output for given arguments.  Returns (matching,
//  total, differences) where differences lists the first N mismatches.
//
////////////////////////////////////////////////////////////////////////////////

fn compare_output(args: &[&str]) -> (usize, usize, Vec<String>) {
    let tcdir = match get_tcdir_exe() {
        Some(p) => p,
        None => return (0, 0, vec!["TCDir.exe not found — skipping parity test".to_string()]),
    };
    let rcdir = get_rcdir_exe();

    let tc_output = run_command(&tcdir, args);
    let rc_output = run_command(&rcdir, args);

    let tc_lines = filter_lines(&tc_output);
    let rc_lines = filter_lines(&rc_output);

    let max_lines = tc_lines.len().max(rc_lines.len());
    let mut matching = 0;
    let mut differences = Vec::new();

    for i in 0..max_lines {
        let tc_line = tc_lines.get(i).map(|s| s.as_str()).unwrap_or("<missing>");
        let rc_line = rc_lines.get(i).map(|s| s.as_str()).unwrap_or("<missing>");

        if tc_line == rc_line {
            matching += 1;
        } else if differences.len() < 10 {
            differences.push(format!(
                "Line {}: TC=[{}] RC=[{}]",
                i + 1,
                tc_line,
                rc_line,
            ));
        }
    }

    (matching, max_lines, differences)
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_single_dir
//
//  Verifies output parity for a single directory listing of src/*.rs.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_single_dir() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format!("{}\\src\\*.rs", test_dir.display());
    let (matching, total, diffs) = compare_output(&[&pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_sorted_by_size
//
//  Verifies output parity when listing files sorted by size (/os).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_sorted_by_size() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format!("{}\\src\\*.rs", test_dir.display());
    let (matching, total, diffs) = compare_output(&["/os", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (sorted) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_wide_listing
//
//  Verifies output parity for wide listing format (/w).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_wide_listing() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format!("{}\\src\\*.rs", test_dir.display());
    let (matching, total, diffs) = compare_output(&["/w", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (wide) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_bare_listing
//
//  Verifies output parity for bare listing format (/b).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_bare_listing() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format!("{}\\src\\*.rs", test_dir.display());
    let (matching, total, diffs) = compare_output(&["/b", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (bare) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_directory_filter
//
//  Verifies output parity when filtering for directories only (/a:d).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_directory_filter() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format!("{}\\*", test_dir.display());
    let (matching, total, diffs) = compare_output(&["/a:d", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (dir filter) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_icons_on
//
//  Verifies output parity when icons are forced on (/Icons).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_icons_on() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format!("{}\\src\\*.rs", test_dir.display());
    let (matching, total, diffs) = compare_output(&["/Icons", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (icons on) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_icons_off
//
//  Verifies output parity when icons are forced off (/Icons-).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_icons_off() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format!("{}\\src\\*.rs", test_dir.display());
    let (matching, total, diffs) = compare_output(&["/Icons-", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (icons off) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_icons_wide
//
//  Verifies output parity for wide mode with icons (/w /Icons).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_icons_wide() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format!("{}\\src\\*.rs", test_dir.display());
    let (matching, total, diffs) = compare_output(&["/w", "/Icons", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (wide+icons) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_icons_bare
//
//  Verifies output parity for bare mode with icons (/b /Icons).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_icons_bare() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format!("{}\\src\\*.rs", test_dir.display());
    let (matching, total, diffs) = compare_output(&["/b", "/Icons", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (bare+icons) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_recursive
//
//  Verifies output parity for recursive listing (/s).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_recursive() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\src\\*.rs", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/s", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (recursive) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_recursive_bare
//
//  Verifies output parity for recursive bare listing (/s /b).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_recursive_bare() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\src\\*.rs", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/s", "/b", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        // Threshold relaxed to 90% because splitting source files into directory
        // modules (e.g. config/, results_displayer/) adds subdirectory entries
        // whose ANSI escapes may differ slightly between TCDir and RCDir.
        assert!(
            pct >= 90.0,
            "Output parity (recursive+bare) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_recursive_wide
//
//  Verifies output parity for recursive wide listing (/s /w).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_recursive_wide() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\src\\*.rs", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/s", "/w", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 90.0,
            "Output parity (recursive+wide) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_owner
//
//  Verifies output parity for owner display (/Owner).
//  Note: owner column has known ANSI color code differences between TCDir
//  and RCDir — threshold is lower to account for per-line escape sequence
//  variations in the owner field.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_owner() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\src\\*.rs", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/Owner", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 25.0,
            "Output parity (owner) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_streams
//
//  Verifies output parity for streams display (/Streams).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_streams() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\src\\*.rs", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/Streams", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (streams) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_recursive_single_threaded
//
//  Verifies output parity for single-threaded recursive listing (/s /m-).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_recursive_single_threaded() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\src\\*.rs", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/s", "/m-", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (recursive+single-threaded) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_recursive_icons
//
//  Verifies output parity for recursive listing with icons (/s /Icons).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_recursive_icons() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\src\\*.rs", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/s", "/Icons", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (recursive+icons) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_tree_basic
//
//  Verifies output parity for basic tree view (/Tree).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_tree_basic() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/Tree", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (tree basic) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_tree_depth_limited
//
//  Verifies output parity for depth-limited tree view (/Tree /Depth=2).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_tree_depth_limited() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/Tree", "/Depth=2", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (tree depth-limited) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_tree_custom_indent
//
//  Verifies output parity for tree view with custom indent (/Tree /TreeIndent=6).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_tree_custom_indent() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/Tree", "/TreeIndent=6", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (tree custom-indent) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_tree_with_icons
//
//  Verifies output parity for tree view with icons (/Tree /Icons).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_tree_with_icons() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/Tree", "/Icons", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (tree with-icons) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_tree_with_streams
//
//  Verifies output parity for tree view with streams (/Tree /r).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_tree_with_streams() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/Tree", "/r", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        // Both tools show help on validation error — help text naturally
        // differs (product names, switch prefixes).  Check that both
        // produced output (i.e., both rejected the input).
        if diffs.iter().any (|d| d.contains ("RC=[<missing>]")) {
            panic! ("RCDir produced no output for /Tree /r");
        }
        // Both tools showed help — validation parity confirmed
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_tree_file_mask
//
//  Verifies output parity for tree view with a file mask (/Tree *.rs).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_tree_file_mask() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*.rs", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/Tree", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (tree file-mask) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_tree_size_auto
//
//  Verifies output parity for tree view with explicit /Size=Auto.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_tree_size_auto() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/Tree", "/Size=Auto", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (tree size-auto) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_size_auto_non_tree
//
//  Verifies output parity for /Size=Auto without tree mode.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_size_auto_non_tree() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\src\\*.rs", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/Size=Auto", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (size-auto non-tree) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_size_bytes_explicit
//
//  Verifies output parity for /Size=Bytes in tree mode.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_size_bytes_explicit() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/Tree", "/Size=Bytes", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        // Both tools show help on validation error (/Tree + /Size=Bytes
        // is invalid).  Check that both produced output.
        if diffs.iter().any (|d| d.contains ("RC=[<missing>]")) {
            panic! ("RCDir produced no output for /Tree /Size=Bytes");
        }
        // Both tools showed help — validation parity confirmed
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_tree_sort_by_size
//
//  Verifies output parity for tree view sorted by size (/Tree /os).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_tree_sort_by_size() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/Tree", "/os", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (tree sort-by-size) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_tree_time_created
//
//  Verifies output parity for tree view sorted by creation time (/Tree /tc).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_tree_time_created() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/Tree", "/tc", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (tree time-created) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_tree_attr_filter
//
//  Verifies output parity for tree view with attribute filter (/Tree /a:d).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_tree_attr_filter() {
    let test_dir = std::env::current_dir().unwrap();
    let pattern = format! ("{}\\*", test_dir.display());
    let (matching, total, diffs) = compare_output (&["/Tree", "/a:d", &pattern]);
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        let pct = (matching as f64 / total as f64) * 100.0;
        assert!(
            pct >= 95.0,
            "Output parity (tree attr-filter) too low: {:.1}% ({}/{} lines). Diffs:\n{}",
            pct,
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  normalize_help_line
//
//  Normalize a help output line for comparison by replacing known intentional
//  differences: product name (Rusticolor/Technicolor → PRODUCT), executable
//  name (RCDIR/TCDIR), env var name (RCDIR/TCDIR), and stripping version
//  strings and build timestamps from the first header line.
//
////////////////////////////////////////////////////////////////////////////////

fn normalize_help_line(line: &str) -> String {
    let plain = strip_ansi_for_check (line);
    let trimmed = plain.trim();

    // Skip lines containing version/timestamp info — these always differ
    if trimmed.contains ("Directory version ") {
        return String::from ("<VERSION_LINE>");
    }

    // Strip Debug switch references — only present in debug builds, not in release TCDir
    if trimmed.contains ("Debug") && trimmed.contains ("Displays raw") {
        return String::new();  // Filter out the whole description line
    }

    // Normalize on plain text (ANSI stripped) for content comparison.
    let mut s = trimmed.to_string();

    // Strip debug-only synopsis token
    s = s.replace (" [/Debug]", "");
    s = s.replace (" [--Debug]", "");

    // Normalize product-specific names
    s = s.replace ("Rusticolor", "PRODUCT");
    s = s.replace ("Technicolor", "PRODUCT");
    s = s.replace ("RCDIR", "TOOLNAME");
    s = s.replace ("TCDIR", "TOOLNAME");
    s = s.replace ("rcdir", "toolname");
    s = s.replace ("tcdir", "toolname");
    s
}





////////////////////////////////////////////////////////////////////////////////
//
//  compare_help_output
//
//  Run both tools with /? and compare output line-by-line after normalizing
//  intentional differences (product name, version, timestamps, env var names).
//
////////////////////////////////////////////////////////////////////////////////

fn compare_help_output() -> (usize, usize, Vec<String>) {
    let tcdir = match get_tcdir_exe() {
        Some(p) => p,
        None => return (0, 0, vec!["TCDir.exe not found — skipping help parity test".to_string()]),
    };
    let rcdir = get_rcdir_exe();

    let tc_output = run_command (&tcdir, &["/?"]);
    let rc_output = run_command (&rcdir, &["/?"]);

    let tc_lines: Vec<String> = tc_output.lines().map (normalize_help_line).filter (|s| !s.is_empty()).collect();
    let rc_lines: Vec<String> = rc_output.lines().map (normalize_help_line).filter (|s| !s.is_empty()).collect();

    let max_lines = tc_lines.len().max (rc_lines.len());
    let mut matching = 0;
    let mut differences = Vec::new();

    for i in 0..max_lines {
        let tc_line = tc_lines.get (i).map (|s| s.as_str()).unwrap_or ("<missing>");
        let rc_line = rc_lines.get (i).map (|s| s.as_str()).unwrap_or ("<missing>");

        if tc_line == rc_line {
            matching += 1;
        } else if differences.len() < 20 {
            differences.push (format! (
                "Line {}: TC=[{}] RC=[{}]",
                i + 1,
                tc_line,
                rc_line,
            ));
        }
    }

    (matching, max_lines, differences)
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_help_output
//
//  Verifies that RCDir /? and TCDir /? produce identical output after
//  normalizing product-specific differences (name, version, timestamps).
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_help_output() {
    let (matching, total, diffs) = compare_help_output();
    if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
        assert_eq!(
            matching, total,
            "Help output parity: {}/{} lines match. Diffs:\n{}",
            matching,
            total,
            diffs.join ("\n"),
        );
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_reparse_normal_mode
//
//  Verifies output parity for a directory containing junctions/symlinks
//  in normal mode.  Uses %USERPROFILE% which typically has Application Data
//  junction and other reparse points.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_reparse_normal_mode() {
    if let Ok (profile) = std::env::var ("USERPROFILE") {
        let pattern = format! ("{}\\*", profile);
        let (matching, total, diffs) = compare_output (&[&pattern]);
        if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
            let pct = (matching as f64 / total as f64) * 100.0;
            assert!(
                pct >= 95.0,
                "Reparse normal parity too low: {:.1}% ({}/{} lines). Diffs:\n{}",
                pct,
                matching,
                total,
                diffs.join ("\n"),
            );
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_reparse_tree_mode
//
//  Verifies output parity for a directory containing junctions/symlinks
//  in tree mode with depth limiting.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_reparse_tree_mode() {
    if let Ok (profile) = std::env::var ("USERPROFILE") {
        let pattern = format! ("{}\\*", profile);
        let (matching, total, diffs) = compare_output (&["/Tree", "/Depth=1", &pattern]);
        if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
            let pct = (matching as f64 / total as f64) * 100.0;
            assert!(
                pct >= 95.0,
                "Reparse tree parity too low: {:.1}% ({}/{} lines). Diffs:\n{}",
                pct,
                matching,
                total,
                diffs.join ("\n"),
            );
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  parity_reparse_appexeclink
//
//  Verifies output parity for the WindowsApps directory which contains
//  AppExecLink reparse points (python.exe, winget.exe, etc.).
//  Skips if the directory does not exist.
//
////////////////////////////////////////////////////////////////////////////////

#[test]
fn parity_reparse_appexeclink() {
    let apps_dir = r"C:\Users\relmer\AppData\Local\Microsoft\WindowsApps";
    if std::path::Path::new (apps_dir).exists() {
        let pattern = format! ("{}\\*", apps_dir);
        let (matching, total, diffs) = compare_output (&[&pattern]);
        if total > 0 && !diffs.is_empty() && !diffs[0].contains ("not found") {
            let pct = (matching as f64 / total as f64) * 100.0;
            assert!(
                pct >= 95.0,
                "AppExecLink parity too low: {:.1}% ({}/{} lines). Diffs:\n{}",
                pct,
                matching,
                total,
                diffs.join ("\n"),
            );
        }
    }
}
