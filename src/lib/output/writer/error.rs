use thiserror::Error;

#[derive(Error, Debug)]
pub enum WriterError {
    #[error("IO error: {0}")]
    IoError(String),
}
