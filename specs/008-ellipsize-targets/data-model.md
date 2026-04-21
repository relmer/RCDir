# Data Model: Ellipsize Long Link Target Paths

**Date**: 2026-04-20
**Feature**: 008-ellipsize-targets

## Entities

### EllipsizedPath (new — in `src/path_ellipsis.rs`)

Return type from `ellipsize_path()`. Enables the displayer to render prefix and suffix in the source file's color with the `…` in Default color.

| Field | Type | Description |
|-------|------|-------------|
| `prefix` | `String` | Path text before the ellipsis (e.g., `C:\Program Files\`). Full path if not truncated. |
| `suffix` | `String` | Path text after the ellipsis (e.g., `\python3.12.exe`). Empty if not truncated. |
| `truncated` | `bool` | `true` if the path was middle-truncated, `false` if shown in full. |

**Rules:**
- When `truncated` is false: `prefix` contains the full path, `suffix` is empty
- When `truncated` is true: display is `prefix` + `…` + `suffix`
- Total display width when truncated: `prefix.len() + 1 + suffix.len()` (the `…` is 1 char wide)

### Config (modified — in `src/config/mod.rs`)

| Field | Type | Default | New? |
|-------|------|---------|------|
| `ellipsize` | `Option<bool>` | `None` (treated as true) | Yes |

### CommandLine (modified — in `src/command_line.rs`)

| Field | Type | Default | New? |
|-------|------|---------|------|
| `ellipsize` | `Option<bool>` | `None` (treated as true) | Yes |

## State Transitions

None. `EllipsizedPath` is computed per-line during display, not stored.

## Relationships

```
NormalDisplayer / TreeDisplayer
    │
    │ Computes available_width from metadata column widths + console.width()
    │
    ▼
ellipsize_path(&reparse_target, available_width)
    │
    ▼
EllipsizedPath { prefix, suffix, truncated }
    │
    ▼
if truncated:
    writef(text_attr, prefix) + printf(Default, "…") + writef_line(text_attr, suffix)
else:
    writef_line(text_attr, prefix)
```

## Validation Rules

- `available_width` is computed from `console.width()` minus all metadata columns — never negative (clamped to 0)
- Truncation only applies when `cmd.ellipsize.unwrap_or(true)` is true
- Paths with fewer than 3 components are never truncated (nothing to elide)
- The `…` character must save space: if the truncated form isn't shorter than the original, return the original
