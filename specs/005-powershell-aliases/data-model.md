# Data Model: PowerShell Alias Configuration

**Feature**: 005-powershell-aliases | **Date**: 2026-03-25

## Entities

### PowerShellVersion (enum)

Detected PowerShell version of the calling shell.

| Value | Description |
| ------- | ------------- |
| `PowerShell` | PowerShell 7+ (`pwsh.exe`). Profile dir: `PowerShell\` |
| `WindowsPowerShell` | Windows PowerShell 5.1 (`powershell.exe`). Profile dir: `WindowsPowerShell\` |
| `Unknown` | Parent process is neither pwsh.exe nor powershell.exe |

---

### ProfileScope (enum)

One of the four standard PowerShell profile scopes.

| Value | Variable Name | Filename |
| ------- | -------------- | ---------- |
| `CurrentUserCurrentHost` | `$PROFILE.CurrentUserCurrentHost` | `Microsoft.PowerShell_profile.ps1` |
| `CurrentUserAllHosts` | `$PROFILE.CurrentUserAllHosts` | `profile.ps1` |
| `AllUsersCurrentHost` | `$PROFILE.AllUsersCurrentHost` | `Microsoft.PowerShell_profile.ps1` |
| `AllUsersAllHosts` | `$PROFILE.AllUsersAllHosts` | `profile.ps1` |

**Constraints**: AllUsers scopes require administrator privileges for write operations.

---

### ProfileLocation (struct)

A resolved profile file path with metadata.

| Field | Type | Description |
| ------- | ------ | ------------- |
| `scope` | `ProfileScope` | Which of the four scopes |
| `variable_name` | `String` | Display name (e.g., `$PROFILE.CurrentUserAllHosts`) |
| `resolved_path` | `PathBuf` | Full filesystem path |
| `exists` | `bool` | Whether the file currently exists on disk |
| `requires_admin` | `bool` | Whether writing requires admin privileges |
| `has_alias_block` | `bool` | Whether rcdir marker block was found in the file |

**Derived from**: `PowerShellVersion` + `SHGetKnownFolderPath(FOLDERID_Documents | FOLDERID_ProgramData)` + `ProfileScope`

---

### AliasDefinition (struct)

A single alias (root or sub) to be generated.

| Field | Type | Description |
| ------- | ------ | ------------- |
| `name` | `String` | Alias function name (e.g., `d`, `dt`, `dd`, `ds`) |
| `flags` | `String` | rcdir flags to prepend (empty for root; e.g., `-a:d` for dirs-only) |
| `description` | `String` | Human-readable description (e.g., `Tree view`, `Directories only`) |
| `enabled` | `bool` | Whether selected by user in the checkbox step |

**Validation Rules**:

- Root alias: 1-4 alphanumeric characters (FR-021)
- Sub-alias name: root + suffix (suffix from fixed set: `t`, `d`, `s`, `sb`, `w`)

---

### AliasConfig (struct)

The complete user configuration from the TUI wizard.

| Field | Type | Description |
| ------- | ------ | ------------- |
| `root_alias` | `String` | Root alias name chosen by user (default: `d`) |
| `rcdir_invocation` | `String` | How to invoke rcdir (`rcdir` or full path) |
| `sub_aliases` | `Vec<AliasDefinition>` | Sub-aliases with enabled/disabled state |
| `target_scope` | `ProfileScope` | Chosen profile location (or session-only sentinel) |
| `target_path` | `PathBuf` | Resolved path for the chosen profile |
| `session_only` | `bool` | True if "Current session only" was chosen |
| `what_if` | `bool` | Dry-run mode — preview only, no file writes |

---

### AliasBlock (struct)

A parsed alias block found in an existing profile file.

| Field | Type | Description |
| ------- | ------ | ------------- |
| `start_line` | `usize` | 0-based line index of the opening marker |
| `end_line` | `usize` | 0-based line index of the closing marker |
| `root_alias` | `String` | Detected root alias name (parsed from block content) |
| `alias_names` | `Vec<String>` | All function names found in the block |
| `function_lines` | `Vec<String>` | Full `function xxx { ... }` lines from the block |
| `version` | `String` | rcdir version that generated the block |

---

## State Transitions

### TUI Wizard Steps (--set-aliases)

```text
[Start] → Step1_RootAlias → Step2_SubAliases → Step3_ProfileLocation → Step4_Preview → [Write/WhatIf]
                                                                                          ↓
[Escape at any step] ──────────────────────────────────────────────────────────→ [Cancelled]
```

| State | Input Widget | Output |
| ------- | ------------- | -------- |
| Step1_RootAlias | Text input (default: `d`) | `root_alias` → recalculate sub-alias names |
| Step2_SubAliases | Checkbox list | `sub_aliases` with enabled states |
| Step3_ProfileLocation | Radio button list | `target_scope` + `target_path` |
| Step4_Preview | Confirmation (Y/N) | Proceed to write or cancel |

### Remove Wizard Steps (--remove-aliases)

```text
[Start] → Scan profiles → [No aliases found → exit] 
                         → [Aliases found] → Step1_SelectProfiles → [Remove/WhatIf]
                                                                         ↓
[Escape] ─────────────────────────────────────────────────────→ [Cancelled]
```

| State | Input Widget | Output |
| ------- | ------------- | -------- |
| Step1_SelectProfiles | Checkbox list (multi-select, unchecked by default) | Selected profiles for removal |

---

## Relationships

```text
PowerShellVersion ──determines──→ ProfileLocation (4 per version)
                                        │
AliasConfig ──references──→ ProfileLocation (chosen target)
    │
    ├── contains ──→ AliasDefinition[] (root + sub-aliases)
    │
    └── produces ──→ Alias Block text (via AliasBlockGenerator)

AliasBlock ──parsed from──→ existing profile file content
```
