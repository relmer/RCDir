# Data Model: Symlink & Junction Target Display

## Entities

### ReparseDataBuffer (manual struct definition)

Mirrors the Windows `REPARSE_DATA_BUFFER` layout. Not available in the `windows` crate; defined manually.

```rust
#[repr(C)]
struct ReparseDataBufferHeader {
    reparse_tag:         u32,    // IO_REPARSE_TAG_MOUNT_POINT, SYMLINK, or APPEXECLINK
    reparse_data_length: u16,    // Length of data after this header
    reserved:            u16,    // Must be 0
}
```

**Junction / Mount Point buffer** (after header):
```rust
#[repr(C)]
struct MountPointReparseBuffer {
    substitute_name_offset: u16,
    substitute_name_length: u16,
    print_name_offset:      u16,
    print_name_length:      u16,
    // Followed by PathBuffer: [u16; N] containing both strings
}
```

**Symlink buffer** (after header):
```rust
#[repr(C)]
struct SymbolicLinkReparseBuffer {
    substitute_name_offset: u16,
    substitute_name_length: u16,
    print_name_offset:      u16,
    print_name_length:      u16,
    flags:                  u32,   // SYMLINK_FLAG_RELATIVE = 0x00000001
    // Followed by PathBuffer: [u16; N] containing both strings
}
```

**AppExecLink buffer** (after header):
```
[u32 version]              // Must be 3
[NUL-terminated UTF-16]    // Package family name
[NUL-terminated UTF-16]    // App user model ID
[NUL-terminated UTF-16]    // Target executable path ← displayed
```

### FileInfo (existing, modified)

| Field | Type | Change | Description |
|-------|------|--------|-------------|
| `reparse_tag` | `u32` | Existing | Already populated from `WIN32_FIND_DATA.dwReserved0` |
| `reparse_target` | `String` | **NEW** | Resolved target path; empty if not a supported reparse point or resolution failed |

### Supported Reparse Tags

| Constant | Value | Type | Target Source |
|----------|-------|------|---------------|
| `IO_REPARSE_TAG_MOUNT_POINT` | `0xA000_0003` | Junction | PrintName (preferred) or SubstituteName with `\??\` stripped |
| `IO_REPARSE_TAG_SYMLINK` | `0xA000_000C` | Symlink | PrintName (preferred) or SubstituteName (strip `\??\` for absolute only) |
| `IO_REPARSE_TAG_APPEXECLINK` | `0x8000_001B` | App alias | Third NUL-terminated string in version-3 buffer |

### Display Format

```
{filename} → {target_path}
         ^   ^
         |   └─ filename's own color attribute
         └─ Information color attribute (U+2192 RIGHTWARDS ARROW)
```

- Exactly one space before and after the arrow
- Arrow character: `→` (U+2192)
- Only displayed in normal mode and tree mode
- NOT displayed in wide mode or bare mode

## Relationships

```
FileInfo (1) ──has──> (0..1) reparse_target: String
  │
  ├── reparse_tag determines which parser to invoke
  ├── FILE_ATTRIBUTE_REPARSE_POINT flag gates resolution
  └── Empty string = not resolved / not applicable / error

ReparseResolver
  ├── resolve_reparse_target()  ──calls──> CreateFileW + DeviceIoControl
  ├── parse_junction_buffer()   ──reads──> MountPointReparseBuffer
  ├── parse_symlink_buffer()    ──reads──> SymbolicLinkReparseBuffer
  ├── parse_app_exec_link_buffer() ──reads──> GenericReparseBuffer
  └── strip_device_prefix()     ──strips──> "\??\" prefix
```

## Validation Rules

- `reparse_tag` must match one of the three supported tags before attempting resolution
- `FILE_ATTRIBUTE_REPARSE_POINT` attribute must be set (early exit otherwise)
- Buffer size must be ≥ header size (8 bytes) before parsing
- PrintName/SubstituteName offsets + lengths must not exceed buffer bounds
- AppExecLink version must be 3; reject other versions silently
- NUL terminators must be present within remaining buffer for AppExecLink strings

## State Transitions

N/A — no state machine. Resolution is a one-shot operation during enumeration.
