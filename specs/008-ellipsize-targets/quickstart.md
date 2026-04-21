# Quickstart: Ellipsize Long Link Target Paths

**Date**: 2026-04-20
**Feature**: 008-ellipsize-targets

## What's Changing

Long link target paths are middle-truncated with `…` to prevent line wrapping:

```
Before (wraps at 120 chars):
python.exe → C:\Program Files\WindowsApps\Microsoft.DesktopAppInstaller_1.29.30.0_arm64__8wekyb3d8bbwe\AppInstallerPythonRedirector.exe

After (fits in 120 chars):
python.exe → C:\Program Files\…\AppInstallerPythonRedirector.exe
```

New switch: `--Ellipsize` (default on), `--Ellipsize-` to disable.

## Files to Modify

| File | Change |
|------|--------|
| `src/path_ellipsis.rs` | **New** — `EllipsizedPath` struct, `ellipsize_path()` pure function, `#[cfg(test)]` unit tests |
| `src/lib.rs` | Add `pub mod path_ellipsis;` |
| `src/command_line.rs` | Add `ellipsize: Option<bool>` field, parse `--Ellipsize` / `--Ellipsize-`, add to recognized switches, apply config defaults |
| `src/config/mod.rs` | Add `ellipsize: Option<bool>` field, bump `SWITCH_COUNT` 9→10, add to `SWITCH_MEMBER_ORDER` |
| `src/config/env_overrides.rs` | Add `ellipsize`/`ellipsize-` to `SWITCH_MAPPINGS`, `switch_name_to_source_index` |
| `src/results_displayer/normal.rs` | Compute available width, call `ellipsize_path()`, render with split colors |
| `src/results_displayer/tree.rs` | Same as normal + account for tree prefix width |
| `src/usage.rs` | Add `--Ellipsize` to help output, `SWITCH_INFOS`, and `--Settings` display |
| `tests/output_parity.rs` | Add parity test cases for ellipsize in normal and tree modes |

## Build & Test

```powershell
# Build (uses VS Code task or Build.ps1 — never raw cargo build)
# Use VS Code task: "Build Debug (current arch)"

# Test
cargo test

# Clippy
cargo clippy -- -D warnings
```

## Key Design Decisions

1. **Pure function** — `ellipsize_path()` takes a path string and available width, returns a struct with prefix/suffix split
2. **Arithmetic width calculation** — no character counter needed; compute from known column widths + `console.width()`
3. **Priority-based truncation** — first two dirs + leaf dir + filename > first two dirs + filename > first dir + filename > leaf only
4. **Ellipsis color** — `Attribute::Default`, not file color, so it's visually distinct
5. **Default on** — most users benefit from truncation; `--Ellipsize-` opts out
6. **Same pattern as --Icons/--Tree** — `Option<bool>` with conditional merge in `apply_config_defaults`
