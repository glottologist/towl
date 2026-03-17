use regex::{Regex, RegexBuilder};

use crate::comment::todo::TodoType;

use super::error::TowlParserError;

pub(super) const MAX_PATTERN_LENGTH: usize = 256;
pub(super) const REGEX_SIZE_LIMIT: usize = 262_144;
pub(super) const MAX_TOTAL_PATTERNS: usize = 50;

pub(super) struct Pattern {
    pub(super) regex: Regex,
    pub(super) todo_type: TodoType,
}

use super::types::Parser;

impl Parser {
    pub(super) fn build_regex(pattern: &str) -> Result<Regex, TowlParserError> {
        if pattern.len() > MAX_PATTERN_LENGTH {
            return Err(TowlParserError::PatternTooLong(
                pattern.len(),
                MAX_PATTERN_LENGTH,
            ));
        }
        RegexBuilder::new(pattern)
            .size_limit(REGEX_SIZE_LIMIT)
            .build()
            .map_err(|e| {
                TowlParserError::InvalidRegexPattern(pattern.to_string(), e) // clone: error owns pattern
            })
    }
}
