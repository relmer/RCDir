# Contributing to RCDir

## Commit Messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/). Please format your commit messages as:

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

| Type       | Description                                      |
|------------|--------------------------------------------------|
| `feat`     | New feature                                      |
| `fix`      | Bug fix                                          |
| `docs`     | Documentation only                               |
| `style`    | Formatting, no code change                       |
| `refactor` | Code change that neither fixes nor adds feature  |
| `perf`     | Performance improvement                          |
| `test`     | Adding or fixing tests                           |
| `chore`    | Build process, tooling, dependencies             |
| `ci`       | CI/CD changes                                    |
| `build`    | Build system changes                             |

### Examples

```
feat(console): add colorized output support
fix(display): correct column alignment for wide filenames
docs: update README with new command-line options
chore(build): add ARM64 cross-compilation support
```

## Building

Requires Rust toolchain. Install from https://rustup.rs/

```powershell
# Install required targets (one-time setup)
rustup target add x86_64-pc-windows-msvc
rustup target add aarch64-pc-windows-msvc

# Build Debug for current architecture
.\scripts\Build.ps1

# Build Release for all platforms
.\scripts\Build.ps1 -Target BuildAllRelease

# Run tests
.\scripts\RunTests.ps1

# Run clippy lints
.\scripts\Build.ps1 -Target Clippy
```

## Code Style

- Follow standard Rust idioms
- Run `cargo clippy` before committing
- All warnings should be resolved

## Testing

- Unit tests go in `#[cfg(test)]` modules alongside the code they test
- Integration tests go in the `tests/` directory
- Run tests with `cargo test` or `.\scripts\RunTests.ps1`
