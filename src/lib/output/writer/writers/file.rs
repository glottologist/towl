use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::info;

use crate::output::writer::{error::WriterError, Writer};

pub(crate) struct FileWriter {
    path: PathBuf,
}

impl FileWriter {
    /// Creates a new file writer for the given path.
    ///
    /// # Errors
    /// Returns `WriterError::PathTraversal` if the path contains `..` components.
    /// Returns `WriterError::OutputOutsideCwd` if a relative path resolves (via symlinks) outside CWD.
    pub(crate) fn new(path: PathBuf) -> Result<Self, WriterError> {
        if crate::contains_path_traversal(&path) {
            return Err(WriterError::PathTraversal(path));
        }
        let was_relative = path.is_relative();
        let resolved = Self::resolve_symlinks(path);
        if was_relative {
            Self::verify_within_cwd(&resolved)?;
        }
        Ok(Self { path: resolved })
    }

    fn resolve_symlinks(path: PathBuf) -> PathBuf {
        if let Some(parent) = path.parent() {
            match parent.canonicalize() {
                Ok(canonical_parent) => {
                    if let Some(filename) = path.file_name() {
                        return canonical_parent.join(filename);
                    }
                }
                Err(_) => return path,
            }
        }
        path
    }

    fn verify_within_cwd(resolved: &PathBuf) -> Result<(), WriterError> {
        let cwd = std::env::current_dir().map_err(WriterError::IoError)?;
        let canonical_cwd = cwd.canonicalize().map_err(WriterError::IoError)?;

        let check_path = if resolved.is_relative() {
            canonical_cwd.join(resolved)
        } else {
            resolved.clone() // clone: need owned value for comparison and error variant
        };

        if !check_path.starts_with(&canonical_cwd) {
            return Err(WriterError::OutputOutsideCwd {
                resolved: check_path,
                cwd: canonical_cwd,
            });
        }

        Ok(())
    }
}

impl Writer for FileWriter {
    async fn write(&self, content: Vec<String>) -> Result<(), WriterError> {
        let mut file = File::create(&self.path).await?;

        for item in content {
            file.write_all(item.as_bytes()).await?;
            file.write_all(b"\n").await?;
        }

        file.flush().await?;

        info!("Written todos to file: {}", self.path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_path_traversal_rejected() {
        let result = FileWriter::new(PathBuf::from("../malicious/path.txt"));
        assert!(result.is_err());
        assert!(matches!(result, Err(WriterError::PathTraversal(_))));
    }

    #[test]
    fn test_nested_traversal_rejected() {
        let result = FileWriter::new(PathBuf::from("safe/../../etc/passwd"));
        assert!(result.is_err());
    }

    #[test]
    fn test_safe_path_accepted() {
        let result = FileWriter::new(PathBuf::from("output/todos.json"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_absolute_path_allowed() {
        let result = FileWriter::new(PathBuf::from("/tmp/escape/output.json"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_symlink_escape_rejected() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("outside");
        std::fs::create_dir(&target_dir).unwrap();

        let cwd = std::env::current_dir().unwrap();
        let symlink_path = cwd.join("_test_symlink_escape");

        #[cfg(unix)]
        {
            if std::os::unix::fs::symlink(&target_dir, &symlink_path).is_ok() {
                let result = FileWriter::new(PathBuf::from("_test_symlink_escape/output.json"));
                let _ = std::fs::remove_file(&symlink_path);
                assert!(result.is_err(), "Symlink escaping CWD should be rejected");
                assert!(matches!(result, Err(WriterError::OutputOutsideCwd { .. })));
            }
        }
    }

    proptest! {
        #[test]
        fn prop_safe_paths_accepted(
            components in prop::collection::vec("[a-zA-Z0-9_-]{1,10}", 1..5),
        ) {
            let mut path = PathBuf::new();
            for component in &components {
                path.push(component);
            }
            path.push("output.json");

            let result = FileWriter::new(path);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn prop_traversal_paths_rejected(
            prefix in prop::collection::vec("[a-zA-Z0-9_-]{1,10}", 1..3),
            suffix in prop::collection::vec("[a-zA-Z0-9_-]{1,10}", 1..3),
        ) {
            let mut path = PathBuf::new();
            for component in &prefix {
                path.push(component);
            }
            path.push("..");
            for component in &suffix {
                path.push(component);
            }

            let result = FileWriter::new(path);
            prop_assert!(result.is_err());
        }
    }
}
