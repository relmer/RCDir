# CLI Contract: Nerd Font File Icons

**Feature**: 003-file-icons | **Date**: 2026-02-16

---

## Command-Line Switches

### `/Icons`

| Property | Value |
|----------|-------|
| Switch names | `/Icons`, `-Icons`, `--Icons` |
| Case-sensitive | No (`/icons`, `/ICONS` all accepted) |
| Type | Boolean (presence = on) |
| Effect | Force-enable icon display regardless of auto-detection |
| Conflicts | `/Icons-` — first wins on command line; if both present, first takes effect |
| Default | Not present → defer to RCDIR env var, then auto-detect |

### `/Icons-`

| Property | Value |
|----------|-------|
| Switch names | `/Icons-`, `-Icons-`, `--Icons-` |
| Case-sensitive | No |
| Type | Boolean (presence = off) |
| Effect | Force-disable icon display regardless of auto-detection |
| Conflicts | `/Icons` — first wins |
| Default | Not present |

### Priority Cascade

```
1. CLI /Icons or /Icons-       (highest — overrides everything)
2. RCDIR env var Icons/Icons-  (middle)
3. Auto-detection result       (lowest — used when neither CLI nor env specifies)
```

---

## RCDIR Environment Variable Extensions

### Icons/Icons- Switch

Added to the existing `;`-separated `RCDIR` environment variable alongside other switches.

| Entry | Effect |
|-------|--------|
| `Icons` | Enable icon display (equivalent to `/Icons` on CLI) |
| `Icons-` | Disable icon display (equivalent to `/Icons-`) |

**Conflict handling**: If both `Icons` and `Icons-` appear in the same `RCDIR` value, the first one wins. An `ErrorInfo` is recorded for the duplicate.

### Comma Syntax for Icon Overrides

The existing `key=value` override entries in `RCDIR` are extended with an optional icon specifier after a comma:

```
RCDIR=".rs=Yellow,U+E7A8;.py=Green,U+E606"
```

#### Grammar

```
entry       := key "=" value
value       := color_part [ "," icon_part ]
color_part  := <existing color syntax: name, hex, or empty>
icon_part   := <empty> | glyph_literal | "U+" hex_digits

glyph_literal := <single Unicode character>
hex_digits    := /[0-9A-Fa-f]{4,6}/
```

#### Icon Part Semantics

| Icon part | Meaning |
|-----------|---------|
| Absent (no comma) | Icon unchanged from default (backward compatible) |
| Empty (comma but nothing after) | Icon **suppressed** — 2 spaces emitted for alignment (FR-007) |
| Single character | Literal BMP glyph (e.g., `★`) |
| `U+XXXX` to `U+XXXXXX` | Code point (4–6 hex digits, range 0x0001–0x10FFFF, not D800–DFFF) |

#### Examples

```
RCDIR=".rs=Yellow,U+E7A8"       # Yellow color, Rust icon
RCDIR=".rs=,U+E7A8"             # Default color, Rust icon
RCDIR=".rs=Yellow,"             # Yellow color, icon SUPPRESSED
RCDIR=".rs=Yellow"              # Yellow color, icon unchanged (no comma → backward compat)
RCDIR=".py=BrightGreen,U+E606"  # BrightGreen, Python icon
RCDIR="dir:.git=,U+E65D"        # Default color for .git dirs, Seti git icon
RCDIR="attr:H=DarkGray,U+F023"  # DarkGray for hidden files, lock icon

# Error cases:
RCDIR=".rs=Yellow,U+ZZZZ"       # ErrorInfo: invalid hex digits
RCDIR=".rs=Yellow,U+D800"       # ErrorInfo: surrogate range
RCDIR=".rs=Yellow,AB"           # ErrorInfo: multi-char icon value
```

#### Scope

The comma syntax applies to all three override types:

| Override type | Key format | Example |
|--------------|-----------|---------|
| Extension | `.ext` | `.rs=Yellow,U+E7A8` |
| Well-known directory | `dir:name` | `dir:.git=,U+E65D` |
| File attribute | `attr:X` | `attr:H=DarkGray,U+F023` |

---

## Display Behavior

### Normal Mode

```
[icon] [space] [filename]
```

- Icon glyph rendered in the same color as the filename (same `text_attr`)
- One space separator between icon and filename
- If icon suppressed: 2 spaces emitted in place of `[icon] [space]` (FR-007)
- If icons are off: no icon column at all (zero-width, byte-identical to pre-feature output)

### Wide Mode

```
[icon] [space] [filename]
```

- Same as normal mode for the filename portion
- Bracket column suppressed when icons are active (FR-013)
- Cloud status uses NF glyphs instead of Unicode circles when icons active (FR-014)

### Bare Mode

```
[icon] [space] [path]
```

- Same icon behavior, applied to the path output

---

## `/Config` Display

When icons are active, the `/Config` output includes:

1. **Icons status**: "Icons: On (auto-detected)" or "Icons: On (forced)" or "Icons: Off"
2. **Extension icon table**: Extension → icon glyph + code point hex (similar to color table)
3. **Well-known dir icon table**: Dir name → icon glyph + code point hex
4. **Override source annotations**: `[default]` vs `[env]` for each entry

---

## `/Env` Help

The `/Env` help text documents:

1. The `Icons` and `Icons-` switch entries
2. The comma syntax: `key=[color][,icon]`
3. Valid icon formats: literal glyph, `U+XXXX`, empty (suppressed)
4. Examples for extensions, directories, and attributes
