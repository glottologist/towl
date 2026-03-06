use crate::output::writer::{error::WriterError, Writer};
pub(crate) struct StdoutWriter;

impl StdoutWriter {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for StdoutWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl Writer for StdoutWriter {
    async fn write(&self, content: Vec<String>) -> Result<(), WriterError> {
        for item in content {
            println!("{item}");
        }

        Ok(())
    }
}
