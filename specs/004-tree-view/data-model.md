# Data Model: Tree View Display Mode

**Feature**: 004-tree-view
**Date**: 2026-02-28
**Reference**: TCDir `specs/004-tree-view/data-model.md` — adapted for Rust/RCDir architecture

## Entities

### 1. CommandLine (extended)

Existing struct in `src/command_line.rs`, extended with new fields for tree view switches.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `tree` | `bool` | `false` | `--Tree` switch: activate tree view display mode |
| `max_depth` | `i32` | `0` | `--Depth=N`: max recursion depth (0 = unlimited) |
| `tree_indent` | `i32` | `4` | `--TreeIndent=N`: characters per tree indent level (1–8) |
| `size_format` | `SizeFormat` | `Default` | `--Size=Auto\|Bytes`: file size display format |

**`SizeFormat` enum** (new, in `command_line.rs`):
- `Default` — not explicitly set; tree mode uses `Auto`, non-tree uses `Bytes`
- `Auto` — Explorer-style abbreviated (1024-based, 3 significant digits, fixed 7-char width)
- `Bytes` — exact byte count with comma separators (existing behavior)

**Validation rules** (checked in `validate_switch_combinations()` or equivalent post-parse step):
- `tree` + `wide_listing` → error
- `tree` + `bare_listing` → error
- `tree` + `recurse` → error
- `tree` + `show_owner` → error
- `tree` + `size_format == Bytes` → error
- `max_depth > 0` without `tree` → error
- `tree_indent` outside [1, 8] → error
- `tree_indent != 4` without `tree` → error
- `max_depth ≤ 0` when explicitly specified → error (default value of 0 means unlimited; the parser rejects user-supplied values ≤ 0 before storage)
- `size_format` values other than `Auto` or `Bytes` → error

**Parsing changes to `handle_long_switch()`**:
- Add parameterized long switch support: split on `=` to extract `key` and `value`
- `"tree"` → sets `self.tree = true`
- `"tree-"` → sets `self.tree = false`
- `"depth"` → parse value as `i32`, store in `self.max_depth`
- `"treeindent"` → parse value as `i32`, store in `self.tree_indent`
- `"size"` → match value `"auto"` or `"bytes"` case-insensitively, store in `self.size_format`
- When `=` is absent for parameterized switches, consume the next argument as the value (space-separated form)

### 2. Config (extended)

Existing struct in `src/config/mod.rs`, extended with new `Option` fields and a new color attribute.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `tree` | `Option<bool>` | `None` | `Tree` / `Tree-` in RCDIR env var |
| `max_depth` | `Option<i32>` | `None` | `Depth=N` in RCDIR env var |
| `tree_indent` | `Option<i32>` | `None` | `TreeIndent=N` in RCDIR env var |
| `size_format` | `Option<SizeFormat>` | `None` | `Size=Auto` / `Size=Bytes` in RCDIR env var |

**New Attribute variant**:
- `Attribute::TreeConnector` — color attribute for tree connector characters; default color is DarkGrey

**Environment variable parsing** (`env_overrides.rs`):
- Add `"tree"` / `"tree-"` to the switch dispatch table
- Add integer switch parsing for `"depth"` and `"treeindent"` entries (format: `Depth=3`, `TreeIndent=2`)
- Add enum switch parsing for `"size"` entries (format: `Size=Auto`, `Size=Bytes`)
- Config-to-CLI override via existing `apply_config_defaults()` pattern

### 3. TreeConnectorState (new)

New struct in `src/tree_connector_state.rs`. Lightweight struct tracking the tree drawing state as the main thread recurses through the directory tree. Passed by mutable reference through the display call chain.

| Field | Type | Description |
|-------|------|-------------|
| `ancestor_has_sibling` | `Vec<bool>` | One entry per nesting depth. `true` = ancestor at that level has more siblings coming (draw `│`); `false` = ancestor was last at that level (draw space) |
| `tree_indent` | `i32` | Characters per indent level (from `CommandLine::tree_indent`, default 4) |

**Methods**:

