# Configuration

towl uses a `.towl.toml` file in the project root for configuration. All fields have sensible defaults -- you only need to override what you want to change.

## Config File

Create `.towl.toml` manually or run `towl init`:

```toml
[parsing]
file_extensions = ["rs", "toml", "json", "yaml", "yml", "sh", "bash"]
exclude_patterns = ["target/*", ".git/*"]
include_context_lines = 3

[github]
owner = "your-username"
repo = "your-repo"
```

## Parsing Section

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `file_extensions` | `string[]` | `["rs", "toml", "json", "yaml", "yml", "sh", "bash"]` | File extensions to scan |
| `exclude_patterns` | `string[]` | `["target/*", ".git/*"]` | Glob patterns to exclude |
| `include_context_lines` | `integer` | `3` | Number of surrounding lines to capture (1-50) |
| `comment_prefixes` | `string[]` | `["//", "^\\s*#", "/\\*", "^\\s*\\*"]` | Regex patterns for comment line detection |
| `todo_patterns` | `string[]` | See below | Regex patterns for TODO extraction |
| `function_patterns` | `string[]` | See below | Regex patterns for function context detection |

### Default TODO Patterns

```toml
todo_patterns = [
    "(?i)\\bTODO:\\s*(.*)",
    "(?i)\\bFIXME:\\s*(.*)",
    "(?i)\\bHACK:\\s*(.*)",
    "(?i)\\bNOTE:\\s*(.*)",
    "(?i)\\bBUG:\\s*(.*)",
]
```

All patterns are case-insensitive by default. Each pattern must contain a capture group `(.*)` for extracting the description text.

### Default Function Patterns

```toml
function_patterns = [
    "^\\s*(pub\\s+)?fn\\s+(\\w+)",            # Rust
    "^\\s*def\\s+(\\w+)",                      # Python
    "^\\s*(async\\s+)?function\\s+(\\w+)",     # JavaScript
    "^\\s*(public|private|protected)?\\s*(static\\s+)?\\w+\\s+(\\w+)\\s*\\(",  # Java/C#
    "^\\s*func\\s+(\\w+)",                     # Go/Swift
]
```

### Pattern Limits

Each pattern field is limited to 100 entries. Individual regex patterns are limited to 256 characters. These limits prevent denial-of-service via malicious configuration files.

## GitHub Section

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `owner` | `string` | Auto-detected from git remote | GitHub repository owner |
| `repo` | `string` | Auto-detected from git remote | GitHub repository name |
| `token` | -- | -- | Set via environment variable only |

> **Note:** The GitHub token is never stored in the config file. Use the `TOWL_GITHUB_TOKEN` environment variable.

## Environment Variables

Three environment variables override config file values:

| Variable | Overrides | Description |
|----------|-----------|-------------|
| `TOWL_GITHUB_TOKEN` | `github.token` | GitHub personal access token (stored as `SecretString`, masked in logs) |
| `TOWL_GITHUB_OWNER` | `github.owner` | GitHub repository owner |
| `TOWL_GITHUB_REPO` | `github.repo` | GitHub repository name |

## Config Loading Order

1. Built-in defaults
2. `.towl.toml` file (or path specified with `--path`)
3. Environment variable overrides (`TOWL_GITHUB_TOKEN`, `TOWL_GITHUB_OWNER`, `TOWL_GITHUB_REPO`)

If no `.towl.toml` exists, defaults are used without error.

## Viewing Active Configuration

```bash
towl config
```

Example output:

```text
📋 Towl Configuration
┌─ Parsing
│  ├─ File Extensions: bash, json, rs, sh, toml, yaml, yml
│  ├─ Exclude Patterns: target/*, .git/*
│  ├─ Context Lines: 3
│  ├─ Comment Prefixes:
│  │  ├─ //
│  │  ├─ ^\s*#
│  │  ├─ /\*
│  │  └─ ^\s*\*
│  ├─ TODO Patterns:
│  │  ├─ (?i)\bTODO:\s*(.*)
│  │  ...
│  └─ Function Patterns:
│     ├─ ^\s*(pub\s+)?fn\s+(\w+)
│     ...
└─ GitHub
   ├─ Owner: glottologist
   ├─ Repo: towl
   └─ Token: not set
```
