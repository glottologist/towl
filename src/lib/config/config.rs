use super::error::TowlConfigError;
use super::git::GitRepoInfo;
use async_trait::async_trait;
use config::{Config as ConfigBuilder, Environment, File};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tracing::debug;

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

impl fmt::Display for TowlConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "📋 Towl Configuration")?;
        writeln!(f, "┌─ Parsing")?;
        writeln!(
            f,
            "│  ├─ File Extensions: {}",
            self.parsing.file_extensions.join(", ")
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
        writeln!(f, "│  ├─ Comment Prefixes:")?;
        for (i, pattern) in self.parsing.comment_prefixes.iter().enumerate() {
            let prefix = if i == self.parsing.comment_prefixes.len() - 1 {
                "│  │  └─"
            } else {
                "│  │  ├─"
            };
            writeln!(f, "{} {}", prefix, pattern)?;
        }
        writeln!(f, "│  ├─ TODO Patterns:")?;
        for (i, pattern) in self.parsing.todo_patterns.iter().enumerate() {
            let prefix = if i == self.parsing.todo_patterns.len() - 1 {
                "│  │  └─"
            } else {
                "│  │  ├─"
            };
            writeln!(f, "{} {}", prefix, pattern)?;
        }
        writeln!(f, "│  └─ Function Patterns:")?;
        for (i, pattern) in self.parsing.function_patterns.iter().enumerate() {
            let prefix = if i == self.parsing.function_patterns.len() - 1 {
                "│     └─"
            } else {
                "│     ├─"
            };
            writeln!(f, "{} {}", prefix, pattern)?;
        }
        writeln!(f, "├─ Output")?;
        writeln!(
            f,
            "│  ├─ Dry Run: {}",
            if self.output.dry_run { "✓" } else { "✗" }
        )?;
        writeln!(
            f,
            "│  ├─ Backup Files: {}",
            if self.output.backup_files {
                "✓"
            } else {
                "✗"
            }
        )?;
        writeln!(
            f,
            "│  ├─ Progress Bar: {}",
            if self.output.progress_bar {
                "✓"
            } else {
                "✗"
            }
        )?;
        writeln!(
            f,
            "│  └─ Verbose: {}",
            if self.output.verbose { "✓" } else { "✗" }
        )?;
        writeln!(f, "└─ GitHub")?;
        writeln!(f, "   ├─ Owner: {}", self.github.owner)?;
        writeln!(f, "   ├─ Repo: {}", self.github.repo)?;
        write!(
            f,
            "   └─ Token: {}",
            if self.github.token.is_empty() {
                "not set"
            } else {
                "configured"
            }
        )
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
        let _ = Self::validate_path(path)?;

        let mut config_to_save = self.clone();

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
    fn load(path: Option<&PathBuf>) -> Result<TowlConfig, TowlConfigError>;
}
impl LoadConfig for TowlConfig {
    fn load(path: Option<&PathBuf>) -> Result<TowlConfig, TowlConfigError> {
        let config_path = match path {
            Some(p) => p,
            None => &PathBuf::from(DEFAULT_CONFIG_PATH),
        };
        let _ = Self::validate_path(&config_path)?;

        let mut builder = ConfigBuilder::builder().add_source(
            config::Config::try_from(&TowlConfig::default())
                .map_err(|e| TowlConfigError::CouldNotCreateConfig(e))?,
        );

        if config_path.exists() {
            builder = builder.add_source(File::from(config_path.as_path()));
        } else {
            debug!("Config file {} does not exist", config_path.display());
        }

        builder = builder.add_source(Environment::with_prefix("TOWL").separator("_"));

        let built: config::Config = builder.build().map_err(|e| {
            tracing::error!("Config build error: {:?}", e);
            TowlConfigError::CouldNotCreateConfig(e)
        })?;

        let config: TowlConfig = built.try_deserialize().map_err(|e| {
            tracing::error!("Config deserialization error: {:?}", e);
            TowlConfigError::CouldNotCreateConfig(e)
        })?;
        Ok(config)
    }
}

fn default_file_extensions() -> Vec<String> {
    // Extensions ordered by expected frequency in typical Rust projects
    vec![
        "rs".to_string(),
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

fn default_comment_prefixes() -> Vec<String> {
    // Comment patterns ordered by frequency for performance
    // Matches: '//' anywhere, '#' at line start, '/*' start, '*' continuation
    vec![
        r"//".to_string(),     // C-style single-line comments (anywhere on line)
        r"^\s*#".to_string(),  // Shell/Python style comments (start of line only)
        r"/\*".to_string(),    // Start of C-style multi-line comments
        r"^\s*\*".to_string(), // Continuation of multi-line comments
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

fn default_backup_files() -> bool {
    true
}

fn default_progress_bar() -> bool {
    true
}

fn default_function_patterns() -> Vec<String> {
    vec![
        r"^\s*(pub\s+)?fn\s+(\w+)".to_string(),         // Rust
        r"^\s*def\s+(\w+)".to_string(),                 // Python
        r"^\s*(async\s+)?function\s+(\w+)".to_string(), // JavaScript
        r"^\s*(public|private|protected)?\s*(static\s+)?\w+\s+(\w+)\s*\(".to_string(), // Java/C#
        r"^\s*func\s+(\w+)".to_string(),                // Go
    ]
}
