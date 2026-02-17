# Implementation Plan: Nerd Font File & Folder Icons

**Branch**: `003-file-icons` | **Date**: 2026-02-16 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/003-file-icons/spec.md`

## Summary

Add Nerd Font file and folder icons to RCDir directory listings, matching TCDir's implementation identically. Icons are auto-detected via a layered strategy (WezTerm → conhost GDI canary → system font enumeration), controllable via `/Icons`/`/Icons-` CLI flags and the `RCDIR` environment variable's extended comma syntax (`key=[color][,icon]`). The implementation adds 3 new modules (`icon_mapping`, `nerd_font_detector`, `file_attribute_map`), extends `Config` and `CommandLine` with icon fields, and modifies all 3 displayers to emit icon glyphs before filenames. Cloud status symbols upgrade to NF glyphs when icons are active.

## Implementation Approach

**This feature is a direct port from TCDir's C++ implementation.** For every task, the implementer must examine the corresponding TCDir source file first, understand the algorithm and data flow, then translate it to idiomatic Rust — same logic, same behavior, same edge-case handling. Do not invent new approaches where TCDir already has a working solution.

**Reference source files** (in the `TCDir/TCDirCore/` workspace folder):

| RCDir module | TCDir source | What to port |
|---|---|---|
| `src/icon_mapping.rs` | `IconMapping.h`, `IconMapping.cpp` | NF constants, extension table, well-known dir table, attribute precedence |
| `src/nerd_font_detector.rs` | `NerdFontDetector.h`, `NerdFontDetector.cpp` | Detection enums, 5-step detect chain, GDI canary probe, font enumeration |
| `src/file_attribute_map.rs` | `IconMapping.cpp` (precedence array) | `ATTRIBUTE_PRECEDENCE` order (PSHERC0TA) |
| `src/config.rs` | `Config.h`, `Config.cpp` | `FileDisplayStyle`, icon fields, icon maps, `parse_icon_value`, `get_display_style_for_file`, comma-syntax override pipeline |
| `src/command_line.rs` | `CommandLine.h`, `CommandLine.cpp` | `icons` field, `/Icons`/`/Icons-` parsing, config merge |
| `src/results_displayer.rs` | `ResultsDisplayerNormal.cpp`, `ResultsDisplayerWide.cpp`, `ResultsDisplayerBare.cpp` | Icon emission, column width adjustment, bracket suppression, cloud NF glyphs |
| `src/cloud_status.rs` | `Config.cpp` (`GetCloudStatusIcon`) | `nf_symbol()` method on `CloudStatus` enum |
| `src/usage.rs` | `Usage.cpp` | `/Icons` switch docs, comma syntax docs, config display with icons |
| `src/lib.rs` | `TCDir.cpp` (`CreateDisplayer`) | `resolve_icons()` — CLI → env var → auto-detect priority cascade |

**Key principle**: When in doubt about behavior, check what TCDir does and match it exactly.

## Technical Context

**Language/Version**: Rust stable (edition 2024, toolchain 1.85+)
**Primary Dependencies**: `windows` crate 0.62 (Win32 API), `widestring` 1 (UTF-16)
**Storage**: N/A (all in-memory static tables + runtime hash maps)
**Testing**: `cargo test` — inline `#[cfg(test)]` unit tests + `tests/output_parity.rs` integration
**Target Platform**: Windows 10/11, x64 and ARM64
**Project Type**: Single binary (`rcdir`)
**Performance Goals**: <5% overhead on 1000+ file directories (SC-004)
**Constraints**: Zero regression when icons are off; byte-identical output to pre-feature version (SC-002)
**Scale/Scope**: ~180 extension mappings, ~60 well-known directory mappings, 9 attribute precedence entries

