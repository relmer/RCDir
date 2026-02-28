# Research: Tree View Display Mode

**Feature**: 004-tree-view
**Date**: 2026-02-28
**Reference**: TCDir `specs/004-tree-view/research.md` — all research items were validated during the C++ implementation. This document adapts findings for Rust/RCDir.

## R1: Tree Connector Insertion Point

**Decision**: Insert tree prefix between the cloud-status column and the icon/filename in `TreeDisplayer::display_single_entry`. The `TreeDisplayer` wraps `NormalDisplayer` via composition and calls existing column-rendering helper functions, injecting the tree prefix before the filename.

**Rationale**: The existing file line rendering in `normal.rs` emits columns in this order: date/time → attributes → file size → cloud status → debug attrs → owner → icon → filename. Tree connectors logically belong with the filename (they describe the file's position in the hierarchy), so they insert just before the icon glyph (or before the filename if no icon). This preserves all metadata column alignment. The `TreeDisplayer` struct holds a `NormalDisplayer` internally and delegates column rendering to it, overriding only the display flow and entry-level rendering to add tree prefixes. Note: `--Owner` is incompatible with `--Tree` (per-directory owner column widths vary, breaking alignment), so the owner column is not rendered in tree mode.

**Alternatives considered**:
- Prepending connectors before the date column (shifts all columns, breaks alignment)
- Modifying `NormalDisplayer` in-place with conditional branching (tangles tree flow with normal flow; tree mode fundamentally changes the display flow — no per-subdir path headers, tree-indented summaries, recursive child walking)
- Trait-object based inheritance (Rust prefers composition over inheritance; `NormalDisplayer`'s helper functions aren't trait methods)

## R2: Parameterized Long Switch Convention

**Decision**: Long switches that take values use `=` as separator (`--Depth=3`), with space also accepted (`--Depth 3`). Parsed via `handle_long_switch` with `=` splitting in `command_line.rs`.

**Rationale**: No existing long switch in RCDir takes a value — `--owner`, `--streams`, `--icons`, `--env`, `--config` are all boolean. The `=` convention is the GNU standard and matches `eza`/`lsd` (`--level=3`). It distinguishes cleanly from the short-switch colon pattern (`/T:C`). Space support is added because many users expect it.

**Alternatives considered**:
- Colon separator (`--Depth:3`) — conflicts with existing short-switch convention
- Separate arguments only (`--Depth 3`) — ambiguous when value looks like a switch or file mask

## R3: Config Integer Value Parsing

**Decision**: Extend `Config` with new `Option<i32>` fields and add integer-typed entries to the switch/config parsing system in `env_overrides.rs` alongside the existing `Option<bool>` pattern.

**Rationale**: The existing `SWITCH_MAPPINGS` table in `env_overrides.rs` only supports `Option<bool>` fields. Adding `Depth=N` and `TreeIndent=N` to the env var requires either extending the existing dispatch or adding a parallel handler for integer-valued settings. A new `is_integer_switch_name()` check (or match arm in `process_color_override_entry`) before the existing boolean switch lookup keeps the existing path untouched.

**Alternatives considered**:
- String-parsing with `str::parse::<i32>()` inline in the existing switch handler (ad-hoc, doesn't scale)
- Generic variant-based switch table (over-engineered for 2 integers)

## R4: Architecture for Tree Display

**Decision**: Reuse the multi-threaded `print_directory_tree` recursive pattern on the main thread. Add tree connector state as a parameter threaded through the recursion. Display through `TreeDisplayer` (wrapping `NormalDisplayer` via composition).

**Rationale**: The existing MT path already builds a tree of `DirectoryInfo` nodes (via `children: Vec<WorkItem>`) and walks it depth-first on the main thread in `print_directory_tree`. For tree view, the same traversal adds a `Vec<bool>` tracking which ancestor levels still have siblings — this determines whether each level draws `│` (more siblings) or ` ` (no more siblings). The connector character for each entry (`├──` vs `└──`) is determined by whether it's the last entry at its level. `TreeDisplayer` wraps `NormalDisplayer` and overrides the display flow (no per-subdir path headers, indented summaries) and `display_single_entry` to prepend tree connectors, reusing all column helpers.

**Alternatives considered**:
- Building the tree in the displayer (wrong responsibility boundary)
- Post-processing approach — enumerate everything, store, then display (loses streaming benefits)

## R5: Single-Threaded Tree Path

**Decision**: Tree mode always uses the MT lister (even with `-M-`), since the MT lister already builds the tree structure needed for determining last-entry connectors.

**Rationale**: The ST path in `process_single_threaded` uses one-shot `DirectoryInfo` objects — it doesn't build a tree of children. Tree view requires knowing all entries at each level to determine which is "last" (for `└──` vs `├──`). The MT path already builds this tree. The cleanest approach: when `--Tree` is active, always use the MT code path for enumeration (which builds children), regardless of `-M` flag. The MT lister handles single-worker scenarios correctly anyway.

**Alternatives considered**:
- Refactoring ST path to build children (unnecessary duplication of MT logic)
- Two-pass ST: enumerate all, build tree, then display (works but slower)

## R6: Interleaved Dir/File Sorting for Tree

**Decision**: In tree mode, sort all entries (files and directories together) by the active sort order, instead of the current behavior that groups directories before files.

**Rationale**: The current `sort_files` in `file_comparator.rs` sorts `matches`. In current non-tree mode, directories are listed separately from files because the display path header/footer structure creates natural grouping. In tree mode, interleaving is more natural — users see entries in their sorted position regardless of type. A directory entry is followed by its children (indented), then the next sibling entry continues. This is controlled by a new `interleaved_sort` parameter passed to `sort_files`.

**Alternatives considered**:
- Keep grouping (directories first, then files) — less natural for tree view

## R7: Cycle Detection

**Decision**: There is NO existing cycle detection. Add reparse-point checking to the shared recursion path so both `-S` (recursive) and `--Tree` modes are protected from infinite recursion through junctions/symlinks.

**Rationale**: The current codebase unconditionally recurses into all directories. A junction or symlink creating a cycle causes infinite recursion — this is an existing bug in `-S` mode, not just a tree concern. The fix belongs in the common recursion point (worker thread function in `multi_threaded_lister.rs`): check `FILE_ATTRIBUTE_REPARSE_POINT` before recursing; if set, list the directory but don't expand its children, and show a `[→ target]` indicator. Fixing it once protects both modes.

**Alternatives considered**:
- Canonical path set tracking (more robust but more expensive)
- Rely on OS protections (unreliable — NTFS junctions can bypass)
- Tree-only fix (leaves existing `-S` mode vulnerable to the same bug)

## R8: Per-Directory Summary in Tree Mode

**Decision**: Tree mode suppresses per-directory summaries entirely and shows only the grand total (full traversal summary) at the end.

**Rationale**: In tree mode, all subdirectory contents are expanded inline — the user already sees every file at every level. Showing per-directory summaries is redundant and confusing because it counts only the root's immediate contents, not the full tree. The grand total at the end provides the only meaningful aggregate.

**Alternatives considered**:
- Per-directory summaries at each level (redundant with visible tree contents; clutters output)
- Both per-directory and grand total (rejected because per-directory counts confused users into thinking they were the full tree total)

## R9: Tree Connector Color

**Decision**: Add a new `Attribute::TreeConnector` variant to the `Attribute` enum in `config/mod.rs`, configurable via the `RCDIR` environment variable using the existing color system.

**Rationale**: The existing color system supports per-element color attributes (Date, Time, FileAttributePresent, Size, Directory, etc.). Adding a `TreeConnector` attribute follows the same pattern. The default color is DarkGrey, providing subtle visual distinction from content without being distracting.

**Alternatives considered**:
- Hardcoded color (not configurable — violates UX consistency principle)
- Inherit from Default attribute (may be too bright/distracting)

## R10: Displayer Architecture

**Decision**: Create a new `TreeDisplayer` struct that wraps `NormalDisplayer` via composition. It implements the `ResultsDisplayer` trait. The `Displayer` enum gains a `Tree(TreeDisplayer)` variant alongside the existing `Normal`, `Wide`, `Bare` variants.

**Rationale**: Tree mode isn't just "normal + prefix column" — it fundamentally changes the display flow: no separate path headers per subdirectory, summaries suppressed, directory-then-children recursion drives display order, stream continuation lines need `│` prefixes. Trying to branch all of this inside the existing `NormalDisplayer::display_results` tangles two concerns. A separate struct encapsulates tree-specific flow cleanly. Using composition (holding a `NormalDisplayer` field) rather than trait inheritance is idiomatic Rust — `TreeDisplayer` delegates column-rendering to its inner `NormalDisplayer`'s helper methods, overriding only the flow. Note: `--Owner` is incompatible with `--Tree` because per-directory owner column widths vary, breaking tree connector alignment across directory levels.

**Alternatives considered**:
- Modifying `NormalDisplayer` in-place (tangles tree and normal flow logic)
- Trait-object dynamic dispatch (unnecessary complexity; enum dispatch is simpler and faster)
- Deriving from a common base trait with default methods (Rust traits don't provide data inheritance; composition is cleaner)

## R11: Streaming Output — Flush Before Child Recursion

**Decision**: Flush the console buffer before recursing into each child directory in the tree walk, and again after the entry loop completes (for trailing file entries).

**Rationale**: Without explicit flushes, all output is buffered until the entire tree traversal completes — the user sees zero output for potentially minutes on large trees, making it appear hung. The tree display must feel live. Flushing before each child directory guarantees the parent's entry line (and any preceding siblings) are visible before the (potentially slow) child subtree begins. A second flush after the entry loop ensures trailing file entries are also visible promptly. Both flushes exist only in the tree walk path — the `-S` recursive path is untouched since it already flushes per-directory via `display_results`.

**Alternatives considered**:
- Flushing only at the end (user sees nothing until the entire subtree is done — defeats streaming)
- Flushing after every entry (excessive system calls, measurable perf impact)
- Reducing the Console buffer size (would affect all modes, not just tree)

## R12: Per-Directory Display State Preservation Across Recursion

**Decision**: Save and restore the per-directory display state (largest file size string length, sync root flag) around each recursive child call in the tree walk, via `save_directory_state()` / `restore_directory_state()` on `TreeDisplayer`. A `DirectoryDisplayState` struct captures these fields.

**Rationale**: Per-directory computed state (field widths, sync root flag) is stored in member fields on the displayer. When tree mode recurses into a child directory, the child's setup overwrites the parent's state. After returning from the child, the parent's remaining entries would render with the wrong column widths, causing misaligned output. Saving and restoring the state around each recursive descent preserves the parent's column layout. The state struct is small and the copy is negligible relative to I/O.

**Alternatives considered**:
- Pre-computing a global max across the entire tree before display (eliminates streaming benefit, terrible latency on large trees — TCDir tried this and rejected it)
- Passing state as a parameter instead of member fields (large refactor of the normal displayer)

## R13: Fixed-Width Abbreviated File Sizes for Tree Alignment

**Decision**: Add a `--Size=Auto` mode that formats file sizes using Explorer-style abbreviated format (1024-based, 3 significant digits) in a fixed 7-character column. Tree mode defaults to `--Size=Auto`; non-tree mode defaults to `--Size=Bytes` (existing exact-byte format).

**Rationale**: In tree mode, entries from different directories are interleaved in a single output stream. Each directory may have a different largest file, producing different size column widths. Variable-width size columns cause tree connectors to misalign between directories at different levels. A global pre-scan to compute a uniform max was tried in the TCDir implementation but rejected because it requires traversing the entire tree before displaying any output — defeating the streaming flush strategy (R11). A fixed-width abbreviated format solves alignment permanently with zero pre-scan cost. The Explorer-style format is familiar to Windows users.

**Format**: 1024-based division with 3 significant digits:
- `0 B` to `999 B` (exact bytes with `B` suffix)
- `1 KB` to `9.99 KB` (2 decimal places)
- `10.0 KB` to `99.9 KB` (1 decimal place)
- `100 KB` to `999 KB` (integer)
- Same pattern for MB, GB, TB
- `<DIR>` centered in the same 7-char field
- Right-justified, max 7 characters

**Alternatives considered**:
- Global max file size pre-scan before display (implemented in TCDir's earlier iteration; works but eliminates streaming — terrible latency on large trees)
- Variable-width columns with per-directory max (the original approach — causes tree connector misalignment)

## R14: Thread-Safe Empty Subdirectory Pruning in Tree Mode with File Masks

**Decision**: Use a producer-side upward-propagation design using two `AtomicBool` fields per `DirectoryInfo` node (`descendant_match_found`, `subtree_complete`) and a `Weak<(Mutex<DirectoryInfo>, Condvar)>` parent back-reference. The display thread uses a look-ahead pattern, waiting on the existing `Condvar` until enough information is available to determine each entry's visibility and `├──` vs `└──` connector.

**Problem**: A naive `has_descendant_files` tree-walk is racy because producer threads may not have finished building descendants when the display thread walks the tree. The display thread doesn't wait for the entire tree — it displays nodes as they become ready, so walking `children` of nodes that haven't finished enumerating reads incomplete data.

**Design**:

*New fields on `DirectoryInfo`* (tree mode with file mask only):
- `descendant_match_found: AtomicBool` — set `true` when any descendant (or self) has `file_count > 0`; propagated upward to all ancestors
- `subtree_complete: AtomicBool` — set `true` when this node AND all of its descendants have finished enumeration
- `parent: Option<Weak<(Mutex<DirectoryInfo>, Condvar)>>` — back-reference for upward propagation; only set when tree-pruning is active

*New field on `MultiThreadedLister`*:
- `tree_pruning_active: bool` — `true` when `tree && !all_star_mask` (tree mode with a non-`*` file mask); gates all pruning logic

*Producer side* (worker thread):
1. After enumeration completes, if `file_count > 0`, call `propagate_descendant_match(dir_info)` which walks up the parent chain via `parent`, setting `descendant_match_found = true` and notifying the `Condvar` on each ancestor.
2. After enumeration completes, check if the node has zero children (leaf) — if so, set `subtree_complete = true`, notify, and call `try_signal_parent_subtree_complete(parent)`.
3. `try_signal_parent_subtree_complete` checks whether ALL of the parent's children have `subtree_complete == true`. If so, sets the parent's `subtree_complete = true`, notifies, and recurses to grandparent.

*Display side* (main thread tree walk):
1. When `tree_pruning_active` is `false`, every directory is visible — no waiting.
2. When `tree_pruning_active` is `true`, for each directory entry in `matches`, call `wait_for_tree_visibility(child)` which blocks on `Condvar` until either `descendant_match_found` or `subtree_complete` becomes `true`.
3. If `descendant_match_found` → visible. If `subtree_complete && !descendant_match_found` → invisible (skip). When skipping, decrement the parent's `subdirectory_count` so that totals only count directories actually shown in the output.
4. Uses look-ahead to determine `is_last`: after the current entry, peek forward through subsequent entries to find the next visible one. For the next directory entry, call `wait_for_tree_visibility` to resolve it. Files are always visible.
5. If no next visible entry exists, the current entry is last (`└──`); otherwise it's middle (`├──`).

*Gating* — this entire mechanism is inert unless `tree_pruning_active`:
- Worker thread only sets `parent` when `tree_pruning_active`.
- Propagation helpers exit immediately when `parent` is `None`.
- The `-S` recursion path is completely untouched.

**Rationale**: The producer threads already know when they've finished enumerating each directory and how many files matched. By propagating this information upward at the point of knowledge (in the producer), the display thread never needs to walk the tree — it simply waits for a signal on each node. This eliminates the race condition because the display thread blocks until the answer is definitively known. The `Condvar` already exists on each node for the `wait_for_node_completion` mechanism, so reusing it adds no new synchronization primitives. The `Weak` parent back-reference avoids reference cycles.

**Performance**: The upward propagation is O(depth) per matching directory — typically very shallow. The `AtomicBool` checks are lock-free. The display thread only waits when it actually needs to display a directory entry and the answer isn't yet known.

**Alternatives considered**:
- Post-enumeration pass over the full tree (requires waiting for the entire tree to finish — defeats streaming output)
- Separate tree-mode lister (massive code duplication for a small behavioral difference)
- Pre-computing full visibility map upfront (the naive approach — racy because it walks incomplete subtrees)

## R15: Output Fidelity Testing — Byte-for-Byte TCDir/RCDir Comparison

**Decision**: Extend the existing `tests/output_parity.rs` integration test suite with tree-mode-specific parity tests that run both `tcdir.exe` and `rcdir.exe` with identical arguments and compare output byte-for-byte (after filtering timing/free-space lines). Additionally, add a dedicated cross-tool comparison script (`scripts/CompareOutput.ps1`) that can be run manually against arbitrary command lines and target directories for ad-hoc fidelity testing.

**Rationale**: RCDir and TCDir must produce identical output for the same input — alignment, spacing, connector characters, ANSI escape sequences, and column widths must all match exactly. The existing `tests/output_parity.rs` infrastructure already runs both exes and compares, but only covers non-tree modes (16 tests at present). Tree mode introduces new output elements (tree connectors, abbreviated sizes, interleaved sort order, per-level indentation) that must all match TCDir byte-for-byte. The 90-95% match threshold in existing parity tests should be raised to 100% for tree mode tests since tree mode output is fully deterministic (no per-directory timestamp or free-space lines that vary between runs).

**New parity tests to add** (in `tests/output_parity.rs`):
- `parity_tree_basic` — `--Tree` on a known directory
- `parity_tree_depth_limited` — `--Tree --Depth=2`
- `parity_tree_custom_indent` — `--Tree --TreeIndent=2`
- `parity_tree_with_icons` — `--Tree --Icons`
- `parity_tree_with_streams` — `--Tree --Streams`
- `parity_tree_file_mask` — `--Tree *.rs`
- `parity_tree_size_auto` — `--Tree --Size=Auto`
- `parity_size_auto_non_tree` — `--Size=Auto` (without tree)
- `parity_size_bytes_explicit` — `--Size=Bytes` (without tree)

**Ad-hoc comparison script** (`scripts/CompareOutput.ps1`):
- Takes arbitrary command-line arguments (passed to both tools)
- Optionally takes a target directory
- Runs `tcdir.exe` and `rcdir.exe` with identical args
- Filters out timing and free-space lines
- Compares output line-by-line, showing first N differences
- Exit code 0 = identical, 1 = differences found
- Used for manual verification during development and in CI

**Alternatives considered**:
- Golden-file snapshots (brittle — break when test directory contents change, or on different machines)
- Manual visual comparison only (not automated, doesn't catch subtle alignment differences)
- Comparing only non-ANSI text (misses color code differences that indicate attribute miscategorization)

## R16: Test Parity Audit — TCDir Unit Tests vs RCDir Unit Tests

**Decision**: Systematically ensure that RCDir has equivalent test coverage for every test category present in TCDir's test suite, particularly for features shared between the two tools. Document all deltas and close them as part of tree-view implementation.

**Audit results** (current state):

### Existing parity (RCDir already has equivalent tests):
- **Command line parsing**: RCDir has 31 tests vs TCDir's 84 — RCDir covers the same categories (sort, attributes, time, long switches, config defaults) but TCDir has more exhaustive case coverage. *Tree-specific parsing tests (26 in TCDir) must be ported.*
- **Config/env var parsing**: RCDir has 37 tests vs TCDir's ~134 — RCDir covers color, extension, attribute, switch overrides. *TCDir tree env var tests (7) must be ported.*
- **File comparator**: RCDir has 9 tests vs TCDir's 9 — coverage matches except *interleaved sort tests (3 in TCDir, 0 in RCDir) must be added.*
- **Icon mapping**: RCDir has 8 tests vs TCDir's 12 — similar coverage, RCDir has slightly different structure.
- **Nerd font detection**: RCDir has 12 tests vs TCDir's 9 — RCDir actually has better coverage here.
- **Console/display**: RCDir has 10 tests vs TCDir's 22 — RCDir tests formatting helpers; TCDir also tests ANSI color output and usage alignment. *Usage alignment tests should be added.*
- **Cloud status**: RCDir has 6 tests vs TCDir's 8 (in ResultsDisplayerTests) — equivalent coverage.
- **Mask grouping**: RCDir has 6 tests vs TCDir's 11 — partial overlap.

### Missing from RCDir (must be added for tree feature):
- **TreeConnectorState tests**: 0 in RCDir vs 17 in TCDir — all 17 must be ported
- **Abbreviated size formatter tests**: 0 in RCDir vs 12 in TCDir — all 12 must be ported
- **Interleaved sort tests**: 0 in RCDir vs 3 in TCDir — all 3 must be ported
- **Tree mode integration/scenario tests**: 0 in RCDir vs 18 in TCDir — equivalent scenarios must be created
- **Tree switch parsing tests**: 0 in RCDir vs 26 in TCDir — all must be ported
- **Tree config env var tests**: 0 in RCDir vs 7 in TCDir — all must be ported

### Missing from RCDir (pre-existing delta, not tree-specific):
- **Usage alignment tests**: 0 in RCDir vs 4 in TCDir — recommended but not blocking
- **EHM integration tests**: N/A — RCDir uses Rust `Result<T, E>`, not EHM macros
- **DirectoryListerScenarioTests infrastructure**: TCDir uses IAT-hooked mock filesystem for integration tests; RCDir has no equivalent mock filesystem. RCDir parity tests use the real filesystem. *Consider adding a mock filesystem for deterministic tree integration tests.*

**Alternatives considered**:
- Skip parity audit (risks divergent behavior between tools)
- Port every single TCDir test verbatim (some tests are C++/CppUnitTestFramework-specific and don't translate directly; focus on behavioral equivalence instead)
