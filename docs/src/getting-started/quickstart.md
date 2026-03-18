# Quick Start

## 1. Initialise Configuration

Inside a git repository with a GitHub remote:

```bash
towl init
```

This creates `.towl.toml` with sensible defaults. GitHub owner/repo are auto-detected from `git remote get-url origin` at runtime (not stored in the config file).

If `.towl.toml` already exists, use `--force` to overwrite:

```bash
towl init --force
```

## 2. Scan for TODOs (Interactive)

```bash
# Scan the current directory (opens TUI)
towl scan

# Scan a specific path
towl scan src/
```

The interactive TUI lets you browse, filter, sort, peek at source code, and create GitHub issues from selected TODOs.

## 3. Scan for TODOs (Non-Interactive)

Use `--non-interactive` / `-N` for CI pipelines and scripting:

```bash
# Terminal table output
towl scan -N

# Enable verbose output (file counts, timing)
towl scan -N -v
```

## 4. Choose an Output Format

Non-interactive mode supports multiple output formats:

```bash
# Terminal table (default)
towl scan -N

# JSON file
towl scan -N -f json -o todos.json

# CSV file
towl scan -N -f csv -o todos.csv

# Markdown file
towl scan -N -f markdown -o todos.md

# TOML file
towl scan -N -f toml -o todos.toml
```

> **Note:** File-based formats (`json`, `csv`, `toml`, `markdown`) require the `-o` flag with a matching file extension. Terminal/table formats always output to stdout.

## 5. Filter by Type

```bash
# Only TODO comments
towl scan -N -t todo

# Only FIXME comments
towl scan -N -t fixme

# Only BUG comments
towl scan -N -t bug
```

Available types: `todo`, `fixme`, `hack`, `note`, `bug`

## 6. Create GitHub Issues

Set your GitHub token:

```bash
export TOWL_GITHUB_TOKEN=ghp_your_token_here
```

Then create issues from TODOs:

```bash
# Create GitHub issues (non-interactive)
towl scan -N -g

# Preview issues without creating them
towl scan -N -g -n
```

In interactive mode, select TODOs with `Space` and press `Enter` to create issues.

## 7. View Configuration

```bash
towl config
```

Displays a tree view of all active settings including file extensions, exclude patterns, comment prefixes, TODO patterns, and GitHub configuration.
