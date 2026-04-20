# Implementation Plan: Symlink & Junction Target Display

**Branch**: `007-symlink-junction-targets` | **Date**: 2026-04-19 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/007-symlink-junction-targets/spec.md`

## Summary

Display `→ target_path` after symlinks, junctions, and AppExecLinks in normal and tree mode listings. Requires a new reparse point resolver module that reads reparse data via `DeviceIoControl` + `FSCTL_GET_REPARSE_POINT`, parses the three supported buffer formats (junction, symlink, AppExecLink), and stores the resolved target in `FileInfo`. Display integration adds the arrow (Information color) and target path (filename color) after the entry name. Ported from TCDir spec 007.

## Technical Context

**Language/Version**: Rust stable (latest stable release, per rust-toolchain.toml)
**Primary Dependencies**: `windows` crate (Win32 API: `CreateFileW`, `DeviceIoControl`, `FSCTL_GET_REPARSE_POINT`)
**Storage**: N/A (filesystem reads only, no persistent state)
**Testing**: `cargo test` — pure-function buffer parsing tests with synthetic byte arrays
**Target Platform**: Windows 10/11, x64 and ARM64
**Project Type**: CLI application (directory lister)
**Performance Goals**: Zero overhead for non-reparse files (single attribute flag check); <100µs per reparse point resolution
**Constraints**: Stack-allocated 16KB buffer for reparse data; no heap allocation in hot path
**Scale/Scope**: Typically 0–5 reparse points per directory listing; negligible impact on overall listing time

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Code Quality | ✅ PASS | `Result<T, E>` for all Win32 calls; no `unwrap()` in production; borrowing preferred; idiomatic Rust patterns |
| II. Testing Discipline | ✅ PASS | Pure-function buffer parsers tested with synthetic byte arrays; no file system dependency in unit tests |
| III. User Experience Consistency | ✅ PASS | Arrow uses Information color (matches TCDir); target uses filename color; output parity with TCDir verified |
| IV. Performance Requirements | ✅ PASS | Single flag check for non-reparse files; stack-allocated 16KB buffer; no heap allocation in resolver |
| V. Simplicity & Maintainability | ✅ PASS | Single new module (reparse_resolver); pure parsing functions separated from I/O; minimal display changes |

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── file_info.rs              # Add reparse_target: String field
├── reparse_resolver.rs       # NEW: Win32 reparse data reading + buffer parsing
├── directory_lister.rs       # Call resolve_reparse_target() in add_match_to_list()
├── multi_threaded_lister.rs  # Same integration as directory_lister
└── results_displayer/
    ├── normal.rs             # Append → target after filename
    └── tree.rs               # Append → target after filename

tests/
└── (inline #[cfg(test)])     # Pure-function buffer parsing tests in reparse_resolver.rs
```

**Structure Decision**: Single new module `reparse_resolver.rs` at src/ root level. Buffer parsing functions are pure (no I/O), testable inline. Display changes are minimal additions to existing normal and tree displayers.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |
