# towl 🦉

![TODOOwl](./docs/media/towl_trans_small.png)

A fast command-line tool that scans your codebase for TODO comments and outputs them in various formats (JSON, CSV, Markdown, TOML, and more).

## Features

- 🔍 **Smart Detection**: Finds TODO, FIXME, HACK, NOTE, and BUG comments
- 📁 **Multiple Output Formats**: JSON, CSV, Markdown, TOML, Terminal table
- 🎯 **Filtering**: Filter by TODO type
- ⚡ **Fast**: Built with Rust for maximum performance
- 🔧 **Configurable**: Customize file extensions, patterns, and exclusions
- 📍 **Context-Aware**: Shows surrounding code and function context

## Installation

```bash
cargo install towl
```

## Quick Start

```bash
# Scan current directory
towl scan

# Output to JSON file
towl scan -f json -o todos.json

# Filter by type
towl scan -t todo

# Initialize config
towl init

# Show current config
towl config
```

## Usage

```bash
towl scan [OPTIONS] [PATH]

Options:
  -f, --format <FORMAT>       Output format [default: terminal]
                              [possible values: table, json, csv, toml, markdown, terminal]
  -o, --output <OUTPUT>       Output file path (required for json, csv, toml, markdown)
  -t, --todo-type <TYPE>      Filter by TODO type
                              [possible values: todo, fixme, hack, note, bug]
  -v, --verbose               Enable verbose output

towl init [OPTIONS]

Options:
  -p, --path <PATH>           Config file path [default: .towl.toml]
  -F, --force                 Overwrite existing config file

towl config                   Show current configuration
```

## Configuration

Create a `.towl.toml` file in your project root (or run `towl init`):

```toml
[parsing]
file_extensions = ["rs", "toml", "json", "yaml", "yml", "sh", "bash"]
exclude_patterns = ["target/*", ".git/*"]
include_context_lines = 3

[github]
owner = "your-github-username"
repo = "your-repo-name"
```

The GitHub token can be set via the `TOWL_GITHUB_TOKEN` environment variable. Owner and repo are auto-detected from the git remote on `towl init`.

## License

MIT
