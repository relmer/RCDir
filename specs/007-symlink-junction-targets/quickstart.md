# Quickstart: Symlink & Junction Target Display

## What This Feature Does

Displays `→ target_path` after symlinks, junctions, and AppExecLink entries in `rcdir` listings (normal and tree modes).

## Key Files

| File | Role |
|------|------|
| `src/reparse_resolver.rs` | **NEW** — Win32 reparse data reading + buffer parsing |
| `src/file_info.rs` | Add `reparse_target: String` field |
| `src/directory_lister.rs` | Call resolver in `add_match_to_list()` |
| `src/multi_threaded_lister.rs` | Same integration as directory_lister |
| `src/results_displayer/normal.rs` | Append `→ target` after filename |
| `src/results_displayer/tree.rs` | Append `→ target` after filename |

## Architecture at a Glance

```
Enumeration (directory_lister / multi_threaded_lister)
  │
  ├── For each file: check FILE_ATTRIBUTE_REPARSE_POINT flag
  │     └── If set: call reparse_resolver::resolve_reparse_target()
  │           ├── CreateFileW (open link itself, not target)
  │           ├── DeviceIoControl(FSCTL_GET_REPARSE_POINT)
  │           └── Dispatch to parse_{junction,symlink,app_exec_link}_buffer()
  │
  └── Store result in file_info.reparse_target (empty string on failure)

Display (results_displayer/normal.rs, tree.rs)
  │
  └── If reparse_target is non-empty:
        ├── Print " → " with Information color
        └── Print target path with filename's color
```

## How to Build & Test

```powershell
cargo check                   # Quick compilation check
cargo test                    # Run all tests including new buffer parsing tests
cargo clippy -- -D warnings   # Lint check
```

## Reparse Tag Constants

```rust
const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;  // Junction
const IO_REPARSE_TAG_SYMLINK:     u32 = 0xA000_000C;  // Symlink
const IO_REPARSE_TAG_APPEXECLINK: u32 = 0x8000_001B;  // App exec alias
```

## Display Output Examples

```
Normal mode:
  04/19/2026  10:00 AM    <DIR>          Projects → C:\Dev\Projects
  04/19/2026  10:00 AM             0     config.yml → ..\shared\config.yml
  04/19/2026  10:00 AM             0     python.exe → C:\Program Files\WindowsApps\...\python3.12.exe

Tree mode:
  ├── Projects → C:\Dev\Projects
  ├── config.yml → ..\shared\config.yml
  └── python.exe → C:\Program Files\WindowsApps\...\python3.12.exe
```

## Testing Strategy

Pure-function buffer parsers tested with synthetic byte arrays:
- `build_junction_buffer(print_name, substitute_name) -> Vec<u8>`
- `build_symlink_buffer(print_name, substitute_name, flags) -> Vec<u8>`
- `build_app_exec_link_buffer(version, pkg_id, app_id, target_exe) -> Vec<u8>`

No filesystem mocking needed — Win32 I/O integration tested manually.
