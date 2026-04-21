# Implementation Plan: Ellipsize Long Link Target Paths

**Branch**: `008-ellipsize-targets` | **Date**: 2026-04-20 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `specs/008-ellipsize-targets/spec.md`

## Summary

Middle-truncate long link target paths using `…` (U+2026) to prevent line wrapping in normal and tree modes. Truncation preserves first two directory components and leaf filename where possible, falling back gracefully. New `--Ellipsize` switch (default on) with `--Ellipsize-` to disable. Ellipsis rendered in `Default` color attribute to be visually distinct from path text.

## Technical Context

**Language/Version**: Rust stable (latest)
**Primary Dependencies**: `windows` crate (Win32 API), `widestring` (UTF-16)
**Storage**: N/A
**Testing**: `cargo test` (Rust built-in `#[test]` + `#[cfg(test)]`)
**Target Platform**: Windows 10/11, x64 and ARM64
**Project Type**: CLI tool (desktop)
**Performance Goals**: Pure string operations — no I/O, no measurable cost
**Constraints**: Truncation logic must be a pure function testable with synthetic data
**Scale/Scope**: One new module (`path_ellipsis.rs`), switch plumbing in 3 files, display changes in 2 displayers

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Code Quality | PASS | Pure function for truncation logic; `Result` not needed (infallible string operations); follows existing module patterns |
| II. Testing Discipline | PASS | Pure truncation function testable with synthetic data; real WindowsApps paths as test inputs; no system state accessed; output parity tests added |
| III. UX Consistency | PASS | New `--Ellipsize` switch follows `--Icons`/`--Tree` pattern; documented in `-?` help and `--Settings`; `…` in Default color for visual distinction |
| IV. Performance | PASS | String operations only, called 0–N times per directory for reparse entries — negligible cost |
| V. Simplicity | PASS | One pure function, minimal display changes, follows existing switch infrastructure exactly |

**Gate result: PASS** — No violations.

## Project Structure

### Documentation (this feature)

```text
specs/008-ellipsize-targets/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (files affected)

```text
src/
├── path_ellipsis.rs                  # New — EllipsizedPath struct, ellipsize_path() pure function, unit tests
├── lib.rs                            # Add `pub mod path_ellipsis;`
├── command_line.rs                   # Add `ellipsize: Option<bool>` field, parse --Ellipsize/--Ellipsize- switch
├── config/
│   ├── mod.rs                        # Add `ellipsize: Option<bool>` field, bump SWITCH_COUNT 9→10, add to SWITCH_MEMBER_ORDER
│   └── env_overrides.rs              # Add ellipsize/ellipsize- to SWITCH_MAPPINGS, switch_name_to_source_index
├── results_displayer/
│   ├── normal.rs                     # Compute available width, call ellipsize_path(), render with split colors
│   └── tree.rs                       # Same as normal + account for tree prefix width
├── usage.rs                          # Add --Ellipsize to help output, SwitchInfo, and --Settings display

tests/
└── output_parity.rs                  # Add parity test cases for ellipsize in normal and tree modes
```

**Structure Decision**: New `path_ellipsis.rs` module encapsulates the truncation logic as a pure function — keeps displayers clean and makes the algorithm independently testable without mocks. Follows the same pattern as other single-purpose modules (`reparse_resolver.rs`, `cloud_status.rs`).

## Complexity Tracking

No constitution violations — this section is empty.
