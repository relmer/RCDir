# Quickstart — RCDir Development

**Date**: 2026-02-08

---

## Prerequisites

- **Rust**: 1.93.0+ stable (install via [rustup](https://rustup.rs))
- **Targets**: `aarch64-pc-windows-msvc` (native), `x86_64-pc-windows-msvc` (cross)
- **OS**: Windows 10/11 (x64 or ARM64)
- **Editor**: VS Code with rust-analyzer extension

### Install Rust Targets

```powershell
rustup target add x86_64-pc-windows-msvc
rustup target add aarch64-pc-windows-msvc
```

---

## Build

### Quick Build (current architecture)

```powershell
cargo build                    # Debug, native arch
cargo build --release          # Release, native arch
```

### Cross-Architecture Build

```powershell
cargo build --target x86_64-pc-windows-msvc           # x64 Debug
cargo build --target aarch64-pc-windows-msvc          # ARM64 Debug
cargo build --release --target x86_64-pc-windows-msvc # x64 Release
```

### Via Build Script (full output + timing)

```powershell
.\scripts\Build.ps1                                    # Debug, auto-detect arch
.\scripts\Build.ps1 -Configuration Release -Platform x64
.\scripts\Build.ps1 -Target BuildAllRelease            # Both x64 + ARM64 Release
.\scripts\Build.ps1 -Target Clippy                     # Lint check
```

### VS Code Tasks

Use `Ctrl+Shift+B` → select a build task (Build Debug, Build Release, etc.)

---

## Test

```powershell
cargo test                     # All tests, native arch
.\scripts\RunTests.ps1         # Via script (Debug, auto-detect arch)
.\scripts\RunTests.ps1 -Configuration Release -Platform x64
```

### VS Code Tasks

Use `Ctrl+Shift+P` → "Run Test Task" → select platform/config.

---

## Run

```powershell
# After building:
cargo run                       # Run with default args
cargo run -- /s *.rs            # Recursive listing of .rs files
cargo run -- /os /a:-hsd        # Sort by size, exclude hidden/system/dirs
cargo run -- /?                 # Show help
```

---

## Deploy

```powershell
# Set deploy target:
$env:RCDIR_DEPLOY_PATH = "C:\Tools"

# Deploy release builds:
.\scripts\Deploy.ps1
```

Copies:
- `target/x86_64-pc-windows-msvc/release/rcdir.exe` → `$RCDIR_DEPLOY_PATH\RCDir.exe`
- `target/aarch64-pc-windows-msvc/release/rcdir.exe` → `$RCDIR_DEPLOY_PATH\RCDir_ARM64.exe`

---

## Project Structure

```
src/
├── main.rs              # Entry point
├── lib.rs               # Library root
├── command_line.rs       # CLI parser (custom, no clap)
├── config.rs            # Color config + RCDIR env var parsing
├── console.rs           # Buffered console output + ANSI colors
├── color.rs             # Color types and constants
├── ansi_codes.rs         # ANSI SGR escape sequences
├── directory_lister.rs   # Main listing orchestration
├── directory_info.rs     # Directory tree node
├── file_info.rs          # File entry (WIN32_FIND_DATAW wrapper)
├── file_comparator.rs    # Sort logic
├── drive_info.rs         # Volume information
├── mask_grouper.rs       # Multi-mask directory grouping
├── multi_threaded_lister.rs  # Parallel enumeration (std::thread + Condvar)
├── results_displayer.rs  # Display trait + Normal/Wide/Bare impls
├── listing_totals.rs     # Running totals
├── cloud_status.rs       # Cloud Files API
├── streams.rs            # NTFS alternate data streams
├── owner.rs              # File ownership
├── perf_timer.rs         # Performance timing
├── ehm.rs                # Error types
└── environment_provider.rs # Env var abstraction

tests/
└── output_parity/        # Integration tests vs TCDir
```

---

## Key Dependencies

```toml
[dependencies]
windows = { version = "0.62", features = [...] }  # Win32 APIs
widestring = "1"                                    # UTF-16 interop
```

---

## Reference Implementation

The C++ TCDir source is in the workspace under `TCDir/`. It is the authoritative reference for any behavior not specified in docs. **Never modify TCDir files.**

Key source files for reference:
- `TCDir/TCDirCore/CommandLine.cpp` — CLI parsing
- `TCDir/TCDirCore/Config.cpp` — Color config + env var parsing
- `TCDir/TCDirCore/Console.cpp` — Console output
- `TCDir/TCDirCore/DirectoryLister.cpp` — Main orchestration
- `TCDir/TCDirCore/ResultsDisplayerNormal.cpp` — Full listing format
- `TCDir/TCDirCore/MultiThreadedLister.cpp` — Parallel enumeration
