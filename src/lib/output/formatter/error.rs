use thiserror::Error;

#[derive(Error, Debug)]
pub enum FormatterError {
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Integer overflow: value {0} exceeds i64 range")]
    IntegerOverflow(usize),
}
