use std::path::{Path, PathBuf};

use ignore::{overrides::OverrideBuilder, WalkBuilder};
use tracing::{debug, error};

use crate::{comment::todo::TodoComment, config::config::ParsingConfig, parser::parser::Parser};

use super::error::TowlScannerError;

pub struct Scanner {
    parser: Parser,
    config: ParsingConfig,
}

impl Scanner {
    pub fn new(config: ParsingConfig) -> Result<Self, TowlScannerError> {
        let parser = Parser::new(&config).map_err(TowlScannerError::ParsingError)?;
        Ok(Scanner {
            parser,
            config: config.clone(),
        })
    }

    fn should_file_be_scanned(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }

        if path.to_string_lossy().contains("..") {
            return false;
        }

        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                return self.config.file_extensions.contains(&ext_str.to_string());
            }
        }

        false
    }
    async fn scan_file(&self, path: &Path) -> Result<Vec<TodoComment>, TowlScannerError> {
        match path.canonicalize() {
            Ok(canonical) => {
                if canonical.to_string_lossy().contains("..") {
                    return Err(TowlScannerError::PathTraversalAttempt {
                        path: path.to_path_buf(),
                    });
                }
            }
            Err(_) => {
                return Err(TowlScannerError::InvalidPath {
                    path: path.to_path_buf(),
                });
            }
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| TowlScannerError::UnableToReadFileAtPath(path.to_path_buf(), e))?;

        self.parser
            .parse(path, &content)
            .map_err(TowlScannerError::ParsingError)
    }

    pub async fn scan(&self, path: PathBuf) -> Result<Vec<TodoComment>, TowlScannerError> {
        tracing::debug!("Scanning {}", path.display());
        let mut todos = Vec::new();
        let mut builder = WalkBuilder::new(&path);
        builder.hidden(false).git_ignore(false);

        let mut overrides = OverrideBuilder::new(&path);
        for pattern in &self.config.exclude_patterns {
            if let Err(e) = overrides.add(&format!("!{}", pattern)) {
                debug!("Failed to add exclude pattern '{}': {}", pattern, e);
            }
        }
        if let Ok(overrides) = overrides.build() {
            builder.overrides(overrides);
        }

        let file_walker = builder.build();

        for walk in file_walker {
            let entry = walk.map_err(TowlScannerError::UnableToWalkFile)?;
            let path = entry.path();

            if !self.should_file_be_scanned(path) {
                debug!("{0} will not be scanned", path.display());
                continue;
            }

            match self.scan_file(path).await {
                Ok(mut file_todos) => {
                    debug!("Found {} TODOs in {}", file_todos.len(), path.display());
                    todos.append(&mut file_todos);
                }
                Err(e) => {
                    error!("Error scanning {}: {}", path.display(), e);
                }
            }
        }
        Ok(todos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rstest::rstest;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config() -> ParsingConfig {
        ParsingConfig {
            file_extensions: vec!["rs".to_string(), "py".to_string(), "txt".to_string()],
            exclude_patterns: vec!["target/*".to_string(), "*.log".to_string()],
            include_context_lines: 3,
            comment_prefixes: vec![
                r"//".to_string(),
                r"^\s*#".to_string(),
                r"/\*".to_string(),
                r"^\s*\*".to_string(),
            ],
            todo_patterns: vec![
                r"(?i)\bTODO:\s*(.*)".to_string(),
                r"(?i)\bFIXME:\s*(.*)".to_string(),
                r"(?i)\bHACK:\s*(.*)".to_string(),
                r"(?i)\bNOTE:\s*(.*)".to_string(),
                r"(?i)\bBUG:\s*(.*)".to_string(),
            ],
            function_patterns: vec![
                r"^\s*(pub\s+)?fn\s+(\w+)".to_string(),
                r"^\s*def\s+(\w+)".to_string(),
            ],
        }
    }

    #[rstest]
    #[case("test.rs", true)]
    #[case("test.py", true)]
    #[case("test.txt", true)]
    #[case("test.js", false)]
    #[case("test.log", false)]
    #[case("README.md", false)]
    fn test_file_extension_filtering(#[case] filename: &str, #[case] should_scan: bool) {
        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(filename);
        fs::write(&file_path, "// TODO: test content").unwrap();

        let result = scanner.should_file_be_scanned(&file_path);
        assert_eq!(
            result, should_scan,
            "File {} scan decision incorrect",
            filename
        );
    }

    #[rstest]
    #[case("normal/path/file.rs", true)]
    #[case("../../../etc/passwd", false)]
    #[case("..\\..\\windows\\system32", false)]
    #[case("subdir/../file.rs", false)]
    fn test_path_traversal_protection(#[case] path_str: &str, #[case] _should_be_safe: bool) {
        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let path = PathBuf::from(path_str);
        let result = scanner.should_file_be_scanned(&path);

        if path_str.contains("..") {
            assert!(
                !result,
                "Path with traversal should not be scanned: {}",
                path_str
            );
        }
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

        let todos = scanner.scan(temp_dir.path().to_path_buf()).await.unwrap();

        assert_eq!(todos.len(), 3);

        let descriptions: Vec<_> = todos.iter().map(|t| &t.description).collect();
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
            format!("{}.{}", name, ext)
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
    format!("// {}: {}", keyword, description)
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

                if ["rs", "py", "txt"].contains(&extension) && !filename.ends_with(".log") {
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
                let _ = tokio_test::block_on(async {
                    let temp_dir = TempDir::new().unwrap();
                    let file_path = temp_dir.path().join(&filename);

                    let mut all_lines = regular_lines;
                    all_lines.extend(todo_comments.clone());
                    let content = all_lines.join("\n");

    fs::write(&file_path, &content).unwrap();

                    let config = create_test_config();
                    let scanner = Scanner::new(config).unwrap();

                    let todos = scanner.scan(temp_dir.path().to_path_buf()).await.unwrap();

                    prop_assert!(todos.len() >= todo_comments.len(),
                               "Should find at least {} TODOs, found {}", todo_comments.len(), todos.len());

                    for todo_comment in &todo_comments {
                        let found = todos.iter().any(|t| {
                todo_comment.contains(&t.description) || t.original_text.contains(todo_comment)
                        });
            prop_assert!(found, "Should find TODO comment: {}", todo_comment);
                    }

                    Ok(())
    });
            }

            #[test]
            fn prop_test_path_safety(
                safe_components in prop::collection::vec(r"[a-zA-Z0-9_-]{1,10}", 1..5),
                dangerous_components in prop::collection::vec(r"\.\.(/|\\)?", 1..3)
            ) {
                let config = create_test_config();
                let scanner = Scanner::new(config).unwrap();

                let _safe_path = PathBuf::from(safe_components.join("/"));

                let mut dangerous_parts = safe_components.clone();
                dangerous_parts.extend(dangerous_components);
                let dangerous_path = PathBuf::from(dangerous_parts.join("/"));

                let dangerous_result = scanner.should_file_be_scanned(&dangerous_path);
                prop_assert!(!dangerous_result,
                            "Dangerous path should not be scanned: {}", dangerous_path.display());
            }
        }

    #[tokio::test]
    async fn test_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let todos = scanner.scan(temp_dir.path().to_path_buf()).await.unwrap();
        assert!(todos.is_empty());
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

        let todos = scanner.scan(temp_dir.path().to_path_buf()).await.unwrap();
        assert_eq!(todos.len(), 1);
        assert!(todos[0].description.contains("Nested file"));
    }

    #[tokio::test]
    async fn test_large_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large.rs");

        let mut content = String::new();
        for i in 0..1000 {
            content.push_str(&format!("// TODO: Item number {}\n", i));
            content.push_str("fn dummy_function() {}\n");
        }

        fs::write(&file_path, &content).unwrap();

        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let todos = scanner.scan(temp_dir.path().to_path_buf()).await.unwrap();
        assert_eq!(todos.len(), 1000);
    }

    #[tokio::test]
    async fn test_unicode_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("unicode.rs");

        let content = r#"
