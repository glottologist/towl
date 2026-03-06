use std::path::PathBuf;
use thiserror::Error;

use crate::parser::error::TowlParserError;

#[derive(Error, Debug)]
pub enum TowlScannerError {
    #[error("Unable to walk file {0}")]
    UnableToWalkFile(#[from] ignore::Error),
    #[error("Parsing error {0}")]
    ParsingError(#[from] TowlParserError),
    #[error("Unable to read file at path {0}: {1}")]
    UnableToReadFileAtPath(PathBuf, tokio::io::Error),
    #[error("Invalid Path. {path}")]
    InvalidPath { path: PathBuf },
    #[error("File too large: {path} ({size} bytes exceeds maximum of {max_allowed} bytes)")]
    FileTooLarge {
        path: PathBuf,
        size: u64,
        max_allowed: u64,
    },
    #[error("Too many TODOs in file: {path} (found {count}, truncated to {max_allowed})")]
    TooManyTodos {
        path: PathBuf,
        count: usize,
        max_allowed: usize,
    },
    #[error("Too many files scanned ({count} exceeds limit of {max_allowed})")]
    TooManyFiles { count: usize, max_allowed: usize },
}
