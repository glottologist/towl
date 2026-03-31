use super::defaults::{
    default_comment_prefixes, default_exclude_patterns, default_file_extensions,
    default_function_patterns, default_include_context_lines, default_llm_max_retries,
    default_llm_max_tokens, default_llm_model, default_llm_provider, default_max_analyse_count,
    default_max_concurrent_analyses, default_rate_limit_delay_ms, default_todo_patterns,
};
use super::error::TowlConfigError;
use super::git::GitRepoInfo;
use super::newtypes::{Owner, Repo};
use config::{Config as ConfigBuilder, File};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

pub const DEFAULT_CONFIG_PATH: &str = ".towl.toml";

/// Root configuration combining parsing rules and GitHub settings.
///
/// Load from a `.towl.toml` file with [`TowlConfig::load`], or create a new
/// config file with [`TowlConfig::init`].
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TowlConfig {
    #[serde(default)]
    pub parsing: ParsingConfig,
    #[serde(default)]
    pub github: GitHubConfig,
    #[serde(default)]
    pub llm: LlmConfig,
}

impl TowlConfig {
    /// Initializes a new config file at the given path.
    ///
    /// # Errors
    /// Returns `TowlConfigError::ConfigAlreadyExists` if the file exists and `force` is false.
    /// Returns `TowlConfigError` if the git repo cannot be found or the file cannot be written.
    pub async fn init(path: &Path, force: bool) -> Result<(), TowlConfigError> {
        Self::validate_path(path)?;

        GitRepoInfo::from_path(".").await?;

        let config = if !force && path.exists() {
            let existing = tokio::fs::read_to_string(path)
                .await
                .map_err(|e| TowlConfigError::WriteToFileError(path.to_path_buf(), e))?; // clone: error owns PathBuf
            toml::from_str::<Self>(&existing).map_err(|e| {
                TowlConfigError::CouldNotCreateConfig(config::ConfigError::Foreign(Box::new(e)))
            })?
        } else {
            Self::default()
        };

        let toml_string =
            toml::to_string_pretty(&config).map_err(TowlConfigError::UnableToParseToml)?;

        crate::atomic_write(path, toml_string.as_bytes())
            .await
            .map_err(|e| {
                TowlConfigError::WriteToFileError(path.to_path_buf(), e) // clone: error owns PathBuf
            })?;

        Ok(())
    }

    /// Loads configuration from a file, falling back to defaults.
    ///
    /// # Errors
    /// Returns `TowlConfigError` if the config file is malformed or cannot be parsed.
    pub fn load(path: Option<&PathBuf>) -> Result<Self, TowlConfigError> {
        let env_path = std::env::var("TOWL_CONFIG").ok().map(PathBuf::from);
        let default_path = PathBuf::from(DEFAULT_CONFIG_PATH);
        let config_path = path.or(env_path.as_ref()).unwrap_or(&default_path);
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
            Self::check_string_length("TOWL_GITHUB_TOKEN", &token)?;
            config.github.token = SecretString::from(token);
        }

        if let Ok(owner) = std::env::var("TOWL_GITHUB_OWNER") {
            config.github.owner = Owner::try_new(owner)?;
        } else if let Ok(info) = GitRepoInfo::from_path_sync(".") {
            config.github.owner = info.owner;
            if std::env::var("TOWL_GITHUB_REPO").is_err() {
                config.github.repo = info.repo;
            }
        }
        if let Ok(repo) = std::env::var("TOWL_GITHUB_REPO") {
            config.github.repo = Repo::try_new(repo)?;
        }

        if let Ok(key) = std::env::var("TOWL_LLM_API_KEY") {
            Self::check_string_length("TOWL_LLM_API_KEY", &key)?;
            config.llm.api_key = SecretString::from(key);
        }
        if let Ok(provider) = std::env::var("TOWL_LLM_PROVIDER") {
            config.llm.provider = provider;
        }
        if let Ok(model) = std::env::var("TOWL_LLM_MODEL") {
            config.llm.model = model;
        }
        if let Ok(url) = std::env::var("TOWL_LLM_BASE_URL") {
            config.llm.base_url = Some(url);
        }

        Self::validate_pattern_counts(&config.parsing)?;
        Self::validate_string_lengths(&config.parsing)?;
        Self::validate_context_lines(&config.parsing)?;
        Self::validate_rate_limit_delay(&config.github)?;

        Ok(config)
    }
}

/// Controls which files to scan, what patterns to match, and how much context to capture.
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

/// GitHub integration settings for issue creation.
///
/// The token is loaded from the `TOWL_GITHUB_TOKEN` environment variable (never
/// serialised to disk). Owner and repo can be auto-detected from the git remote.
#[derive(Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    #[serde(skip)]
    pub token: SecretString,
    #[serde(skip)]
    pub owner: Owner,
    #[serde(skip)]
    pub repo: Repo,
    #[serde(default = "default_rate_limit_delay_ms")]
    pub rate_limit_delay_ms: u64,
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            token: SecretString::default(),
            owner: Owner::default(),
            repo: Repo::default(),
            rate_limit_delay_ms: default_rate_limit_delay_ms(),
        }
    }
}

impl fmt::Debug for GitHubConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GitHubConfig")
            .field("token", &"[REDACTED]")
            .field("owner", &self.owner)
            .field("repo", &self.repo)
            .field("rate_limit_delay_ms", &self.rate_limit_delay_ms)
            .finish()
    }
}