// TODO: Fix café rendering
// FIXME: Handle señor properly
// HACK: Temporary fix for 中文
"#;

        fs::write(&file_path, content).unwrap();

        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let todos = scanner.scan(temp_dir.path().to_path_buf()).await.unwrap();
        assert_eq!(todos.len(), 3);

        let descriptions: Vec<_> = todos.iter().map(|t| &t.description).collect();
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
    }

    #[test]
    fn test_scanner_creation_with_invalid_config() {
        let mut config = create_test_config();
        config.todo_patterns = vec!["[invalid regex".to_string()];

        let result = Scanner::new(config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_concurrent_file_access() {
        let temp_dir = TempDir::new().unwrap();

        for i in 0..10 {
            let file_path = temp_dir.path().join(format!("test{}.rs", i));
            fs::write(&file_path, format!("// TODO: File {}", i)).unwrap();
        }

        let config = create_test_config();
        let scanner = Scanner::new(config).unwrap();

        let todos = scanner.scan(temp_dir.path().to_path_buf()).await.unwrap();
        assert_eq!(todos.len(), 10);

        for i in 0..10 {
            let found = todos
                .iter()
                .any(|t| t.description.contains(&format!("File {}", i)));
            assert!(found, "Should find TODO from file {}", i);
        }
    }
}
