# Research: Ellipsize Long Link Target Paths

**Date**: 2026-04-20
**Feature**: 008-ellipsize-targets

## R1: Available Width Calculation

### Decision
Compute available width arithmetically from known column widths. No character counter needed.

### Rationale
`Console` has no column tracker — it buffers ANSI-escaped text and flushes in bulk. But all metadata columns have deterministic widths computable from the same data the displayers already have access to.

### Formula (Normal Mode)
```
available_width = console.width()
    - 21                           // date+time: "MM/dd/yyyy  hh:mm tt "
    - 7                            // attributes: 7 single-char flags + trailing space
    - (2 + max(max_size_width, 5)) // file size (Bytes) or 9 (Auto/<DIR>) + leading/trailing space
    - cloud_status_width           // 3-4 chars if in sync root, 0 otherwise
    - debug_width                  // 14 if debug mode, 0 otherwise
    - owner_width                  // max_owner_len + 1 if --Owner, 0 otherwise
    - icon_width                   // 2 if icons active, 0 otherwise
    - filename_len                 // source filename length
    - 3                            // " → " (space + arrow + space)
```

### Formula (Tree Mode)
Same as normal mode, but additionally subtract:
```
    - tree_prefix_width            // tree_state.get_prefix(is_last).len()
```
Tree prefix width varies with depth and `--TreeIndent` setting.

### Alternatives Considered
- **Add a column counter to Console**: Invasive change, modifies every write method. Rejected — arithmetic is sufficient and matches TCDir's approach.
- **Measure buffer length before/after**: Fragile — buffer contains ANSI escape sequences that don't correspond to visible character positions.

## R2: ellipsize_path Pure Function Design

### Decision
Create `ellipsize_path(target_path: &str, available_width: usize) -> EllipsizedPath` as a pure function in `src/path_ellipsis.rs`. Returns a struct with prefix/suffix split for split-color rendering.

### Algorithm
```
if target_path.len() <= available_width:
    return EllipsizedPath::full(target_path)  // fits, no truncation

Split target_path into path components using '\' separator.

Try these forms in priority order, return first that fits:
  1. components[0] + "\" + components[1] + "\…\" + components[N-2] + "\" + components[N-1]
     (first two dirs + … + leaf dir + filename)
  2. components[0] + "\" + components[1] + "\…\" + components[N-1]
     (first two dirs + … + filename)
  3. components[0] + "\…\" + components[N-1]
     (first dir + … + filename)
  4. Leaf filename only, truncated to available_width with trailing …
     (e.g., "DesktopStickerEdito…")
```

### Return Type
```rust
struct EllipsizedPath {
    prefix:    String,  // Path text before the ellipsis (full path if not truncated)
    suffix:    String,  // Path text after the ellipsis (empty if not truncated)
    truncated: bool,    // true if path was middle-truncated
}
```

When `truncated` is true, the displayer renders: `prefix` (in file color) + `…` (in Default color) + `suffix` (in file color).

### Rationale
- Pure function = trivially testable with synthetic data, no mocks needed
- Struct return enables split-color rendering for the ellipsis character
- Priority order matches user's information preference (first two dirs + leaf are most actionable)
- Graceful degradation to leaf-only ensures something always displays

### Alternatives Considered
- **Return a single truncated string**: Rejected — cannot render ellipsis in a different color without knowing the split point
- **Return indices into original string**: More fragile, not worth the allocation savings for a path (short string)

## R3: Switch Infrastructure

### Decision
Add `ellipsize: Option<bool>` to both `Config` and `CommandLine`. Default behavior: on (truncation active). Follow the exact same pattern as `--Icons` and `--Tree` (supports negation via `-` suffix, conditional merge in `apply_config_defaults`).

### Changes Required
1. **config/mod.rs**: Add `pub ellipsize: Option<bool>`; bump `SWITCH_COUNT` from 9 to 10; add to `SWITCH_MEMBER_ORDER`
2. **config/env_overrides.rs**: Add `("ellipsize", true, ...)` and `("ellipsize-", false, ...)` to `SWITCH_MAPPINGS`; add `"ellipsize" => Some(9)` to `switch_name_to_source_index`; update error message to include "Ellipsize"
3. **command_line.rs**: Add `pub ellipsize: Option<bool>` field; add `("ellipsize", ..., "ellipsize-", ...)` to `bool_switches` table in `handle_long_switch`; add `"ellipsize"` to `is_recognized_long_switch`; add to `apply_config_defaults` with conditional merge
4. **usage.rs**: Add `SwitchInfo` entry; add to help text; add to `--Settings` display

### Default Behavior
When `ellipsize` is `None` (not set by user), treat as **true** (on). The displayer checks: `cmd.ellipsize.unwrap_or(true)`.

## R4: Ellipsis Color

### Decision
Render the `…` character using `Attribute::Default` color — visually distinct from the path text which uses the source file's color attribute (`text_attr`).

### Implementation
The display code in `normal.rs` and `tree.rs` splits the rendering:
```rust
let ep = ellipsize_path(&file_info.reparse_target, available_width);
if ep.truncated {
    console.writef(text_attr, format_args!("{}", ep.prefix));
    console.printf(config.attributes[Attribute::Default as usize], "\u{2026}");
    console.writef(text_attr, format_args!("{}", ep.suffix));
    console.puts(Attribute::Default, "");
} else {
    console.writef_line(text_attr, format_args!("{}", ep.prefix));
}
```

### Rationale
Default attribute ensures the ellipsis stands out regardless of the file's color assignment, signaling to the user that path components were elided.

## R5: Separator Character

### Decision
Use `\` (backslash) as the path separator for splitting and reassembly, matching Windows conventions. The ellipsis forms use `\…\` as the elision marker.

### Rationale
All target paths in RCDir come from Windows reparse point resolution, which always produces backslash-separated paths. No need to handle forward slashes.
