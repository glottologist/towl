# Config

The config module loads settings from `.towl.toml`, merges environment variables, and provides the `init` command for generating default configuration.

## `TowlConfig`

```rust
pub struct TowlConfig {
    pub parsing: ParsingConfig,
    pub github: GitHubConfig,
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
2. `.towl.toml` file (or custom path via `--path`)
3. Environment variable overrides

If no config file exists, defaults are used without error.

### `init`

```rust
pub async fn init(path: &Path, force: bool) -> Result<(), TowlConfigError>
```

Creates a `.towl.toml` file at the given path. Auto-detects GitHub owner and repo from `git remote get-url origin`.

- Fails if the file already exists (unless `force` is `true`)
- Validates the path for traversal attacks
- Serializes `ParsingConfig` defaults to TOML

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
| `include_context_lines` | `3` |
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
}
```

- `token` is stored as `secrecy::SecretString` and masked in debug/display output
- `owner` and `repo` are newtype wrappers over `String`

### Environment Variable Overrides

| Variable | Overrides |
|----------|-----------|
| `TOWL_GITHUB_TOKEN` | `github.token` |
| `TOWL_GITHUB_OWNER` | `github.owner` |
| `TOWL_GITHUB_REPO` | `github.repo` |

## `Owner` / `Repo`

Newtype wrappers providing type safety:

```rust
pub struct Owner(String);
pub struct Repo(String);
```

Both implement:
- `new(s: impl Into<String>) -> Self`
- `Display`, `Default`, `Debug`, `Clone`, `PartialEq`, `Eq`
- `Serialize`, `Deserialize`

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
