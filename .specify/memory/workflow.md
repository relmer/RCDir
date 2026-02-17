# Implementation Workflow Directives

These directives govern how Copilot behaves during `/speckit.implement` sessions. They apply to all feature implementations in this repository.

## Commit Granularity

- Each git commit covers **at most one phase** from the feature's `tasks.md`
- Splitting a phase into multiple commits is acceptable for large phases
- **Never** combine tasks from different phases into a single commit
- Use conventional commit format: `feat(<scope>): phase N — brief description`
- Include the updated `tasks.md` in each phase commit (with completed tasks marked `[x]`)

## Task Tracking

- Mark tasks as `[x]` in `tasks.md` as they are completed
- Use the todo list tool to track in-session progress
- Commit `tasks.md` alongside code changes for each phase

## Autonomous Execution

- Implement all tasks continuously without stopping for manual approval
- Do not ask "should I continue?" or "want me to proceed?" — just keep going
- After completing a phase's commit, immediately begin the next phase
- Do not yield back to the user until **all phases are complete** or a genuine blocker is hit that cannot be resolved from the artifacts + reference implementation source
- If an ambiguity arises, resolve it by checking the reference implementation (TCDir C++ source in the `TCDir/TCDirCore/` workspace folder) and match its behavior

## Quality Gates Per Phase

- After each phase, run `cargo check` (or `cargo build` for later phases)
- Run `cargo clippy` and `cargo test` after Phase 2 and every phase thereafter
- Fix any issues before committing
- All clippy warnings must be resolved before a phase commit

## Reference Implementation

- For every task, examine the corresponding reference source file first (per the mapping table in `plan.md`)
- Understand the algorithm, then translate to idiomatic Rust
- Same logic, same behavior, same edge cases

## Session Continuity

- If a session runs out of token budget mid-implementation, the user will start a new conversation
- The completed tasks marked `[x]` in `tasks.md` indicate where to resume
- Resume prompt: "Continue `/speckit.implement` from Phase N"

## Formatting

- Follow `.github/copilot-instructions.md` exactly
- 5 blank lines between top-level constructs
- Function call spacing rules (space before `(` with args, no space for `()`)
- Header comment blocks on every function

---

**Version**: 1.0.0 | **Created**: 2026-02-17
