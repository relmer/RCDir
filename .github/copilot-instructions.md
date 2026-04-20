````instructions
# Global Copilot Instructions for RCDir (Rust)

## Code Formatting - CRITICAL RULES

### **NEVER** Delete Blank Lines
- **NEVER** delete blank lines between file-level constructs (functions, structs, impls, modules)
- **NEVER** delete blank lines between different use groups
- **NEVER** delete blank lines between field declaration blocks in structs
- Preserve all existing vertical spacing in code

### Vertical Spacing Between Top-Level Constructs
- Use **exactly 5 blank lines** between top-level constructs:
  - Between functions
  - Between `impl` blocks
  - Between `struct`/`enum`/`trait` definitions
  - Between `mod` declarations/blocks
  - Between groups of `use` statements and the next construct
  - Between global/static constants and the next construct
- This applies everywhere: file-level constructs, inside `impl` blocks, inside `mod` blocks (including `mod tests`)
- Section-divider comment blocks (e.g., `////////////////...`) count as part of the construct below them — the 5 blank lines go **above** the divider

### Function Call and Macro Parenthesis Spacing
- **With arguments**: place a space before the opening parenthesis: `func (with, args)`
- **Without arguments**: NO space before the opening parenthesis: `func()`
- This applies to function calls, method calls, and macro invocations
- Examples:
  ```rust
  // CORRECT:
  println! ("Hello, {}", name);
  writeln! (f, "{}", value)?;
  some_function (arg1, arg2);
  foo.method (x, y);
  Vec::with_capacity (16);
  result.unwrap();
  String::new();
  vec.len();

  // WRONG:
  println!("Hello, {}", name);    // missing space before (
  some_function(arg1, arg2);       // missing space before (
  result.unwrap ();                // unwanted space before ()
  String::new ();                  // unwanted space before ()
  ```

### **NEVER** Break Column Alignment
- **NEVER** break existing column alignment in struct field declarations
- **NEVER** break alignment of:
  - Field names
  - Field types
  - Assignment operators (`=`)
  - Initialization values
- **ALWAYS** preserve exact column positions when replacing lines
- When modifying a line, ensure replacement maintains same indentation as original

### Indentation Rules
- **ALWAYS** preserve exact indentation when replacing code
- **NEVER** start code at column 1 unless original was at column 1
- Count spaces carefully - if original had 12 spaces, replacement must have 12 spaces
- Use 4 spaces for indentation (Rust standard)

### Example of CORRECT editing:
```rust
// Original:
            eprintln!("Error: {}", msg);

// CORRECT replacement (preserves 12-space indentation):
            console.print_error(&format!("Error: {}", msg));

// WRONG replacement (broken indentation):
console.print_error(&format!("Error: {}", msg));
```

---

## File Modification Rules

### Scope of Changes
- **ONLY** modify the files explicitly requested
- If a change requires modifying other files, **ASK FIRST**
- When told to modify file X, do not make "helpful" changes to files Y or Z

---

## Code Changes - Best Practices

### When Replacing Code
1. Read the original line(s) carefully
2. Note the exact indentation level (count spaces)
3. Note any column alignment with surrounding lines
4. Apply changes while preserving formatting
5. Double-check indentation before submitting

### When Showing Code Changes
- **NEVER** show full file contents unless explicitly asked
- Use minimal, surgical edits with `// ...existing code...` comments
- Show only the lines being changed plus minimal context

### Before Applying Changes
- Verify you understand which files should/shouldn't be modified
- Check if files are from other projects (read-only)
- Confirm you're preserving all formatting rules above

---

## Rust Specific Guidelines

### Function Header Comments
- Every function must have a TCDir-style header comment block above it
- Top-level functions use 80-slash dividers; indented functions (inside `impl` blocks) use 76-slash dividers (4 spaces + 76 = 80 chars)
- Format:
  ```rust
  ////////////////////////////////////////////////////////////////////////////////
  //
  //  function_name
  //
  //  Brief description of what the function does.
  //
  ////////////////////////////////////////////////////////////////////////////////
  ```
- **Trait impls with a single function**: use one header block describing the trait impl above the `impl` line; no separate function header inside
- **Trait impls with multiple functions**: the `impl` block gets a header, and each function inside also gets its own function header
- The header block goes above `#[test]` or other attributes when present
- 1 blank line between the closing divider and the `fn` (or its attribute)

### Module Organization
- Use `mod.rs` or direct file naming based on existing project convention
- Organize `use` statements in groups: std, external crates, local modules
- Keep a blank line between use groups

### Error Handling
- Use `Result<T, E>` for fallible operations
- Prefer custom error types or `thiserror` for library code
- Use `anyhow` for application-level error handling if appropriate
- Handle all `Result` and `Option` types explicitly (no silent unwrap in production)
- Functions should return `Result` rather than panicking

### Ownership and Borrowing
- Prefer borrowing (`&T`, `&mut T`) over ownership when possible
- Use `Clone` sparingly - only when semantically appropriate
- Prefer `&str` over `String` in function parameters when ownership isn't needed
- Use `Cow<str>` for functions that might or might not need to allocate

### Modern Rust Features
- Use `?` operator for error propagation
- Prefer `impl Trait` in function signatures where appropriate
- Use pattern matching for control flow
- Leverage iterators and functional combinators

### Windows-Specific
- Use `windows` crate for Win32 API interop
- Handle wide strings (`OsString`, `OsStr`) for Windows paths
- Use `widestring` crate for UTF-16 string handling

