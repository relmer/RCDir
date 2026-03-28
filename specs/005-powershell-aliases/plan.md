# Implementation Plan: PowerShell Alias Configuration

**Branch**: `005-powershell-aliases` | **Date**: 2026-03-25 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/005-powershell-aliases/spec.md`

## Summary

Add three new long switches (`--set-aliases`, `--get-aliases`, `--remove-aliases`) to rcdir that manage PowerShell alias functions in the user's profile file. `--set-aliases` provides an interactive TUI wizard for configuring a root alias and sub-aliases. New source modules handle PS version detection (parent process inspection), profile path resolution (Windows shell APIs), profile file I/O (marker-delimited blocks), alias block generation, and a stepped TUI built on Console ReadInput APIs.

## Technical Context

**Language/Version**: Rust (stable toolchain, MSVC target)
**Primary Dependencies**: `windows` crate (Console API, Shell API, Process API, TlHelp32)
**Storage**: User's PowerShell profile files (text files on disk)
**Testing**: Built-in Rust test framework (`#[test]`, `#[cfg(test)]`)
**Target Platform**: Windows 10/11, x86_64-pc-windows-msvc and aarch64-pc-windows-msvc
**Project Type**: Single binary crate with library (`src/lib.rs` + `src/main.rs` + `tests/`)
**Performance Goals**: Interactive responsiveness (<50ms per keypress); `--get-aliases` completes in <1s
**Constraints**: Minimal external crates; no PowerShell child processes for path resolution
**Scale/Scope**: ~8 profile paths to scan (4 per PS version × 2 versions, scoped to detected version at runtime = 4); TUI has 4 wizard steps

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Code Quality | ✅ PASS | New modules follow Rust idioms, `Result<T, E>` error handling, formatting rules. No existing code modified except adding switch entries to `command_line.rs` and dispatch in `main.rs` |
| II. Testing Discipline | ✅ PASS | All new modules (profile_path_resolver, profile_file_manager, alias_block_generator, TUI widgets, PS version detector) will have unit tests. Integration tests for end-to-end wizard flow |
| III. UX Consistency | ✅ PASS | Uses `Console` for output. New `--` long switches follow existing pattern. `--set-aliases`/`--get-aliases`/`--remove-aliases` added to `usage.rs` help. Mutual exclusivity enforced in switch validation |
| IV. Performance | ✅ PASS | No hot-path changes. Alias operations are one-shot interactive; profile scanning is 4 file reads max. No impact on directory listing performance |
| V. Simplicity | ✅ PASS | Each new module has single responsibility. Minimal crate additions. Windows APIs used via `windows` crate (SHGetKnownFolderPath, CreateToolhelp32Snapshot, ReadConsoleInput) |

**Gate Result: PASS — no violations. Proceed to Phase 0.**

## Project Structure

### Documentation (this feature)

```text
specs/005-powershell-aliases/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (new files in existing project)

```text
src/
├── alias_manager.rs             # Top-level orchestrator for --set/--get/--remove-aliases
├── profile_path_resolver.rs     # PS version detection + profile path resolution
├── profile_file_manager.rs      # Read/write/backup profile files, marker block parsing
├── alias_block_generator.rs     # Generate PowerShell alias function code
├── tui_widgets.rs               # Interactive TUI components (text input, radio, checkbox, confirmation)
├── command_line.rs              # (MODIFIED) Add new switch entries + validation
├── ansi_codes.rs                # (MODIFIED) Add cursor hide/show, erase line codes
├── main.rs                      # (MODIFIED) Add dispatch to alias_manager
├── usage.rs                     # (MODIFIED) Add --set-aliases/--get-aliases/--remove-aliases help text
└── lib.rs                       # (MODIFIED) Add new module declarations

tests/
└── alias_integration.rs         # Integration tests for alias commands
```

**Structure Decision**: New source modules added to existing crate. Each module is a focused struct with `impl` blocks and a clear single responsibility: path resolution, file I/O, code generation, TUI rendering, and orchestration. Unit tests live in `#[cfg(test)] mod tests` within each module.

## Complexity Tracking

> No constitution violations — this table is empty.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| (none)    |            |                                     |

## Contracts

N/A — This feature has no REST/GraphQL API surface. It is a native Rust tool that reads/writes local profile files. The "contract" is the alias block format defined in FR-040 through FR-044 of the spec.

## Post-Design Constitution Re-Check

| Principle | Pre-Design | Post-Design | Delta |
|-----------|-----------|-------------|-------|
| I. Code Quality | ✅ PASS | ✅ PASS | No change. New modules follow Rust idioms, `Result<T, E>` error handling, formatting rules |
| II. Testing Discipline | ✅ PASS | ✅ PASS | No change. 5 new modules with `#[cfg(test)]` unit tests + integration test |
| III. UX Consistency | ✅ PASS | ✅ PASS | No change. `Console` output, `--` switches, `usage.rs` updated |
| IV. Performance | ✅ PASS | ✅ PASS | No change. No hot-path impact |
| V. Simplicity | ✅ PASS | ✅ PASS | No change. 5 focused modules, no external deps |

**Post-design gate: PASS**
