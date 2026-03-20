# Interactive TUI

By default, `towl scan` opens an interactive terminal interface powered by [ratatui](https://ratatui.rs). The TUI lets you browse, filter, sort, and peek at TODOs, then create GitHub issues from selected items.

## Launching

```bash
# Opens TUI with TODOs from current directory
towl scan

# Opens TUI with TODOs from a specific path
towl scan src/
```

To bypass the TUI (for CI/scripting), use `--non-interactive` / `-N`.

## Modes

The TUI has six modes:

### Browse

The main view. Displays all TODOs in a scrollable list with type, description, file path, and line number.

When launched with `--ai`, each row shows a validity indicator (`V`/`I`/`?`) and is colour-coded: green for valid, red for invalid, yellow for uncertain.

| Key | Action |
|-----|--------|
| `j` / `Down` | Move cursor down |
| `k` / `Up` | Move cursor up |
| `Space` | Toggle selection on current item |
| `a` | Select all visible TODOs |
| `n` | Deselect all |
| `f` | Cycle type filter (All, TODO, FIXME, HACK, NOTE, BUG) |
| `s` | Cycle sort field (File, Line, Type, Priority) |
| `r` | Reverse sort order |
| `p` | Open peek view for current TODO |
| `d` | Delete selected invalid TODOs (requires `--ai`) |
| `Enter` | Confirm selection and proceed to create GitHub issues |
| `q` / `Esc` | Quit |
| `Ctrl+C` | Force quit (works in any mode) |

### Peek

Shows the source code surrounding the selected TODO with syntax context. The TODO line is highlighted. When `--ai` is active, the AI Analysis section is displayed below the source code with the validity, confidence score, and reasoning. The reasoning text word-wraps to fit the popup width.

| Key | Action |
|-----|--------|
| `j` / `Down` | Scroll down |
| `k` / `Up` | Scroll up |
| `p` / `q` / `Esc` | Close peek and return to browse |

### Confirm

Appears after pressing `Enter` in browse mode with selected TODOs. Shows a summary of the TODOs that will be created as GitHub issues.

| Key | Action |
|-----|--------|
| `y` / `Enter` | Confirm and start creating issues |
| `n` / `q` / `Esc` | Cancel and return to browse |

### Creating

Displays a progress view while GitHub issues are being created. Shows the current phase (initialising client, loading existing issues, creating issues, replacing TODOs in files) and a progress counter.

No keyboard input is accepted during creation (except `Ctrl+C` to force quit).

### Done

Shows the results after issue creation completes -- number of issues created, any errors encountered. Press `q`, `Esc`, or `Enter` to exit.

### Delete Confirm (requires `--ai`)

Appears after pressing `d` in Browse mode with selected invalid TODOs. Lists the TODOs that will be removed from source files.

| Key | Action |
|-----|--------|
| `y` / `Enter` | Confirm and delete the TODO comment lines |
| `n` / `q` / `Esc` | Cancel and return to browse |

Only TODOs marked as Invalid by the AI are eligible for deletion.

## Workflow

1. Run `towl scan` to open the TUI (with `--ai`, a progress bar shows during analysis)
2. Browse the TODO list -- use `f` to filter by type, `s`/`r` to sort
3. Press `p` to peek at source code around a TODO
4. Select TODOs with `Space` (or `a` to select all visible)
5. Press `Enter` to review selected TODOs
6. Press `y` to create GitHub issues
7. towl creates the issues, skips duplicates, and replaces TODO comments with issue links in source files
