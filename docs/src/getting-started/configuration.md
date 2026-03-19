# Configuration

towl uses a `.towl.toml` file in the project root for configuration. All fields have sensible defaults -- you only need to override what you want to change.

You can point to a config file in a different location using:

- `--config` / `-c` flag on `scan` and `config` commands
- `TOWL_CONFIG` environment variable

The `--config` flag takes precedence over `TOWL_CONFIG`, which takes precedence over the default `.towl.toml`.

## Config File

Create `.towl.toml` manually or run `towl init`:

```toml
[parsing]
file_extensions = ["rs", "toml", "json", "yaml", "yml", "sh", "bash"]
exclude_patterns = ["target/*", ".git/*"]
include_context_lines = 3
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
| `rate_limit_delay_ms` | `integer` | `100` | Delay in ms between GitHub API calls |

Owner and repo are **always** auto-detected from `git remote get-url origin` at runtime -- they are not stored in the config file. Use `TOWL_GITHUB_OWNER` and `TOWL_GITHUB_REPO` environment variables to override if needed.

> **Note:** The GitHub token is never stored in the config file. Use the `TOWL_GITHUB_TOKEN` environment variable.

## LLM Section

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | `string` | `claude` | LLM provider: `"claude"`, `"openai"`, `"claude-code"`, or `"codex"` |
| `model` | `string` | `claude-opus-4-6` | Model identifier |
| `base_url` | `string` | Provider default | Custom endpoint URL (for Ollama, vLLM, etc.) |
| `max_concurrent_analyses` | `integer` | `5` | Max concurrent LLM requests (1-20) |
| `max_analyse_count` | `integer` | `50` | Max TODOs to analyse per scan (1-500) |
| `max_tokens` | `integer` | `4096` | LLM response token limit |
| `command` | `string` | Auto (provider-dependent) | Override CLI binary path |
| `args` | `string[]` | Auto (provider-dependent) | Override CLI arguments |

> **Note:** The LLM API key is never stored in the config file. Use the `TOWL_LLM_API_KEY` environment variable. See [AI Analysis](../guides/ai-analysis.md) for usage details.

## Environment Variables

Eight environment variables override defaults:

| Variable | Overrides | Description |
|----------|-----------|-------------|
| `TOWL_CONFIG` | `DEFAULT_CONFIG_PATH` | Path to a `.towl.toml` file (overridden by `--config` flag) |
| `TOWL_GITHUB_TOKEN` | -- | GitHub personal access token (stored as `SecretString`, masked in logs) |
| `TOWL_GITHUB_OWNER` | git remote detection | GitHub repository owner |
| `TOWL_GITHUB_REPO` | git remote detection | GitHub repository name |
| `TOWL_LLM_API_KEY` | `llm.api_key` | LLM API key (stored as `SecretString`, env-only) |
| `TOWL_LLM_PROVIDER` | `llm.provider` | LLM provider (`"claude"` or `"openai"`) |
| `TOWL_LLM_MODEL` | `llm.model` | LLM model identifier |
| `TOWL_LLM_BASE_URL` | `llm.base_url` | Custom LLM endpoint URL |

## Config Loading Order

1. Built-in defaults
2. Config file resolved as: `--config` flag > `TOWL_CONFIG` env var > `.towl.toml`
3. Git remote auto-detection for owner/repo
4. Environment variable overrides (`TOWL_GITHUB_*`, `TOWL_LLM_*`)

If no config file exists at the resolved path, defaults are used without error.

## Viewing Active Configuration

```bash
# Show config from default .towl.toml
towl config

# Show config from a custom path
towl config -c .config/.towl.toml
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
