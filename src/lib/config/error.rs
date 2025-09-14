use config::ConfigError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TowlConfigError {
    #[error("Config file should be under the repo root: {0} ")]
    PathTraversalAttempt(PathBuf),
    #[error("Config file could not be written to path {0}: {1} ")]
    WriteToFileError(PathBuf, std::io::Error),
    #[error("Could not parse toml for config {0}")]
    UnableToParseToml(#[from] toml::ser::Error),
    #[error("Could not create config {0}")]
    CouldNotCreateConfig(#[from] ConfigError),
    #[error("Git repository not found: {message}")]
    GitRepoNotFound { message: String },
    #[error("Git remote not found: {message}")]
    GitRemoteNotFound { message: String },
    #[error("Invalid Git URL '{url}': {message}")]
    GitInvalidUrl { url: String, message: String },
}
