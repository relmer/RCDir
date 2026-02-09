// RCDir - Rust Technicolor Directory
// A fast, colorized directory listing tool for Windows

use std::process;

fn main() {
    if let Err(e) = rcdir::run() {
        eprintln!("{}", e);
        process::exit(1);
    }
}
