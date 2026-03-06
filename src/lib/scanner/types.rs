use std::path::{Path, PathBuf};

use ignore::{overrides::OverrideBuilder, WalkBuilder};
use tracing::{debug, error, info, warn};

use crate::{comment::todo::TodoComment, config::ParsingConfig, parser::Parser};

use super::error::TowlScannerError;

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
const MAX_TODO_COUNT: usize = 10_000;
const MAX_TOTAL_TODO_COUNT: usize = 100_000;
const MAX_FILES_SCANNED: usize = 100_000;

/// Structured result from a scan operation, distinguishing "no TODOs found"
/// from "all files failed to scan".
#[derive(Debug)]
pub struct ScanResult {
    pub todos: Vec<TodoComment>,
    pub files_scanned: usize,
    pub files_skipped: usize,
    pub files_errored: usize,
    pub duration: std::time::Duration,
}

impl ScanResult {
    /// Returns true if no files were successfully scanned but errors occurred.
    #[must_use]
    pub const fn all_files_failed(&self) -> bool {
        self.files_scanned == 0 && self.files_errored > 0
    }

    /// Returns true if the scan completed without errors and found no TODOs.
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.todos.is_empty() && self.files_errored == 0
    }
}

/// Scans files for TODO comments with configurable patterns and resource limits.
///
/// The scanner walks directory trees, filtering files by extension and exclude patterns,
/// while enforcing safety limits to prevent resource exhaustion.
pub struct Scanner {
    parser: Parser,
    config: ParsingConfig,
}

impl Scanner {
    /// Creates a new scanner with the provided parsing configuration.
    ///
    /// # Errors
    /// Returns `TowlScannerError::ParsingError` if regex patterns in config are invalid.
    ///
    /// # Example
    /// ```no_run
    /// use towl::scanner::Scanner;
    /// use towl::config::ParsingConfig;
    ///
    /// let config = ParsingConfig::default();
    /// let scanner = Scanner::new(config)?;
    /// # Ok::<(), towl::scanner::error::TowlScannerError>(())
    /// ```
    pub fn new(config: ParsingConfig) -> Result<Self, TowlScannerError> {
        let parser = Parser::new(&config).map_err(TowlScannerError::ParsingError)?;
        Ok(Self { parser, config })
    }

