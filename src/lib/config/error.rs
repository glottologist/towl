use config::ConfigError;
use std::path::PathBuf;
use thiserror::Error;

const MAX_URL_DISPLAY_LEN: usize = 500;

fn truncate_url(url: &str) -> String {
    if url.len() <= MAX_URL_DISPLAY_LEN {
        url.to_string()
    } else {
        let boundary = url
            .char_indices()
            .map(|(i, _)| i)
            .take_while(|&i| i <= MAX_URL_DISPLAY_LEN)
            .last()
            .unwrap_or(0);
        format!("{}...[truncated]", &url[..boundary])
    }
}

#[derive(Error, Debug)]
pub enum TowlConfigError {
    #[error("Config file should be under the repo root: {0} ")]
    PathTraversalAttempt(PathBuf),
    #[error("Config file already exists at {0} (use --force to overwrite)")]
    ConfigAlreadyExists(PathBuf),
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
    #[error("Invalid Git URL '{}': {message}", truncate_url(url))]
    GitInvalidUrl { url: String, message: String },
    #[error("Config has too many {field} ({count}, max {max_allowed})")]
    TooManyConfigPatterns {
        field: String,
        count: usize,
        max_allowed: usize,
    },
    #[error("Config value in {field} exceeds max length ({length}, max {max_length})")]
    ConfigValueTooLong {
        field: String,
        length: usize,
        max_length: usize,
    },
    #[error("Config context_lines value {value} is out of range ({min}..={max})")]
    ContextLinesOutOfRange {
        value: usize,
        min: usize,
        max: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_truncate_url_never_panics(url in "\\PC{0,1500}") {
            let result = truncate_url(&url);
            prop_assert!(result.len() <= url.len() + "...[truncated]".len());
            prop_assert!(result.is_char_boundary(result.len()));
        }

        #[test]
        fn prop_truncate_url_short_strings_unchanged(url in "[a-zA-Z0-9:/._-]{0,500}") {
            let result = truncate_url(&url);
            prop_assert_eq!(result, url);
        }
    }
}
