# Errors

towl uses typed errors throughout, built with `thiserror`. Each module defines its own error enum, and the top-level `TowlError` aggregates them.

## Error Hierarchy

```text
TowlError
├── TowlConfigError
├── TowlScannerError
│   └── TowlParserError
│       └── TowlCommentError
├── TowlOutputError
│   ├── FormatterError
│   └── WriterError
├── TowlGitHubError
├── TowlProcessorError
└── TowlTuiError
```

## `TowlError`

Top-level error type used by the CLI binary. All sub-error types convert automatically via `#[from]`.

```rust
pub enum TowlError {
    Config(TowlConfigError),
    Scanner(TowlScannerError),
    Output(TowlOutputError),
    GitHub(TowlGitHubError),
    Processor(TowlProcessorError),
    Tui(TowlTuiError),
}
```

## `TowlConfigError`

Errors during configuration loading, initialisation, and validation.

| Variant | Cause |
|---------|-------|
| `PathTraversalAttempt(PathBuf)` | Config path contains `..` |
| `ConfigAlreadyExists(PathBuf)` | `towl init` without `--force` on existing file |
| `WriteToFileError(PathBuf, io::Error)` | Failed to write config file |
| `UnableToParseToml(toml::ser::Error)` | TOML serialisation failure |
| `CouldNotCreateConfig(ConfigError)` | Config crate loading error |
| `GitRepoNotFound { message }` | Not inside a git repository |
| `GitRemoteNotFound { message }` | No `origin` remote |
| `GitInvalidUrl { url, message }` | Cannot parse owner/repo from remote URL |
| `TooManyConfigPatterns { field, count, max_allowed }` | Pattern array exceeds 100 entries |
| `ConfigValueTooLong { field, length, max_length }` | Config string exceeds 512 characters |
| `ContextLinesOutOfRange { value, min, max }` | Context lines outside 1..=50 |
| `RateLimitDelayTooHigh { value, max }` | Rate limit delay exceeds maximum |

## `TowlScannerError`

Errors during directory walking and file reading.

| Variant | Cause |
|---------|-------|
| `UnableToWalkFile(ignore::Error)` | Directory traversal error |
| `ParsingError(TowlParserError)` | Parser failure (propagated) |
| `UnableToReadFileAtPath(PathBuf, io::Error)` | File I/O error |
| `InvalidPath { path }` | Path could not be canonicalised |
| `FileTooLarge { path, size, max_allowed }` | File exceeds 10 MB |
| `TooManyTodos { path, count, max_allowed }` | File exceeds 10,000 TODOs |

## `TowlParserError`

Errors during regex compilation and TODO extraction.

| Variant | Cause |
|---------|-------|
| `InvalidRegexPattern(String, regex::Error)` | Regex failed to compile |
| `UnknownConfigPattern(TowlCommentError)` | Pattern matched but type unknown |
| `RegexGroupMissing` | Pattern lacks a capture group `(.*)` |
| `PatternTooLong(usize, usize)` | Pattern exceeds 256 characters |
| `TooManyTotalPatterns { count, max_allowed }` | Total patterns across all categories exceeds 50 |

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
| `SerializationError(String)` | JSON/TOML/CSV serialisation failure |
| `IntegerOverflow(usize)` | Count exceeds safe integer bounds |

## `WriterError`

Errors in output writing.

| Variant | Cause |
|---------|-------|
| `IoError(io::Error)` | File system I/O error |
| `PathTraversal(PathBuf)` | Output path contains `..` |

## `TowlGitHubError`

Errors from GitHub API interactions.

| Variant | Cause |
|---------|-------|
| `ApiError { message, source }` | General GitHub API failure |
| `AuthError` | 401 response -- invalid or expired token |
| `RateLimitExceeded { retry_after_secs }` | 403 with rate limit message |
| `IssueAlreadyExists { title }` | Duplicate detected before creation |
| `RepositoryNotFound { owner, repo }` | 404 response -- owner/repo not found |
| `MissingToken` | `TOWL_GITHUB_TOKEN` not set or empty |

## `TowlProcessorError`

Errors from replacing TODO comments with issue links in source files.

| Variant | Cause |
|---------|-------|
| `FileReadError(PathBuf, io::Error)` | Failed to read source file |
| `FileWriteError(PathBuf, io::Error)` | Failed to write modified file |
| `LineOutOfBounds { path, line, total_lines }` | TODO line number exceeds file length |
| `CommentPrefixNotFound { path, line }` | Column offset points past end of line |
| `PathOutsideRoot { path, root }` | File is outside the repository root |
| `InvalidIssueUrl { url }` | URL does not start with `https://github.com/` |

## `TowlTuiError`

Errors from the interactive terminal UI.

| Variant | Cause |
|---------|-------|
| `Io(io::Error)` | Terminal I/O error from crossterm or ratatui |

## Error Propagation

Errors propagate upward using `?` and `#[from]`:

```text
TowlCommentError  -->  TowlParserError    -->  TowlScannerError  -->  TowlError
FormatterError    -->  TowlOutputError     -->  TowlError
WriterError       -->  TowlOutputError     -->  TowlError
TowlGitHubError   ---------------------------->  TowlError
TowlProcessorError --------------------------->  TowlError
TowlTuiError      ---------------------------->  TowlError
```

All errors implement `std::fmt::Display` with human-readable messages and `std::error::Error` for standard error handling.
