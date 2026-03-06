# Output Formats

towl supports five output formats. Terminal-based formats write to stdout; file-based formats require the `-o` flag with a matching file extension.

## Terminal / Table (default)

```bash
towl scan
# or explicitly:
towl scan -f table
towl scan -f terminal
```

Renders an ASCII table to stdout:

```text
┌──────┬─────────────────────────┬──────────────────┬──────┬──────────┐
│ Type │ Description             │ File             │ Line │ Function │
├──────┼─────────────────────────┼──────────────────┼──────┼──────────┤
│ TODO │ Implement caching       │ src/lib/cache.rs │   42 │ process  │
│ FIXME│ Handle timeout          │ src/lib/net.rs   │  108 │ connect  │
└──────┴─────────────────────────┴──────────────────┴──────┴──────────┘
```

> **Note:** `table` and `terminal` are aliases -- both produce the same output.

## JSON

```bash
towl scan -f json -o todos.json
```

Produces structured JSON with a summary and TODOs grouped by type:

```json
{
  "summary": {
    "total": 2,
    "by_type": {
      "TODO": 1,
      "FIXME": 1
    }
  },
  "todos": {
    "TODO": [
      {
        "id": "abc123",
        "file_path": "src/lib/cache.rs",
        "line_number": 42,
        "column_start": 5,
        "column_end": 30,
        "todo_type": "Todo",
        "description": "Implement caching",
        "original_text": "// TODO: Implement caching",
        "context_lines": ["fn process() {", "    // TODO: Implement caching", "    unimplemented!()"],
        "function_context": "process"
      }
    ]
  }
}
```

## CSV

```bash
towl scan -f csv -o todos.csv
```

Produces a CSV file with a header row:

```csv
Type,Description,File,Line,Column Start,Column End,Function,Original Text,Context Lines
TODO,Implement caching,src/lib/cache.rs,42,5,30,process,// TODO: Implement caching,"fn process() {|    // TODO: Implement caching|    unimplemented!()"
```

Context lines are joined with `|` separators within a single quoted field.

## Markdown

```bash
towl scan -f markdown -o todos.md
```

Produces a Markdown document with sections grouped by TODO type:

```markdown
# TODOs

## TODO (1)

### Implement caching
- **File:** src/lib/cache.rs
- **Line:** 42
- **Function:** process

**Context:**
> fn process() {
>     // TODO: Implement caching
>     unimplemented!()
```

## TOML

```bash
towl scan -f toml -o todos.toml
```

Produces a TOML file with a summary table and grouped items:

```toml
[summary]
total = 2

[summary.by_type]
TODO = 1
FIXME = 1

[[todos.TODO]]
description = "Implement caching"
file_path = "src/lib/cache.rs"
line_number = 42
function_context = "process"
```

## Extension Validation

File-based formats require the output path to have a matching extension:

| Format | Required extension |
|--------|--------------------|
| `json` | `.json` |
| `csv` | `.csv` |
| `toml` | `.toml` |
| `markdown` | `.md` |

Mismatched extensions produce an error:

```text
Error: Invalid output path: expected .json extension for JSON format
```

## Choosing a Format

| Use case | Format |
|----------|--------|
| Quick terminal check | `table` (default) |
| CI/CD integration | `json` |
| Spreadsheet import | `csv` |
| Documentation / reports | `markdown` |
| Config-style tooling | `toml` |
