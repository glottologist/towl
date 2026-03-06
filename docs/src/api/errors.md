# Errors

towl uses typed errors throughout, built with `thiserror`. Each module defines its own error enum, and the top-level `TowlError` aggregates them.

## Error Hierarchy

```text
TowlError
├── TowlConfigError
├── TowlScannerError
│   └── TowlParserError
│       └── TowlCommentError
└── TowlOutputError
    ├── FormatterError
    └── WriterError
```

## `TowlError`

Top-level error type used by the CLI binary.

```rust
pub enum TowlError {
    Config(TowlConfigError),
    Scanner(TowlScannerError),
    Output(TowlOutputError),
}
```

All sub-error types convert automatically via `#[from]`.

## `TowlConfigError`

Errors during configuration loading, initialization, and validation.

| Variant | Cause |
|---------|-------|
| `PathTraversalAttempt(PathBuf)` | Config path contains `..` |
| `ConfigAlreadyExists(PathBuf)` | `towl init` without `--force` on existing file |
| `WriteToFileError(PathBuf, io::Error)` | Failed to write config file |
| `UnableToParseToml(toml::ser::Error)` | TOML serialization failure |
| `CouldNotCreateConfig(ConfigError)` | Config crate loading error |
| `GitRepoNotFound { message }` | Not inside a git repository |
| `GitRemoteNotFound { message }` | No `origin` remote |
| `GitInvalidUrl { url, message }` | Cannot parse owner/repo from remote URL |
| `TooManyConfigPatterns { field, count, max_allowed }` | Pattern array exceeds 100 entries |

## `TowlScannerError`

Errors during directory walking and file reading.

| Variant | Cause |
|---------|-------|
| `UnableToWalkFile(ignore::Error)` | Directory traversal error |
| `ParsingError(TowlParserError)` | Parser failure (propagated) |
| `UnableToReadFileAtPath(PathBuf, io::Error)` | File I/O error |
| `InvalidPath { path }` | Path contains traversal components |
| `FileTooLarge { path, size, max_allowed }` | File exceeds 10 MB |
| `TooManyTodos { path, count, max_allowed }` | File exceeds 10,000 TODOs |
| `TooManyFiles { count, max_allowed }` | Scan exceeds 100,000 files |

## `TowlParserError`

Errors during regex compilation and TODO extraction.

| Variant | Cause |
|---------|-------|
| `InvalidRegexPattern(String, regex::Error)` | Regex failed to compile |
| `UnknownConfigPattern(TowlCommentError)` | Pattern matched but type unknown |
| `RegexGroupMissing` | Pattern lacks a capture group `(.*)` |
| `PatternTooLong(usize, usize)` | Pattern exceeds 256 characters |

## `TowlCommentError`

Errors in comment type resolution.

| Variant | Cause |
|---------|-------|
| `UnknownTodoType { comment }` | String does not map to a known `TodoType` |

## `TowlOutputError`

Errors during formatting and writing.

| Variant | Cause |
|---------|-------|
| `InvalidOutputPath(String)` | Missing/wrong extension, terminal format with file path |
| `UnableToFormatTodos(FormatterError)` | Formatter failure |
| `UnableToWriteTodos(WriterError)` | Writer failure |

## `FormatterError`

Errors in output formatting.

| Variant | Cause |
|---------|-------|
| `SerializationError(String)` | JSON/TOML/CSV serialization failure |
| `IntegerOverflow(usize)` | Count exceeds safe integer bounds |

## `WriterError`

Errors in output writing.

| Variant | Cause |
|---------|-------|
| `IoError(io::Error)` | File system I/O error |
| `PathTraversal(PathBuf)` | Output path contains `..` |

## Error Propagation

Errors propagate upward using `?` and `#[from]`:

```text
TowlCommentError  ──►  TowlParserError  ──►  TowlScannerError  ──►  TowlError
FormatterError    ──►  TowlOutputError   ──►  TowlError
WriterError       ──►  TowlOutputError   ──►  TowlError
```

All errors implement `std::fmt::Display` with human-readable messages and `std::error::Error` for standard error handling.