**New Windows API features required** (additions to `Cargo.toml` `windows` features):
- `Win32_Graphics_Gdi` — `CreateCompatibleDC`, `CreateFontW`, `SelectObject`, `GetGlyphIndicesW`, `DeleteObject`, `DeleteDC`, `EnumFontFamiliesExW`, `GetCurrentConsoleFontEx` (for canary probe and font enumeration)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Code Quality | **PASS** | New modules follow existing patterns; `Result<T,E>` throughout; no `unwrap()` in production |
| II. Testing Discipline | **PASS** | Unit tests for icon mapping lookup, env var parsing, detection logic; integration parity tests; `MockEnvironmentProvider` for detection; virtual methods replaced by trait-based injection |
| III. User Experience Consistency | **PASS** | `/Icons`/`/Icons-` mirrors existing switch conventions; RCDIR env var comma syntax extends existing format; colors inherit existing precedence chain |
| IV. Performance Requirements | **PASS** | Static icon tables are compile-time; HashMap lookups are O(1); GDI detection runs once at startup; no per-file allocations in hot path |
| V. Simplicity & Maintainability | **PASS** | 3 new focused modules; icon_mapping is pure data + lookup; nerd_font_detector encapsulates all Win32 GDI behind a trait; no unnecessary abstractions |
| Technology Constraints | **PASS** | Only `windows` crate (already a dependency) gains GDI features; no new external crates |
| Development Workflow | **PASS** | `cargo clippy`, `cargo test`, standard change process |

**No violations to justify.**

## Project Structure

### Documentation (this feature)

```text
specs/003-file-icons/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── cli-contract.md  # Phase 1 output
├── nerd-font-glyphs.md  # Glyph reference (from TCDir)
├── glyph-preview.html   # Visual reference (from TCDir)
├── checklists/
│   └── requirements.md  # Quality checklist
└── tasks.md             # Phase 2 output (NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── icon_mapping.rs          # NEW — NfIcon constants, default tables, HashMap lookups
├── nerd_font_detector.rs    # NEW — Layered NF detection (WezTerm, GDI canary, font enum)
├── file_attribute_map.rs    # NEW — Attribute precedence array (PSHERC0TA order)
├── command_line.rs          # MODIFIED — Add icons: Option<bool> field, /Icons switch parsing
├── config.rs                # MODIFIED — Add icon hash maps, comma-syntax parsing, get_display_style_for_file()
├── results_displayer.rs     # MODIFIED — Icon glyph emission in Normal/Wide/Bare displayers
├── cloud_status.rs          # MODIFIED — NF glyph alternatives for cloud symbols
├── console.rs               # MODIFIED — Helper for writing char → UTF-16 to buffer
├── usage.rs                 # MODIFIED — Document /Icons and /Icons- in help output
├── lib.rs                   # MODIFIED — Add 3 new pub mod declarations, wire detection into run()
└── ...                      # Unchanged files
tests/
└── output_parity.rs         # MODIFIED — Add icon-mode parity test scenarios
```

**Structure Decision**: Single-project Rust binary. New modules follow existing flat `src/` layout. No subdirectories needed — the feature adds 3 focused files and extends 8 existing ones.

## Constitution Re-Check (Post-Design)

*Re-evaluated after Phase 1 design artifacts (data-model.md, contracts/, quickstart.md).*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Code Quality | **PASS** | `FileDisplayStyle`, `FontProber` trait, `parse_icon_value()` all use `Result`; no `unwrap()` in production; idiomatic Rust patterns throughout |
| II. Testing Discipline | **PASS** | Every new module has #[cfg(test)] section planned; MockFontProber for detection; comma-syntax parsing edge cases; integration parity tests |
| III. User Experience Consistency | **PASS** | `/Icons`/`/Icons-` mirrors existing switch conventions; comma syntax extends existing RCDIR format; `FileDisplayStyle` carries same information as TCDir's `SFileDisplayStyle` |
| IV. Performance Requirements | **PASS** | O(1) HashMap lookups; zero per-file allocations; one-time GDI startup cost; char is Copy type pushed directly to buffer |
| V. Simplicity & Maintainability | **PASS** | 3 focused new modules; icon_mapping is pure data; nerd_font_detector is GDI behind a trait; file_attribute_map is a single const array |
| Technology Constraints | **PASS** | Only addition: `Win32_Graphics_Gdi` feature to existing `windows` crate; no new external crates |
| Development Workflow | **PASS** | All quality gates (clippy, test, build) applicable; quickstart includes verification checklist |

**No violations. No complexity tracking needed.**

## Complexity Tracking

No constitution violations — table not needed.
