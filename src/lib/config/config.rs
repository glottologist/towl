use super::error::TowlConfigError;
use super::git::GitRepoInfo;
use async_trait::async_trait;
use config::{Config as ConfigBuilder, Environment, File};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub const DEFAULT_CONFIG_PATH: &str = ".towl.toml";

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Owner(pub String);

impl Owner {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Owner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Owner {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

impl Default for Owner {
    fn default() -> Self {
        Self::new("no owner")
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Repo(pub String);

impl Repo {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Repo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Repo {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

impl Default for Repo {
    fn default() -> Self {
        Self::new("no repo")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TowlConfig {
    pub parsing: ParsingConfig,
    pub output: OutputConfig,
    pub github: GitHubConfig,
}

impl Default for TowlConfig {
    fn default() -> Self {
        Self {
            parsing: ParsingConfig::default(),
            output: OutputConfig::default(),
            github: GitHubConfig::default(),
        }
    }
}
impl TowlConfig {
    pub async fn init(path: &PathBuf) -> Result<(), TowlConfigError> {
        let git_repo_info = GitRepoInfo::from_path(".")?;
        let config = TowlConfig {
            github: GitHubConfig {
                token: String::new(),
                owner: git_repo_info.owner,
                repo: git_repo_info.repo,
            },
            ..Default::default()
        };

        config.save(path).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsingConfig {
    #[serde(default = "default_file_extensions")]
    pub file_extensions: Vec<String>,
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
    #[serde(default = "default_include_context_lines")]
    pub include_context_lines: usize,
    #[serde(default = "default_todo_patterns")]
    pub todo_patterns: Vec<String>,
}

impl Default for ParsingConfig {
    fn default() -> Self {
        Self {
            file_extensions: default_file_extensions(),
            exclude_patterns: default_exclude_patterns(),
            include_context_lines: default_include_context_lines(),
            todo_patterns: default_todo_patterns(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default = "default_backup_files")]
    pub backup_files: bool,
    #[serde(default = "default_progress_bar")]
    pub progress_bar: bool,
    #[serde(default)]
    pub verbose: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            dry_run: false,
            backup_files: default_backup_files(),
            progress_bar: default_progress_bar(),
            verbose: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    #[serde(skip)]
    pub token: String, // Always loaded from environment variable
    pub owner: Owner,
    pub repo: Repo,
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            token: Default::default(),
            owner: Default::default(),
            repo: Default::default(),
        }
    }
}

pub trait ValidConfigPath {
    fn validate_path(path: &Path) -> Result<(), TowlConfigError>;
}

impl ValidConfigPath for TowlConfig {
    fn validate_path(path: &Path) -> Result<(), TowlConfigError> {
        if path.to_string_lossy().contains("..") {
            return Err(TowlConfigError::PathTraversalAttempt(path.to_path_buf()));
        }
        Ok(())
    }
}

#[async_trait]
pub trait SaveConfig {
    async fn save(&self, path: &Path) -> Result<(), TowlConfigError>;
}

#[async_trait]
impl SaveConfig for TowlConfig {
    async fn save(&self, path: &Path) -> Result<(), TowlConfigError> {
        let _ = Self::validate_path(path);

        let mut config_to_save = self.clone();

        // Overwrite token for security
        config_to_save.github.token = String::new();

        let toml_string =
            toml::to_string_pretty(&config_to_save).map_err(TowlConfigError::UnableToParseToml)?;
        tokio::fs::write(path, toml_string)
            .await
            .map_err(|e| TowlConfigError::WriteToFileError(path.to_path_buf(), e))?;

        Ok(())
    }
}

pub trait LoadConfig {
    fn load(&self, path: Option<&Path>) -> Result<TowlConfig, TowlConfigError>;
}
impl LoadConfig for TowlConfig {
    fn load(&self, path: Option<&Path>) -> Result<TowlConfig, TowlConfigError> {
        let config_path = match path {
            Some(p) => p.to_path_buf(),
            None => PathBuf::from(DEFAULT_CONFIG_PATH),
        };

        let _ = Self::validate_path(&config_path);

        let mut builder = ConfigBuilder::builder().add_source(
            config::Config::try_from(&TowlConfig::default())
                .map_err(|e| TowlConfigError::CouldNotCreateConfig(e))?,
        );

        // Only add the file source if it exists
        if config_path.exists() {
            builder = builder.add_source(File::from(config_path.as_path()));
        }

        // Add environment variables with TOWL_ prefix
        builder = builder.add_source(Environment::with_prefix("TOWL").separator("_"));

        let config: TowlConfig = builder.build()?.try_deserialize()?;
        Ok(config)
    }
}

fn default_file_extensions() -> Vec<String> {
    vec![
        "rs".to_string(),
        "sh".to_string(),
        "toml".to_string(),
        "json".to_string(),
        "yaml".to_string(),
        "yml".to_string(),
        "sh".to_string(),
        "bash".to_string(),
    ]
}

fn default_exclude_patterns() -> Vec<String> {
    vec!["target/*".to_string(), ".git/*".to_string()]
}

fn default_include_context_lines() -> usize {
    3
}

fn default_todo_patterns() -> Vec<String> {
    vec![
        r"(?i)TODO:?\s*(.*)".to_string(),
        r"(?i)FIXME:?\s*(.*)".to_string(),
        r"(?i)HACK:?\s*(.*)".to_string(),
        r"(?i)NOTE:?\s*(.*)".to_string(),
        r"(?i)BUG:?\s*(.*)".to_string(),
    ]
}

fn default_backup_files() -> bool {
    true
}

fn default_progress_bar() -> bool {
    true
}
