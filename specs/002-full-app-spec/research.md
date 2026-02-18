# Phase 0: Research — RCDir (C++ to Rust Port)

**Date**: 2026-02-08 | **Branch**: `002-full-app-spec`

---

## R-01: CLI Argument Parser

**Decision**: Custom parser (no `clap`)

**Rationale**: TCDir uses Windows-style `/` switch prefix, compound switches (`/os`, `/a:h-d`, `/t:c`), and specific error behavior that no Rust argument parsing library supports:
- `clap` 4.x hardcodes `-`/`--` prefixes; no `/` support (open issue clap-rs/clap#2468, unresolved)
- Pre-processing args (replacing `/` → `-`) fails because compound values like `/a:hs` and `/osne` require custom character-by-character parsing within the switch
- Error messages must match TCDir exactly (usage screen + exit code 1, no per-switch error text)
- `switch_prefix` tracking (last-used `-` or `/` affects help display) is a custom semantic

The TCDir parser is ~200 lines of actual logic — a custom Rust parser maps 1:1 from the C++ code, is fully testable, and gives complete control over edge cases.

**Alternatives Considered**:
- `clap` (derive or builder) — rejected: no `/` prefix, no compound switch semantics
- `pico-args`, `lexopt`, `argh` — rejected: same fundamental prefix limitation
- Pre-processing + `clap` — rejected: can't model `/a:h-d` or `/o:-s` semantics through generic parser

**Key Implementation Notes**:
- Long switch detection: length ≥ 3 AND second char is not `:` or `-` → routes to long handler
- Single-dash long switch rejection: `-env` → error, `--env` and `/env` → valid
- `/osne` → parsed as long switch (length 4, second char `s`) → error (no match)
- `/os` → single-char path, `OrderByHandler("s")` → sort by size
- Attribute `-` scope resets after each character: `/a:h-ds` = require H, exclude D, require S
- Sort handler reads only first char after optional `:`/`-`; remainder silently ignored
- On parse error: display usage (not specific error message) + exit code 1

---

## R-02: Windows Crate Selection

**Decision**: `windows` crate v0.62.x (not `windows-sys`)

**Rationale**: The `windows` crate provides ergonomic Rust wrappers with `Result<()>` returns, `PCWSTR` param traits, newtype wrappers with `Debug`/`PartialEq`, and bitwise ops on flag types. The compile-time cost (~2x vs `windows-sys`) is acceptable for a project of this size.

**Alternatives Considered**:
- `windows-sys` — rejected: raw integer types, manual `BOOL` checking, no convenience wrappers. Only beneficial when compile time is critical.
- Direct FFI — rejected: unnecessary when official crate exists

**Feature Flags Required**:
```toml
[dependencies.windows]
version = "0.62"
features = [
    "Win32_Storage_FileSystem",
    "Win32_Storage_CloudFilters",
    "Win32_System_Console",
    "Win32_System_Environment",
    "Win32_System_IO",
    "Win32_System_Time",
    "Win32_Security",
    "Win32_Security_Authorization",
    "Win32_Globalization",
    "Win32_NetworkManagement_WNet",
]
```

**Critical API Groups**:
| Group | Key APIs |
|-------|---------|
| Filesystem | `FindFirstFileW`, `FindNextFileW`, `FindClose`, `GetVolumeInformationW`, `GetDiskFreeSpaceExW`, `GetDriveTypeW` |
| Streams | `FindFirstStreamW`, `FindNextStreamW` |
| Console | `GetStdHandle`, `WriteConsoleW`, `GetConsoleScreenBufferInfo`, `GetConsoleMode`, `SetConsoleMode` |
| I/O | `WriteFile` (redirected output fallback) |
| Cloud | `CfGetPlaceholderStateFromAttributeTag`, `CfGetSyncRootInfoByPath` |
| Security | `GetNamedSecurityInfoW`, `LookupAccountSidW` |
| Date/Time | `GetDateFormatEx`, `GetTimeFormatEx`, `FileTimeToSystemTime`, `SystemTimeToTzSpecificLocalTime` |
| Network | `WNetGetConnectionW` |

**HANDLE Safety**: Use RAII wrapper structs with `Drop`:
- `FindHandle` → `FindClose` (for FindFirst/FindNext handles)
- `SafeHandle` → `CloseHandle` (for console/file handles)
- Critical: these are NOT interchangeable handle types

---

## R-03: Console Output Strategy

**Decision**: Single large buffer + ANSI VT sequences + single flush

**Rationale**: TCDir's proven pattern — all output accumulates in a pre-allocated 10 MB `wstring`, color changes are ANSI SGR sequences inline in the buffer, and the entire buffer is flushed in one `WriteConsoleW` call. This minimizes system calls and is the primary performance optimization.

**Alternatives Considered**:
- `crossterm` crate — rejected: adds abstraction layer over what is essentially simple ANSI sequence generation; TCDir's pattern is straightforward to implement directly
- Per-line I/O — rejected: massive performance penalty from syscall overhead
- `SetConsoleTextAttribute` for colors — rejected: TCDir already uses ANSI VT sequences exclusively

**Key Implementation Details**:
1. **Buffer**: `String::with_capacity(10_485_760)` (~10 MB pre-allocated)
2. **Color output**: ANSI SGR sequences written inline: `\x1b[{fg};{bg}m`
3. **Color elision**: Track `prev_attr: Option<u16>`; skip ANSI emission if unchanged
4. **Redirected detection**: `GetConsoleMode` success → real console; failure → redirected
5. **Dual output path**:
   - Console: `WriteConsoleW` with UTF-16 (the buffer is built as UTF-8/ANSI-compatible, but `WriteConsoleW` needs UTF-16 conversion)
   - Redirected: `WriteFile` with UTF-8 encoding
6. **Console width**: `GetConsoleScreenBufferInfo` for real console; default 80 for redirected
7. **Color markers**: `{Error}`, `{Date}`, etc. resolved at write time via lookup table
8. **Reset**: Append `\x1b[0m` before final flush to restore terminal defaults

**Color Model**:
- Internal: Windows WORD (4-bit fg + 4-bit bg, 16 colors)
- Output: ANSI SGR codes via lookup table (Windows index → ANSI code, +60 for bright)

---

## R-04: Parallel Directory Enumeration

**Decision**: Direct thread pool with work queue + per-node `Mutex`/`Condvar` (no `rayon`)

**Rationale**: TCDir uses a **streaming producer/consumer model** where the main thread (consumer) prints directory results as soon as each individual node completes, walking the tree depth-first concurrently with worker threads (producers). This is user-observable behavior — on large recursive listings, output appears progressively.

`rayon::scope_fifo` is **incompatible** with this model because:
- `scope_fifo` blocks the calling thread until ALL spawned tasks complete
- The consumer cannot start printing until the entire tree is enumerated
- This would produce a visible behavioral difference: no output until everything is done
- Wrapping the scope in a background thread doesn't help because rayon's task spawning semantics don't integrate with per-node condition variables

The correct Rust architecture is a direct translation of TCDir's model:

**Alternatives Considered**:
- `rayon::scope_fifo` — rejected: blocks until all work completes, loses streaming output
- `rayon::spawn` (unscoped) — rejected: no lifetime guarantees, can't reference stack data
- `rayon` in background thread + per-node Condvar — rejected: awkward hybrid, rayon adds no value when we need manual control over the queue and signaling
- `tokio` async — rejected: CPU-bound filesystem work, async adds complexity without benefit
- `crossbeam` work-stealing deque — viable but unnecessary; TCDir's simple FIFO queue has no contention issues in practice

**Architecture (mirrors TCDir exactly)**:
```
┌─────────────────────────────────────────────────────────┐
│                    Main Thread                          │
│                                                         │
│  1. Create root DirectoryInfo                           │
│  2. Enqueue root onto work queue                        │
│  3. Spawn N worker threads                              │
│  4. PrintDirectoryTree (recursive depth-first walk):    │
│     ├── WaitForNodeCompletion(node) ← blocks on CV     │
│     ├── SortResults(node)                               │
│     ├── DisplayResults(node)         ← prints immediately│
│     ├── AccumulateTotals(node)                          │
│     └── for child in node.children:                     │
│         └── PrintDirectoryTree(child) ← recurse         │
│  5. PrintRecursiveSummary                               │
│  6. StopWorkers → join                                  │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│                 Worker Thread (×N)                       │
│                                                         │
│  loop:                                                  │
│    item = queue.pop()   ← blocks until work or shutdown │
│    if shutdown: break                                   │
│    node = item.dir_info                                 │
│    lock(node.mutex) → set status = InProgress           │
│    EnumerateMatchingFiles(node)                         │
│    EnumerateSubdirectories(node):                       │
│      for each subdir found:                             │
│        create child DirectoryInfo                       │
│        lock(node.mutex) → add to node.children          │
│        queue.push(child)                                │
│    lock(node.mutex) → set status = Done/Error           │
│    node.condvar.notify_one()  ← wakes consumer          │
└─────────────────────────────────────────────────────────┘
```

**Key Implementation Details**:

| Component | Rust Implementation |
|-----------|-------------------|
| Work queue | `Mutex<VecDeque<WorkItem>>` + `Condvar` (or `crossbeam-channel`) |
| Per-node sync | `Mutex<DirNodeState>` + `Condvar` on each `DirectoryInfo` |
| Worker threads | `std::thread::spawn` × `available_parallelism()` |
| Stop signal | `AtomicBool` shared flag (replaces `stop_source`/`stop_token`) |
| Thread join | `JoinHandle<()>` stored in vec, joined on cleanup |
| Tree walking | Recursive function on main thread, blocks on per-node `Condvar` |

**Key Mapping from C++ to Rust**:
| C++ (TCDir) | Rust (RCDir) |
|---|---|
| `CWorkQueue<WorkItem>` | `WorkQueue` struct with `Mutex<VecDeque>` + `Condvar` |
| `jthread` + `stop_source` | `std::thread::spawn` + `Arc<AtomicBool>` |
| `CDirectoryInfo::m_mutex` | `Mutex<DirNodeInner>` on each node |
| `CDirectoryInfo::m_cvStatusChanged` | `Condvar` on each node |
| `WorkerThreadFunc` loop | Worker closure: `loop { queue.pop()...; enumerate(); notify(); }` |
| `EnqueueChildDirectory` | `lock(parent) → push child; queue.push(child)` |
| `WaitForNodeCompletion` | `condvar.wait_while(lock, \|state\| state.status < Done)` |
| `PrintDirectoryTree` recursive | `print_directory_tree()` recursive, same structure |
| `StopWorkers` | Set `AtomicBool`, signal queue done, join all handles |

**Why not `rayon`**: The fundamental issue is that rayon is designed for **fork-join parallelism** (divide work → compute in parallel → join results). TCDir's model is **pipeline parallelism** (producers and consumer run concurrently, communicating via per-node signals). These are different paradigms. Trying to use rayon here would either lose the streaming behavior or require such an awkward hybrid that it adds complexity without benefit.

**Dependency change**: This removes `rayon` from the dependency list. The only external crates needed are `windows` and `widestring`.

---

## R-05: Number Formatting

**Decision**: Evaluate `num-format` vs `std::fmt` with manual locale handling

**Rationale**: TCDir uses `std::format(locale(""), L"{:L}", number)` for locale-aware thousands separators. Rust's `std::fmt` does not support locale-aware number formatting natively so we need a strategy.

**Options**:
1. **`num-format` crate**: Provides locale-aware number formatting. Simple API. May not match Windows locale exactly.
2. **Win32 `GetNumberFormatEx`**: The definitive way to match Windows locale behavior. Available via `windows` crate under `Win32_Globalization`. Ensures byte-identical output with TCDir.
3. **Manual formatting**: Read locale settings from Windows, implement thousands separator insertion manually.

**Decision**: Use Win32 `GetNumberFormatEx` for byte-identical locale-aware formatting. This guarantees output matches TCDir exactly since both use the same Windows APIs.

**Alternatives Considered**:
- `num-format` — backup option if `GetNumberFormatEx` proves too cumbersome
- Manual comma insertion — rejected: doesn't respect user locale (some locales use `.` or ` ` as separator)

---

## R-06: Date/Time Formatting

**Decision**: Use Win32 `GetDateFormatEx` / `GetTimeFormatEx` (same as TCDir)

**Rationale**: For byte-identical output, the same date/time formatting APIs must be used. TCDir calls `GetDateFormatEx(LOCALE_NAME_USER_DEFAULT, DATE_SHORTDATE, ...)` and `GetTimeFormatEx(LOCALE_NAME_USER_DEFAULT, TIME_NOSECONDS, ...)`. Using `chrono` or Rust's standard library would produce different formatting for different locales.

**Alternatives Considered**:
- `chrono` crate — rejected: doesn't call Windows locale APIs, would produce different formats
- `time` crate — rejected: same issue
- Manual FILETIME conversion + format — rejected: reinventing what the Win32 API already does

**Implementation**: `FILETIME` → `FileTimeToSystemTime` → `SystemTimeToTzSpecificLocalTime` → `GetDateFormatEx`/`GetTimeFormatEx`

---

## R-07: String Handling (UTF-16 Interop)

**Decision**: `widestring` crate for UTF-16 conversion at Win32 API boundaries

**Rationale**: Windows APIs operate on UTF-16 (`*W` functions). Rust strings are UTF-8. Need efficient conversion at API boundaries. The `widestring` crate provides `U16CString`, `U16String`, and conversion utilities optimized for Windows interop.

**Key Patterns**:
- Win32 filename output (`WIN32_FIND_DATAW.cFileName` is `[u16; 260]`) → `OsString::from_wide()` or `widestring::U16CStr`
- Path arguments to Win32 APIs → `U16CString::from_os_str(&path)`
- Internal string operations (sorting, display) → UTF-8 `String`
- Console output buffer → UTF-8 `String` with ANSI codes, converted to UTF-16 only for `WriteConsoleW`

**Alternatives Considered**:
- `std::os::windows::ffi::OsStringExt` only — rejected: doesn't provide null-terminated wide strings needed for PCWSTR params
- `windows` crate `HSTRING` — viable but heavier; `widestring` is more focused

---

## R-08: Sort Comparison

**Decision**: `lstrcmpiW` via `windows` crate for name/extension, `CompareFileTime` for dates

**Rationale**: TCDir uses `lstrcmpiW` for case-insensitive locale-aware string comparison. Using Rust's `.to_lowercase()` or `.cmp()` would produce different sort orders for non-ASCII characters. Byte-identical sorting requires the same comparison function.

**Implementation**:
- Implement `Ord` on `FileInfo` with configurable sort key
- `sort_by()` closure that dispatches based on `CommandLine.sort_order`
- Directories always sort before files (partition, then sort each group)
- Tiebreaker chain: primary → name → date → extension → size

---

## R-09: Error Handling Pattern

**Decision**: Custom `AppError` enum implementing `std::error::Error`, with `Result<T, AppError>` throughout

**Rationale**: TCDir uses `HRESULT` + EHM macros (`CHR`, `CBR`, `CWRA`). The Rust equivalent is `Result<T, E>` with `?` operator. A custom error type allows mapping Win32 errors, I/O errors, and parse errors into a unified type.

**Structure**:
```rust
enum AppError {
    Win32(windows::core::Error),
    Io(std::io::Error),
    InvalidArg(String),
    PathNotFound(PathBuf),
}
```

**Exit Code Mapping**: Any `Err` → exit code 1 (matching TCDir's HRESULT failure → exit 1 behavior)

---

## R-10: Performance Timer

**Decision**: `std::time::Instant` (wraps `QueryPerformanceCounter` on Windows)

**Rationale**: Rust's `Instant::now()` and `elapsed()` use `QueryPerformanceCounter`/`QueryPerformanceFrequency` on Windows, which is exactly what TCDir uses. No need for direct Win32 calls.

**Output Format**: `"RCDir time elapsed:  {:.2} msec\n"` (two spaces after colon, 2 decimal places)

---

## R-11: Cloud Files API Considerations

**Decision**: Use `CfGetPlaceholderStateFromAttributeTag` (not `CfGetPlaceholderStateFromFindData`)

**Rationale**: `CfGetPlaceholderStateFromFindData` takes `WIN32_FIND_DATAA` (ANSI struct). TCDir casts the `WIN32_FIND_DATAW` to `A`, which works because the attribute/reparse-tag fields are at the same offset. In Rust, it's cleaner to use `CfGetPlaceholderStateFromAttributeTag` directly, passing `dwFileAttributes` and `dwReserved0` from the `WIN32_FIND_DATAW`.

**Sync Root Detection**: `CfGetSyncRootInfoByPath` — returns success if path is under a cloud sync root, failure otherwise.

**API Availability**: Cloud Files API requires Windows 10 1709+. The APIs are in `cldapi.dll`.

---

## R-12: crate vs no-crate Decisions Summary

| Functionality | Use Crate? | Crate / Approach |
|---------------|-----------|-----------------|
| CLI parsing | No | Custom parser (~200 lines) |
| Win32 API | Yes | `windows` 0.62.x |
| UTF-16 strings | Yes | `widestring` |
| Parallelism | No | `std::thread` + `Mutex`/`Condvar` work queue (mirrors TCDir) |
| Number formatting | No | Win32 `GetNumberFormatEx` |
| Date formatting | No | Win32 `GetDateFormatEx`/`GetTimeFormatEx` |
| Console colors | No | Direct ANSI SGR (no `crossterm`) |
| Sorting | No | `lstrcmpiW` via `windows` crate |
| Error handling | No | Custom `AppError` enum |
| Performance timing | No | `std::time::Instant` |
| Env var parsing | No | Custom parser (matches TCDir's `CConfig::Parse`) |

**Total external crates**: 2 (`windows`, `widestring`)
