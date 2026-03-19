# Architecture

towl follows a pipeline architecture: Config -> Scanner -> Parser -> TUI / Output. Each stage is a separate module with clear boundaries and typed errors.

## Pipeline

```text
                ┌──────────┐
                │  Config   │  --config / TOWL_CONFIG / .towl.toml + env vars
                └────┬─────┘
                     │
                ┌────▼─────┐
                │  Scanner  │  Walks directory tree, scans files concurrently
                └────┬─────┘
                     │
                ┌────▼─────┐
                │  Parser   │  Matches comment prefixes + TODO patterns
                └────┬─────┘
                     │
              ┌──────┴──────┐
              │             │
        ┌─────▼────┐  ┌────▼─────┐
        │   TUI     │  │  Output   │  Non-interactive: formats + writes
        │ (default) │  │  (-N)     │
        └─────┬────┘  └──────────┘
              │
        ┌─────▼─────┐
        │ Processor  │  Replaces TODOs with GitHub issue links
        └───────────┘
```

## Module Boundaries

### Config (`src/lib/config/`)

- Resolves config file path: `--config` flag > `TOWL_CONFIG` env var > `.towl.toml`
- Loads config using the `config` crate
- Merges environment variable overrides (`TOWL_GITHUB_*`, `TOWL_LLM_*`)
- Discovers GitHub owner/repo from `git remote get-url origin`
- Validates pattern array sizes
- Produces `TowlConfig` containing `ParsingConfig` + `GitHubConfig` + `LlmConfig`

Submodules:
- `types.rs` -- `TowlConfig`, `ParsingConfig`, `GitHubConfig`
- `defaults.rs` -- Default values for config fields
- `display.rs` -- `Display` implementation for config tree view
- `newtypes.rs` -- `Owner` and `Repo` newtype wrappers
- `validation.rs` -- Config validation logic
- `git.rs` -- `GitRepoInfo` for parsing git remotes
- `error.rs` -- `TowlConfigError`

### Scanner (`src/lib/scanner/`)

- Accepts a `ParsingConfig` and a root path
- Walks the directory tree using the `ignore` crate (respects `.gitignore`)
- Filters files by extension and exclude patterns
- Scans files concurrently with bounded parallelism (up to 64 files)
- Reads files asynchronously via `tokio::fs`
- Enforces resource limits (file size, TODO counts, file counts)
- Delegates content parsing to the `Parser`
- Returns `ScanResult` with TODOs and scan metrics

Submodules:
- `types.rs` -- `Scanner` implementation
- `limits.rs` -- `ScanResult` and resource limit constants
- `walker.rs` -- Directory walker construction
- `error.rs` -- `TowlScannerError`

### Parser (`src/lib/parser/`)

- Compiles regex patterns once during construction
- Identifies comment lines via `comment_prefixes`
- Extracts TODO items via `todo_patterns`
- Captures context lines (configurable window, 1-50)
- Detects enclosing function names via `function_patterns`
- Produces `Vec<TodoComment>`

Submodules:
- `types.rs` -- `Parser` implementation
- `context.rs` -- Context line extraction logic
- `pattern.rs` -- Pattern compilation and matching
- `error.rs` -- `TowlParserError`

### TUI (`src/lib/tui/`)

- Full-screen terminal interface using ratatui and crossterm
- Browse, filter, sort, and peek at TODOs
- Select TODOs and create GitHub issues with progress tracking
- Replaces TODO comments in source files with issue links via the Processor

Submodules:
- `app.rs` -- `App` state machine and `AppMode` enum (Browse, Peek, Confirm, Creating, Done)
- `input.rs` -- Keyboard event handling and action dispatch
- `render.rs` -- UI rendering (list, peek popup, confirm dialog, progress view)
- `error.rs` -- `TowlTuiError`

### Processor (`src/lib/processor/`)

- Replaces TODO comments in source files with GitHub issue links
- Groups replacements by file for efficient batch processing
- Validates file paths stay within the repository root
- Returns `ProcessorResult` with counts and error details

Submodules:
- `types.rs` -- `Processor` and `ProcessorResult`
- `error.rs` -- `TowlProcessorError`

### GitHub (`src/lib/github/`)

- Creates GitHub issues from `TodoComment` items via the Octocrab API
- Loads existing issues to detect and skip duplicates
- Constructs issue titles and bodies with file/line metadata

### Output (`src/lib/output/`)

- Combines a `FormatterImpl` and a `WriterImpl`
- Groups TODOs by type before formatting
- Uses enum dispatch (not trait objects) for zero-cost abstraction

```text
Output
├── FormatterImpl (enum dispatch)
│   ├── CsvFormatter
│   ├── JsonFormatter
│   ├── MarkdownFormatter
│   ├── TableFormatter
│   └── TomlFormatter
└── WriterImpl (enum dispatch)
    ├── StdoutWriter
    └── FileWriter
```

## Key Design Decisions

### Enum Dispatch Over Trait Objects

