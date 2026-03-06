use thiserror::Error;

use crate::comment::error::TowlCommentError;

#[derive(Error, Debug)]
pub enum TowlParserError {
    #[error("Pattern {0} is not a valid regex pattern for a supported todo")]
    InvalidRegexPattern(String, regex::Error),
    #[error("Config pattern {0} is not valid")]
    UnknownConfigPattern(#[from] TowlCommentError),
    #[error("Regex capture group 0 missing from successful match")]
    RegexGroupMissing,
    #[error("Pattern length {0} exceeds maximum of {1} characters")]
    PatternTooLong(usize, usize),
    #[error("Total pattern count {count} exceeds maximum of {max_allowed} across all categories")]
    TooManyTotalPatterns { count: usize, max_allowed: usize },
}