| Method | Returns | Description |
|--------|---------|-------------|
| `new(tree_indent: i32)` | `Self` | Create a new empty state at depth 0 |
| `get_prefix(&self, is_last_entry: bool)` | `String` | Generates the full tree prefix string for the current entry. Iterates `ancestor_has_sibling` to build continuation lines, then appends `├── ` or `└── ` based on `is_last_entry` |
| `get_stream_continuation(&self)` | `String` | Generates the prefix for a stream line: same as regular prefix but replaces the connector with `│` + padding (vertical continuation only) |
| `push(&mut self, has_sibling: bool)` | `()` | Push a new depth level (entering a subdirectory) |
| `pop(&mut self)` | `()` | Pop a depth level (leaving a subdirectory) |
| `depth(&self)` | `usize` | Current nesting depth (length of `ancestor_has_sibling`) |

**State transitions**:
- Start: empty (depth 0, root directory)
- When entering a subdirectory's children: `push(parent_has_more_siblings)`
- When leaving a subdirectory's children: `pop()`
- At depth 0 (root): no prefix generated (top-level entries have no connectors)

### 4. TreeDisplayer (new)

New struct in `src/results_displayer/tree.rs`. Wraps `NormalDisplayer` via composition, implementing `ResultsDisplayer` trait. Added as a `Tree(TreeDisplayer)` variant to the `Displayer` enum.

**Struct fields**:

| Field | Type | Description |
|-------|------|-------------|
| `inner` | `NormalDisplayer` | The wrapped normal displayer, providing all column-rendering helpers |

**Methods**:

| Method | Trait/New | Description |
|--------|-----------|-------------|
| `new(console, cmd, config, icons_active)` | New | Creates a `TreeDisplayer` wrapping a new `NormalDisplayer` |
| `display_results(...)` | ResultsDisplayer | Delegates to inner `NormalDisplayer` for base display; tree-walking flow is driven externally by `MultiThreadedLister::print_directory_tree_mode` for streaming output (see Design Note below) |
| `display_recursive_summary(...)` | ResultsDisplayer | Delegates to inner for final summary display |
| `display_single_entry(...)` | New (pub) | Renders one file line — calls inner's column helpers, inserts tree prefix from `TreeConnectorState`, then icon + filename. Public because the MT lister calls it directly rather than through `display_results`. |
| `display_file_streams_with_tree_prefix(...)` | New | Like inner's stream display but prepends tree continuation prefix (`│   `) to each stream line |
| `save_directory_state()` | New | Captures per-directory display state into a `DirectoryDisplayState` struct |
| `restore_directory_state(state)` | New | Restores previously saved per-directory display state |
| `begin_directory(dir_info)` | New | Sets up per-directory state (field widths, sync root) on the inner displayer for a new directory level |
| `display_tree_root_header(drive_info, dir_info)` | New | Displays the drive header and path header for the root directory only |
| `display_tree_root_summary(totals)` | New | Displays the grand total summary at the end of the tree |
| `into_console(self)` | New | Consumes self, returns the inner Console |
| `console_mut(&mut self)` | New | Returns mutable reference to inner Console for flushing |

**`DirectoryDisplayState` struct** (used by `save_directory_state`/`restore_directory_state`):

| Field | Type | Description |
|-------|------|-------------|
| `largest_file_size_str_len` | `usize` | Cached string length of the largest file size in the current directory |
| `in_sync_root` | `bool` | Whether the current directory is under a cloud sync root |

**Why save/restore is needed**: Per-directory computed state is stored in fields on the `NormalDisplayer`. When tree mode recurses into a child directory, the child's setup overwrites the parent's state. Without save/restore, the parent's remaining entries after returning from the child render with incorrect column widths, causing misaligned output.

**Design Note — Lister-Driven Architecture**: The MT lister drives the tree walk (`print_directory_tree_mode` → `display_tree_entries` → `recurse_into_child_directory`) and calls tree-specific public methods (`display_single_entry`, `begin_directory`, `display_tree_root_header`, `display_tree_root_summary`) directly. This inversion is necessary because streaming output (FR-020) requires flush points between entry display and child recursion, and the MT lister already owns the `DirectoryInfo` tree and the console flush logic. `display_results` and `display_recursive_summary` delegate to the inner `NormalDisplayer` for non-tree paths.

