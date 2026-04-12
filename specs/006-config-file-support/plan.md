# Implementation Plan: Config File Support

**Branch**: `006-config-file-support` | **Date**: 2026-04-11 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/006-config-file-support/spec.md`

## Summary

Add config file support to rcdir so users can place settings in `%USERPROFILE%\.rcdirconfig` instead of cramming everything into the `RCDIR` environment variable. The config file uses the same entry syntax as the env var (one entry per line, `#` comments). Precedence: built-in defaults < config file < RCDIR env var < CLI flags. Error reporting includes file path + line numbers. The `--config` command is repurposed for config file diagnostics, and a new `--settings` command replaces the existing merged configuration view.

## Technical Context

**Language/Version**: Rust stable (latest stable release)
**Primary Dependencies**: `windows` crate for Win32 console API; standard library for file I/O (`std::fs::read`)
**Storage**: Single flat file (`%USERPROFILE%\.rcdirconfig`), UTF-8 with optional BOM
**Testing**: Rust built-in (`#[test]`, `#[cfg(test)]`, `cargo test`); integration tests in `tests/`
**Target Platform**: Windows 10/11, x64 and ARM64
**Project Type**: CLI tool (native Windows console application)
**Performance Goals**: Config file parsing adds no perceptible startup delay (50 settings < 1ms)
**Constraints**: Minimal external dependencies; file I/O via `std::fs::read`
**Scale/Scope**: Config files expected 20-150 lines max

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Code Quality | PASS | `Result<T, E>` for all fallible operations; idiomatic Rust patterns; formatting per copilot-instructions.md |
| II. Testing Discipline | PASS | Unit tests in inline `#[cfg(test)]` modules; integration tests for end-to-end validation |
| III. User Experience Consistency | PASS | Config file syntax matches env var; `--config` parallels `--env`; errors follow existing underline pattern with added line numbers |
| IV. Performance Requirements | PASS | Single file read at startup; no measurable impact on directory listing perf |
| V. Simplicity & Maintainability | PASS | No external dependencies; reuses existing `process_color_override_entry` for parsing; new code is a thin file-reading layer + line splitting |

No violations. Gate passes.

## Project Structure

### Documentation (this feature)

```text
specs/006-config-file-support/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
└── tasks.md             # Phase 2 output (created by /speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── config/
│   ├── mod.rs               # Extended: config file loading, source tracking, AttributeSource::ConfigFile
│   ├── env_overrides.rs     # Extended: source parameter threaded through override methods
│   └── file_reader.rs       # NEW: read file, BOM handling, line splitting
├── command_line.rs          # Extended: show_settings bool
├── usage.rs                 # Extended: --config repurposed, --settings new, error grouping
├── lib.rs                   # Extended: --settings dispatch, config file loaded in initialize()
```

**Structure Decision**: No new crates or modules beyond `config/file_reader.rs`. All changes are within the existing `src/` tree. The new file reader module parallels the existing `env_overrides.rs` pattern — a focused module for one config source. All tests are inline `#[cfg(test)]` modules within their respective source files.

## Complexity Tracking

No constitution violations. No complexity justifications needed.
