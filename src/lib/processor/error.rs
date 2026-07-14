use std::path::PathBuf;
use thiserror::Error;

/// Errors from replacing TODO comments with issue links in source files.
#[derive(Error, Debug)]
pub enum TowlProcessorError {
    #[error("Failed to read file {0}: {1}")]
    FileReadError(PathBuf, std::io::Error),
    #[error("Failed to write file {0}: {1}")]
    FileWriteError(PathBuf, std::io::Error),
    #[error("Line {line} out of bounds in {path} (file has {total_lines} lines)")]
    LineOutOfBounds {
        path: PathBuf,
        line: usize,
        total_lines: usize,
    },
    #[error("Comment prefix not found at {path}:{line}")]
    CommentPrefixNotFound { path: PathBuf, line: usize },
    #[error("Line {line} in {path} changed since the scan; not replacing")]
    LineContentChanged { path: PathBuf, line: usize },
    #[error("Path {path} is outside the repository root {root}")]
    PathOutsideRoot { path: PathBuf, root: PathBuf },
    #[error("Invalid issue URL: {url}")]
    InvalidIssueUrl { url: String },
}
