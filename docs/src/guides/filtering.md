# Filtering

towl supports filtering scan results by TODO type using the `-t` / `--todo-type` flag.

## Filter by Type

```bash
towl scan -t todo      # Only TODO comments
towl scan -t fixme     # Only FIXME comments
towl scan -t hack      # Only HACK comments
towl scan -t note      # Only NOTE comments
towl scan -t bug       # Only BUG comments
```

The filter value is case-insensitive -- `TODO`, `todo`, and `Todo` all work.

## Available Types

towl recognises five built-in TODO types:

| Type | Matches | Typical use |
|------|---------|-------------|
| `todo` | `TODO:` | Planned work |
| `fixme` | `FIXME:` | Known broken code |
| `hack` | `HACK:` | Temporary workarounds |
| `note` | `NOTE:` | Important context |
| `bug` | `BUG:` | Known defects |

Each type is matched via the corresponding regex pattern in the `todo_patterns` configuration. The default patterns are case-insensitive (`(?i)`).

## Combining with Output Formats

Filtering works with any output format:

```bash
# FIXMEs as JSON
towl scan -t fixme -f json -o fixmes.json

# BUGs as Markdown
towl scan -t bug -f markdown -o bugs.md

# NOTEs in terminal table
towl scan -t note
```

## Without Filtering

When no `-t` flag is provided, all recognised types are included in the output. The results are grouped by type in all formats.

## Custom Patterns

You can add custom TODO patterns in `.towl.toml`. Each pattern must contain a capture group `(.*)` for extracting the description:

```toml
[parsing]
todo_patterns = [
    "(?i)\\bTODO:\\s*(.*)",
    "(?i)\\bFIXME:\\s*(.*)",
    "(?i)\\bHACK:\\s*(.*)",
    "(?i)\\bNOTE:\\s*(.*)",
    "(?i)\\bBUG:\\s*(.*)",
    "(?i)\\bXXX:\\s*(.*)",
]
```

> **Note:** Custom patterns extend the set of matched comments but do not add new filter types to `-t`. The built-in five types are always available for filtering.