    fn should_file_be_scanned(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }

        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                return self.config.file_extensions.contains(ext_str);
            }
        }

        false
    }
    async fn scan_file(&self, path: &Path) -> Result<Vec<TodoComment>, TowlScannerError> {
        use tokio::io::AsyncReadExt;

        let canonical = path
            .canonicalize()
            .map_err(|_| TowlScannerError::InvalidPath {
                path: path.to_path_buf(),
            })?;

        let mut file = tokio::fs::File::open(&canonical)
            .await
            .map_err(|e| TowlScannerError::UnableToReadFileAtPath(path.to_path_buf(), e))?;

        let metadata = file
            .metadata()
            .await
            .map_err(|e| TowlScannerError::UnableToReadFileAtPath(path.to_path_buf(), e))?;

        if metadata.len() > MAX_FILE_SIZE {
            return Err(TowlScannerError::FileTooLarge {
                path: path.to_path_buf(),
                size: metadata.len(),
                max_allowed: MAX_FILE_SIZE,
            });
        }

        let mut content = String::new();
        file.read_to_string(&mut content)
            .await
            .map_err(|e| TowlScannerError::UnableToReadFileAtPath(path.to_path_buf(), e))?;

        let todos = self
            .parser
            .parse(path, &content)
            .map_err(TowlScannerError::ParsingError)?;

        if todos.len() > MAX_TODO_COUNT {
            warn!(
                "File {} contains {} TODOs (limit: {}), rejecting",
                path.display(),
                todos.len(),
                MAX_TODO_COUNT
            );
            return Err(TowlScannerError::TooManyTodos {
                path: path.to_path_buf(),
                count: todos.len(),
                max_allowed: MAX_TODO_COUNT,
            });
        }

        Ok(todos)
    }

    fn build_walker(&self, path: &Path) -> ignore::Walk {
        let mut builder = WalkBuilder::new(path);
        builder.hidden(false).git_ignore(false).follow_links(false);

        if !self.config.exclude_patterns.is_empty() {
            let mut overrides = OverrideBuilder::new(path);
            // Whitelist everything first, then exclude specific patterns
            if let Err(e) = overrides.add("**") {
                debug!("Failed to add wildcard override: {}", e);
            }
            for pattern in &self.config.exclude_patterns {
                if let Err(e) = overrides.add(&format!("!{pattern}")) {
                    debug!("Failed to add exclude pattern '{}': {}", pattern, e);
                }
            }
            if let Ok(overrides) = overrides.build() {
                builder.overrides(overrides);
            }
        }

        builder.build()
    }

    fn log_scan_metrics(
        files_scanned: usize,
        files_skipped: usize,
        files_errored: usize,
        todos_found: usize,
        elapsed: std::time::Duration,
    ) {
        info!(
            files_scanned,
            files_skipped,
            files_errored,
            todos_found,
            duration_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
            "Scan complete"
        );
    }

    async fn process_walk_entry(
        &self,
        path: &Path,
        todos: &mut Vec<TodoComment>,
        files_scanned: &mut usize,
        files_errored: &mut usize,
    ) -> Result<(), TowlScannerError> {
        if *files_scanned >= MAX_FILES_SCANNED {
            warn!(
                "File scan limit reached ({} files), stopping scan",
                MAX_FILES_SCANNED
            );
            return Err(TowlScannerError::TooManyFiles {
                count: *files_scanned,
                max_allowed: MAX_FILES_SCANNED,
            });
        }

        match self.scan_file(path).await {
            Ok(mut file_todos) => {
                *files_scanned += 1;
                debug!("Found {} TODOs in {}", file_todos.len(), path.display());
                todos.append(&mut file_todos);
            }
            Err(e) => {
                *files_errored += 1;
                error!("Error scanning {}: {}", path.display(), e);
                eprintln!("Warning: Failed to scan {}: {}", path.display(), e);
            }
        }
        Ok(())
    }

    /// Recursively scans a directory for TODO comments in supported files.
    ///
    /// Walks the directory tree starting at `path`, scanning files that match
    /// the configured extensions while respecting exclude patterns.
    ///
    /// # Resource Limits
    /// - Skips files larger than 10 MB
    /// - Rejects files with more than 10,000 TODOs
    ///
    /// # Error Handling
    /// Individual file scan errors are logged but don't abort the overall scan.
    /// The scan continues processing remaining files.
    ///
    /// # Errors
    /// Returns `TowlScannerError` if directory traversal fails.
    ///
    /// # Example
    /// ```no_run
    /// use towl::scanner::Scanner;
    /// use towl::config::ParsingConfig;
    /// use std::path::PathBuf;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ParsingConfig::default();
    /// let scanner = Scanner::new(config)?;
    /// let result = scanner.scan(PathBuf::from(".")).await?;
    /// println!("Found {} TODOs in {} files", result.todos.len(), result.files_scanned);
    /// if result.all_files_failed() {
    ///     eprintln!("Warning: all files failed to scan");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn scan(&self, path: PathBuf) -> Result<ScanResult, TowlScannerError> {
        let scan_start = std::time::Instant::now();
        let mut files_scanned: usize = 0;
        let mut files_skipped: usize = 0;
        let mut files_errored: usize = 0;

        debug!("Scanning {}", path.display());
        let mut todos = Vec::new();
        let file_walker = self.build_walker(&path);

        for walk in file_walker {
            let entry = walk.map_err(TowlScannerError::UnableToWalkFile)?;
            let entry_path = entry.path();

            if !self.should_file_be_scanned(entry_path) {
                debug!("{0} will not be scanned", entry_path.display());
                files_skipped += 1;
                continue;
            }

            self.process_walk_entry(
                entry_path,
                &mut todos,
                &mut files_scanned,
                &mut files_errored,
            )
            .await?;

            if todos.len() > MAX_TOTAL_TODO_COUNT {
                warn!(
                    "Aggregate TODO count ({}) exceeds limit ({}), truncating",
                    todos.len(),
                    MAX_TOTAL_TODO_COUNT
                );
                todos.truncate(MAX_TOTAL_TODO_COUNT);
                let elapsed = scan_start.elapsed();
                return Ok(ScanResult {
                    todos,
                    files_scanned,
                    files_skipped,
                    files_errored,
                    duration: elapsed,
                });
            }
        }

        let elapsed = scan_start.elapsed();
        Self::log_scan_metrics(
            files_scanned,
            files_skipped,
            files_errored,
            todos.len(),
            elapsed,
        );

        Ok(ScanResult {
            todos,
            files_scanned,
            files_skipped,
            files_errored,
            duration: elapsed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::fmt::Write;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config() -> ParsingConfig {
        crate::config::test_parsing_config()
    }

    #[tokio::test]
    async fn test_scanner_integration() {
        let temp_dir = TempDir::new().unwrap();

        let rust_file = temp_dir.path().join("test.rs");
        fs::write(
            &rust_file,
            r#"
fn main() {
    // TODO: Implement main function
    println!("Hello");
    // FIXME: Fix this later
}
"#,
        )
        .unwrap();

        let python_file = temp_dir.path().join("test.py");
        fs::write(
            &python_file,
            r#"
def main():
    # TODO: Python TODO
    print("Hello")
"#,
        )
        .unwrap();

        let log_file = temp_dir.path().join("test.log");
        fs::write(&log_file, "// TODO: This should be ignored").unwrap();

        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let result = scanner.scan(temp_dir.path().to_path_buf()).await.unwrap();

        assert_eq!(result.todos.len(), 3);
        assert_eq!(result.files_scanned, 2);
        assert!(!result.all_files_failed());

        let descriptions: Vec<_> = result.todos.iter().map(|t| &t.description).collect();
        assert!(descriptions
            .iter()
            .any(|d| d.contains("Implement main function")));
        assert!(descriptions.iter().any(|d| d.contains("Fix this later")));
        assert!(descriptions.iter().any(|d| d.contains("Python TODO")));
    }

    prop_compose! {
        fn valid_filename()(
            name in r"[a-zA-Z0-9_-]{1,20}",
            ext in r"(rs|py|txt|js|log|md)"
        ) -> String {
            format!("{name}.{ext}")
        }
    }

    prop_compose! {
        fn safe_file_content()(
            lines in prop::collection::vec(r"[a-zA-Z0-9 //\-_.,!?]*", 1..20)
        ) -> String {
            lines.join("\n")
        }
    }

    prop_compose! {
            fn todo_comment()(
    keyword in r"(TODO|FIXME|HACK|NOTE|BUG)",
                description in r"[a-zA-Z0-9 .,!?-]{1,50}"
            ) -> String {
    format!("// {keyword}: {description}")
            }
        }

    proptest! {
        #[test]
        fn prop_test_file_scanning_consistency(
            filename in valid_filename(),
            content in safe_file_content()
        ) {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join(&filename);
            fs::write(&file_path, &content).unwrap();

            let config = create_test_config();
            let scanner = Scanner::new(config).unwrap();

            let should_scan = scanner.should_file_be_scanned(&file_path);
            let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

            if ["rs", "py", "txt"].iter().any(|e| extension.eq_ignore_ascii_case(e)) && !std::path::Path::new(&filename)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("log")) {
                prop_assert!(should_scan, "Should scan file with valid extension: {}", filename);
            } else {
                prop_assert!(!should_scan, "Should not scan file with invalid extension: {}", filename);
            }

        }

        #[test]
        fn prop_test_todo_detection_in_files(
            filename in r"[a-zA-Z0-9_-]{1,20}\.rs",
            todo_comments in prop::collection::vec(todo_comment(), 1..5),
            regular_lines in prop::collection::vec(r"[a-zA-Z0-9 ]*", 0..10)
        ) {
            tokio_test::block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let file_path = temp_dir.path().join(&filename);

                let mut all_lines = regular_lines;
                all_lines.extend(todo_comments.clone());
                let content = all_lines.join("\n");

                fs::write(&file_path, &content).unwrap();

                let config = create_test_config();
                let scanner = Scanner::new(config).unwrap();

                let result = scanner.scan(temp_dir.path().to_path_buf()).await.unwrap();

                prop_assert!(result.todos.len() >= todo_comments.len(),
                           "Should find at least {} TODOs, found {}", todo_comments.len(), result.todos.len());

                for todo_comment in &todo_comments {
                    let found = result.todos.iter().any(|t| {
                        todo_comment.contains(&t.description) || t.original_text.contains(todo_comment)
                    });
                    prop_assert!(found, "Should find TODO comment: {}", todo_comment);
                }

                Ok(())
            })?;
        }

        #[test]
        fn prop_test_path_handling(
            path_components in prop::collection::vec(r"[a-zA-Z0-9_.-]{1,10}", 1..5)
        ) {
            let config = create_test_config();
            let scanner = Scanner::new(config).unwrap();

            let path = PathBuf::from(path_components.join("/"));

            let result = scanner.should_file_be_scanned(&path);
            // Generated paths don't exist on disk, so is_file() returns false
            prop_assert!(!result, "Non-existent paths should not be scanned: {:?}", path);
        }
    }

    #[tokio::test]
    async fn test_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let result = scanner.scan(temp_dir.path().to_path_buf()).await.unwrap();
        assert!(result.todos.is_empty());
        assert!(result.is_clean());
    }

    #[tokio::test]
    async fn test_nested_directories() {
        let temp_dir = TempDir::new().unwrap();

        let nested_dir = temp_dir.path().join("src").join("lib");
        fs::create_dir_all(&nested_dir).unwrap();

        let nested_file = nested_dir.join("test.rs");
        fs::write(&nested_file, "// TODO: Nested file").unwrap();

        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let result = scanner.scan(temp_dir.path().to_path_buf()).await.unwrap();
        assert_eq!(result.todos.len(), 1);
        assert!(result.todos[0].description.contains("Nested file"));
    }

    #[tokio::test]
    async fn test_large_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large.rs");

        let mut content = String::new();
        for i in 0..1000 {
            writeln!(content, "// TODO: Item number {i}").unwrap();
            content.push_str("fn dummy_function() {}\n");
        }

        fs::write(&file_path, &content).unwrap();

        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let result = scanner.scan(temp_dir.path().to_path_buf()).await.unwrap();
        assert_eq!(result.todos.len(), 1000);
    }

    #[tokio::test]
    async fn test_unicode_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("unicode.rs");

        let content = r"
