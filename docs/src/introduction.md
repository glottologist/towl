# Introduction

**towl** is a fast command-line tool built in Rust that scans codebases for TODO comments and outputs them in multiple formats. It detects TODO, FIXME, HACK, NOTE, and BUG comments across many languages, with configurable patterns, context-aware output, and robust resource limits.

## Key Features

- **Multi-language support** -- Scans Rust, Python, JavaScript, Go, Shell, and more via configurable comment prefixes and function patterns
- **Multiple output formats** -- JSON, CSV, Markdown, TOML, and terminal table
- **Type filtering** -- Filter results by TODO type (todo, fixme, hack, note, bug)
- **Context-aware** -- Captures surrounding code lines and enclosing function names
- **Configurable** -- Customise file extensions, exclude patterns, comment prefixes, and TODO patterns via `.towl.toml`
- **Safe by design** -- Path traversal protection, resource limits, symlink resolution, and secret handling for GitHub tokens
- **Fast** -- Async I/O with tokio, compiled regex patterns, and static enum dispatch

## How It Works

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

1. **Config** loads settings from `.towl.toml` (with defaults), merges environment variables for GitHub integration
2. **Scanner** walks the directory tree using the `ignore` crate, filtering files by extension and exclude patterns
3. **Parser** reads each file, matches comment prefixes and TODO patterns via compiled regex, extracts context lines and function names
4. **Output** formats the collected `TodoComment` items into the requested format and writes to a file or stdout

## Quick Example

```bash
# Scan current directory, output as terminal table
towl scan

# Output to JSON file
towl scan -f json -o todos.json

# Filter to only FIXME comments
towl scan -t fixme

# Show current configuration
towl config
```
