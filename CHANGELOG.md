# Changelog

All notable changes to RCDir are documented in this file.

## [Unreleased]

## [5.2.1398] - 2026-03-28

### Added

- `--set-aliases`: interactive TUI wizard for configuring PowerShell aliases
  - Root alias name (1-4 chars, default `d`) with derived sub-aliases (`dt`, `dw`, `dd`, `ds`, `dsb`)
  - Sub-alias checkbox selection (tree view, wide, directories-only, recursive, recursive bare)
  - Profile location radio selection (4 PS profile scopes + session-only)
  - Alias block preview with confirmation before writing
  - Conflict detection against existing PowerShell commands/aliases
  - Auto-detection of calling PowerShell version (7+ or 5.1) via parent process inspection
  - AllUsers profile paths resolved from `$PSHOME` (PS install directory)
  - CurrentUser profile paths resolved via `SHGetKnownFolderPath` with OneDrive KFM support
  - Spinner animation during profile scanning
  - Timestamped `.bak` backups before modifying profile files
- `--get-aliases`: non-interactive display of all configured rcdir aliases across profile files
- `--remove-aliases`: interactive checkbox-based removal from one or more profiles (opt-in selection)
- `--whatif`: dry-run modifier for `--set-aliases` and `--remove-aliases` (preview without file changes)
- TUI widget infrastructure: text input, checkbox list, radio button list, confirmation prompt
  - Multi-line label support with blank-line separators
  - In-place re-rendering via cursor movement (no screen flicker)
  - RAII console mode/cursor restore on Ctrl+C or Escape
- 392 unit tests (up from 288)

### Changed

- Minor version bump from 5.1 to 5.2

## [5.1.1132] - 2026-02-28

### Added

- `/Tree` switch: hierarchical directory tree view with Unicode box-drawing connectors (`├──`, `└──`, `│`)
- `/Depth=N` switch: limit tree recursion depth (e.g., `/Depth=2` shows two levels)
- `/TreeIndent=N` switch: configurable indent width per tree level (1–8, default 4)
- `/Size=Auto` switch: Explorer-style abbreviated file sizes (e.g., `8.90 KB`, `1.00 MB`, `2.38 GB`) with fixed 7-character width — default in tree mode
- `/Size=Bytes` switch: explicit opt-in for exact comma-separated sizes (existing default for non-tree modes)
- Tree connector color (`TreeConnector`) configurable via `RCDIR` environment variable
- RCDIR env var support for `Tree`, `Tree-`, `Depth=N`, `TreeIndent=N`, `Size=Auto`, `Size=Bytes`
- Thread-safe empty subdirectory pruning when file masks are active (producer-side upward propagation via parent back-pointers and condition variables)
- Reparse-point cycle guard: junction/symlink directories are listed but not expanded, preventing infinite loops in both `/S` and `/Tree` modes
- Interleaved sort in tree mode: directories and files sorted together (not grouped)
- Per-directory summary at each tree level plus grand total at end
- Comprehensive test suite: 314 tests covering tree connectors, depth limiting, pruning, streams, icons, reparse points, and column alignment

### Changed

- Minor version bump from 5.0 to 5.1

### Incompatibilities

- `/Tree` cannot be combined with `/W` (wide), `/B` (bare), `/S` (recurse), or `/Owner`
- `/Size=Bytes` cannot be used with `/Tree` (tree mode requires fixed-width sizes)
- `/Depth` and `/TreeIndent` require `/Tree`

## [5.0.1131] - 2026-02-27

### Added

- Application icon and VERSIONINFO resource block for Windows file properties
- WinGet package manifest (`relmer.RCDir`) and automated publish step

### Changed

- Statically link the C runtime (eliminates vcruntime140.dll dependency)

### Fixed

- WinGet workflow: install wingetcreate before publish step
- WinGet workflow: use `env` context for PAT secret in step conditions

## [5.0.1129] - 2026-02-19

Minor fixes.

### Fixed

- `/O:d-` (and similar trailing characters after the sort key) now correctly produces an error instead of being silently ignored
- Mapped network drives could resolve UNC path information incorrectly
- Cloud status help section now shows Nerd Font glyphs when a Nerd Font is active

## [5.0] - 2026-02-17

Added Nerd Fonts icon support for file extensions and well-known directories.  Port of initial TCDir implementation of this feature.

### Added

#### Core Listing

- Initial release — complete Rust port of TCDir with full feature parity.
- Colorized directory listing with headers and footers
- File size, date/time, and attribute columns
- Sort options: name, extension, size, date (`/O` switch)
- Attribute filtering (`/A:` switch with all standard + cloud attributes)
- Mask grouping: multiple patterns targeting the same directory grouped and deduplicated
- Page-at-a-time display (`/P` switch)

#### Recursive Listing

- Recursive directory listing (`/S` switch)
- Multi-threaded producer-consumer architecture with streaming output
- Single-threaded fallback for single-file mask patterns

#### Display Modes

- Wide listing mode (`/W` switch) with dynamic column layout
- Bare listing mode (`/B` switch) with full paths

#### Cloud Status

- Cloud sync status visualization for OneDrive, iCloud, etc.
- Status symbols: ○ (cloud-only), ◐ (local), ● (pinned/always available)
- Cloud attribute filters: `/A:O` (cloud-only), `/A:L` (local), `/A:V` (pinned)

#### File Metadata

- File owner display (`/Q` switch) via `GetNamedSecurityInfoW`/`LookupAccountSidW`
- Alternate data streams display (`/R` switch) via `FindFirstStreamW`/`FindNextStreamW`

#### Nerd Font Icons

- ~187 file extension icon mappings aligned with Terminal-Icons default theme
- ~65 well-known directory icon mappings (Terminal-Icons aligned with intentional deviations)
- Auto-detection of Nerd Font availability via system font enumeration
- WezTerm auto-detection (bundles Nerd Font symbols natively)
- ConPTY terminal detection (Windows Terminal, VS Code, etc.)
- `/Icons` and `/Icons-` CLI switches for manual icon override
- `RCDIR=Icons` / `RCDIR=Icons-` env var for persistent icon preference
- `RCDIR=.ext=Color,U+XXXX` icon override syntax for per-extension custom icons
- Cloud status Nerd Font glyph upgrade (nf-md glyphs when NF available)
- Icon colors inherit file type colors from extension color configuration
- Icon display in `/Config` and `/Env` diagnostics

#### Configuration

- `RCDIR` environment variable for persistent configuration
- Extension color customization (e.g., `RCDIR=.rs=Cyan`)
- Background color support (e.g., `RCDIR=.log=White on Blue`)
- Well-known directory color overrides (e.g., `RCDIR=dir:node_modules=DarkGray`)
- Configuration display (`/Config` switch) with color table, icon status, and env var diagnostics
- Color validation: invalid background colors reject entire entry; same fg/bg rejected as unreadable
- Foreground colors matching terminal background auto-corrected with contrasting background

#### Help and Diagnostics

- Colorized help output (`/?` switch)
- Environment variable help (`/Env` switch) with syntax reference and active overrides
- Performance timing (`/T` switch)

#### Build and CI

- Dual-target builds: x86_64-pc-windows-msvc and aarch64-pc-windows-msvc
- Auto-increment build version via `build.rs` and `Version.toml`
- Release profile: LTO, single codegen unit, symbol stripping
- CI/CD pipeline with GitHub Actions (build, clippy, test, artifact upload)
- PowerShell build, test, and deploy scripts
