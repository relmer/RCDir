# CLI Contract — RCDir

**Date**: 2026-02-08 | **Version**: 4.2.x

This document defines the exact command-line interface contract. Any change to this contract is a breaking change that violates port fidelity.

---

## Invocation

```
rcdir [drive:][path][filename] [switches...]
```

Arguments and switches may appear in any order.

---

## Switch Syntax

| Prefix | Short | Long | Disable |
|--------|-------|------|---------|
| `/` | `/S` | `/Owner` | `/S-` |
| `-` | `-S` | `--Owner` | `-S-` |

- Both `-` and `/` are valid prefixes
- Long switches with `-` prefix require `--` (double dash): `--owner`, `--env`
- Long switches with `/` prefix use single `/`: `/owner`, `/env`
- Single-dash long switches are errors: `-owner` → usage + exit 1
- Last-used prefix determines help screen formatting

---

## Switches

### Boolean Switches (toggleable with trailing `-`)

| Short | Purpose | Default |
|-------|---------|---------|
| `/S` | Recursive listing (subdirectories) | `false` |
| `/W` | Wide listing format | `false` |
| `/B` | Bare listing format | `false` |
| `/P` | Performance timer | `false` |
| `/M` | Multi-threaded enumeration | `true` |
| `/?` | Show help | — |

### Long Switches

| Switch | Purpose | Default |
|--------|---------|---------|
| `--env` / `/env` | Show RCDIR environment variable help | — |
| `--config` / `/config` | Show current configuration | — |
| `--owner` / `/owner` | Show file owner column | `false` |
| `--streams` / `/streams` | Show alternate data streams | `false` |
| `--debug` / `/debug` | Debug attribute display (debug builds only) | `false` |

### Value Switches

#### `/O` — Sort Order

```
/O[:][-]<key>
```

| Key | Sort By |
|-----|---------|
| `N` | Name (alphabetic, case-insensitive) |
| `E` | Extension (alphabetic, case-insensitive) |
| `S` | Size (smallest first) |
| `D` | Date/time (oldest first) |

- Optional colon: `/OS` and `/O:S` are equivalent
- Reverse: `/O:-S` or `/O-S` (descending)
- Only first key char is used; remainder silently ignored
- Case-insensitive

#### `/A` — Attribute Filter

```
/A[:]<attrs>
```

| Char | Attribute |
|------|-----------|
| `D` | Directory |
| `H` | Hidden |
| `S` | System |
| `R` | Read-only |
| `A` | Archive |
| `T` | Temporary |
| `E` | Encrypted |
| `C` | Compressed |
| `P` | Reparse point |
| `0` | Sparse file |
| `X` | Not content indexed |
| `I` | Integrity stream |
| `B` | No scrub data |
| `O` | Cloud-only (composite: OFFLINE \| RECALL_ON_OPEN \| RECALL_ON_DATA_ACCESS) |
| `L` | Locally available (unpinned) |
| `V` | Always locally available (pinned) |
| `-` | Exclude next character |

- Required: `(attrs & required) == required`
- Excluded: `(attrs & excluded) == 0`
- `-` applies to next char only, then resets to required mode
- Double `-` is an error
- Case-insensitive

#### `/T` — Time Field

```
/T[:]<field>
```

| Field | Meaning |
|-------|---------|
| `C` | Creation time |
| `A` | Last access time |
| `W` | Last write time (default) |

- Optional colon: `/TC` and `/T:C` are equivalent
- Case-insensitive

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Error (invalid arguments, path not found, etc.) |

---

## Error Behavior

- Invalid switch: displays usage screen, exits with code 1
- No per-switch error message (usage only)
- Path not found: `"Error:   {path} does not exist"` (3 spaces after `Error:`)
- Env var errors: displayed at end of output with underline annotation

---

## Default Mask

If no positional arguments provided, default mask is `*` (all files in CWD).

---

## Environment Variable

**Name**: `RCDIR`

**Grammar**: See spec A.5. Semicolon-separated entries. Each entry is either:
- A switch name (with optional `-` to disable): `W`, `S`, `P`, `M`, `B`, `Owner`, `Streams`, `W-`, `S-`
- A color override: `key=ColorSpec` where ColorSpec is `ColorName [on ColorName]`

**No switch prefixes** (`/`, `-`, `--`) are allowed in the env var.
