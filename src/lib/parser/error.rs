use thiserror::Error;

use crate::comment::error::TowlCommentError;

#[derive(Error, Debug)]
pub enum TowlParserError {
    #[error("Pattern {0} is not a valid regex pattern for a supported todo")]
    InvalidRegexPattern(String, regex::Error),
    #[error("Config pattern {0} is not valid")]
    UnknownConfigPattern(#[from] TowlCommentError),
}
