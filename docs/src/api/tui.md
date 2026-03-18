# TUI

The TUI module provides an interactive terminal interface for browsing, filtering, and acting on TODO comments. Built on [ratatui](https://ratatui.rs) and [crossterm](https://github.com/crossterm-rs/crossterm).

## `run`

```rust
pub fn run(
    todos: Vec<TodoComment>,
    github_config: &GitHubConfig,
    repo_root: &Path,
) -> Result<(), TowlTuiError>
```

Launches the interactive TUI. Takes ownership of the terminal (raw mode, alternate screen). Terminal state is always restored on exit, even on error.

**Errors:**

- `TowlTuiError::Io` -- Terminal I/O failure

## `App`

```rust
pub struct App {
    // private fields
}
```

Core TUI application state. Manages the TODO list, selection set, cursor position, filtering, sorting, and mode transitions.

### Constructor

```rust
pub fn new(todos: Vec<TodoComment>) -> Self
```

Creates a new app with all TODOs visible, no selection, sorted by file path.

### State Accessors

| Method | Returns | Description |
|--------|---------|-------------|
| `todos()` | `&[TodoComment]` | Full TODO list |
| `filtered_indices()` | `&[usize]` | Indices into `todos()` after filtering/sorting |
| `cursor()` | `usize` | Current cursor position in filtered list |
| `filter_type()` | `Option<TodoType>` | Active type filter (`None` = show all) |
| `sort_field()` | `SortField` | Current sort field |
| `sort_ascending()` | `bool` | Sort direction |
| `is_selected(idx)` | `bool` | Whether a TODO index is selected |
| `selected_count()` | `usize` | Number of selected TODOs |
| `selected_todos()` | `Vec<TodoComment>` | Cloned copies of selected TODOs |
| `mode()` | `&AppMode` | Current UI mode |

### Navigation

| Method | Effect |
|--------|--------|
| `move_up()` | Move cursor up (clamped to 0) |
| `move_down()` | Move cursor down (clamped to list end) |

### Selection

| Method | Effect |
|--------|--------|
| `toggle_select()` | Toggle selection on cursor item |
| `select_all_visible()` | Select all items in filtered view |
| `deselect_all()` | Clear all selections |

### Filtering and Sorting

| Method | Effect |
|--------|--------|
| `cycle_filter()` | Cycle: All -> TODO -> FIXME -> HACK -> NOTE -> BUG -> All |
| `cycle_sort()` | Cycle: File -> Line -> Priority -> Type -> File |
| `reverse_sort()` | Toggle ascending/descending |

### Mode Transitions

| Method | Transition |
|--------|-----------|
| `enter_confirm()` | Browse -> Confirm (requires selection) |
| `cancel_confirm()` | Confirm -> Browse |
| `start_creating()` | Confirm -> Creating |
| `finish_creating()` | Creating -> Done |
| `enter_peek()` | Browse -> Peek (loads source context) |
| `exit_peek()` | Peek -> Browse |

## `AppMode`

```rust
pub enum AppMode {
    Browse,
    Peek(PeekState),
    Confirm,
    Creating(CreatingState),
    Done(DoneState),
}
```

The current UI mode determines which view is rendered and which keys are active.

| Mode | View | Input |
|------|------|-------|
| `Browse` | Scrollable TODO list | Navigate, select, filter, sort, peek |
| `Peek` | Source code overlay around a TODO | Scroll, dismiss |
| `Confirm` | Summary of selected TODOs | Confirm or cancel |
| `Creating` | Progress indicator during issue creation | None (Ctrl+C to abort) |
| `Done` | Results summary (issues created, errors) | Dismiss to exit |

## `SortField`

```rust
pub enum SortField {
    File,
    Line,
    Priority,
    Type,
}
```

Field used to sort the TODO list. Cycle with the `s` key in Browse mode.

- **File** -- Sort by file path, then by line number within each file
- **Line** -- Sort by line number globally
- **Priority** -- Sort by TODO type priority (Bug=1, Fixme=2, Hack=3, Todo=4, Note=5)
- **Type** -- Sort alphabetically by type name

## Supporting Types

### `PeekState`

```rust
pub struct PeekState {
    pub lines: Vec<(usize, String)>,
    pub file: String,
    pub todo_line: usize,
    pub scroll: usize,
}
```

State for the source-code peek overlay. Contains numbered source lines around the TODO, with scroll position.

### `CreatingState`

```rust
pub struct CreatingState {
    pub phase: String,
    pub progress: usize,
    pub total: usize,
    pub errors: Vec<String>,
    pub created_issues: Vec<CreatedIssue>,
}
```

State tracked during background GitHub issue creation. Updated via channel messages from the spawned task.

### `DoneState`

```rust
pub struct DoneState {
    pub created_issues: Vec<CreatedIssue>,
    pub errors: Vec<String>,
}
```

Final state after issue creation completes, showing results and any errors.

## `Action`

```rust
pub enum Action {
    Continue,
    Quit,
}
```

Result of processing a keyboard event. `Continue` keeps the event loop running; `Quit` exits the TUI.

## `handle_input`

```rust
pub fn handle_input(
    app: &mut App,
    timeout: std::time::Duration,
) -> std::io::Result<Action>
```

Polls for keyboard input and dispatches to mode-specific handlers. Returns `Action::Quit` on `q`, `Esc` (in appropriate modes), or `Ctrl+C`.

## Errors

```rust
pub enum TowlTuiError {
    Io(std::io::Error),
}
```

Terminal I/O errors from crossterm or ratatui operations.
