use thiserror::Error;

#[derive(Error, Debug)]
pub enum FormatterError {
    #[error("Serialization error: {0}")]
    SerializationError(String),
}
