# Output

The output module combines a formatter and a writer to produce scan results in the requested format and destination.

## `Output`

```rust
pub struct Output {
    writer: WriterImpl,
    formatter: FormatterImpl,
}
```

### Constructor

```rust
pub fn new(
    output_format: OutputFormat,
    output_path: Option<PathBuf>,
) -> Result<Self, TowlOutputError>
```

Creates an output handler by selecting the appropriate formatter and writer.

**Format-to-writer mapping:**

| Format | Writer | Output path |
|--------|--------|-------------|
| `Table` / `Terminal` | `StdoutWriter` | Must be `None` |
| `Json` | `FileWriter` | Required, must end in `.json` |
| `Csv` | `FileWriter` | Required, must end in `.csv` |
| `Toml` | `FileWriter` | Required, must end in `.toml` |
| `Markdown` | `FileWriter` | Required, must end in `.md` |

### `save`

```rust
pub async fn save(&self, todos: &[TodoComment]) -> Result<(), TowlOutputError>
```

Formats the TODOs and writes them to the destination. TODOs are grouped by type before formatting.

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

Used as a CLI argument via `clap::ValueEnum`. `Table` and `Terminal` are treated identically.

## Formatter Dispatch

Internally, `FormatterImpl` is an enum that dispatches to the correct formatter without dynamic dispatch:

```rust
pub(crate) enum FormatterImpl {
    Csv(CsvFormatter),
    Json(JsonFormatter),
    Markdown(MarkdownFormatter),
    Table(TableFormatter),
    Toml(TomlFormatter),
}
```

Each formatter implements the internal `Formatter` trait:

```rust
pub(crate) trait Formatter {
    fn format(
        &self,
        todos: &HashMap<&TodoType, Vec<&TodoComment>>,
        total_count: usize,
    ) -> Result<Vec<String>, FormatterError>;
}
```

## Writer Dispatch

`WriterImpl` dispatches between stdout and file output:

```rust
pub(crate) enum WriterImpl {
    Stdout(StdoutWriter),
    File(FileWriter),
}
```

Each writer implements the internal `Writer` trait:

```rust
pub(crate) trait Writer {
    async fn write(&self, content: Vec<String>) -> Result<(), WriterError>;
}
```

### `FileWriter`

Validates the output path on construction:
- Rejects path traversal (`..` components)
- Resolves symlinks before writing

### `StdoutWriter`

Writes each formatted line to stdout followed by a newline.

## Errors

### `TowlOutputError`

```rust
pub enum TowlOutputError {
    InvalidOutputPath(String),
    UnableToFormatTodos(FormatterError),
    UnableToWriteTodos(WriterError),
}
```

### `FormatterError`

```rust
pub enum FormatterError {
    SerializationError(String),
    IntegerOverflow(usize),
}
```

### `WriterError`

```rust
pub enum WriterError {
    IoError(std::io::Error),
    PathTraversal(PathBuf),
}
```
