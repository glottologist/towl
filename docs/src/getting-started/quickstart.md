# Quick Start

## 1. Initialise Configuration

Inside a git repository with a GitHub remote:

```bash
towl init
```

This creates `.towl.toml` with sensible defaults and auto-detects the GitHub owner/repo from `git remote get-url origin`.

If `.towl.toml` already exists, use `--force` to overwrite:

```bash
towl init --force
```

## 2. Scan for TODOs

```bash
# Scan the current directory (default)
towl scan

# Scan a specific path
towl scan src/

# Enable verbose output (file counts, timing)
towl scan -v
```

## 3. Choose an Output Format

```bash
# Terminal table (default)
towl scan

# JSON file
towl scan -f json -o todos.json

# CSV file
towl scan -f csv -o todos.csv

# Markdown file
towl scan -f markdown -o todos.md

# TOML file
towl scan -f toml -o todos.toml
```

> **Note:** File-based formats (`json`, `csv`, `toml`, `markdown`) require the `-o` flag with a matching file extension. Terminal/table formats always output to stdout.

## 4. Filter by Type

```bash
# Only TODO comments
towl scan -t todo

# Only FIXME comments
towl scan -t fixme

# Only BUG comments
towl scan -t bug
```

Available types: `todo`, `fixme`, `hack`, `note`, `bug`

## 5. View Configuration

```bash
towl config
```

Displays a tree view of all active settings including file extensions, exclude patterns, comment prefixes, TODO patterns, and GitHub configuration.
