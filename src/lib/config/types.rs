use super::error::TowlConfigError;
use super::git::GitRepoInfo;
use config::{Config as ConfigBuilder, File};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

pub const DEFAULT_CONFIG_PATH: &str = ".towl.toml";
const MAX_CONFIG_PATTERNS: usize = 100;
const MAX_CONFIG_STRING_LENGTH: usize = 512;
const MIN_CONTEXT_LINES: usize = 1;
const MAX_CONTEXT_LINES: usize = 50;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Owner(String);

impl Owner {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl fmt::Display for Owner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for Owner {
    fn default() -> Self {
        Self::new("no owner")
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Repo(String);

impl Repo {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl fmt::Display for Repo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for Repo {
    fn default() -> Self {
        Self::new("no repo")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TowlConfig {
    #[serde(default)]
    pub parsing: ParsingConfig,
    #[serde(default)]
    pub github: GitHubConfig,
}

fn fmt_list_section(
    f: &mut fmt::Formatter<'_>,
    label: &str,
    items: &[String],
    is_last: bool,
) -> fmt::Result {
    let branch = if is_last {
        "│  └─"
    } else {
        "│  ├─"
    };
    writeln!(f, "{branch} {label}:")?;
    let (mid, end) = if is_last {
        ("│     ├─", "│     └─")
    } else {
        ("│  │  ├─", "│  │  └─")
    };
    for (i, item) in items.iter().enumerate() {
        let prefix = if i == items.len() - 1 { end } else { mid };
        writeln!(f, "{prefix} {item}")?;
    }
    Ok(())
}

impl fmt::Display for TowlConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "📋 Towl Configuration")?;
        writeln!(f, "┌─ Parsing")?;
        let mut sorted_extensions: Vec<_> = self.parsing.file_extensions.iter().collect();
        sorted_extensions.sort();
        writeln!(
            f,
            "│  ├─ File Extensions: {}",
            sorted_extensions
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )?;
        writeln!(
            f,
            "│  ├─ Exclude Patterns: {}",
            self.parsing.exclude_patterns.join(", ")
        )?;
        writeln!(
            f,
            "│  ├─ Context Lines: {}",
            self.parsing.include_context_lines
        )?;
        fmt_list_section(f, "Comment Prefixes", &self.parsing.comment_prefixes, false)?;
        fmt_list_section(f, "TODO Patterns", &self.parsing.todo_patterns, false)?;
        fmt_list_section(
            f,
            "Function Patterns",
            &self.parsing.function_patterns,
            true,
        )?;
        writeln!(f, "└─ GitHub")?;
        writeln!(f, "   ├─ Owner: {}", self.github.owner)?;
        writeln!(f, "   ├─ Repo: {}", self.github.repo)?;
        write!(
            f,
            "   └─ Token: {}",
            if self.github.token.expose_secret().is_empty() {
                "not set"
            } else {
                "configured"
            }
        )
    }
}
impl TowlConfig {
    /// Validates that a config path does not contain traversal components.
    ///
    /// # Errors
    /// Returns `TowlConfigError::PathTraversalAttempt` if the path contains `..`.
    fn validate_path(path: &Path) -> Result<(), TowlConfigError> {
        if crate::contains_path_traversal(path) {
            return Err(TowlConfigError::PathTraversalAttempt(path.to_path_buf()));
        }
        Ok(())
    }

    fn check_string_length(field: &str, value: &str) -> Result<(), TowlConfigError> {
        if value.len() > MAX_CONFIG_STRING_LENGTH {
            return Err(TowlConfigError::ConfigValueTooLong {
                field: field.to_string(),
                length: value.len(),
                max_length: MAX_CONFIG_STRING_LENGTH,
            });
        }
        Ok(())
    }

    fn validate_string_lengths(parsing: &ParsingConfig) -> Result<(), TowlConfigError> {
        for ext in &parsing.file_extensions {
            Self::check_string_length("file_extensions", ext)?;
        }
        let vec_fields: &[(&str, &[String])] = &[
            ("exclude_patterns", &parsing.exclude_patterns),
            ("comment_prefixes", &parsing.comment_prefixes),
            ("todo_patterns", &parsing.todo_patterns),
            ("function_patterns", &parsing.function_patterns),
        ];
        for &(field, values) in vec_fields {
            for value in values {
                Self::check_string_length(field, value)?;
            }
        }
        Ok(())
    }

    const fn validate_context_lines(parsing: &ParsingConfig) -> Result<(), TowlConfigError> {
        if parsing.include_context_lines < MIN_CONTEXT_LINES
            || parsing.include_context_lines > MAX_CONTEXT_LINES
        {
            return Err(TowlConfigError::ContextLinesOutOfRange {
                value: parsing.include_context_lines,
                min: MIN_CONTEXT_LINES,
                max: MAX_CONTEXT_LINES,
            });
        }
        Ok(())
    }

    fn validate_pattern_counts(parsing: &ParsingConfig) -> Result<(), TowlConfigError> {
        let checks: &[(&str, usize)] = &[
            ("file_extensions", parsing.file_extensions.len()),
            ("exclude_patterns", parsing.exclude_patterns.len()),
            ("comment_prefixes", parsing.comment_prefixes.len()),
            ("todo_patterns", parsing.todo_patterns.len()),
            ("function_patterns", parsing.function_patterns.len()),
        ];
        for &(field, count) in checks {
            if count > MAX_CONFIG_PATTERNS {
                return Err(TowlConfigError::TooManyConfigPatterns {
                    field: field.to_string(),
                    count,
                    max_allowed: MAX_CONFIG_PATTERNS,
                });
            }
        }
        Ok(())
    }

    /// Initializes a new config file at the given path.
    ///
    /// # Errors
    /// Returns `TowlConfigError::ConfigAlreadyExists` if the file exists and `force` is false.
    /// Returns `TowlConfigError` if the git repo cannot be found or the file cannot be written.
    pub async fn init(path: &Path, force: bool) -> Result<(), TowlConfigError> {
        Self::validate_path(path)?;

        let git_repo_info = GitRepoInfo::from_path(".").await?;
        let config = Self {
            github: GitHubConfig {
                token: SecretString::default(),
                owner: git_repo_info.owner,
                repo: git_repo_info.repo,
            },
            ..Default::default()
        };

        let toml_string =
            toml::to_string_pretty(&config).map_err(TowlConfigError::UnableToParseToml)?;

        if !force && path.exists() {
            return Err(TowlConfigError::ConfigAlreadyExists(path.to_path_buf()));
        }
        Self::atomic_write(path, toml_string.as_bytes()).await?;

        Ok(())
    }

    async fn atomic_write(target: &Path, content: &[u8]) -> Result<(), TowlConfigError> {
        use tokio::io::AsyncWriteExt;

        let parent = target.parent().unwrap_or_else(|| Path::new("."));
        let temp = tempfile::Builder::new()
            .prefix(".towl_")
            .tempfile_in(parent)
            .map_err(|e| TowlConfigError::WriteToFileError(target.to_path_buf(), e))?;

        let (std_file, temp_path) = temp.into_parts();
        let mut file = tokio::fs::File::from_std(std_file);

        if let Err(e) = file.write_all(content).await {
            drop(file);
            drop(temp_path);
            return Err(TowlConfigError::WriteToFileError(target.to_path_buf(), e));
        }

        if let Err(e) = file.flush().await {
            drop(file);
            drop(temp_path);
            return Err(TowlConfigError::WriteToFileError(target.to_path_buf(), e));
        }

        drop(file);

        temp_path
            .persist(target)
            .map_err(|e| TowlConfigError::WriteToFileError(target.to_path_buf(), e.error))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParsingConfig {
    #[serde(default = "default_file_extensions")]
    pub file_extensions: HashSet<String>,
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
    #[serde(default = "default_include_context_lines")]
    pub include_context_lines: usize,
    #[serde(default = "default_comment_prefixes")]
    pub comment_prefixes: Vec<String>,
    #[serde(default = "default_todo_patterns")]
    pub todo_patterns: Vec<String>,
    #[serde(default = "default_function_patterns")]
    pub function_patterns: Vec<String>,
}

impl Default for ParsingConfig {
    fn default() -> Self {
        Self {
            file_extensions: default_file_extensions(),
            exclude_patterns: default_exclude_patterns(),
            include_context_lines: default_include_context_lines(),
            comment_prefixes: default_comment_prefixes(),
            todo_patterns: default_todo_patterns(),
            function_patterns: default_function_patterns(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct GitHubConfig {
    #[serde(skip)]
    pub token: SecretString,
    pub owner: Owner,
    pub repo: Repo,
}

impl fmt::Debug for GitHubConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GitHubConfig")
            .field("token", &"[REDACTED]")
            .field("owner", &self.owner)
            .field("repo", &self.repo)
            .finish()
    }
}

impl PartialEq for GitHubConfig {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner && self.repo == other.repo
    }
}

impl Eq for GitHubConfig {}

#[cfg(test)]
impl TowlConfig {
    async fn save(&self, path: &Path) -> Result<(), TowlConfigError> {
        Self::validate_path(path)?;

        let toml_string =
            toml::to_string_pretty(self).map_err(TowlConfigError::UnableToParseToml)?;
        Self::atomic_write(path, toml_string.as_bytes()).await
    }
}

impl TowlConfig {
    /// Loads configuration from a file, falling back to defaults.
    ///
    /// # Errors
    /// Returns `TowlConfigError` if the config file is malformed or cannot be parsed.
    pub fn load(path: Option<&PathBuf>) -> Result<Self, TowlConfigError> {
        let default_path = PathBuf::from(DEFAULT_CONFIG_PATH);
        let config_path = path.unwrap_or(&default_path);
        Self::validate_path(config_path)?;

        let mut builder = ConfigBuilder::builder().add_source(
            config::Config::try_from(&Self::default())
                .map_err(TowlConfigError::CouldNotCreateConfig)?,
        );

        builder = builder.add_source(File::from(config_path.as_path()).required(false));

        let built: config::Config = builder.build().map_err(|e| {
            tracing::error!("Config build error: {:?}", e);
            TowlConfigError::CouldNotCreateConfig(e)
        })?;

        let mut config: Self = built.try_deserialize().map_err(|e| {
            tracing::error!("Config deserialization error: {:?}", e);
            TowlConfigError::CouldNotCreateConfig(e)
        })?;

        if let Ok(token) = std::env::var("TOWL_GITHUB_TOKEN") {
            config.github.token = SecretString::from(token);
        }
        if let Ok(owner) = std::env::var("TOWL_GITHUB_OWNER") {
            config.github.owner = Owner::new(owner);
        }
        if let Ok(repo) = std::env::var("TOWL_GITHUB_REPO") {
            config.github.repo = Repo::new(repo);
        }

        Self::validate_pattern_counts(&config.parsing)?;
        Self::validate_string_lengths(&config.parsing)?;
        Self::validate_context_lines(&config.parsing)?;

        Ok(config)
    }
}

fn default_file_extensions() -> HashSet<String> {
    [
        "rs".to_string(),
        "toml".to_string(),
        "json".to_string(),
        "yaml".to_string(),
        "yml".to_string(),
        "sh".to_string(),
        "bash".to_string(),
    ]
    .into_iter()
    .collect()
}

fn default_exclude_patterns() -> Vec<String> {
    vec!["target/*".to_string(), ".git/*".to_string()]
}

const fn default_include_context_lines() -> usize {
    3
}

const RUST_COMMENT_PREFIX: &str = r"//";
const SHELL_COMMENT_PREFIX: &str = r"^\s*#";
const C_MULTILINE_START: &str = r"/\*";
const MULTILINE_CONTINUATION: &str = r"^\s*\*";

fn default_comment_prefixes() -> Vec<String> {
    vec![
        RUST_COMMENT_PREFIX.to_string(),
        SHELL_COMMENT_PREFIX.to_string(),
        C_MULTILINE_START.to_string(),
        MULTILINE_CONTINUATION.to_string(),
    ]
}

fn default_todo_patterns() -> Vec<String> {
    vec![
        r"(?i)\bTODO:\s*(.*)".to_string(),
        r"(?i)\bFIXME:\s*(.*)".to_string(),
        r"(?i)\bHACK:\s*(.*)".to_string(),
        r"(?i)\bNOTE:\s*(.*)".to_string(),
        r"(?i)\bBUG:\s*(.*)".to_string(),
    ]
}

#[cfg(test)]
#[must_use]
pub fn test_parsing_config() -> ParsingConfig {
    ParsingConfig {
        file_extensions: ["rs".to_string(), "py".to_string(), "txt".to_string()]
            .into_iter()
            .collect(),
        exclude_patterns: vec!["target/*".to_string(), "*.log".to_string()],
        include_context_lines: 3,
        comment_prefixes: vec![
            r"//".to_string(),
            r"^\s*#".to_string(),
            r"/\*".to_string(),
            r"^\s*\*".to_string(),
        ],
        todo_patterns: vec![
            r"(?i)\bTODO:\s*(.*)".to_string(),
            r"(?i)\bFIXME:\s*(.*)".to_string(),
            r"(?i)\bHACK:\s*(.*)".to_string(),
            r"(?i)\bNOTE:\s*(.*)".to_string(),
            r"(?i)\bBUG:\s*(.*)".to_string(),
        ],
        function_patterns: vec![
            r"^\s*(pub\s+)?fn\s+(\w+)".to_string(),
            r"^\s*def\s+(\w+)".to_string(),
        ],
    }
}

fn default_function_patterns() -> Vec<String> {
    vec![
        r"^\s*(pub\s+)?fn\s+(\w+)".to_string(),
        r"^\s*def\s+(\w+)".to_string(),
        r"^\s*(async\s+)?function\s+(\w+)".to_string(),
        r"^\s*(public|private|protected)?\s*(static\s+)?\w+\s+(\w+)\s*\(".to_string(),
        r"^\s*func\s+(\w+)".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_load_without_file_returns_defaults() {
        let nonexistent = PathBuf::from("nonexistent_towl_config_12345.toml");
        let loaded = TowlConfig::load(Some(&nonexistent)).unwrap();
        let defaults = TowlConfig::default();

        assert_eq!(
            loaded.parsing.file_extensions,
            defaults.parsing.file_extensions
        );
        assert_eq!(
            loaded.parsing.exclude_patterns,
            defaults.parsing.exclude_patterns
        );
        assert_eq!(
            loaded.parsing.include_context_lines,
            defaults.parsing.include_context_lines
        );
        assert_eq!(loaded.github.owner, defaults.github.owner);
        assert_eq!(loaded.github.repo, defaults.github.repo);
    }

    #[test]
    fn test_display_token_masked_when_set() {
        let mut config = TowlConfig::default();
        config.github.token = SecretString::from("secret-token-123");
        let display = config.to_string();

        assert!(display.contains("configured"));
        assert!(!display.contains("secret-token-123"));
    }

    proptest! {
        #[test]
        fn prop_validate_path_rejects_traversal(
            components in prop::collection::vec("[a-zA-Z0-9_-]{1,10}", 1..5),
        ) {
            let mut path = PathBuf::new();
            for component in &components {
                path.push(component);
            }
            path.push("..");
            path.push("escaped.toml");

            let result = TowlConfig::validate_path(&path);
            prop_assert!(result.is_err(), "Path with '..' should be rejected: {:?}", path);
        }

        #[test]
        fn prop_validate_path_accepts_safe_paths(
            components in prop::collection::vec("[a-zA-Z0-9_-]{1,10}", 1..5),
        ) {
            let mut path = PathBuf::new();
            for component in &components {
                path.push(component);
            }
            path.push("config.toml");

            let result = TowlConfig::validate_path(&path);
            prop_assert!(result.is_ok(), "Safe path should be accepted: {:?}", path);
        }

        #[test]
        fn prop_config_save_load_roundtrip(
            owner_name in "[a-zA-Z0-9_-]{1,20}",
            repo_name in "[a-zA-Z0-9_-]{1,20}",
            context_lines in 1usize..50,
        ) {
            let config = TowlConfig {
                parsing: ParsingConfig {
                    include_context_lines: context_lines,
                    ..Default::default()
                },
                github: GitHubConfig {
                    token: SecretString::default(),
                    owner: Owner::new(&owner_name),
                    repo: Repo::new(&repo_name),
                },
            };

            let temp_dir = tempfile::TempDir::new().unwrap();
            let config_path = temp_dir.path().join("test.toml");

            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                config.save(&config_path).await.unwrap();
            });

            let loaded = TowlConfig::load(Some(&config_path)).unwrap();

            prop_assert_eq!(loaded.parsing.include_context_lines, context_lines);
            prop_assert_eq!(loaded.github.owner, Owner::new(&owner_name));
            prop_assert_eq!(loaded.github.repo, Repo::new(&repo_name));
            prop_assert_eq!(loaded.parsing.file_extensions, config.parsing.file_extensions);
            prop_assert_eq!(loaded.parsing.exclude_patterns, config.parsing.exclude_patterns);
        }
    }

    #[tokio::test]
    async fn test_atomic_write_produces_valid_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let target = temp_dir.path().join("atomic_test.toml");
        let content = b"[parsing]\ninclude_context_lines = 5\n";

        TowlConfig::atomic_write(&target, content).await.unwrap();

        let read_back = std::fs::read_to_string(&target).unwrap();
        assert_eq!(read_back.as_bytes(), content);

        let temp_files: Vec<_> = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with(".towl_"))
            .collect();
        assert!(temp_files.is_empty(), "Temp file should be cleaned up");
    }

    #[tokio::test]
    async fn test_atomic_write_overwrites_existing() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let target = temp_dir.path().join("overwrite.toml");

        std::fs::write(&target, "old content").unwrap();

        TowlConfig::atomic_write(&target, b"new content")
            .await
            .unwrap();

        let read_back = std::fs::read_to_string(&target).unwrap();
        assert_eq!(read_back, "new content");
    }

    #[tokio::test]
    async fn test_init_force_false_on_fresh_path() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("fresh.toml");

        // Should succeed on a fresh (non-existent) path
        let result = TowlConfig::init(&config_path, false).await;

        // This will only succeed if we're in a git repo with an origin remote
        if result.is_ok() {
            assert!(config_path.exists());
            let loaded = TowlConfig::load(Some(&config_path));
            assert!(loaded.is_ok());
        }
    }

