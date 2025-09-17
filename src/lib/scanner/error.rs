use std::path::PathBuf;
use thiserror::Error;

use crate::parser::error::TowlParserError;

#[derive(Error, Debug)]
pub enum TowlScannerError {
    #[error("Unable to walk file  {0}")]
    UnableToWalkFile(#[from] ignore::Error),
    #[error("Parsing error {0}")]
    ParsingError(#[from] TowlParserError),
    #[error("Unable to read file at path {0}: {1}")]
    UnableToReadFileAtPath(PathBuf, tokio::io::Error),
    #[error("Path traversal is not supported. {path}")]
    PathTraversalAttempt { path: PathBuf },
    #[error("Invalid Path. {path}")]
    InvalidPath { path: PathBuf },
}