### Function Size & Structure
- Keep functions focused and short — ideally under ~50 lines (roughly one screen)
- Aggressively factor out helper functions that do just one thing
- Avoid excessive nesting: if a function requires more than 2–3 levels of indentation, extract that inner logic into its own function
- Each function should have a single clear purpose

### Performance
- Prefer stack allocation over heap when feasible
- Use `Vec::with_capacity` when size is known
- Avoid unnecessary allocations in hot paths
- Profile before optimizing

### Unit Testing — Isolation Rules
- Unit tests **MUST NEVER** rely on or alter real system state
- **ALL** system services **MUST** be mocked or abstracted behind interfaces:
  - **File system**: No reading/writing actual files on disk — use in-memory data or mock I/O
  - **Process execution**: No running real binaries via `Command::new` — mock the output
  - **Registry/environment**: No reading real environment variables — use `MockEnvironmentProvider` or equivalent
  - **Network**: No real HTTP/socket calls — mock network layers
  - **Console/terminal**: No real console API calls — use mock console
  - **Current directory**: No depending on `std::env::current_dir()` — use explicit test paths
- Tests must be **deterministic** and **repeatable** regardless of the machine, directory, or user running them
- If a module uses system APIs, inject its dependencies through a trait so tests can substitute mocks
- **No test may run the real `rcdir` binary** — test the library functions directly with mocked dependencies
- Temp files are acceptable **only** in explicitly marked integration tests, never in unit tests

### Output Parity Tests — Required for All Features
- Every user-visible feature or bug fix MUST include output parity tests in `tests/output_parity.rs`
- Parity tests run both `rcdir` and `tcdir` with the same arguments and assert byte-identical output
- These are an **allowed exception** to the unit test isolation rules above — they run real binaries
- Parity tests gracefully skip when `tcdir.exe` is not available (CI environments)
- When adding a new feature, add parity test cases covering all affected display modes (normal, tree, wide, bare as applicable)

---

## Communication Rules

### When Explaining Changes
- Be concise and direct
- Explain the "why" not just the "what"
- If you make a mistake, acknowledge it immediately and clearly

### Before Major Changes
- Summarize what files will be modified
- Explain pros/cons if there are trade-offs
- Ask for confirmation if approach is unclear

### When Rules Conflict
- **Formatting rules ALWAYS take priority**
- File modification rules come second
- Code style preferences come third
- When in doubt, **ASK** before proceeding

---

## Shell and Terminal Rules

### PowerShell is the Default Shell
- **ALL** terminal windows use PowerShell, not CMD
- **ALWAYS** format commands for PowerShell syntax

---

## Build and Test

### CRITICAL: Always Use Build Tasks — Never Raw Cargo for Builds
- **NEVER** run `cargo build` directly — it skips the version increment
- **ALWAYS** use the VS Code build task (`Build Debug (current arch)`) or `scripts/Build.ps1` to build
- `Build.ps1` calls `IncrementVersion.ps1` before cargo, which bumps the build number in `Version.toml`
- Running `cargo build` directly produces a binary with a stale version number
- `cargo test`, `cargo check`, and `cargo clippy` are fine to run directly — they don't produce release artifacts

### Allowed Direct Cargo Commands (no build task needed)
- `cargo test` — run tests (no version increment needed)
- `cargo check` — quick compilation verification
- `cargo clippy` — lint checking

### Toolchain Currency
- **ALWAYS** run `rustup update stable` before starting work to ensure the local toolchain matches CI
- CI uses `dtolnay/rust-toolchain@stable` which installs the latest stable Rust on every run
- New stable releases (every 6 weeks) can introduce new clippy lints that break `-D warnings`
- A toolchain mismatch between local and CI is the most common cause of "works locally, fails in CI"

### Pre-Push Checklist
- **ALWAYS** run `cargo clippy -- -D warnings` and verify zero errors before pushing
- **ALWAYS** run `cargo test` and verify all tests pass before pushing
- If clippy introduces new warnings after a toolchain update, fix them before pushing

### Pre-Commit Gates
- **ALL** tests MUST pass before committing (`cargo test`)
- Clippy MUST be clean (`cargo clippy -- -D warnings`) before committing
- Build MUST succeed with no errors before committing

### Commit Frequency
- During spec implementation, commit **at least once per completed phase**
- Each commit must leave the codebase in a compilable, tests-passing, clippy-clean state
- Do not batch an entire feature into a single commit

### Build Integration
- Always build after making changes using the build task or `Build.ps1`
- Fix all clippy warnings before considering task complete
- Check for both errors and warnings

---

## Commit Messages

- Use [Conventional Commits](https://www.conventionalcommits.org/) format: `type(scope): description`
- **Scope is always required** — never omit it
- Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`, `ci`, `build`
- Use bullet list in the body for multiple changes OR additional details about the changes
- Use lowercase for type and scope
- Use imperative mood in description ("add" not "added")
- Keep first line under 72 characters
- Add body for complex changes (blank line after subject)
- Examples:
  - `feat(cli): add --owner switch for file ownership display`
  - `fix(color): correct ANSI code for bright magenta`
  - `refactor(config): extract env var parsing to separate module`
  - `docs(readme): add installation instructions`

---

## Remember
- **Formatting preservation is non-negotiable**
- **Read-only files must stay read-only**
- **When in doubt, ask before modifying**
- **Quality over speed - take time to get formatting right**

---

*Last Updated: 2026-04-20*
*These rules apply globally to all RCDir work*

````
