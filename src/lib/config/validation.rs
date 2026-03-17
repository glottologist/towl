use std::path::Path;

use super::error::TowlConfigError;
use super::newtypes::MAX_CONFIG_STRING_LENGTH;
use super::types::{GitHubConfig, ParsingConfig, TowlConfig};
use crate::{MAX_CONTEXT_LINES, MIN_CONTEXT_LINES};

const MAX_CONFIG_PATTERNS: usize = 100;
pub(super) const MAX_RATE_LIMIT_DELAY_MS: u64 = 60_000;

impl TowlConfig {
    pub(crate) fn validate_path(path: &Path) -> Result<(), TowlConfigError> {
        if crate::contains_path_traversal(path) {
            let owned = path.to_path_buf(); // clone: error owns PathBuf
            return Err(TowlConfigError::PathTraversalAttempt(owned));
        }
        Ok(())
    }

    pub(crate) fn check_string_length(field: &str, value: &str) -> Result<(), TowlConfigError> {
        if value.len() > MAX_CONFIG_STRING_LENGTH {
            return Err(TowlConfigError::ConfigValueTooLong {
                field: field.to_string(), // clone: error owns field name
                length: value.len(),
                max_length: MAX_CONFIG_STRING_LENGTH,
            });
        }
        Ok(())
    }

    pub(crate) fn validate_string_lengths(parsing: &ParsingConfig) -> Result<(), TowlConfigError> {
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

    pub(crate) const fn validate_context_lines(
        parsing: &ParsingConfig,
    ) -> Result<(), TowlConfigError> {
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

    pub(crate) const fn validate_rate_limit_delay(
        github: &GitHubConfig,
    ) -> Result<(), TowlConfigError> {
        if github.rate_limit_delay_ms > MAX_RATE_LIMIT_DELAY_MS {
            return Err(TowlConfigError::RateLimitDelayTooHigh {
                value: github.rate_limit_delay_ms,
                max: MAX_RATE_LIMIT_DELAY_MS,
            });
        }
        Ok(())
    }

    pub(crate) fn validate_pattern_counts(parsing: &ParsingConfig) -> Result<(), TowlConfigError> {
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
                    field: field.to_string(), // clone: error owns field name
                    count,
                    max_allowed: MAX_CONFIG_PATTERNS,
                });
            }
        }
        Ok(())
    }
}
