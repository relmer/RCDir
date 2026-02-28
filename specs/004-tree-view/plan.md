# Implementation Plan: Tree View Display Mode

**Branch**: `004-tree-view` | **Date**: 2026-02-28 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/004-tree-view/spec.md`
**Reference**: TCDir implementation (C++) — fully implemented and tested; this plan adapts the design for Rust

## Summary

Add a `--Tree` display mode that renders directory contents hierarchically with Unicode box-drawing connectors, configurable depth (`--Depth=N`), configurable indent width (`--TreeIndent=N`), and configurable connector color. Add a `--Size=Auto|Bytes` switch for fixed-width abbreviated file sizes (Explorer-style 3-significant-digit format, 1024-based) that defaults to `Auto` in tree mode and `Bytes` in non-tree mode, ensuring column alignment without pre-scanning the directory tree. The implementation reuses the existing multi-threaded producer/consumer enumeration model and introduces a new `TreeDisplayer` struct (wrapping `NormalDisplayer` via composition) that provides tree-walking display flow while reusing all normal-mode column rendering helpers. When tree mode is used with file masks, empty subdirectories are pruned using a thread-safe upward-propagation design (see research.md R14) that avoids walking in-progress subtrees.

## Technical Context

**Language/Version**: Rust stable (edition 2024)
**Primary Dependencies**: `windows` crate (Win32 API), `widestring` crate, Rust std library only (no third-party libraries)
**Storage**: N/A (filesystem enumeration, no persistent storage)
**Testing**: Rust built-in test framework (`#[test]`, `#[cfg(test)]`, `cargo test`) + integration tests in `tests/`
**Target Platform**: Windows 10/11, x64 and ARM64
**Project Type**: Single native console application (single `rcdir` binary crate with `src/lib.rs` library)
**Performance Goals**: Tree view must not regress performance of existing modes; measurable via `-P` flag. **Streaming output is the paramount performance concern** — the user must see progressive output as directories are enumerated, not wait for the entire tree to complete. This principle drove several key design decisions: the lister-driven architecture (MT lister controls traversal with flush points, rather than the displayer driving the walk), fixed-width abbreviated sizes (eliminates pre-scan latency), and the producer-side upward-propagation pruning design (display thread waits per-node rather than for the full tree).
**Constraints**: Minimal external dependencies; all console output through `Console`; `Result<T, E>` error handling for all fallible operations; no `unwrap()` in production code
**Scale/Scope**: Targets directories with 1000+ files across 50+ subdirectories; deterministic output with same sort order
**Size Display**: Explorer-style abbreviated sizes (1024-based, 3 significant digits, B/KB/MB/GB/TB, 7-char fixed width); `--Size=Auto` default in tree mode, `--Size=Bytes` (comma-separated exact) default in non-tree mode

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Pre-Research Check

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Code Quality | PASS | Will follow Rust formatting rules, `Result<T, E>` error handling, ownership/borrowing patterns |
| II. Testing Discipline | PASS | Unit tests planned for all new public functions and code paths via `#[cfg(test)]` and integration tests |
| III. UX Consistency | PASS | Uses `Console` for output, new switches documented in `usage.rs`, error messages to stderr, mirrors TCDir CLI |
| IV. Performance | PASS | Reuses MT model, no new hot-path allocations beyond tree state `Vec<bool>`, measurable via `-P` |
| V. Simplicity | PASS | No new external deps, one new displayer + one new struct, reuses existing patterns |
| Technology Constraints | PASS | Rust stable, `windows` crate + std only, x64 and ARM64 |
| Development Workflow | PASS | Build via `cargo build`, tests via `cargo test`, lint via `cargo clippy` |

### Post-Design Check

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Code Quality | PASS | New code follows Rust idioms, `Result<T, E>`, borrowing over ownership, proper error propagation |
| II. Testing Discipline | PASS | Tests cover: switch parsing, validation, connector generation, depth limiting, interleaved sort, abbreviated sizes, output fidelity (byte-for-byte TCDir comparison). See R15/R16 in research.md. |
| III. UX Consistency | PASS | `--Tree`/`--Depth`/`--TreeIndent` follow existing long-switch convention; usage.rs updated; RCDIR env var extended |
| IV. Performance | PASS | Tree connector state is a small `Vec<bool>` threaded through recursion; no per-file allocations |
| V. Simplicity | PASS | Composition-based `TreeDisplayer` wrapping `NormalDisplayer`; new `TreeConnectorState` struct; no new crate deps |
| Technology Constraints | PASS | Pure Rust with `windows` crate; no new dependencies |
| Development Workflow | PASS | Standard cargo build/test/clippy cycle; both architectures verified |

