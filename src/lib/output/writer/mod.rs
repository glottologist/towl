pub mod error;
pub mod writers;
use async_trait::async_trait;
use error::WriterError;

#[async_trait]
pub trait Writer {
    async fn write(&self, content: Vec<String>) -> Result<(), WriterError>;
}