// TODO: Fix café rendering
// FIXME: Handle señor properly
// HACK: Temporary fix for 中文
";

        fs::write(&file_path, content).unwrap();

        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let result = scanner.scan(temp_dir.path().to_path_buf()).await.unwrap();
        assert_eq!(result.todos.len(), 3);

        let descriptions: Vec<_> = result.todos.iter().map(|t| &t.description).collect();
        assert!(descriptions.iter().any(|d| d.contains("café")));
        assert!(descriptions.iter().any(|d| d.contains("señor")));
        assert!(descriptions.iter().any(|d| d.contains("中文")));
    }

    #[tokio::test]
    async fn test_binary_file_handling() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("binary.rs");

        let binary_data = vec![0, 1, 2, 3, 255, 254, 253];
        fs::write(&file_path, &binary_data).unwrap();

        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let result = scanner.scan(temp_dir.path().to_path_buf()).await;
        assert!(result.is_ok());
        let scan_result = result.unwrap();
        assert!(
            scan_result.todos.is_empty(),
            "Binary file should not produce any TODOs"
        );
        assert!(
            scan_result.all_files_failed(),
            "Binary file read should error"
        );
    }

    #[tokio::test]
    async fn test_file_size_limit_enforced() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("huge.rs");

        let content = "a".repeat(11 * 1024 * 1024);
        fs::write(&file_path, &content).unwrap();

        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let result = scanner.scan_file(&file_path).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            TowlScannerError::FileTooLarge {
                size, max_allowed, ..
            } => {
                assert!(size > max_allowed);
                assert_eq!(max_allowed, MAX_FILE_SIZE);
            }
            e => panic!("Expected FileTooLarge error, got: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_file_size_under_limit_accepted() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("small.rs");

        let content = format!("// TODO: Test\n{}", "fn dummy() {}\n".repeat(1000));
        fs::write(&file_path, &content).unwrap();

        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let todos = scanner.scan_file(&file_path).await.unwrap();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].description, "Test");
    }

    #[tokio::test]
    async fn test_scan_file_nonexistent_path_returns_invalid_path() {
        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let result = scanner
            .scan_file(Path::new("/nonexistent/path/file.rs"))
            .await;
        assert!(matches!(result, Err(TowlScannerError::InvalidPath { .. })));
    }

    #[tokio::test]
    async fn test_todo_count_limit_enforced() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("many_todos.rs");

        let mut content = String::new();
        for i in 0..10_001 {
            writeln!(content, "// TODO: Item {i}").unwrap();
        }

        fs::write(&file_path, &content).unwrap();

        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let result = scanner.scan_file(&file_path).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            TowlScannerError::TooManyTodos {
                count, max_allowed, ..
            } => {
                assert!(count > max_allowed);
                assert_eq!(max_allowed, MAX_TODO_COUNT);
            }
            e => panic!("Expected TooManyTodos error, got: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_todo_count_under_limit_accepted() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("reasonable_todos.rs");

        let mut content = String::new();
        for i in 0..100 {
            writeln!(content, "// TODO: Item {i}").unwrap();
        }

        fs::write(&file_path, &content).unwrap();

        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let result = scanner.scan_file(&file_path).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 100);
    }

    #[tokio::test]
    async fn test_resource_limits_in_scan_directory() {
        let temp_dir = TempDir::new().unwrap();

        let normal_file = temp_dir.path().join("normal.rs");
        fs::write(&normal_file, "// TODO: Normal file").unwrap();

        let huge_file = temp_dir.path().join("huge.rs");
        let huge_content = "a".repeat(11 * 1024 * 1024);
        fs::write(&huge_file, &huge_content).unwrap();

        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let result = scanner.scan(temp_dir.path().to_path_buf()).await;
        assert!(result.is_ok());

        let scan_result = result.unwrap();
        assert_eq!(scan_result.todos.len(), 1);
        assert!(scan_result.todos[0].description.contains("Normal file"));
        assert_eq!(scan_result.files_errored, 1);
        assert!(!scan_result.all_files_failed());
    }

    #[tokio::test]
    async fn test_all_files_failed_when_only_errors() {
        let temp_dir = TempDir::new().unwrap();

        let binary_file = temp_dir.path().join("invalid.rs");
        fs::write(&binary_file, [0u8, 1, 2, 255, 254, 253]).unwrap();

        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let result = scanner.scan(temp_dir.path().to_path_buf()).await.unwrap();
        assert!(result.todos.is_empty());
        assert!(result.all_files_failed());
        assert!(!result.is_clean());
    }
}
