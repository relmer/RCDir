# Research: Symlink & Junction Target Display

## Decision 1: Reparse Buffer Parsing Approach

**Decision**: Define `REPARSE_DATA_BUFFER` structure manually in Rust; do not depend on WDK or undocumented crate bindings.

**Rationale**: The `REPARSE_DATA_BUFFER` struct is not exposed by the `windows` crate (it comes from the WDK/ntifs.h). TCDir defines it manually in C++. The structure is stable and well-documented in MSDN. Manual definition avoids a WDK dependency and matches the proven TCDir approach.

**Alternatives considered**:
- `windows` crate WDK bindings — not available in the standard `windows` crate; would require `windows-sys` or WDK feature flags that add complexity
- `std::fs::read_link()` — only works for symlinks, not junctions or AppExecLinks; also resolves the target (we want the raw stored path)
- Third-party crate (e.g., `junction`) — adds dependency; doesn't cover AppExecLinks; doesn't give raw buffer access

## Decision 2: Target Resolution Timing

**Decision**: Resolve reparse targets at enumeration time (in `add_match_to_list`), not at display time.

**Rationale**: This matches TCDir's architecture and keeps the display layer I/O-free. The resolver opens a file handle, which is a blocking Win32 call — doing this during display would interleave I/O with console output. Resolving during enumeration means:
- Multi-threaded lister: each worker resolves targets for its own files (no contention)
- Display code remains pure formatting (no fallible I/O)
- Target string is readily available when needed

**Alternatives considered**:
- Display-time resolution — would require error handling in display code and interleave I/O with output; rejected
- Lazy resolution (resolve on first access) — unnecessary complexity; reparse points are rare enough that eager resolution has negligible cost

## Decision 3: Win32 API Call Sequence

**Decision**: Use `CreateFileW` → `DeviceIoControl(FSCTL_GET_REPARSE_POINT)` → parse buffer.

**Rationale**: This is the standard Windows API for reading reparse data. The `CreateFileW` call uses:
- `dwDesiredAccess = 0` — no read/write access needed, just FSCTL
- `FILE_FLAG_OPEN_REPARSE_POINT` — open the link itself, not follow it
- `FILE_FLAG_BACKUP_SEMANTICS` — required for opening directories

The `FSCTL_GET_REPARSE_POINT` ioctl returns the raw reparse data buffer. This is the same approach used by TCDir and is well-tested.

**Alternatives considered**:
- `GetFinalPathNameByHandle` — resolves to final target but loses relative path information; not suitable for FR-004 (display paths as-stored)
- `NtQueryInformationFile` — undocumented/semi-documented NT API; unnecessary when FSCTL works

## Decision 4: Buffer Allocation Strategy

**Decision**: Stack-allocate a 16KB buffer (`MAXIMUM_REPARSE_DATA_BUFFER_SIZE = 16384`).

**Rationale**: Reparse data buffers are at most 16KB per NTFS specification. Stack allocation:
- Avoids heap allocation per reparse point
- Buffer is used only within the resolver function scope
- 16KB is safe for stack usage (typical thread stack is 1–8MB)
- Matches TCDir's approach exactly

**Alternatives considered**:
- Heap-allocated `Vec<u8>` — unnecessary allocation; rejected
- Smaller initial buffer with retry — reparse data is always ≤16KB so retry logic is pointless

## Decision 5: PrintName vs SubstituteName Preference

**Decision**: Prefer `PrintName` field; fall back to `SubstituteName` with `\??\` prefix stripping.

**Rationale**: 
- `PrintName` is the user-friendly display name (no device prefix, clean path)
- `SubstituteName` is the NT-internal name, often prefixed with `\??\`
- Most reparse points have a non-empty PrintName, but some (especially older junctions created by certain tools) may only have SubstituteName
- The `\??\` prefix must be stripped from SubstituteName to produce a user-readable path
- For relative symlinks (SYMLINK_FLAG_RELATIVE = 0x1), do NOT strip prefix from SubstituteName

**Alternatives considered**:
- Always use SubstituteName — would require stripping logic for all entries; PrintName is cleaner when available
- Always use PrintName, fail if empty — too fragile; SubstituteName fallback handles edge cases

## Decision 6: AppExecLink Buffer Format

**Decision**: Parse AppExecLink as version-3 generic reparse buffer with three NUL-terminated wide strings.

**Rationale**: AppExecLink reparse points (IO_REPARSE_TAG_APPEXECLINK = 0x8000001B) use a non-standard buffer format:
- First 4 bytes: version ULONG (must be 3)
- Followed by three NUL-terminated UTF-16 strings:
  1. Package family name
  2. App user model ID  
  3. Target executable path (this is what we display)

Version check is critical — only version 3 is documented/supported. Return empty string for other versions.

**Alternatives considered**:
- Resolve via `SHGetKnownFolderPath` + registry — overly complex; the target exe is right there in the buffer
- Skip AppExecLink support — leaves `python.exe`, `winget.exe`, etc. in WindowsApps directory without targets; poor UX

## Decision 7: Color Scheme

**Decision**: Arrow (`→`) uses `Information` attribute; target path uses the same color as the source filename.

**Rationale**: This matches TCDir's implementation (FR-006, FR-007). The Information color (typically cyan) provides visual separation between the filename and target. Using the filename's own color for the target maintains visual grouping — the arrow connects the name to its destination.

**Alternatives considered**:
- Extension-based color for file symlink targets — was in early TCDir spec (FR-008) but superseded by FR-007; simpler and more consistent to use the source filename color
- Default/white color for target — loses visual connection between link and destination

## Decision 8: Error Handling Philosophy

**Decision**: All failures return empty string (no target displayed). No error messages, no stderr output.

**Rationale**: Per FR-011, graceful degradation is required. If reparse data can't be read (access denied, corrupted data, unsupported format), the file still displays normally — just without the `→ target` suffix. This is a cosmetic enhancement, not a critical feature, so silent failure is appropriate.

**Alternatives considered**:
- Display `→ <error>` or `→ ???` — adds noise; user can't act on it anyway
- Log to stderr on failure — violates FR-011; clutters output for non-actionable errors

## Decision 9: Module Structure

**Decision**: Single module `src/reparse_resolver.rs` with public pure-parsing functions and one public I/O function.

**Rationale**: Matches the single-responsibility principle. The module exposes:
- `resolve_reparse_target(dir_path, file_info) -> String` — the I/O function (opens file, reads buffer, dispatches to parser)
- `parse_junction_buffer(buffer) -> String` — pure function, testable
- `parse_symlink_buffer(buffer) -> String` — pure function, testable  
- `parse_app_exec_link_buffer(buffer) -> String` — pure function, testable
- `strip_device_prefix(path) -> String` — pure function, testable

Pure functions enable comprehensive unit testing with synthetic byte arrays — no filesystem mocking needed.

**Alternatives considered**:
- Embed in `file_info.rs` — too many concerns; file_info is a data struct, not a resolver
- Separate module per reparse type — over-engineering for ~200 lines of code total
