# towl 🦉

![TODOOwl](./docs/media/towl_trans_small.png)

A fast command-line tool that scans your codebase for TODO comments, lets you browse them in an interactive TUI, and optionally creates GitHub issues from them.

## Features

- **AI Validation**: Use `--ai` to validate TODOs with an LLM (Claude API, OpenAI API, or local CLI agents like Claude Code and Codex) -- filters stale TODOs and enriches GitHub issues
- **Interactive TUI**: Browse, filter, sort, and peek at TODOs in a full-screen terminal interface
- **GitHub Integration**: Create GitHub issues from selected TODOs and replace comments with issue links
- **Smart Detection**: Finds TODO, FIXME, HACK, NOTE, and BUG comments
- **Multiple Output Formats**: JSON, CSV, Markdown, TOML, terminal table (non-interactive mode)
- **Filtering & Sorting**: Filter by TODO type, sort by file, line, type, or priority
- **Fast**: Async I/O, concurrent file scanning, compiled regex patterns
- **Configurable**: Customise file extensions, patterns, and exclusions via `.towl.toml`
- **Context-Aware**: Shows surrounding code lines and enclosing function names

## Installation

```bash
cargo install towl
```

## Quick Start

```bash
# Scan current directory (opens interactive TUI)
towl scan

# Scan in non-interactive mode (CI/scripting)
towl scan -N

# Output to JSON file
towl scan -N -f json -o todos.json

# Filter by type
towl scan -N -t todo

# Create GitHub issues from TODOs
towl scan -N -g

# Preview GitHub issues without creating them
towl scan -N -g -n

# AI-validate TODOs (filters out stale ones)
towl scan -N --ai

# AI + GitHub: create issues for valid TODOs only
towl scan -N --ai -g

# Initialise config
towl init

# Show current config
towl config
```

## Usage

```bash
towl scan [OPTIONS] [PATH]

Options:
  -N, --non-interactive     Disable interactive TUI mode (for CI/scripting)
  -f, --format <FORMAT>     Output format (non-interactive only) [default: terminal]
                            [possible values: table, json, csv, toml, markdown, terminal]
  -o, --output <OUTPUT>     Output file path (required for json, csv, toml, markdown)
  -t, --todo-type <TYPE>    Filter by TODO type
                            [possible values: todo, fixme, hack, note, bug]
  -v, --verbose             Enable verbose output
  -g, --github              Create GitHub issues for found TODOs
  -n, --dry-run             Preview GitHub issues without creating them
      --ai                  Analyse TODOs with AI to validate relevance

towl init [OPTIONS]

Options:
  -p, --path <PATH>         Config file path [default: .towl.toml]
  -F, --force               Overwrite existing config file

towl config                 Show current configuration
```

## Interactive TUI

By default, `towl scan` opens an interactive terminal interface:

| Key | Action |
|-----|--------|
| `j` / `Down` | Move cursor down |
| `k` / `Up` | Move cursor up |
| `Space` | Toggle selection |
| `a` | Select all visible |
| `n` | Deselect all |
| `f` | Cycle type filter |
| `s` | Cycle sort field (file, line, type, priority) |
| `r` | Reverse sort order |
| `p` | Peek at source code around the TODO |
| `d` | Delete selected invalid TODOs (with `--ai`) |
| `Enter` | Confirm selection and create GitHub issues |
| `q` / `Esc` | Quit |

Use `--non-interactive` / `-N` to bypass the TUI for CI pipelines and scripting.

## Configuration

Create a `.towl.toml` file in your project root (or run `towl init`):

```toml
[parsing]
file_extensions = ["rs", "toml", "json", "yaml", "yml", "sh", "bash"]
exclude_patterns = ["target/*", ".git/*"]
include_context_lines = 3

[llm]
provider = "claude"           # "claude", "openai", "claude-code", or "codex"
model = "claude-opus-4-6"
```

GitHub owner and repo are always auto-detected from `git remote get-url origin` at runtime. Set secrets via environment variables (never stored in the config file):

| Variable | Description |
|----------|-------------|
| `TOWL_GITHUB_TOKEN` | GitHub personal access token |
| `TOWL_LLM_API_KEY` | LLM API key (Claude or OpenAI) |

See the [configuration guide](https://glottologist.github.io/towl/getting-started/configuration.html) for all options.

## License

MIT