**Pruning behavior**: When file masks are active (`tree_pruning_active` on `MultiThreadedLister`), empty subdirectories are pruned using a thread-safe event-based approach. Each `DirectoryInfo` node carries `descendant_match_found` and `subtree_complete` atomics (set by producer threads) that the display thread waits on to determine visibility. See R14 in research.md for the full design.

### 5. DirectoryInfo (extended for tree pruning)

Existing struct in `src/directory_info.rs`, extended with three new fields for thread-safe empty-subdirectory pruning in tree mode with file masks. These fields are only meaningful when `tree_pruning_active` is true on the lister.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `parent` | `Option<Weak<(Mutex<DirectoryInfo>, Condvar)>>` | `None` | Back-reference to parent node for upward propagation; avoids reference cycles. Only set when `tree_pruning_active`. |
| `descendant_match_found` | `AtomicBool` | `false` | Set `true` by producer when this node or any descendant has `file_count > 0`. Propagated upward through `parent` chain. |
| `subtree_complete` | `AtomicBool` | `false` | Set `true` by producer when this node AND all descendants have finished enumeration. |

**Thread safety**: `descendant_match_found` and `subtree_complete` are `AtomicBool` — lock-free writes by producers, lock-free reads by the display thread. After setting either atomic, the producer notifies the existing `Condvar` to wake the display thread. The `parent` `Weak` reference is set once during child directory creation (before the child is enqueued) and read-only thereafter — no synchronization needed.

**Invariants**:
- Once `descendant_match_found` is `true`, it never reverts to `false`.
- Once `subtree_complete` is `true`, it never reverts to `false`.
- A node with `subtree_complete == true && descendant_match_found == false` is definitively invisible (no matching descendants).
- A node with `descendant_match_found == true` is definitively visible (regardless of `subtree_complete`).

### 6. MultiThreadedLister (extended for tree pruning)

Existing struct in `src/multi_threaded_lister.rs`, extended with one new field and several new helper methods.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `tree_pruning_active` | `bool` | `false` | Set `true` when `tree && !all_star_mask` (tree mode with a non-`*` file mask). Gates all pruning logic. |

**New methods**:

| Method | Scope | Description |
|--------|-------|-------------|
| `propagate_descendant_match` | private | Walks up from a node through `parent`, setting `descendant_match_found = true` and notifying `Condvar` on each ancestor. Stops when parent is `None` or already flagged. |
| `try_signal_parent_subtree_complete` | private | Checks if all of a parent's children have `subtree_complete == true`. If so, sets the parent's `subtree_complete`, notifies, and recurses to grandparent. |
| `wait_for_tree_visibility` | private | Given a child `DirectoryInfo`, waits on `Condvar` until `descendant_match_found` or `subtree_complete` is true. Returns `true` if visible. |
| `print_directory_tree_mode` | private | Main-thread tree walk: processes entries at each level, flushes before child recursion, determines `├──` vs `└──` via look-ahead, manages `TreeConnectorState`. |
| `display_tree_entries` | private | Iterates entries at one level, calling `TreeDisplayer::display_single_entry` for each, with pruning look-ahead when `tree_pruning_active`. |

### 7. FileComparator (extended)

The existing sort function is extended to support an interleaved sort mode where directories and files are sorted together without grouping.

| Parameter | Type | Description |
|-----------|------|-------------|
| `interleaved_sort` | `bool` | When true, directories and files sort together in `sort_files` instead of directories first |

The `sort_files` function signature gains this parameter, or it's read from the `CommandLine` struct passed in. When `interleaved_sort` is true, the `SortKey` comparison skips the `is_dir` grouping step.

### 8. Abbreviated Size Formatter (new helper)

A new formatting function that converts a byte count to Explorer-style abbreviated format with a fixed 7-character width. The numeric portion is right-justified in a 4-character field, followed by a space, followed by the unit label left-justified in a 2-character field.

**Algorithm** (1024-based division, 3 significant digits):

