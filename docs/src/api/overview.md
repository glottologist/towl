# API Overview

towl is structured as a library (`towl` crate) with a thin binary wrapper. The library exposes modules for configuration, scanning, parsing, output, and error handling.

## Module Map

```text
towl (lib)
├── cli          Command-line argument parsing (clap)
├── comment      TODO types and comment structures
│   ├── todo     TodoType enum, TodoComment struct
│   └── error    TowlCommentError
├── config       Configuration loading and validation
│   ├── types    TowlConfig, ParsingConfig, GitHubConfig, Owner, Repo
│   ├── git      GitRepoInfo (git remote discovery)
│   └── error    TowlConfigError
├── scanner      Directory walking and file filtering
│   ├── types    Scanner, ScanResult
│   └── error    TowlScannerError
├── parser       Regex-based TODO extraction
│   ├── types    Parser, Pattern
│   └── error    TowlParserError
├── output       Formatting and writing results
│   ├── formatter
│   │   ├── formatters   CsvFormatter, JsonFormatter, MarkdownFormatter,
│   │   │                TableFormatter, TomlFormatter
│   │   └── error        FormatterError
│   ├── writer
│   │   ├── writers      StdoutWriter, FileWriter
│   │   └── error        WriterError
│   └── error            TowlOutputError
└── error        Top-level TowlError (aggregates all error types)
```

## Data Flow

```text
TowlConfig ──► Scanner ──► Parser ──► Output
   │              │            │          │
   │              │            │          ├─ FormatterImpl (enum dispatch)
   │              │            │          └─ WriterImpl (enum dispatch)
   │              │            │
   │              │            └─ Vec<TodoComment>
   │              │
   │              └─ ScanResult { todos, files_scanned, ... }
   │
   └─ ParsingConfig + GitHubConfig
```

## Key Types

| Type | Module | Purpose |
|------|--------|---------|
| `TowlConfig` | `config` | Top-level configuration container |
| `ParsingConfig` | `config` | File extensions, patterns, context lines |
| `GitHubConfig` | `config` | Owner, repo, token |
| `Scanner` | `scanner` | Directory walk + file filtering |
| `ScanResult` | `scanner` | Structured scan output with metrics |
| `Parser` | `parser` | Regex-based TODO extraction |
| `TodoComment` | `comment` | A single extracted TODO item |
| `TodoType` | `comment` | Enum: Todo, Fixme, Hack, Note, Bug |
| `Output` | `output` | Formatter + writer combination |
| `TowlError` | `error` | Top-level error aggregating all sub-errors |

## Error Hierarchy

```text
TowlError
├── TowlConfigError    Config loading, TOML parsing, git discovery
├── TowlScannerError   File walk, I/O, resource limits
│   └── TowlParserError   Regex compilation, pattern validation
└── TowlOutputError    Formatting, file writing
    ├── FormatterError    Serialization failures
    └── WriterError       I/O, path traversal
```

All error types use `thiserror` for `Display` and `Error` trait implementations. Conversion between levels uses `#[from]` attributes for ergonomic `?` propagation.

## Constants

| Name | Value | Module | Purpose |
|------|-------|--------|---------|
| `MAX_FILE_SIZE` | 10 MB | scanner | Skip oversized files |
| `MAX_TODO_COUNT` | 10,000 | scanner | Per-file TODO cap |
| `MAX_TOTAL_TODO_COUNT` | 100,000 | scanner | Global TODO cap |
| `MAX_FILES_SCANNED` | 100,000 | scanner | Directory walk cap |
| `MAX_PATTERN_LENGTH` | 256 chars | parser | Regex length limit |
| `REGEX_SIZE_LIMIT` | 256 KB | parser | Compiled regex size limit |
| `MAX_TOTAL_PATTERNS` | 50 | parser | Total patterns across all categories |
| `MAX_CONFIG_PATTERNS` | 100 | config | Per-field pattern array cap |
| `DEFAULT_CONFIG_PATH` | `.towl.toml` | config | Default config file |
