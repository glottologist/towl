use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WriterError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Path traversal detected in output path: {0}")]
    PathTraversal(PathBuf),
    #[error("Output path {resolved} escapes working directory {cwd}")]
    OutputOutsideCwd { resolved: PathBuf, cwd: PathBuf },
}
