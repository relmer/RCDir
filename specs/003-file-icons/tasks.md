# Tasks: Nerd Font File & Folder Icons

**Input**: Design documents from `/specs/003-file-icons/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/cli-contract.md, quickstart.md
**Reference Implementation**: TCDir C++ (`TCDirCore/IconMapping.*`, `NerdFontDetector.*`, `FileAttributeMap.h`, `Config.*`, `CommandLine.*`, `ResultsDisplayer*.*`, `Usage.cpp`)

**Organization**: Tasks are grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- All file paths relative to repository root

---

## Phase 1: Setup

**Purpose**: Project scaffolding and dependency configuration

- [X] T001 Add `Win32_Graphics_Gdi` feature to `windows` crate in Cargo.toml
- [X] T002 [P] Add `pub mod icon_mapping` declaration in src/lib.rs
- [X] T003 [P] Add `pub mod nerd_font_detector` declaration in src/lib.rs
- [X] T004 [P] Add `pub mod file_attribute_map` declaration in src/lib.rs

**Checkpoint**: `cargo check` passes with the new feature flag and empty modules

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core data structures and detection logic that ALL user stories depend on. Port directly from TCDir.

**CRITICAL**: No user story work can begin until this phase is complete.

- [X] T005 Create NF named constants in src/icon_mapping.rs — port all `NfIcon` constants from TCDirCore/IconMapping.h (111 `pub const char` values, 8 groups: Custom, Seti, Dev, Fae, Oct, Fa, Md, Cod)
- [X] T006 [P] Create `DEFAULT_EXTENSION_ICONS` static table in src/icon_mapping.rs — port `g_rgDefaultExtensionIcons[]` from TCDirCore/IconMapping.cpp (209 `(&str, char)` entries)
- [X] T007 [P] Create `DEFAULT_WELL_KNOWN_DIR_ICONS` static table in src/icon_mapping.rs — port `g_rgDefaultWellKnownDirIcons[]` from TCDirCore/IconMapping.cpp (60 `(&str, char)` entries)
- [X] T008 Create `ATTRIBUTE_PRECEDENCE` array in src/file_attribute_map.rs — port `g_rgAttributePrecedenceOrder[]` from TCDirCore/IconMapping.cpp (9 entries, PSHERC0TA order)
- [X] T009 Create `IconActivation` and `DetectionResult` enums in src/nerd_font_detector.rs — port `EDetectionResult` from TCDirCore/NerdFontDetector.h
- [X] T010 Create `FontProber` trait in src/nerd_font_detector.rs — port virtual methods from `CNerdFontDetector` (probe_console_font_for_glyph, is_nerd_font_installed)
- [X] T011 Implement `DefaultFontProber` struct in src/nerd_font_detector.rs — port `CNerdFontDetector::ProbeConsoleFontForGlyph()` (GetCurrentConsoleFontEx → CreateCompatibleDC → CreateFontW → SelectObject → GetGlyphIndicesW) and `IsNerdFontInstalled()` (EnumFontFamiliesExW + callback) from TCDirCore/NerdFontDetector.cpp
- [X] T012 Implement `detect()` function in src/nerd_font_detector.rs — port 5-step detection chain from `CNerdFontDetector::Detect()` in TCDirCore/NerdFontDetector.cpp (WezTerm → ConPTY check → canary probe → font enum → fallback)
- [X] T013 Implement `is_wezterm()` and `is_conpty_terminal()` helpers in src/nerd_font_detector.rs — port `IsWezTerm()` and `IsConPtyTerminal()` from TCDirCore/NerdFontDetector.cpp
- [X] T014 Add `FileDisplayStyle` struct to src/config.rs — port `SFileDisplayStyle` from TCDirCore/Config.h (text_attr: u16, icon_code_point: Option\<char\>, icon_suppressed: bool)
- [X] T015 Add icon fields to `Config` struct in src/config.rs — port icon map fields from TCDirCore/Config.h (extension_icons, well_known_dir_icons, file_attr_icons, icons: Option\<bool\>, type fallback icons, cloud status icons)
- [X] T016 Implement `initialize_extension_icons()` in src/config.rs — port `PopulateIconMap()` for extensions from TCDirCore/Config.cpp (seed HashMap from DEFAULT_EXTENSION_ICONS)
- [X] T017 Implement `initialize_well_known_dir_icons()` in src/config.rs — port `PopulateIconMap()` for well-known dirs from TCDirCore/Config.cpp (seed HashMap from DEFAULT_WELL_KNOWN_DIR_ICONS)
- [X] T018 Wire `initialize_extension_icons()` and `initialize_well_known_dir_icons()` calls into `Config::initialize()` / `initialize_with_provider()` in src/config.rs — port from TCDirCore/Config.cpp `Initialize()`
- [X] T019 Implement `parse_icon_value()` in src/config.rs — port `ParseIconValue()` from TCDirCore/Config.cpp (empty → suppressed, single char → literal, U+XXXX → code point)
- [X] T020 Extend `process_color_override_entry()` in src/config.rs to split on first comma — port comma-syntax parsing from `ParseOverrideValue()` in TCDirCore/Config.cpp (left = color, right = icon)
- [X] T021 Add `Icons`/`Icons-` switch handling in src/config.rs — port switch recognition from `s_switchMappings[]` in TCDirCore/Config.cpp
- [X] T022 Add `icons: Option<bool>` field to `CommandLine` struct in src/command_line.rs — port `m_fIcons` from TCDirCore/CommandLine.h
- [X] T023 Handle `/Icons` and `/Icons-` switch parsing in src/command_line.rs — port switch detection (trailing `-` negation, case-insensitive) from TCDirCore/CommandLine.cpp
- [X] T024 Implement `apply_config_defaults()` merge for icons in src/command_line.rs — port config→cmdline merge from TCDirCore/CommandLine.cpp (~line 73: if config has icons and cmdline doesn't, inherit)

**Checkpoint**: `cargo build` succeeds. All foundational types and data tables compile. Detection chain implemented.

---

## Phase 3: User Story 1 — Display File-Type Icons in Normal Listings (Priority: P1, MVP)

**Goal**: Each file entry shows an appropriate icon glyph prepended to the filename in normal display mode.

**Independent Test**: Run `rcdir` with a Nerd Font and verify icon glyphs appear before each filename.

### Implementation for User Story 1

- [ ] T025 [US1] Implement `get_display_style_for_file()` in src/config.rs — port `GetDisplayStyleForFile()` from TCDirCore/Config.cpp (unified color+icon precedence walk: attributes → well-known dir → extension → type fallback)
- [ ] T026 [US1] Implement `resolve_directory_style()` helper in src/config.rs — port `ResolveDirectoryStyle()` from TCDirCore/Config.cpp (well-known dir name lookup → reparse point check → default dir icon)
- [ ] T027 [US1] Implement `resolve_extension_style()` helper in src/config.rs — port `ResolveExtensionStyle()` from TCDirCore/Config.cpp (extension lookup for both color + icon)
- [ ] T028 [US1] Implement `resolve_file_attribute_style()` helper in src/config.rs — port `ResolveFileAttributeStyle()` from TCDirCore/Config.cpp (walk ATTRIBUTE_PRECEDENCE for color + icon)
- [ ] T029 [US1] Implement `resolve_file_fallback_icon()` helper in src/config.rs — port `ResolveFileFallbackIcon()` from TCDirCore/Config.cpp (symlink/junction/default file icon)
- [ ] T030 [US1] Implement `resolve_icons()` function in src/lib.rs — port icon activation resolution from `CreateDisplayer()` in TCDirCore/TCDir.cpp (CLI → env var → auto-detect priority cascade)
- [ ] T031 [US1] Add `icons_active: bool` field to displayer structs in src/results_displayer.rs — port `m_fIconsActive` from TCDirCore/ResultsDisplayerNormal.cpp (pass through constructor)
- [ ] T032 [US1] Add icon emission to `display_file_results()` in src/results_displayer.rs for Normal mode — port icon rendering from TCDirCore/ResultsDisplayerNormal.cpp (~line 90: if icons_active && icon != 0 && !suppressed → emit glyph + space; if suppressed → emit 2 spaces)
- [ ] T033 [US1] Wire `resolve_icons()` into `run()` flow in src/lib.rs — port from TCDirCore/TCDir.cpp `CreateDisplayer()` (call after Config+CommandLine init, pass result to displayer creation)

**Checkpoint**: Icons appear in normal listings. Generic file/folder/symlink/junction icons display correctly. Extension-specific icons map to the correct glyph.

---

## Phase 4: User Story 2 — Auto-Detection of Nerd Font with Manual Override (Priority: P1)

**Goal**: Icons are auto-detected at startup; user can force-enable or force-disable via CLI or env var.

**Independent Test**: On Nerd Font system → icons auto-appear. On non-NF system → classic output. /Icons forces on, /Icons- forces off.

### Implementation for User Story 2

- [ ] T034 [US2] Wire `DefaultFontProber` and `DefaultEnvironmentProvider` into `resolve_icons()` in src/lib.rs — port from TCDirCore/TCDir.cpp (instantiate CNerdFontDetector, call Detect with console handle + env provider)
- [ ] T035 [US2] Pass console output handle to `resolve_icons()` in src/lib.rs — port handle acquisition from TCDirCore/TCDir.cpp (use Console's stdout handle for GDI canary probe)
- [ ] T036 [US2] Verify `/Icons` CLI flag overrides auto-detect in src/lib.rs — port priority 1 check from TCDirCore/TCDir.cpp CreateDisplayer()
- [ ] T037 [US2] Verify `RCDIR=Icons` env var overrides auto-detect in src/lib.rs — port priority 2 check from TCDirCore/TCDir.cpp CreateDisplayer()

**Checkpoint**: Auto-detection works in WezTerm, classic conhost, and ConPTY terminals. CLI and env var overrides behave correctly.

---

## Phase 5: User Story 3 — Icon Colors Match File Type Colors (Priority: P1)

**Goal**: Icon glyph inherits the same color as the filename for each entry.

**Independent Test**: Run `rcdir` with icons active; verify icon color matches filename color for directories, extensions, and attribute overrides.

### Implementation for User Story 3

- [ ] T038 [US3] Ensure icon emission uses `style.text_attr` in `display_file_results()` in src/results_displayer.rs — port from TCDirCore/ResultsDisplayerNormal.cpp (Printf with textAttr for icon glyph, same attribute used for filename)
- [ ] T039 [US3] Verify attribute-level color carries through to icon in `get_display_style_for_file()` in src/config.rs — port from TCDirCore/Config.cpp (when attribute matches, color locks but icon can fall through to extension level per FR-020)

**Checkpoint**: Icon color is visually identical to filename color for all file types.

---

## Phase 6: User Story 4 — Environment Variable Icon Configuration (Priority: P2)

**Goal**: Users customize icons via `RCDIR` env var using comma syntax `key=[color][,icon]`.

**Independent Test**: Set `RCDIR=.py=Green,U+E606` and verify Python files show the custom icon in Green.

### Implementation for User Story 4

- [ ] T040 [US4] Implement `process_extension_icon_override()` in src/config.rs — port `ApplyIconOverride()` extension path from TCDirCore/Config.cpp (insert into extension_icons HashMap with source tracking)
- [ ] T041 [US4] Implement `process_well_known_dir_icon_override()` in src/config.rs — port `ApplyIconOverride()` dir path from TCDirCore/Config.cpp (insert into well_known_dir_icons HashMap)
- [ ] T042 [US4] Implement `process_file_attribute_icon_override()` in src/config.rs — port `ApplyIconOverride()` attribute path from TCDirCore/Config.cpp (insert into file_attr_icons HashMap)
- [ ] T043 [US4] Wire comma-syntax icon overrides into `apply_user_color_overrides()` pipeline in src/config.rs — port from TCDirCore/Config.cpp (call icon override methods after parsing comma value for extension, dir:, attr: entries)
- [ ] T044 [US4] Handle duplicate/conflicting icon entries in src/config.rs — port first-write-wins + ErrorInfo from TCDirCore/Config.cpp

**Checkpoint**: All comma-syntax examples from cli-contract.md work correctly. Backward compatibility maintained for entries without commas.

- [ ] T070 [US4] Add unit test verifying entries without comma produce identical behavior to pre-feature format (FR-024) — parse `RCDIR=.py=Green` and confirm: color set, icon unchanged, no side effects vs. old code path

---

## Phase 7: User Story 5 — Icons in Wide and Bare Display Modes (Priority: P3)

**Goal**: Icons appear correctly in `/W` (wide) and `/B` (bare) modes with proper column alignment.

**Independent Test**: Run `rcdir /W` and `rcdir /B` with icons active; verify icons appear with correct spacing.

### Implementation for User Story 5

- [ ] T045 [US5] Add icon emission to Wide display mode in src/results_displayer.rs — port from TCDirCore/ResultsDisplayerWide.cpp (~line 175-184: icon glyph before filename, cchName += 2 for column width)
- [ ] T046 [US5] Adjust column width calculation for icon in Wide mode in src/results_displayer.rs — port from TCDirCore/ResultsDisplayerWide.cpp (~line 217: account for icon+space +2 width)
- [ ] T047 [US5] Suppress directory brackets `[name]` in Wide mode when icons active in src/results_displayer.rs — port from TCDirCore/ResultsDisplayerWide.cpp (folder icon provides visual distinction, FR-008a)
- [ ] T048 [US5] Add icon emission to Bare display mode in src/results_displayer.rs — port from TCDirCore/ResultsDisplayerBare.cpp (~line 54-62: icon + space before path)

**Checkpoint**: Wide columns align correctly with icon width. Bare mode shows icon + space + path. Directory brackets suppressed in wide mode with icons.

---

## Phase 8: User Story 6 — Well-Known Folder Icons (Priority: P3)

**Goal**: Directories like `.git`, `node_modules`, `src` show specific icons instead of the generic folder icon.

**Independent Test**: Create directories with well-known names and verify each shows its distinct icon.

### Implementation for User Story 6

- [ ] T049 [US6] Verify `resolve_directory_style()` performs well-known dir name lookup in src/config.rs — port from TCDirCore/Config.cpp ResolveDirectoryStyle() (case-insensitive name match in well_known_dir_icons HashMap)
- [ ] T050 [US6] Verify user `dir:` prefix overrides built-in well-known dir icons in src/config.rs — port from TCDirCore/Config.cpp (RCDIR=dir:src=,U+ABCD overrides default)

**Checkpoint**: 60 well-known directory names show specific icons. User `dir:` overrides work.

---

## Phase 9: User Story 7 — Enhanced Cloud Status Symbols (Priority: P3)

**Goal**: Cloud status column upgrades from Unicode circles to Nerd Font glyphs when icons are active.

**Independent Test**: Run `rcdir` with icons in a OneDrive folder; verify cloud symbols are NF glyphs.

### Implementation for User Story 7

- [ ] T051 [US7] Add `nf_symbol()` method to `CloudStatus` enum in src/cloud_status.rs — port cloud icon lookup from `Config::GetCloudStatusIcon()` in TCDirCore/Config.cpp (CloudOnly → cloud_outline, Local → cloud_check, Pinned → pin)
- [ ] T052 [US7] Modify `display_cloud_status_symbol()` in src/results_displayer.rs to use NF glyphs when icons active — port from TCDirCore/ResultsDisplayerNormal.cpp (~line 303: if icons_active → nf_symbol, else → Unicode symbols)
- [ ] T053 [US7] Add cloud status NF glyph emission in Wide mode in src/results_displayer.rs — port from TCDirCore/ResultsDisplayerWide.cpp (~line 133-150: NF cloud glyphs before icon/filename)

**Checkpoint**: Cloud status uses NF glyphs when icons on, Unicode circles when off. Zero regression.

---

## Phase 10: Diagnostics & Help

**Purpose**: Documentation and configuration display for the icons feature.

- [ ] T054 Add `/Icons` and `/Icons-` to `display_usage()` in src/usage.rs — port switch table and syntax line from TCDirCore/Usage.cpp
- [ ] T055 Add comma syntax and `dir:` prefix documentation to `display_env_var_help()` in src/usage.rs — port env help text from TCDirCore/Usage.cpp (icon formats, U+XXXX, literal glyph, empty=suppressed, per FR-028)
- [ ] T056 Add icon status line to `display_current_configuration()` in src/usage.rs — port icon detection result display from TCDirCore/Usage.cpp (FR-026: "Icons: On (auto-detected)" etc.)
- [ ] T057 Add icon glyphs to extension color table in `display_current_configuration()` in src/usage.rs — port inline icon display from TCDirCore/Usage.cpp (~line 501-514: when icons active, show glyph before each extension entry, FR-027)
- [ ] T058 Add well-known directory icon table to `display_current_configuration()` in src/usage.rs — port separate dir icon table from TCDirCore/Usage.cpp (FR-027: show when icons active, with source indicators)
- [ ] T059 Add cloud status NF glyphs to config display item table in src/usage.rs — port from TCDirCore/Usage.cpp (FR-031: show NF glyph instead of Unicode shape when icons active)
- [ ] T060 Display icon-related RCDIR overrides in `display_env_var_help()` in src/usage.rs — port from TCDirCore/Usage.cpp (FR-029: show parsed icon overrides from current RCDIR value)

**Checkpoint**: `/Icons` documented in `/?`. Comma syntax documented in `/env`. Icon status shown in `/config`.

---

## Phase 11: Polish & Cross-Cutting Concerns

**Purpose**: Final integration, validation, and cleanup

- [ ] T061 Verify `display_env_var_issues()` reports icon-related errors in src/usage.rs — port error display for invalid U+XXXX, multi-char icons, duplicate icon entries from TCDirCore error handling
- [ ] T062 Run `cargo clippy` and fix all warnings across modified and new files
- [ ] T063 Run `cargo build --release` and verify clean build
- [ ] T064 Run `cargo test` and verify all existing tests still pass (zero regression)
- [ ] T065 Manual verification: icons appear with Nerd Font terminal, classic output without Nerd Font
- [ ] T066 Manual verification: `/Icons` forces on, `/Icons-` forces off, `RCDIR=Icons`/`Icons-` work
- [ ] T067 Manual verification: Wide mode columns align with icons, brackets suppressed
- [ ] T068 Manual verification: Cloud status NF glyphs in OneDrive folder
- [ ] T069 Run quickstart.md verification checklist (all items)
- [ ] T071 Verify all icon glyphs in DEFAULT_EXTENSION_ICONS and DEFAULT_WELL_KNOWN_DIR_ICONS are single-width Nerd Font code points (FR-006) — add unit test that iterates both tables and asserts each glyph is in the Nerd Font PUA range (U+E000–U+F8FF or U+F0000–U+FFFFF) and is single-width
- [ ] T072 Verify icon emission spacing: icon+space = 2 chars when present, 2 spaces when suppressed (FR-007) — add unit test that checks Normal, Wide, and Bare mode output formatting for both active-icon and suppressed-icon entries
- [ ] T073 Add unit test verifying attribute display order (RHSATECP0 in FileAttributeMap) is independent from attribute precedence order (PSHERC0TA in file_attribute_map.rs) (FR-011) — assert the two arrays contain the same set of attributes but in different orders
- [ ] T074 Add unit test verifying every extension in the built-in color table has a corresponding entry in DEFAULT_EXTENSION_ICONS (SC-005) — iterate Config's default color mappings and assert each extension key exists in icon_mapping::DEFAULT_EXTENSION_ICONS
- [ ] T075 Add icon-mode test scenarios to tests/output_parity.rs — port from TCDir UnitTest patterns: run rcdir with /Icons in a temp directory containing known file types, verify icon glyphs appear in output; run without /Icons and verify classic output unchanged

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies — start immediately
- **Phase 2 (Foundational)**: Depends on Phase 1 — BLOCKS all user stories
- **Phases 3–5 (US1–US3, P1)**: All depend on Phase 2 completion. US1 must complete before US3 (icon color requires icon emission). US2 can proceed in parallel with US1.
- **Phase 6 (US4, P2)**: Depends on Phase 2 + US1 (needs comma syntax wired into config override pipeline)
- **Phases 7–8 (US5–US6, P3)**: Depend on Phase 2 + US1. Can proceed in parallel with each other.
- **Phase 9 (US7, P3)**: Depends on Phase 2 + US1.
- **Phase 10 (Diagnostics)**: Depends on US1 + US2 at minimum (icons must work before documenting)
- **Phase 11 (Polish)**: Depends on all prior phases

### User Story Dependencies

- **US1 (Display Icons)**: Foundation only → MVP core
- **US2 (Auto-Detection)**: Foundation only → can run in parallel with US1
- **US3 (Icon Colors)**: Depends on US1 (icon emission must exist to verify color)
- **US4 (Env Var Config)**: Depends on Foundation + US1 icon resolution
- **US5 (Wide/Bare Modes)**: Depends on US1 (normal mode icons must work first)
- **US6 (Well-Known Dirs)**: Depends on Foundation (well-known dir table + resolve_directory_style)
- **US7 (Cloud Status)**: Depends on US1 (icons_active flag must be wired)

### Within Each User Story

- Config/data changes before displayer changes
- Resolution logic before emission logic
- Core implementation before integration wiring

### Parallel Opportunities

**Phase 2 parallel batch** (different files, no deps):
- T005, T008 can start together (icon_mapping.rs vs file_attribute_map.rs)
- T006, T007 can run in parallel (both in icon_mapping.rs but independent table sections)
- T009, T010 can start together (nerd_font_detector.rs types)
- T014, T015 can start together (config.rs new types + fields)
- T022, T023 can start together (command_line.rs field + parsing)

**After Phase 2, these story tracks can parallelize**:
- US1 (T025–T033) and US2 (T034–T037) — different areas of code
- US5 (T045–T048) and US6 (T049–T050) — after US1 completes
- US7 (T051–T053) can run alongside US5/US6

---

## Implementation Strategy

### MVP First (US1 + US2 + US3)

1. Complete Phase 1: Setup (T001–T004)
2. Complete Phase 2: Foundational (T005–T024)
3. Complete Phase 3: US1 — icons display in normal mode (T025–T033)
4. Complete Phase 4: US2 — auto-detection works (T034–T037)
5. Complete Phase 5: US3 — icon colors correct (T038–T039)
6. **STOP and VALIDATE**: Icons work end-to-end with correct colors

### Incremental Delivery

7. Phase 6: US4 — env var customization (T040–T044)
8. Phase 7: US5 — wide/bare modes (T045–T048)
9. Phase 8: US6 — well-known dir icons (T049–T050)
10. Phase 9: US7 — cloud status NF glyphs (T051–T053)
11. Phase 10: Diagnostics (T054–T060)
12. Phase 11: Polish (T061–T075)

### Approach

Every task ports directly from the TCDir C++ reference implementation. Examine the TCDir source for each task, understand the algorithm, then translate to idiomatic Rust using the same logic, same data flow, same behavior.

---

## Summary

| Metric | Value |
|--------|-------|
| Total tasks | 75 |
| Phase 1 (Setup) | 4 |
| Phase 2 (Foundational) | 20 |
| US1 (Display Icons) | 9 |
| US2 (Auto-Detection) | 4 |
| US3 (Icon Colors) | 2 |
| US4 (Env Var Config) | 6 |
| US5 (Wide/Bare) | 4 |
| US6 (Well-Known Dirs) | 2 |
| US7 (Cloud Status) | 3 |
| Diagnostics | 7 |
| Polish | 14 |
| Parallel opportunities | T005–T007, T009–T010, T014–T015, T022–T023 (foundational); US1∥US2 (story-level) |
| MVP scope | US1 + US2 + US3 (Phases 1–5, T001–T039) |
