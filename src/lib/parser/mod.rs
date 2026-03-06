pub mod error;
mod types;

pub(crate) use types::*;

use std::path::Path;

use crate::comment::todo::TodoComment;
use crate::config::ParsingConfig;
use error::TowlParserError;

/// Validates that all patterns in the config compile to valid regexes.
///
/// # Errors
/// Returns `TowlParserError` if any pattern fails to compile.
pub fn validate_patterns(config: &ParsingConfig) -> Result<(), TowlParserError> {
    Parser::new(config).map(|_| ())
}

/// Parses file content for TODO comments using the provided configuration.
///
/// # Errors
/// Returns `TowlParserError` if pattern compilation or parsing fails.
pub fn parse_content(
    config: &ParsingConfig,
    path: &Path,
    content: &str,
) -> Result<Vec<TodoComment>, TowlParserError> {
    let parser = Parser::new(config)?;
    parser.parse(path, content)
}
