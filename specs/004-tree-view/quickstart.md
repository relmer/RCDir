# Quickstart: Tree View Display Mode

**Feature**: 004-tree-view
**Date**: 2026-02-28

## Prerequisites

- Rust stable (latest stable release, edition 2024)
- VS Code with Rust Analyzer extension
- Windows 10/11 (x64 or ARM64)
- PowerShell 7 (`pwsh`)

## Build & Test

```powershell
# From repo root:
cargo build
cargo test
cargo clippy -- -D warnings

# Or use project scripts:
.\scripts\Build.ps1
.\scripts\RunTests.ps1 -Configuration Debug -Platform Auto
```

## Quick Verification

After implementing the feature, verify with:

```powershell
# Basic tree (should show hierarchy with connectors)
rcdir --Tree

# Depth-limited tree
rcdir --Tree --Depth=2

# Tree with custom indent
rcdir --Tree --TreeIndent=2

# Tree with metadata options
rcdir --Tree --Icons --Streams

# Note: --Owner is incompatible with --Tree (owner column widths vary
# per-directory, breaking tree connector alignment)

# Abbreviated sizes (Explorer-style, default in tree mode)
rcdir --Tree                # Auto mode — sizes as KB/MB/GB/TB, 7-char fixed width
rcdir --Size=Auto           # Abbreviated sizes in non-tree mode
rcdir --Size=Bytes          # Explicit exact bytes (default in non-tree)

# Verify size alignment in tree mode
rcdir --Tree .              # All size columns should be uniformly 7 chars wide
rcdir --Tree --Depth=3      # Alignment preserved across depth levels

# Error cases (should all produce clear error messages)
rcdir --Tree -W       # Error: incompatible with wide
rcdir --Tree -B       # Error: incompatible with bare
rcdir --Tree -S       # Error: incompatible with recurse
rcdir --Tree --Owner  # Error: incompatible with owner
rcdir --Tree --Size=Bytes  # Error: incompatible with exact sizes
rcdir --Depth=3       # Error: requires --Tree
rcdir --TreeIndent=10 # Error: value out of range
rcdir --Size=Invalid  # Error: invalid value

# Environment variable configuration
$env:RCDIR = "Tree;Depth=2"
rcdir                 # Should show tree with depth 2
$env:RCDIR = "Size=Auto"
rcdir                 # Should show abbreviated sizes
$env:RCDIR = ""
```

## Key Files to Modify

| File | What to change |
|------|---------------|
| `src/command_line.rs` | Add `tree`, `max_depth`, `tree_indent`, `size_format` fields; parse `--Tree`, `--Depth=N`, `--TreeIndent=N`, `--Size=Auto\|Bytes`; add `validate_switch_combinations()` |
| `src/config/mod.rs` | Add `tree`, `max_depth`, `tree_indent`, `size_format` `Option` fields; `Attribute::TreeConnector` variant |
| `src/config/env_overrides.rs` | Parse `Tree`/`Tree-`/`Depth=N`/`TreeIndent=N`/`Size=Auto\|Bytes` from RCDIR env var |
| `src/tree_connector_state.rs` | **NEW** — `TreeConnectorState` struct with `get_prefix()`, `push()`, `pop()`, `depth()` |
| `src/results_displayer/mod.rs` | Add `Tree(TreeDisplayer)` variant to `Displayer` enum |
| `src/results_displayer/normal.rs` | Extract column-rendering helpers as `pub(crate)`; add `format_abbreviated_size()` |
| `src/results_displayer/tree.rs` | **NEW** — `TreeDisplayer` struct wrapping `NormalDisplayer`; tree display flow + tree-prefixed rendering |
| `src/directory_info.rs` | Add `parent`, `descendant_match_found`, `subtree_complete` fields for tree pruning |
| `src/multi_threaded_lister.rs` | Route `--Tree` to MT lister; thread tree state through recursion; depth checks; pruning |
| `src/file_comparator.rs` | Add `interleaved_sort` parameter to `sort_files()` |
| `src/usage.rs` | Document `--Tree`, `--Depth`, `--TreeIndent`, `--Size` in help output |
| `src/lib.rs` | Add `pub mod tree_connector_state;` |

## New Files

| File | Purpose |
|------|---------|
| `src/tree_connector_state.rs` | Encapsulates tree prefix generation logic — `push`/`pop` depth levels, generate `├── `/`└── `/`│   ` prefixes |
| `src/results_displayer/tree.rs` | Tree displayer struct — wraps `NormalDisplayer` via composition; overrides display flow for tree-walking and tree prefix insertion; reuses inherited column helpers |
| `tests/tree_mode_tests.rs` | Integration tests for tree mode output verification |
| `scripts/CompareOutput.ps1` | Ad-hoc cross-tool output comparison script — runs both `tcdir.exe` and `rcdir.exe` with identical args, compares output byte-for-byte |

## Output Fidelity Verification

**CRITICAL**: After implementing each step, verify output fidelity between RCDir and TCDir:

```powershell
# Ad-hoc comparison (after CompareOutput.ps1 is created)
.\scripts\CompareOutput.ps1 -Arguments "--Tree" -Directory "."
.\scripts\CompareOutput.ps1 -Arguments "--Tree","--Depth=2" -Directory "."
.\scripts\CompareOutput.ps1 -Arguments "--Tree","--TreeIndent=2"
.\scripts\CompareOutput.ps1 -Arguments "--Tree","--Icons"
.\scripts\CompareOutput.ps1 -Arguments "--Tree","*.rs"

# Run all parity tests
cargo test --test output_parity

# Run tree-specific parity tests
cargo test --test output_parity parity_tree

# Run all tests
cargo test
```

## Testing Order

1. **Unit tests first**: CommandLine parsing, Config parsing, TreeConnectorState prefix generation
2. **Unit tests**: Abbreviated size formatter, interleaved sort
3. **Integration**: FileComparator interleaved sort, switch validation
4. **Output parity**: Extend `tests/output_parity.rs` with tree-mode parity tests after each display feature is done
5. **Scenario**: End-to-end tree output with known directory structures
6. **Ad-hoc**: Run `scripts/CompareOutput.ps1` against varied directories to catch alignment/spacing differences
7. **Manual**: Visual verification of connector alignment across multiple depth levels

## Implementation Sequence

1. Tree connector constants + `TreeConnectorState` struct + unit tests (port 17 TCDir tests)
2. Switch parsing (`command_line.rs` + `config/`) + validation + unit tests (port 26 parsing tests + 7 config tests)
3. `TreeDisplayer` struct (wraps NormalDisplayer) + basic `display_single_entry`
4. Wire tree state through `MultiThreadedLister::print_directory_tree`; instantiate tree displayer in `lib.rs`
5. Interleaved sort in `file_comparator.rs` + unit tests (port 3 interleaved sort tests)
6. Depth limiting
7. Stream continuation lines (tree displayer)
8. `--Size=Auto|Bytes` switch + abbreviated size formatter + unit tests (port 12 formatter tests)
9. Usage help text
10. Reparse point (cycle) guard (shared path — protects both `-S` and `--Tree`)
11. Thread-safe empty subdirectory pruning
12. Environment variable configuration
13. Output parity tests — add 9 tree-mode parity tests to `tests/output_parity.rs` (see R15)
14. Cross-tool comparison script — create `scripts/CompareOutput.ps1` (see R15)
15. Final fidelity sweep — run `CompareOutput.ps1` against multiple directories and verify byte-for-byte match
