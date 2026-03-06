pub mod error;
pub mod writers;
use error::WriterError;
use writers::{file::FileWriter, stdout::StdoutWriter};

pub(crate) trait Writer {
    async fn write(&self, content: Vec<String>) -> Result<(), WriterError>;
}

/// Enum dispatch for Writer implementations
///
/// This enum provides zero-cost abstraction over different writer types,
/// avoiding the object-safety issues with async trait methods in trait objects.
/// Using enum dispatch instead of `Box<dyn Writer>` allows async methods
/// without manual Future boxing.
pub(crate) enum WriterImpl {
    Stdout(StdoutWriter),
    File(FileWriter),
}

impl WriterImpl {
    pub(crate) async fn write(&self, content: Vec<String>) -> Result<(), WriterError> {
        match self {
            Self::Stdout(writer) => writer.write(content).await,
            Self::File(writer) => writer.write(content).await,
        }
    }
}
