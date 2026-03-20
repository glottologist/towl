# Introduction

**towl** is a fast command-line tool built in Rust that scans codebases for TODO comments. It provides an interactive TUI for browsing and managing TODOs, can create GitHub issues from them, and supports multiple output formats for CI/scripting. It detects TODO, FIXME, HACK, NOTE, and BUG comments across many languages, with configurable patterns, context-aware output, and robust resource limits.

## Key Features

- **Interactive TUI** -- Browse, filter, sort, and peek at TODOs in a full-screen terminal interface powered by ratatui
- **GitHub integration** -- Create GitHub issues from selected TODOs and automatically replace comments with issue links
- **Multi-language support** -- Scans Rust, Python, JavaScript, Go, Shell, and more via configurable comment prefixes and function patterns
- **Multiple output formats** -- JSON, CSV, Markdown, TOML, and terminal table (non-interactive mode)
- **Type filtering & sorting** -- Filter results by TODO type; sort by file, line, type, or priority
- **Context-aware** -- Captures surrounding code lines and enclosing function names
- **Configurable** -- Customise file extensions, exclude patterns, comment prefixes, and TODO patterns via `.towl.toml` (override with `--config` or `TOWL_CONFIG` env var)
- **AI validation** -- LLM-powered TODO analysis using Claude, OpenAI, or local CLI agents (Claude Code, Codex)
- **Safe by design** -- Path traversal protection, resource limits, symlink resolution, and secret handling for tokens
- **Fast** -- Concurrent file scanning, async I/O with tokio, compiled regex patterns, and static enum dispatch

## How It Works

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
                ┌────▼─────┐
                │  LLM      │  --ai: validates TODOs with AI (optional)
                └────┬─────┘
                     │
              ┌──────┴──────┐
              │             │
        ┌─────▼────┐  ┌────▼─────┐
        │   TUI     │  │  Output   │  Non-interactive: formats + writes
        │ (default) │  │  (-N)     │
        └─────┬────┘  └──────────┘
              │
        ┌─────▼────┐
        │ Processor │  Replaces TODOs with GitHub issue links
        └──────────┘
```

1. **Config** loads settings from `.towl.toml` (or a custom path via `--config` / `TOWL_CONFIG`), merges environment variables for GitHub and LLM integration
2. **Scanner** walks the directory tree using the `ignore` crate, scanning matching files concurrently with bounded parallelism
3. **Parser** reads each file, matches comment prefixes and TODO patterns via compiled regex, extracts context lines and function names
4. **LLM** (optional, `--ai`) validates each TODO with an AI model, classifying them as Valid, Invalid, or Uncertain
5. **TUI** (default) presents an interactive interface for browsing, filtering, and selecting TODOs to create as GitHub issues
6. **Output** (non-interactive) formats the collected `TodoComment` items into the requested format and writes to a file or stdout
7. **Processor** replaces TODO comments in source files with GitHub issue links after issues are created

## Quick Example

```bash
# Scan current directory (opens interactive TUI)
towl scan

# Non-interactive: output as terminal table
towl scan -N

# Non-interactive: output to JSON file
towl scan -N -f json -o todos.json

# Filter to only FIXME comments
towl scan -N -t fixme

# AI analysis: validate TODOs and filter out invalid ones
towl scan -N --ai

# Create GitHub issues
towl scan -N -g

# Show current configuration
towl config
```
