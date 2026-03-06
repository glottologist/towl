# Architecture

towl follows a pipeline architecture: Config -> Scanner -> Parser -> Output. Each stage is a separate module with clear boundaries and typed errors.

## Pipeline

```text
                ┌──────────┐
                │  Config   │  .towl.toml + env vars + git remote
                └────┬─────┘
                     │
                ┌────▼─────┐
                │  Scanner  │  Walks directory tree, filters by extension
                └────┬─────┘
                     │
                ┌────▼─────┐
                │  Parser   │  Matches comment prefixes + TODO patterns
                └────┬─────┘
                     │
                ┌────▼─────┐
                │  Output   │  Formats (JSON/CSV/...) + Writes (file/stdout)
                └──────────┘
```

## Module Boundaries

### Config (`src/lib/config/`)

- Loads `.towl.toml` using the `config` crate
- Merges environment variable overrides (`TOWL_GITHUB_*`)
- Discovers GitHub owner/repo from `git remote get-url origin`
- Validates pattern array sizes
- Produces `TowlConfig` containing `ParsingConfig` + `GitHubConfig`

### Scanner (`src/lib/scanner/`)

- Accepts a `ParsingConfig` and a root path
- Walks the directory tree using the `ignore` crate (respects `.gitignore`)
- Filters files by extension and exclude patterns
- Reads files asynchronously via `tokio::fs`
- Enforces resource limits (file size, TODO counts, file counts)
- Delegates content parsing to the `Parser`
- Returns `ScanResult` with TODOs and scan metrics

### Parser (`src/lib/parser/`)

- Compiles regex patterns once during construction
- Identifies comment lines via `comment_prefixes`
- Extracts TODO items via `todo_patterns`
- Captures context lines (configurable window, 1-50)
- Detects enclosing function names via `function_patterns`
- Produces `Vec<TodoComment>`

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

### Async I/O

File reading uses `tokio::fs` for non-blocking I/O. The scanner is async, allowing integration into async applications. The CLI uses `#[tokio::main]`.

### Error Type Hierarchy

Each module owns its error type. Errors propagate upward via `#[from]` conversions:

```text
TowlCommentError → TowlParserError → TowlScannerError → TowlError
FormatterError → TowlOutputError → TowlError
WriterError → TowlOutputError → TowlError
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
    │   ├── git.rs            GitRepoInfo
    │   └── error.rs          TowlConfigError
    ├── scanner/
    │   ├── mod.rs
    │   ├── types.rs          Scanner, ScanResult
    │   └── error.rs          TowlScannerError
    ├── parser/
    │   ├── mod.rs
    │   ├── types.rs          Parser, Pattern
    │   └── error.rs          TowlParserError
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
└── property/                  Property-based tests
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing |
| `tokio` | Async runtime and file I/O |
| `serde` / `serde_json` / `toml` | Serialization |
| `regex` | TODO pattern matching |
| `ignore` | Directory walking (respects `.gitignore`) |
| `thiserror` | Error type derivation |
| `secrecy` | Secret string handling |
| `config` | Configuration file loading |
| `proptest` | Property-based testing |
| `rstest` | Parameterized testing |
