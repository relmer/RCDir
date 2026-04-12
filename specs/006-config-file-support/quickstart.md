# Quickstart: Config File Support

## Build & Test

```powershell
# Build
cargo build

# Run tests
cargo test

# Clippy
cargo clippy -- -D warnings
```

## Try It

1. Create a config file:
```powershell
@"
# My rcdir config
tree
icons

# Colors
.cpp = LightGreen
.h   = Yellow on Blue
D    = LightCyan
"@ | Set-Content "$env:USERPROFILE\.rcdirconfig" -Encoding UTF8
```

2. Run rcdir — settings from the file should be applied:
```powershell
rcdir
```

3. Override with env var — env var wins for conflicting keys:
```powershell
$env:RCDIR = ".cpp=Red"
rcdir    # .cpp is now Red (env var), but tree/icons/.h/D still from config file
```

4. Inspect config file diagnostics:
```powershell
rcdir --config
```

5. View merged settings with sources:
```powershell
rcdir --settings
```

## Key Files

| File | Purpose |
|------|---------|
| `src/config/file_reader.rs` | NEW: read file, BOM handling, line splitting |
| `src/config/mod.rs` | Extended: `load_config_file`, source tracking, switch/param sources |
| `src/config/env_overrides.rs` | Extended: source parameter threaded through override methods |
| `src/command_line.rs` | Extended: `--settings` switch |
| `src/usage.rs` | Extended: `--config` repurposed, `--settings` new, error display with `show_hint` |
| `src/lib.rs` | Extended: initialization flow, `--settings` dispatch, end-of-run error display |
| `tests/config_file_tests.rs` | NEW: config file parsing, loading, precedence, error tests |

## Implementation Order

1. `file_reader.rs` + tests (file reading layer — BOM, UTF-8, line splitting)
2. `Config::load_config_file` + `process_config_lines` + source tracking + tests (parsing layer)
3. Error model extension (`ErrorInfo` line numbers) + tests
4. `--settings` command + `--config` repurposing + tests
5. Help text updates
