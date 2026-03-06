# Scanning for TODOs

The `towl scan` command walks a directory tree, reads each matching file, and extracts TODO-style comments using compiled regex patterns.

## Basic Usage

```bash
# Scan current directory
towl scan

# Scan a specific directory
towl scan src/

# Verbose output (file counts, timing)
towl scan -v
```

## How Scanning Works

1. **Directory walk** -- Uses the `ignore` crate to traverse the file tree, respecting `.gitignore` rules automatically
2. **Extension filter** -- Only files matching `file_extensions` in config are read (default: `rs`, `toml`, `json`, `yaml`, `yml`, `sh`, `bash`)
3. **Exclude patterns** -- Files matching `exclude_patterns` are skipped (default: `target/*`, `.git/*`)
4. **Content parsing** -- Each file is read and scanned for lines matching `comment_prefixes`, then checked against `todo_patterns`
5. **Context extraction** -- Surrounding lines and enclosing function names are captured

## Verbose Mode

The `-v` / `--verbose` flag prints scan metrics to stderr:

```bash
towl scan -v
```

```text
Files scanned: 42
Files skipped: 3
Files errored: 0
Scan duration: 12ms
```

## Filtering by Type

Restrict results to a single TODO type:

```bash
towl scan -t todo      # Only TODO comments
towl scan -t fixme     # Only FIXME comments
towl scan -t hack      # Only HACK comments
towl scan -t note      # Only NOTE comments
towl scan -t bug       # Only BUG comments
```

The filter value is case-insensitive on the command line but stored lowercase internally.

## Combining Options

Options compose freely:

```bash
# Scan src/, output FIXME comments as JSON to a file, verbose
towl scan src/ -t fixme -f json -o fixmes.json -v
```

## Resource Limits

towl enforces hard limits to prevent runaway scans:

| Limit | Value | Purpose |
|-------|-------|---------|
| Max file size | 10 MB | Skips binary/generated files |
| Max TODOs per file | 10,000 | Prevents single-file explosion |
| Max total TODOs | 100,000 | Caps overall result set |
| Max files scanned | 100,000 | Bounds directory walk |

When a limit is hit, scanning stops gracefully and returns the results collected so far.

## Scan Result

The scan produces a `ScanResult` containing:

- **todos** -- The list of extracted `TodoComment` items
- **files_scanned** -- Number of files successfully read
- **files_skipped** -- Number of files skipped (wrong extension, excluded, too large)
- **files_errored** -- Number of files that failed to read (permissions, encoding)
- **duration** -- Wall-clock time for the scan

Two convenience checks:

- `all_files_failed()` -- Returns `true` when no files were scanned but errors occurred (likely a permissions or path issue)
- `is_clean()` -- Returns `true` when zero TODOs were found and zero files errored

## Path Safety

- **Path traversal** -- Paths containing `..` components are rejected
- **Symlink resolution** -- Symlinks are resolved before processing to prevent escape from the scan root
- **.gitignore** -- Respected automatically via the `ignore` crate
