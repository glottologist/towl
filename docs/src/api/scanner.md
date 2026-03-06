# Scanner

The scanner walks a directory tree, filters files by extension and exclude patterns, reads content, and delegates to the parser for TODO extraction.

## `Scanner`

```rust
pub struct Scanner {
    parser: Parser,
    config: ParsingConfig,
}
```

### Constructor

```rust
pub fn new(config: ParsingConfig) -> Result<Self, TowlScannerError>
```

Creates a new scanner. Compiles all regex patterns from the config during construction so pattern errors are caught early.

### `scan`

```rust
pub async fn scan(&self, path: PathBuf) -> Result<ScanResult, TowlScannerError>
```

Recursively scans `path` for TODO comments. Returns a `ScanResult` on success.

**Behaviour:**

1. Validates the path (rejects path traversal)
2. Walks the directory using the `ignore` crate (respects `.gitignore`)
3. Filters files by extension (`file_extensions` config)
4. Skips files matching `exclude_patterns`
5. Skips files larger than `MAX_FILE_SIZE` (10 MB)
6. Reads and parses each file asynchronously via `tokio::fs`
7. Collects results until a resource limit is reached or the walk completes

**Errors:**

- `InvalidPath` -- Path contains traversal components (`..`)
- `FileTooLarge` -- File exceeds 10 MB
- `TooManyTodos` -- Single file exceeds 10,000 TODOs
- `TooManyFiles` -- Walk exceeds 100,000 files
- `UnableToReadFileAtPath` -- I/O error reading a specific file
- `UnableToWalkFile` -- Directory walk error
- `ParsingError` -- Regex or parsing failure (propagated from parser)

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

### Methods

```rust
pub const fn all_files_failed(&self) -> bool
```

Returns `true` when `files_scanned == 0` and `files_errored > 0`. Indicates a likely permissions or path issue where no files could be read.

```rust
pub const fn is_clean(&self) -> bool
```

Returns `true` when `todos` is empty and `files_errored == 0`. A clean scan with no issues.

## Resource Limits

| Constant | Value | Trigger |
|----------|-------|---------|
| `MAX_FILE_SIZE` | 10,485,760 bytes (10 MB) | File skipped |
| `MAX_TODO_COUNT` | 10,000 | Error for that file |
| `MAX_TOTAL_TODO_COUNT` | 100,000 | Scan stops, returns partial |
| `MAX_FILES_SCANNED` | 100,000 | Scan stops, returns partial |

## Example

```rust
use towl::config::ParsingConfig;
use towl::scanner::Scanner;
use std::path::PathBuf;

let config = ParsingConfig::default();
let scanner = Scanner::new(config)?;
let result = scanner.scan(PathBuf::from(".")).await?;

println!("Found {} TODOs in {} files", result.todos.len(), result.files_scanned);

if result.all_files_failed() {
    eprintln!("Warning: no files could be read");
}
```
