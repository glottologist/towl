# Types

Core data types used across the towl library.

## `TodoType`

```rust
pub enum TodoType {
    Todo,
    Fixme,
    Hack,
    Note,
    Bug,
}
```

Represents the category of a TODO comment.

### Display

| Variant | Display |
|---------|---------|
| `Todo` | `TODO` |
| `Fixme` | `FIXME` |
| `Hack` | `HACK` |
| `Note` | `NOTE` |
| `Bug` | `BUG` |

### `as_filter_str`

```rust
pub const fn as_filter_str(&self) -> &'static str
```

Returns the lowercase filter string used for CLI filtering:

| Variant | Filter string |
|---------|---------------|
| `Todo` | `"todo"` |
| `Fixme` | `"fixme"` |
| `Hack` | `"hack"` |
| `Note` | `"note"` |
| `Bug` | `"bug"` |

### Conversions

- `TryFrom<&str>` -- Case-insensitive conversion from string
- `clap::ValueEnum` -- CLI argument parsing

### Trait Implementations

`Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`, `ValueEnum`

## `TodoComment`

```rust
pub struct TodoComment {
    pub id: String,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub column_start: usize,
    pub column_end: usize,
    pub todo_type: TodoType,
    pub original_text: String,
    pub description: String,
    pub context_lines: Vec<String>,
    pub function_context: Option<String>,
}
```

A single TODO comment extracted from a source file.

| Field | Description |
|-------|-------------|
| `id` | Unique identifier (generated per extraction) |
| `file_path` | Path to the source file |
| `line_number` | 1-based line number |
| `column_start` | 0-based start column of the TODO marker |
| `column_end` | 0-based end column of the TODO marker |
| `todo_type` | Category (`Todo`, `Fixme`, etc.) |
| `original_text` | The full original comment line |
| `description` | Extracted description text after the marker |
| `context_lines` | Surrounding source lines (configurable window) |
| `function_context` | Enclosing function name, if detected |

### Trait Implementations

`Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`

## `ScanResult`

```rust
pub struct ScanResult {
    pub todos: Vec<TodoComment>,
    pub files_scanned: usize,
    pub files_skipped: usize,
    pub files_errored: usize,
    pub duration: std::time::Duration,
}
```

Returned by `Scanner::scan()`. See [Scanner](./scanner.md) for details.

## `Owner` / `Repo`

Newtype wrappers for GitHub owner and repository names:

```rust
pub struct Owner(String);
pub struct Repo(String);
```

Both provide `new(impl Into<String>)` and `Display`. See [Config](./config.md) for details.

## `OutputFormat`

```rust
pub enum OutputFormat {
    Table,
    Json,
    Csv,
    Toml,
    Markdown,
    Terminal,
}
```

CLI-facing enum for selecting output format. `Table` and `Terminal` produce identical output. See [Output](./output.md) for details.
