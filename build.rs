// build.rs — Auto-increment build number and inject version + timestamp.
//
// On every compile:
// 1. Reads Version.toml (major, minor, build)
// 2. Increments `build` by 1
// 3. Writes Version.toml back
// 4. Emits cargo:rustc-env directives so src/ can use env!() macros
//
// Env vars injected:
//   RCDIR_VERSION_STRING  e.g. "0.1.42"
//   RCDIR_VERSION_YEAR    e.g. "2026"
//   RCDIR_BUILD_TIMESTAMP e.g. "Feb  9 2026 14:30"

use std::fs;
use std::path::Path;

use chrono::Local;





// ── Version data ──────────────────────────────────────────────────────────────

struct Version {
    major: u32,
    minor: u32,
    build: u32,
}





impl std::fmt::Display for Version {
    fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write! (f, "{}.{}.{}", self.major, self.minor, self.build)
    }
}





// ── Read / write Version.toml ─────────────────────────────────────────────────

fn read_version (path: &Path) -> Version {
    let contents = fs::read_to_string (path).expect ("Failed to read Version.toml");
    let mut major: u32 = 0;
    let mut minor: u32 = 0;
    let mut build: u32 = 0;



    for line in contents.lines() {
        let line = line.trim();
        
        
        
        if line.starts_with ('#') || line.is_empty() {
            continue;
        }

        if let Some ((key, value)) = line.split_once ('=') {
            let val = value.trim();



            match key.trim() {
                "major" => major = val.parse().unwrap_or (0),
                "minor" => minor = val.parse().unwrap_or (0),
                "build" => build = val.parse().unwrap_or (0),
                _ => {}
            }
        }
    }

    Version { major, minor, build }
}





fn write_version (path: &Path, version: &Version) {
    let contents = format! (
        "# RCDir version — build number auto-incremented by build.rs on every compile.\n\
         # Major and minor are updated manually.\n\
         major = {}\n\
         minor = {}\n\
         build = {}\n",
        version.major, version.minor, version.build,
    );



    fs::write (path, contents).expect ("Failed to write Version.toml");
}





// ── Timestamp ─────────────────────────────────────────────────────────────────

fn build_timestamp() -> String {
    Local::now().format ("%b %e %Y %H:%M").to_string()
}





fn current_year() -> String {
    Local::now().format ("%Y").to_string()
}





// ── Emit cargo env vars ───────────────────────────────────────────────────────

fn emit_env_vars (version: &Version, timestamp: &str, year: &str) {
    println! ("cargo:rustc-env=RCDIR_VERSION_STRING={version}");
    println! ("cargo:rustc-env=RCDIR_VERSION_YEAR={year}");
    println! ("cargo:rustc-env=RCDIR_BUILD_TIMESTAMP={timestamp}");
}





// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let version_path = Path::new ("Version.toml");
    let mut version  = read_version(version_path);
    let timestamp    = build_timestamp();
    let year         = current_year();



    println! ("cargo:rerun-if-changed=Version.toml");

    version.build += 1;
    write_version (version_path, &version);

    emit_env_vars (&version, &timestamp, &year);
}
