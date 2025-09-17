use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::info;

use crate::output::writer::{error::WriterError, Writer};

pub struct FileWriter {
    path: PathBuf,
}

impl FileWriter {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

#[async_trait]
impl Writer for FileWriter {
    async fn write(&self, content: Vec<String>) -> Result<(), WriterError> {
        let mut file = File::create(&self.path)
            .await
            .map_err(|e| WriterError::IoError(e.to_string()))?;

        for item in content {
            file.write_all(item.as_bytes())
                .await
                .map_err(|e| WriterError::IoError(e.to_string()))?;
            file.write_all(b"\n")
                .await
                .map_err(|e| WriterError::IoError(e.to_string()))?;
        }

        file.flush()
            .await
            .map_err(|e| WriterError::IoError(e.to_string()))?;

        info!("Written todos to file: {}", self.path.display());
        Ok(())
    }
}
