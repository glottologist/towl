use async_trait::async_trait;

use crate::output::writer::{error::WriterError, Writer};
pub struct StdoutWriter;

impl StdoutWriter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StdoutWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Writer for StdoutWriter {
    async fn write(&self, content: Vec<String>) -> Result<(), WriterError> {
        for item in content {
            println!("{}", item);
        }

        Ok(())
    }
}
