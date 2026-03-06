pub mod error;
pub mod formatters;

use crate::comment::todo::{TodoComment, TodoType};
use error::FormatterError;
use formatters::{
    csv::CsvFormatter, json::JsonFormatter, markdown::MarkdownFormatter, table::TableFormatter,
    toml::TomlFormatter,
};
use std::collections::HashMap;

pub(crate) trait Formatter {
    /// Formats grouped TODO comments into output strings.
    ///
    /// # Errors
    /// Returns `FormatterError::SerializationError` if serialization fails,
    /// or `FormatterError::IntegerOverflow` if a count exceeds `i64` range.
    fn format(
        &self,
        todos: &HashMap<&TodoType, Vec<&TodoComment>>,
        total_count: usize,
    ) -> Result<Vec<String>, FormatterError>;
}

/// Enum dispatch for Formatter implementations.
///
/// Mirrors the `WriterImpl` pattern — avoids `Box<dyn Formatter>` and dynamic
/// dispatch, enabling zero-cost abstraction and eliminating object-safety
/// constraints (no `Send + Sync` bounds needed).
pub(crate) enum FormatterImpl {
    Csv(CsvFormatter),
    Json(JsonFormatter),
    Markdown(MarkdownFormatter),
    Table(TableFormatter),
    Toml(TomlFormatter),
}

impl FormatterImpl {
    pub(crate) fn format(
        &self,
        todos: &HashMap<&TodoType, Vec<&TodoComment>>,
        total_count: usize,
    ) -> Result<Vec<String>, FormatterError> {
        match self {
            Self::Csv(f) => f.format(todos, total_count),
            Self::Json(f) => f.format(todos, total_count),
            Self::Markdown(f) => f.format(todos, total_count),
            Self::Table(f) => f.format(todos, total_count),
            Self::Toml(f) => f.format(todos, total_count),
        }
    }
}