Both `FormatterImpl` and `WriterImpl` use enum variants rather than `Box<dyn Trait>`. This provides:

- Static dispatch (no vtable overhead)
- Exhaustive matching at compile time
- Simpler lifetime management

### Regex Compilation Strategy

All regex patterns are compiled once during `Scanner::new()` / `Parser::new()` and reused for every file. This avoids per-file compilation overhead.

### Concurrent File Scanning

The scanner discovers all scannable files first, then scans them concurrently using `futures::stream::buffer_unordered` with a concurrency limit of 64. This provides significant speedup on large codebases while bounding resource usage.

### Async I/O

File reading uses `tokio::fs` for non-blocking I/O. The scanner is async, allowing integration into async applications. The CLI uses `#[tokio::main]`.

### TUI Event Loop

The TUI uses a synchronous event loop with crossterm polling. GitHub issue creation runs in a background tokio task, communicating progress back to the UI via an `mpsc` channel. This keeps the UI responsive during network operations.

### Error Type Hierarchy

Each module owns its error type. Errors propagate upward via `#[from]` conversions:

```text
TowlCommentError → TowlParserError → TowlScannerError → TowlError
FormatterError → TowlOutputError → TowlError
WriterError → TowlOutputError → TowlError
TowlProcessorError → TowlError
TowlTuiError → TowlError
```

### Newtype Pattern

`Owner` and `Repo` are newtype wrappers over `String`, preventing accidental misuse (e.g., passing an owner where a repo is expected).

### Secret Handling

The GitHub token is stored as `secrecy::SecretString`, which:

- Masks the value in `Debug` and `Display` output
- Zeroes memory on drop
- Prevents accidental logging

## Directory Layout

```text
src/
├── bin/
│   └── towl.rs              CLI binary
└── lib/
    ├── mod.rs                Library root
    ├── cli/
    │   └── mod.rs            Clap argument definitions
    ├── comment/
    │   ├── mod.rs
    │   ├── todo.rs           TodoType, TodoComment
    │   └── error.rs          TowlCommentError
    ├── config/
    │   ├── mod.rs
    │   ├── types.rs          TowlConfig, ParsingConfig, GitHubConfig
    │   ├── defaults.rs       Default config values
    │   ├── display.rs        Config Display implementation
    │   ├── newtypes.rs       Owner, Repo newtypes
    │   ├── validation.rs     Config validation
    │   ├── git.rs            GitRepoInfo
    │   └── error.rs          TowlConfigError
    ├── scanner/
    │   ├── mod.rs
    │   ├── types.rs          Scanner
    │   ├── limits.rs         ScanResult, resource limits
    │   ├── walker.rs         Directory walker construction
    │   └── error.rs          TowlScannerError
    ├── parser/
    │   ├── mod.rs
    │   ├── types.rs          Parser
    │   ├── context.rs        Context line extraction
    │   ├── pattern.rs        Pattern compilation
    │   └── error.rs          TowlParserError
    ├── github/
    │   ├── mod.rs
    │   ├── client.rs         GitHubClient
    │   ├── types.rs          CreatedIssue
    │   └── error.rs          TowlGitHubError
    ├── processor/
    │   ├── mod.rs
    │   ├── types.rs          Processor, ProcessorResult
    │   └── error.rs          TowlProcessorError
    ├── tui/
    │   ├── mod.rs             TUI entry point and event loop
    │   ├── app.rs             App state machine, AppMode
    │   ├── input.rs           Keyboard input handling
    │   ├── render.rs          UI rendering
    │   └── error.rs           TowlTuiError
    ├── output/
    │   ├── mod.rs             Output
    │   ├── error.rs           TowlOutputError
    │   ├── formatter/
    │   │   ├── mod.rs         FormatterImpl
    │   │   ├── error.rs       FormatterError
    │   │   └── formatters/
    │   │       ├── mod.rs     Formatter dispatch
    │   │       ├── csv.rs
    │   │       ├── json.rs
    │   │       ├── markdown.rs
    │   │       ├── table.rs
    │   │       └── toml.rs
    │   └── writer/
    │       ├── mod.rs         WriterImpl
    │       ├── error.rs       WriterError
    │       └── writers/
    │           ├── file.rs    FileWriter
    │           └── stdout.rs  StdoutWriter
    └── error/
        └── mod.rs             TowlError

tests/
├── integration/               Integration tests
├── property/                  Property-based tests
└── fixtures/                  Test fixtures
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing |
| `tokio` | Async runtime and file I/O |
| `serde` / `serde_json` / `toml` | Serialisation |
| `regex` | TODO pattern matching |
| `ignore` | Directory walking (respects `.gitignore`) |
| `thiserror` | Error type derivation |
| `secrecy` | Secret string handling |
| `config` | Configuration file loading |
| `octocrab` | GitHub API client |
| `ratatui` | Terminal UI framework |
| `crossterm` | Terminal input/output |
| `futures` | Async stream utilities |
| `proptest` | Property-based testing |
| `rstest` | Parameterised testing |
| `insta` | Snapshot testing |
