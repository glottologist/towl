# Config

The config module loads settings from `.towl.toml`, merges environment variables, and provides the `init` command for generating default configuration.

## `TowlConfig`

```rust
pub struct TowlConfig {
    pub parsing: ParsingConfig,
    pub github: GitHubConfig,
    pub llm: LlmConfig,
}
```

### `TowlConfig::load`

```rust
impl TowlConfig {
    pub fn load(path: Option<&PathBuf>) -> Result<Self, TowlConfigError>;
}
```

Loads configuration with this precedence:

1. Built-in defaults
2. Config file resolved as: explicit `path` argument > `TOWL_CONFIG` env var > `.towl.toml`
3. Git remote auto-detection for owner/repo
4. Environment variable overrides (`TOWL_GITHUB_*`, `TOWL_LLM_*`)

If no config file exists, defaults are used without error.

### `init`

```rust
pub async fn init(path: &Path, force: bool) -> Result<(), TowlConfigError>
```

Creates a `.towl.toml` file at the given path. Validates that a GitHub git remote exists but does not write owner/repo to the file (they are always detected at runtime).

- Fails if the file already exists (unless `force` is `true`)
- Validates the path for traversal attacks
- Serializes `ParsingConfig` and `LlmConfig` defaults to TOML

## `ParsingConfig`

```rust
pub struct ParsingConfig {
    pub file_extensions: HashSet<String>,
    pub exclude_patterns: Vec<String>,
    pub include_context_lines: usize,
    pub comment_prefixes: Vec<String>,
    pub todo_patterns: Vec<String>,
    pub function_patterns: Vec<String>,
}
```

All fields have defaults via `#[serde(default)]`:

| Field | Default |
|-------|---------|
| `file_extensions` | `rs`, `toml`, `json`, `yaml`, `yml`, `sh`, `bash` |
| `exclude_patterns` | `target/*`, `.git/*` |
| `include_context_lines` | `10` |
| `comment_prefixes` | `//`, `^\s*#`, `/\*`, `^\s*\*` |
| `todo_patterns` | `TODO:`, `FIXME:`, `HACK:`, `NOTE:`, `BUG:` (case-insensitive) |
| `function_patterns` | Rust, Python, JS, Java/C#, Go patterns |

Each pattern array is limited to `MAX_CONFIG_PATTERNS` (100) entries.

## `GitHubConfig`

```rust
pub struct GitHubConfig {
    pub token: SecretString,
    pub owner: Owner,
    pub repo: Repo,
    pub rate_limit_delay_ms: u64,
}
```

- `token` is stored as `secrecy::SecretString` and masked in debug/display output
- `owner` and `repo` are auto-detected from `git remote get-url origin` at runtime (not serialised to config)
- `rate_limit_delay_ms` adds a delay between GitHub API calls (default: 1000ms)

### Environment Variable Overrides

| Variable | Overrides |
|----------|-----------|
| `TOWL_CONFIG` | `DEFAULT_CONFIG_PATH` (overridden by explicit `path` argument) |
| `TOWL_GITHUB_TOKEN` | -- (env-only) |
| `TOWL_GITHUB_OWNER` | git remote detection |
| `TOWL_GITHUB_REPO` | git remote detection |

## `Owner` / `Repo`

Validated newtype wrappers providing type safety:

```rust
pub struct Owner(String);
pub struct Repo(String);
```

### `try_new`

```rust
pub fn try_new(s: impl Into<String>) -> Result<Self, TowlConfigError>
```

Constructs a new `Owner` or `Repo`, rejecting values exceeding `MAX_CONFIG_STRING_LENGTH` (512 characters).

**Errors:**

- `ConfigValueTooLong` -- Value exceeds 512 characters

Both also implement:
- `Display`, `Default`, `Debug`, `Clone`, `PartialEq`, `Eq`
- `Serialize`, `Deserialize`

## `LlmConfig`

```rust
pub struct LlmConfig {
    pub provider: String,
    pub model: String,
    pub base_url: Option<String>,
    pub api_key: SecretString,
    pub max_concurrent_analyses: usize,
    pub max_analyse_count: usize,
    pub max_tokens: u32,
    pub max_retries: usize,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
}
```

- `api_key` is stored as `secrecy::SecretString` and masked in debug output (env-only via `TOWL_LLM_API_KEY`)
- `provider` selects the LLM backend: `"claude"`, `"openai"`, `"claude-code"`, or `"codex"`
- `command` and `args` allow overriding the CLI binary path and arguments for CLI providers

| Field | Default |
|-------|---------|
| `provider` | `"claude"` |
| `model` | `"claude-opus-4-6"` |
| `base_url` | `None` (provider default) |
| `max_concurrent_analyses` | `5` |
| `max_analyse_count` | `50` |
| `max_tokens` | `4096` |
| `max_retries` | `3` |

### Environment Variable Overrides

| Variable | Overrides |
|----------|-----------|
| `TOWL_LLM_API_KEY` | -- (env-only) |
| `TOWL_LLM_PROVIDER` | `llm.provider` |
| `TOWL_LLM_MODEL` | `llm.model` |
| `TOWL_LLM_BASE_URL` | `llm.base_url` |

## `GitRepoInfo` (internal)

```rust
pub(crate) struct GitRepoInfo {
    pub owner: Owner,
    pub repo: Repo,
}
```

### `from_path`

```rust
pub(crate) async fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, TowlConfigError>
```

Internal function that discovers the git remote URL by running `git remote get-url origin` and parses the owner and repo name. Supports both HTTPS and SSH URL formats. Not part of the public API.

**Errors:**
- `GitRepoNotFound` -- Not inside a git repository
- `GitRemoteNotFound` -- No `origin` remote configured
- `GitInvalidUrl` -- Could not parse owner/repo from the URL

## Errors

```rust
pub enum TowlConfigError {
    PathTraversalAttempt(PathBuf),
    ConfigAlreadyExists(PathBuf),
    WriteToFileError(PathBuf, std::io::Error),
    UnableToParseToml(toml::ser::Error),
    CouldNotCreateConfig(ConfigError),
    GitRepoNotFound { message: String },
    GitRemoteNotFound { message: String },
    GitInvalidUrl { url: String, message: String },
    TooManyConfigPatterns { field: String, count: usize, max_allowed: usize },
    ConfigValueTooLong { field: String, length: usize, max_length: usize },
    ContextLinesOutOfRange { value: usize, min: usize, max: usize },
}
```

## Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `DEFAULT_CONFIG_PATH` | `.towl.toml` | Default config file name |
| `MAX_CONFIG_PATTERNS` | 100 | Maximum entries per pattern array |
| `MAX_CONFIG_STRING_LENGTH` | 512 | Maximum length for any single config string |
| `MIN_CONTEXT_LINES` | 1 | Minimum `include_context_lines` value |
| `MAX_CONTEXT_LINES` | 50 | Maximum `include_context_lines` value |