    #[tokio::test]
    async fn test_init_force_false_rejects_existing() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("existing.toml");

        std::fs::write(&config_path, "[parsing]").unwrap();

        let result = TowlConfig::init(&config_path, false).await;
        assert!(matches!(
            result,
            Err(TowlConfigError::ConfigAlreadyExists(_))
        ));
    }

    #[test]
    fn test_validate_pattern_counts_rejects_excess() {
        let parsing = ParsingConfig {
            todo_patterns: (0..101).map(|i| format!("pattern_{i}")).collect(),
            ..Default::default()
        };
        let result = TowlConfig::validate_pattern_counts(&parsing);
        assert!(matches!(
            result,
            Err(TowlConfigError::TooManyConfigPatterns { field, count: 101, max_allowed: 100 })
            if field == "todo_patterns"
        ));
    }

    #[test]
    fn test_validate_string_lengths_rejects_long_pattern() {
        let parsing = ParsingConfig {
            todo_patterns: vec!["x".repeat(513)],
            ..Default::default()
        };
        let result = TowlConfig::validate_string_lengths(&parsing);
        assert!(matches!(
            result,
            Err(TowlConfigError::ConfigValueTooLong { field, length: 513, max_length: 512 })
            if field == "todo_patterns"
        ));
    }

    #[test]
    fn test_validate_string_lengths_accepts_valid() {
        let parsing = ParsingConfig::default();
        let result = TowlConfig::validate_string_lengths(&parsing);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_context_lines_rejects_zero() {
        let result = TowlConfig::validate_context_lines(&ParsingConfig {
            include_context_lines: 0,
            ..Default::default()
        });
        assert!(matches!(
            result,
            Err(TowlConfigError::ContextLinesOutOfRange {
                value: 0,
                min: 1,
                max: 50
            })
        ));
    }

    #[test]
    fn test_validate_context_lines_rejects_excess() {
        let result = TowlConfig::validate_context_lines(&ParsingConfig {
            include_context_lines: 51,
            ..Default::default()
        });
        assert!(matches!(
            result,
            Err(TowlConfigError::ContextLinesOutOfRange {
                value: 51,
                min: 1,
                max: 50
            })
        ));
    }

    #[test]
    fn test_validate_string_lengths_rejects_long_extension() {
        let mut extensions = std::collections::HashSet::new();
        extensions.insert("x".repeat(513));
        let parsing = ParsingConfig {
            file_extensions: extensions,
            ..Default::default()
        };
        let result = TowlConfig::validate_string_lengths(&parsing);
        assert!(matches!(
            result,
            Err(TowlConfigError::ConfigValueTooLong { field, length: 513, max_length: 512 })
            if field == "file_extensions"
        ));
    }
}