## Project Structure

### Documentation (this feature)

```text
specs/004-tree-view/
├── plan.md              # This file
├── research.md          # Phase 0 output (completed)
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (N/A — no API contracts for CLI tool)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (affected files)

```text
src/
├── command_line.rs        # Add tree, max_depth, tree_indent, size_format fields; parse --Tree, --Depth=N, --TreeIndent=N, --Size=Auto|Bytes; validate conflicts
├── config/
│   ├── mod.rs             # Add tree, max_depth, tree_indent, size_format Option fields; Attribute::TreeConnector variant
│   └── env_overrides.rs   # Parse Tree/Tree-/Depth=N/TreeIndent=N/Size=Auto|Bytes from RCDIR env var
├── tree_connector_state.rs  # NEW: TreeConnectorState struct with get_prefix(), push(), pop(), depth()
├── results_displayer/
│   ├── mod.rs             # Add TreeDisplayer variant to Displayer enum; extend ResultsDisplayer trait
│   ├── normal.rs          # Extract column-rendering helpers as pub(crate)-visible; add format_abbreviated_size
│   └── tree.rs            # NEW: TreeDisplayer struct (wraps NormalDisplayer); tree display flow + tree-prefixed rendering
├── directory_info.rs      # Add parent weak ref, descendant_match_found, subtree_complete atomics for tree pruning
├── multi_threaded_lister.rs # Route --Tree to MT lister path; thread tree state through recursion; depth checks; producer-side propagation; display-side look-ahead
├── file_comparator.rs     # Add interleaved sort mode (no dir-first grouping)
├── usage.rs               # Document --Tree, --Depth, --TreeIndent, --Size in help output
├── lib.rs                 # Add tree_connector_state module; wire tree displayer creation
└── main.rs                # No changes expected

tests/
├── output_parity.rs       # Extend with tree mode parity tests (byte-for-byte TCDir comparison; see R15)
└── tree_mode_tests.rs     # NEW: tree-specific integration tests

scripts/
└── CompareOutput.ps1      # NEW: ad-hoc cross-tool output comparison script (see R15)
```

**Structure Decision**: No new crates or projects. The feature extends the existing `rcdir` binary crate. One new source file (`tree_connector_state.rs`) encapsulates tree prefix logic. One new file (`results_displayer/tree.rs`) implements the tree displayer via composition around `NormalDisplayer`. The `Displayer` enum gains a `Tree(TreeDisplayer)` variant, following the existing pattern of `Normal(NormalDisplayer)`, `Wide(WideDisplayer)`, `Bare(BareDisplayer)`.

## Output Fidelity Strategy

**CRITICAL**: RCDir and TCDir must produce byte-identical output for the same inputs (same command line, same target directory). This applies to ALL output modes including tree mode. See R15 and R16 in research.md for the full design.

### Testing layers

1. **Unit tests** (`#[cfg(test)]` in source files) — verify individual functions: connector generation, size formatting, switch parsing, sort order.
2. **Integration tests** (`tests/output_parity.rs`) — run both `rcdir.exe` and `tcdir.exe` with identical arguments, compare output line-by-line. Existing 16 tests cover non-tree modes; 9 new tree-mode tests to be added.
3. **Integration tests** (`tests/tree_mode_tests.rs`) — tree-specific structural verification (connector patterns, depth limiting, pruning behavior).
4. **Ad-hoc comparison** (`scripts/CompareOutput.ps1`) — manual developer tool for testing arbitrary command lines and directories against both tools.

### Test parity requirements

All tree-specific test categories from TCDir (see R16 audit) must have Rust equivalents before the feature is considered complete:
- TreeConnectorState: 17 tests
- Command-line tree parsing: 26 tests
- Config tree env vars: 7 tests
- Interleaved sort: 3 tests
- Abbreviated size formatting: 12 tests
- Tree mode integration/scenarios: 18 tests
- Output parity (cross-tool): 12 new tests

## Complexity Tracking

No constitution violations. No complexity justifications needed.
