```markdown
<!--
================================================================================
SYNC IMPACT REPORT
================================================================================
Version change: N/A → 1.0.0 (initial ratification)
Modified principles: N/A (new document)
Added sections:
  - Core Principles (5 principles)
  - Technology Constraints
  - Development Workflow
  - Governance
Removed sections: N/A
Templates requiring updates:
  ✅ plan-template.md - Constitution Check section aligns with new principles
  ✅ spec-template.md - Requirements/edge cases align with UX consistency principle
  ✅ tasks-template.md - Test phases align with Testing Discipline principle
Follow-up TODOs: None
================================================================================
-->

# RCDir Constitution

## Core Principles

### I. Code Quality (NON-NEGOTIABLE)

All code MUST adhere to established formatting and structural standards:

- **Formatting Preservation**: NEVER delete blank lines between file-level constructs, NEVER break column alignment in declarations
- **Indentation Exactness**: Preserve exact indentation when modifying code; use 4 spaces per indent level (Rust standard)
- **Error Handling**: Use `Result<T, E>` for all fallible operations; avoid `unwrap()` and `expect()` in production code except where panic is the correct semantic
- **Ownership Clarity**: Prefer borrowing over ownership; use `Clone` only when semantically appropriate
- **Idiomatic Rust**: Follow Rust conventions - use iterators, pattern matching, and the type system to prevent errors at compile time

**Rationale**: Consistent formatting enables efficient code review and reduces merge conflicts. Idiomatic Rust code is safer, more maintainable, and performs better.

### II. Testing Discipline

All production code MUST have corresponding tests:

- **Test Framework**: Use Rust's built-in test framework (`#[test]`, `#[cfg(test)]`)
- **Test Coverage**: Every public function and significant code path MUST be covered by tests
- **Test Independence**: Each test MUST be independently runnable and MUST NOT depend on execution order
- **Build Verification**: Tests MUST pass before any merge or release; use `cargo test`
- **Test Organization**: Tests reside in `tests/` for integration tests, inline `#[cfg(test)]` modules for unit tests

**Rationale**: Automated tests catch regressions early and serve as living documentation of expected behavior.

### III. User Experience Consistency

All user-facing output MUST follow established patterns:

- **Colorized Output**: Use consistent color theming; respect RCDIR environment variable color configuration
- **CLI Syntax**: Follow existing switch patterns; new switches MUST mirror TCDir's established syntax
- **Error Messages**: Errors go to stderr; user-facing messages MUST be clear, actionable, and consistent in tone
- **Help System**: All features MUST be documented in `-?` help output and `--env`/`--config` where applicable
- **Backward Compatibility**: Maintain compatibility with TCDir's command-line interface

**Rationale**: RCDir is a Rust port of TCDir; users expect consistent behavior with the original.

### IV. Performance Requirements

Performance is a core feature, not an afterthought:

- **Console API**: Use Windows Console API directly via the `windows` crate for optimal performance
- **Buffering Strategy**: Use large internal buffers to minimize system calls; flush strategically, not per-write
- **Multi-Threading**: Default to multi-threaded enumeration using `std::thread` workers with work queue and `Condvar`; single-threaded mode available via `/M-`
- **Measurable**: Use `/P` flag infrastructure to measure and report performance; major features MUST NOT regress timing
- **Zero-Cost Abstractions**: Leverage Rust's zero-cost abstractions; avoid unnecessary allocations in hot paths

**Rationale**: RCDir is a replacement for `dir`; users expect it to be faster and better in every way.

### V. Simplicity & Maintainability

Complexity MUST be justified:

- **YAGNI**: Do not implement features "just in case"; implement when needed
- **Single Responsibility**: Each module/struct SHOULD have one clear purpose
- **Self-Documenting Code**: Prefer clear naming over comments; add comments only for non-obvious "why" explanations
- **Minimal Dependencies**: Keep external crate dependencies minimal; prefer the standard library where practical
- **File Scope**: Modify only files explicitly required; ask before making "helpful" changes to unrelated files
- **Function Size & Structure**: Keep functions focused and relatively short. Extract complex logic into helper functions rather than deep nesting.

**Rationale**: Simple code is easier to understand, test, and maintain over time.

## Technology Constraints

**Language/Version**: Rust stable (latest stable release)
**Build System**: Cargo
**Target Platforms**: Windows 10/11, x64 and ARM64 architectures
**Testing Framework**: Rust built-in (`#[test]`, `cargo test`)
**Dependencies**: Minimize external crates; prefer `windows` crate for Win32 API, standard library for everything else
**Build Configurations**: Debug and Release for both x64 and ARM64
**Linting**: `cargo clippy` with warnings as errors

## Development Workflow

### Tool Preference

When automation tooling exists, prefer it over raw terminal commands:

- **Build/Test**: Use `cargo build`, `cargo test`, `cargo clippy`
- **Formatting**: Follow manual formatting rules in `.github/copilot-instructions.md` (no rustfmt)
- **Errors**: Use `get_errors` tool instead of parsing compiler output manually
- **File Operations**: Use provided tools (read_file, replace_string_in_file, etc.) over terminal commands when appropriate

**Rationale**: Established tooling is tested, consistent, and integrates with the development environment.

### Quality Gates

1. **Pre-Commit**: Code MUST compile without errors or warnings (`cargo clippy -- -D warnings`)
2. **Build Verification**: Run `cargo test` to ensure all tests pass before considering work complete
3. **Formatting**: Manually verify formatting follows rules in `.github/copilot-instructions.md`
4. **Architecture Coverage**: Verify changes work on both x64 and ARM64 when touching platform-sensitive code

### Change Process

1. Make minimal, surgical edits; show only changed lines with context
2. Preserve all formatting (indentation, alignment, blank lines)
3. Run `cargo check` or `cargo build` after changes
4. Verify tests pass with `cargo test`
5. Run `cargo clippy` and address all warnings

## Governance

This constitution supersedes all ad-hoc practices. All code changes MUST verify compliance with these principles.

**Amendment Process**:
1. Propose change with rationale
2. Document impact on existing code/practices
3. Update constitution version following semantic versioning:
   - MAJOR: Backward-incompatible principle removal or redefinition
   - MINOR: New principle or materially expanded guidance
   - PATCH: Clarifications, wording, non-semantic refinements
4. Update dependent templates if affected

**Compliance Review**: Periodically review codebase against constitution principles; document exceptions with justification.

**Guidance Reference**: See `.github/copilot-instructions.md` for detailed runtime development guidance and code style rules.

**Version**: 1.0.0 | **Ratified**: 2026-02-07 | **Last Amended**: 2026-02-07

```
