use thiserror::Error;

use super::formatter::error::FormatterError;
use super::writer::error::WriterError;

#[derive(Error, Debug)]
pub enum TowlOutputError {
    #[error("Unable to format todos: {0}")]
    UnableToFormatTodos(#[from] FormatterError),
    #[error("Unable to write todos: {0}")]
    UnableToWriteTodos(#[from] WriterError),
    #[error("Invalid output path: {0}")]
    InvalidOutputPath(String),
}