impl PartialEq for GitHubConfig {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner
            && self.repo == other.repo
            && self.rate_limit_delay_ms == other.rate_limit_delay_ms
    }
}

impl Eq for GitHubConfig {}

/// LLM configuration for AI-powered TODO validation.
///
/// API key is loaded from `TOWL_LLM_API_KEY` (never serialised to disk).
/// Provider and model can be overridden via environment variables.
#[derive(Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_llm_provider")]
    pub provider: String,
    #[serde(default = "default_llm_model")]
    pub model: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(skip)]
    pub api_key: SecretString,
    #[serde(default = "default_max_concurrent_analyses")]
    pub max_concurrent_analyses: usize,
    #[serde(default = "default_max_analyse_count")]
    pub max_analyse_count: usize,
    #[serde(default = "default_llm_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_llm_max_retries")]
    pub max_retries: usize,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_llm_provider(),
            model: default_llm_model(),
            base_url: None,
            api_key: SecretString::default(),
            max_concurrent_analyses: default_max_concurrent_analyses(),
            max_analyse_count: default_max_analyse_count(),
            max_tokens: default_llm_max_tokens(),
            max_retries: default_llm_max_retries(),
            command: None,
            args: None,
        }
    }
}

impl fmt::Debug for LlmConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LlmConfig")
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("base_url", &self.base_url)
            .field("api_key", &"[REDACTED]")
            .field("max_concurrent_analyses", &self.max_concurrent_analyses)
            .field("max_analyse_count", &self.max_analyse_count)
            .field("max_tokens", &self.max_tokens)
            .field("max_retries", &self.max_retries)
            .field("command", &self.command)
            .field("args", &self.args)
            .finish()
    }
}

impl PartialEq for LlmConfig {
    fn eq(&self, other: &Self) -> bool {
        self.provider == other.provider
            && self.model == other.model
            && self.base_url == other.base_url
            && self.max_concurrent_analyses == other.max_concurrent_analyses
            && self.max_analyse_count == other.max_analyse_count
            && self.max_tokens == other.max_tokens
            && self.max_retries == other.max_retries
            && self.command == other.command
            && self.args == other.args
    }
}

impl Eq for LlmConfig {}

#[cfg(test)]
impl TowlConfig {
    async fn save(&self, path: &Path) -> Result<(), TowlConfigError> {
        Self::validate_path(path)?;

        let toml_string =
            toml::to_string_pretty(self).map_err(TowlConfigError::UnableToParseToml)?;
        crate::atomic_write(path, toml_string.as_bytes())
            .await
            .map_err(|e| TowlConfigError::WriteToFileError(path.to_path_buf(), e))
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rstest::rstest;

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
        assert_ne!(
            loaded.github.owner.to_string(),
            "no owner",
            "Owner should be auto-detected from git"
        );
        assert_ne!(
            loaded.github.repo.to_string(),
            "no repo",
            "Repo should be auto-detected from git"
        );
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
            context_lines in 1usize..50,
        ) {
            let config = TowlConfig {
                parsing: ParsingConfig {
                    include_context_lines: context_lines,
                    ..Default::default()
                },
                ..Default::default()
            };

            let temp_dir = tempfile::TempDir::new().unwrap();
            let config_path = temp_dir.path().join("test.toml");

            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                config.save(&config_path).await.unwrap();
            });

            let loaded = TowlConfig::load(Some(&config_path)).unwrap();

            prop_assert_eq!(loaded.parsing.include_context_lines, context_lines);
            prop_assert_eq!(loaded.parsing.file_extensions, config.parsing.file_extensions);
            prop_assert_eq!(loaded.parsing.exclude_patterns, config.parsing.exclude_patterns);
        }
    }

    #[tokio::test]
    async fn test_atomic_write_overwrites_existing() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let target = temp_dir.path().join("overwrite.toml");

        std::fs::write(&target, "old content").unwrap();

        crate::atomic_write(&target, b"new content").await.unwrap();

        let read_back = std::fs::read_to_string(&target).unwrap();
        assert_eq!(read_back, "new content");
    }

    #[tokio::test]
    async fn test_init_force_false_on_fresh_path() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("fresh.toml");

        let result = TowlConfig::init(&config_path, false).await;

        if result.is_ok() {
            assert!(config_path.exists());
            let loaded = TowlConfig::load(Some(&config_path));
            assert!(loaded.is_ok());
        }
    }

    #[tokio::test]
    async fn test_init_force_false_merges_existing() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("existing.toml");

        std::fs::write(&config_path, "[parsing]\ninclude_context_lines = 42\n").unwrap();

        let result = TowlConfig::init(&config_path, false).await;
        if result.is_ok() {
            let loaded = TowlConfig::load(Some(&config_path)).unwrap();
            assert_eq!(loaded.parsing.include_context_lines, 42);
            let defaults = TowlConfig::default();
            assert_eq!(
                loaded.parsing.file_extensions,
                defaults.parsing.file_extensions,
            );
            assert_eq!(
                loaded.github.rate_limit_delay_ms,
                defaults.github.rate_limit_delay_ms,
            );
        }
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

    #[rstest]
    #[case(0, true)]
    #[case(1, false)]
    #[case(25, false)]
    #[case(50, false)]
    #[case(51, true)]
    fn test_validate_context_lines(#[case] value: usize, #[case] should_err: bool) {
        let result = TowlConfig::validate_context_lines(&ParsingConfig {
            include_context_lines: value,
            ..Default::default()
        });
        assert_eq!(result.is_err(), should_err);
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
