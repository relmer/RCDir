# Changelog

All notable changes to RCDir are documented in this file.

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
