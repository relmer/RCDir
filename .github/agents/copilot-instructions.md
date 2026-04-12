````markdown
# RCDir Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-04-11

## Active Technologies
- Rust stable (edition 2024, toolchain 1.85+) + `windows` crate 0.62 (Win32 API), `widestring` 1 (UTF-16) (003-file-icons)
- N/A (all in-memory static tables + runtime hash maps) (003-file-icons)
- Rust stable (edition 2024) + `windows` crate (Win32 API), `widestring` crate, Rust std library only (no third-party libraries) (004-tree-view)
- N/A (filesystem enumeration, no persistent storage) (004-tree-view)
- Rust stable (latest stable release) + `windows` crate for Win32 console API; standard library for file I/O (`std::fs::read`) (006-config-file-support)
- Single flat file (`%USERPROFILE%\.rcdirconfig`), UTF-8 with optional BOM (006-config-file-support)

- Rust stable (1.93.0), Edition 2024 + `windows` crate (Win32 APIs), `widestring` (UTF-16 interop) (master)

## Project Structure

```text
src/
tests/
```

## Commands

cargo test; cargo clippy

## Code Style

Rust stable (1.93.0), Edition 2024: Follow standard conventions

## Recent Changes
- 006-config-file-support: Added Rust stable (latest stable release) + `windows` crate for Win32 console API; standard library for file I/O (`std::fs::read`)
- 004-tree-view: Added Rust stable (edition 2024) + `windows` crate (Win32 API), `widestring` crate, Rust std library only (no third-party libraries)
- 004-tree-view: Added Rust stable (edition 2024) + `windows` crate (Win32 API), `widestring` crate, Rust std library only (no third-party libraries)


<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->

````