| Range (bytes) | Format | Example | Width |
|---------------|--------|---------|-------|
| 0 | `0 B` | `   0 B ` | 7 |
| 1–999 | `### B` | ` 426 B ` | 7 |
| 1,000–1,023 | `1 KB` | `   1 KB` | 7 |
| 1,024–10,239 | `X.XX KB` | `4.61 KB` | 7 |
| 10,240–102,399 | `XX.X KB` | `17.1 KB` | 7 |
| 102,400–1,048,575 | `### KB` | ` 976 KB` | 7 |
| 1 MB+ | Same 3-sig-digit pattern | `16.7 MB` | 7 |
| 1 GB+ | Same | `1.39 GB` | 7 |
| 1 TB+ | Same | `1.00 TB` | 7 |

**`<DIR>` formatting**: Rendered as `" <DIR>   "` (1 leading space + `<DIR>` + 3 trailing spaces) when abbreviated mode is active, matching the alignment padding used by the Bytes-mode `<DIR>` display path.

**Location**: As a standalone function in `results_displayer/normal.rs` (or a separate `size_formatter.rs` module), callable from both `NormalDisplayer` and `TreeDisplayer`.

**Usage**: Called by the file size display path when `size_format` resolves to `Auto`. The existing comma-separated path is used when it resolves to `Bytes`.

### 9. Output Parity Test Infrastructure (extended)

Existing integration test file `tests/output_parity.rs` extended with tree-mode-specific parity tests. These tests run both `tcdir.exe` and `rcdir.exe` with identical arguments and compare output byte-for-byte (after filtering variable lines).

**New test functions** (added to `tests/output_parity.rs`):

| Test | Arguments | Threshold | Description |
|------|-----------|-----------|-------------|
| `parity_tree_basic` | `--Tree` | 100% | Basic tree listing |
| `parity_tree_depth_limited` | `--Tree --Depth=2` | 100% | Depth-limited tree |
| `parity_tree_custom_indent` | `--Tree --TreeIndent=2` | 100% | Custom indent width |
| `parity_tree_with_icons` | `--Tree --Icons` | 100% | Tree with file type icons |
| `parity_tree_with_streams` | `--Tree --Streams` | 100% | Tree with NTFS streams |
| `parity_tree_file_mask` | `--Tree *.rs` | 100% | Tree with file mask filtering |
| `parity_tree_size_auto` | `--Tree --Size=Auto` | 100% | Tree with explicit auto size |
| `parity_size_auto_non_tree` | `--Size=Auto` | 100% | Abbreviated sizes without tree |
| `parity_size_bytes_explicit` | `--Size=Bytes` | 100% | Explicit bytes format |

**100% threshold rationale**: Tree mode output is fully deterministic — no per-directory timing lines or free-space lines that vary between runs. The only filtered lines are the root directory's timing and free-space lines, after which all remaining lines should match exactly between the two tools.

**Existing infrastructure reused** (no changes needed):
- `get_tcdir_exe()` — locates TCDir.exe
- `get_rcdir_exe()` — locates built RCDir exe
- `run_command()` — captures stdout
- `filter_lines()` — removes timing/free-space lines
- `compare_output()` — line-by-line comparison engine

### 10. Cross-Tool Comparison Script (new)

New PowerShell script `scripts/CompareOutput.ps1` for ad-hoc manual output comparison between `tcdir.exe` and `rcdir.exe`.

**Parameters**:

| Parameter | Type | Description |
|-----------|------|-------------|
| `-Arguments` | `string[]` | Command-line arguments passed to both tools |
| `-Directory` | `string` | Target directory (optional, defaults to CWD) |
| `-MaxDiffs` | `int` | Maximum number of differences to display (default: 20) |
| `-ShowAll` | `switch` | Show all differences instead of first N |

**Behavior**:
1. Locates `tcdir.exe` and `rcdir.exe` via PATH or default build output paths
2. Runs both tools with identical arguments and target directory
3. Filters out timing and free-space lines
4. Compares remaining output line-by-line
5. Displays first N differences with line numbers and both versions
6. Exit code 0 = identical, 1 = differences found

**Usage example**:
```powershell
.\scripts\CompareOutput.ps1 -Arguments "--Tree","--Depth=2" -Directory "C:\Users"
.\scripts\CompareOutput.ps1 -Arguments "--Tree","*.rs" -Directory "."
.\scripts\CompareOutput.ps1 -Arguments "/s","/w" -ShowAll
```

