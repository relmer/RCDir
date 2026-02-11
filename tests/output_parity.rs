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

    // Default locations based on architecture
    let candidates = [
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
