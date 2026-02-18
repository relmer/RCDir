# Implementation Plan: RCDir Full Application (C++ to Rust Port)

**Branch**: `002-full-app-spec` | **Date**: 2026-02-08 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `spec.md`

## Summary

Port TCDir (a fast, colorized Windows directory listing tool) from C++ to Rust. The Rust implementation must be 100% user-identical to TCDir: same CLI, same output, same behavior. The C++ source in the workspace is the authoritative reference for anything not fully specified.

## Technical Context

**Language/Version**: Rust stable (1.93.0), Edition 2024
**Primary Dependencies**: `windows` crate (Win32 APIs), `widestring` (UTF-16 interop)
**Storage**: N/A (reads filesystem only)
**Testing**: `cargo test` (built-in `#[test]` + `#[cfg(test)]` modules, `tests/` for integration)
**Target Platform**: Windows 10/11, x64 (`x86_64-pc-windows-msvc`) and ARM64 (`aarch64-pc-windows-msvc`)
**Project Type**: Single CLI binary with library crate (`src/lib.rs` + `src/main.rs`)
**Performance Goals**: <1s for typical directories (<1000 files), <10s for 10,000+ file recursive listings, 2x+ speedup from multi-threading
**Constraints**: Byte-identical console output to TCDir for same inputs; Windows-only; 16-color console palette
**Scale/Scope**: ~15 C++ source files porting to ~15 Rust modules, ~1600-line spec

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Code Quality | PASS | Rust idioms, `Result<T,E>`, no `unwrap()` in production |
| II. Testing Discipline | PASS | `#[cfg(test)]` unit tests per module, integration tests in `tests/` |
| III. UX Consistency | PASS | 100% output parity with TCDir is the core requirement |
| IV. Performance Requirements | PASS | Win32 Console API via `windows` crate, `std::thread` workers for MT, buffered output |
| V. Simplicity & Maintainability | PASS | Single crate, minimal dependencies (2 external crates), module-per-concern |

No violations. All principles align with the port strategy.

## Implementation Order

**US-13 (Performance Timer)** must be implemented first, before any other feature. This enables tracking RCDir performance versus TCDir as each subsequent feature is ported. The `/p` switch and `PerfTimer` infrastructure are prerequisites for all other user stories.

## Project Structure

### Documentation (this feature)

```text
specs/002-full-app-spec/
├── spec.md              # Feature specification
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── tasks.md             # Implementation tasks
├── checklists/
│   └── requirements.md  # Requirements checklist
└── contracts/
    └── cli-contract.md  # Phase 1 output (CLI contract)
```

### Source Code (repository root)

```text
src/
├── main.rs              # Entry point (thin wrapper calling lib)
├── lib.rs               # Library root, module declarations, run()
├── command_line.rs       # CLI argument parsing (custom)
├── config.rs            # Color/switch configuration from RCDIR env var
├── console.rs           # Console output (WriteConsoleW, buffering, color)
├── ansi_codes.rs         # ANSI escape sequence constants
├── color.rs             # Color types, name mapping, defaults
├── directory_lister.rs   # Main directory listing orchestration
├── directory_info.rs     # Directory state for multi-threaded processing
├── drive_info.rs         # Volume/drive information
├── file_info.rs          # Extended file info (WIN32_FIND_DATA wrapper)
├── file_comparator.rs    # Sort comparison logic
├── mask_grouper.rs       # Multi-mask grouping by directory
├── multi_threaded_lister.rs  # Producer/consumer parallel enumeration (std::thread)
├── work_queue.rs             # Thread-safe FIFO work queue (Mutex + Condvar)
├── results_displayer.rs  # ResultsDisplayer trait + Normal/Wide/Bare impls
├── listing_totals.rs     # Running totals accumulation
├── perf_timer.rs         # Performance timing
├── cloud_status.rs       # Cloud Files API integration
├── streams.rs            # NTFS alternate data streams
├── owner.rs              # File ownership (Security APIs)
├── ehm.rs                # Error handling helpers
└── environment_provider.rs # Environment variable abstraction (for testability)

tests/
└── output_parity/        # Integration tests comparing rcdir vs tcdir output
```

**Structure Decision**: Single crate with `lib.rs` + `main.rs` (binary depends on library). Each C++ class maps to a Rust module. Trait-based display (Normal/Wide/Bare) mirrors C++ `IResultsDisplayer` interface.

## Complexity Tracking

No constitution violations to justify.