## Relationships

```text
CommandLine ──parses──> tree, max_depth, tree_indent, size_format
     │
     ├─applies─> Config (tree, max_depth, tree_indent, size_format from RCDIR env var)
     │
     ├─validates─> Switch conflicts (Tree vs Wide/Bare/Recurse/Owner/Size=Bytes; Depth without Tree)
     │
     └─controls─> MultiThreadedLister::print_directory_tree_mode (lister-driven)
                       │
                       ├─creates─> TreeConnectorState
                       │                │
                       │                ├─push/pop per subdirectory
                       │                │
                       │                └─get_prefix() per entry
                       │
                       ├─calls──> TreeDisplayer (public methods directly)
                       │                │
                       │                ├─> display_single_entry (per-entry rendering)
                       │                ├─> begin_directory (per-directory state setup)
                       │                ├─> display_tree_root_header / display_tree_root_summary
                       │                ├─> save_directory_state / restore_directory_state
                       │                └─> display_file_streams_with_tree_prefix (streams)
                       │
                       ├─flushes─> Console::flush() before child recursion (streaming output)
                       │
                       ├─checks──> max_depth vs TreeConnectorState::depth()
                       │
                       └─prunes──> tree_pruning_active (tree + file mask)
                                       │
                                       ├─ Producer: propagate_descendant_match (upward via parent)
                                       ├─ Producer: try_signal_parent_subtree_complete (upward via parent)
                                       └─ Display:  wait_for_tree_visibility (blocks on Condvar)
```

## Tree Connector Visual Format

Each indent level occupies `tree_indent` characters (default 4). The connector characters are:

| Position | Characters (width 4) | Description |
|----------|---------------------|-------------|
| Middle entry | `├── ` | Vertical+right connector, 2 horizontal dashes, space |
| Last entry | `└── ` | Up+right connector, 2 horizontal dashes, space |
| Continuation (has more siblings) | `│   ` | Vertical line, then spaces to fill width |
| Continuation (no more siblings) | `    ` | Spaces only |
| Stream line | `│   ` | Always vertical continuation within current level |

For `tree_indent=N`, the horizontal dashes after `├`/`└` are `N-2` characters, with the final character being a space.

### Example Output (depth 2, indent 4, icons active)

Icons are 2 display columns wide (shown below as `■` placeholders). Tree connectors for a directory's children start at the same column as that directory's icon, so `├`/`└`/`│` lines sit directly below the parent folder icon.

```text
                                                       col:  0   4   8
                                                             v   v   v
2026/02/19  10:30 AM  ----A---  1.20 KB  ■ README.md
2026/02/19  10:30 AM  D-------   <DIR>   ■ src/
2026/02/19  10:30 AM  ----A---  5.55 KB  ├── ■ main.cpp
2026/02/19  10:30 AM  D-------   <DIR>   ├── ■ utils/
2026/02/19  10:30 AM  ----A---  2.29 KB  │   ├── ■ helpers.cpp
2026/02/19  10:30 AM  ----A---  1.08 KB  │   └── ■ helpers.h
2026/02/19  10:30 AM  ----A---  3.37 KB  └── ■ app.cpp
2026/02/19  10:30 AM  D-------   <DIR>   ■ tests/
2026/02/19  10:30 AM  ----A---  4.46 KB  └── ■ test_main.cpp
```

### Example Output (icons not active)

```text
2026/02/19  10:30 AM  ----A---       1,234  README.md
2026/02/19  10:30 AM  D-------      <DIR>   src/
2026/02/19  10:30 AM  ----A---       5,678  ├── main.cpp
2026/02/19  10:30 AM  D-------      <DIR>   ├── utils/
2026/02/19  10:30 AM  ----A---       2,345  │   ├── helpers.cpp
2026/02/19  10:30 AM  ----A---       1,111  │   └── helpers.h
2026/02/19  10:30 AM  ----A---       3,456  └── app.cpp
2026/02/19  10:30 AM  D-------      <DIR>   tests/
2026/02/19  10:30 AM  ----A---       4,567  └── test_main.cpp
```
