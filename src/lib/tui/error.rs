use thiserror::Error;

/// Errors from the interactive terminal UI.
#[derive(Error, Debug)]
pub enum TowlTuiError {
    #[error("Terminal I/O error: {0}")]
    Io(#[from] std::io::Error),
}
