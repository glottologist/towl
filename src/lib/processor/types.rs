use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::comment::todo::TodoComment;
use crate::github::types::CreatedIssue;

use super::error::TowlProcessorError;

/// Summary of a batch TODO replacement operation.
#[derive(Debug)]
pub struct ProcessorResult {
    pub files_modified: usize,
    pub todos_replaced: usize,
    /// Per-file errors that did not abort the overall operation.
    pub errors: Vec<(PathBuf, TowlProcessorError)>,
}

/// Replaces TODO comments in source files with GitHub issue links.
pub struct Processor;

impl Processor {
    /// Replaces TODO comments with issue links in source files.
    ///
    /// Groups replacements by file to minimize I/O.
    /// Uses atomic writes to prevent partial updates.
    pub async fn replace_todos(
        repo_root: &Path,
        replacements: &[(TodoComment, CreatedIssue)],
    ) -> ProcessorResult {
        if replacements.is_empty() {
            return ProcessorResult {
                files_modified: 0,
                todos_replaced: 0,
                errors: Vec::new(),
            };
        }

        let mut by_file: HashMap<&Path, Vec<(&TodoComment, &CreatedIssue)>> = HashMap::new();
        for (todo, issue) in replacements {
            by_file
                .entry(todo.file_path.as_path())
                .or_default()
                .push((todo, issue));
        }

        let mut files_modified = 0;
        let mut todos_replaced = 0;
        let mut errors: Vec<(PathBuf, TowlProcessorError)> = Vec::new();

        for (path, file_replacements) in &by_file {
            match Self::process_file(repo_root, path, file_replacements).await {
                Ok(count) => {
                    files_modified += 1;
                    todos_replaced += count;
                }
                Err(e) => {
                    errors.push((path.to_path_buf(), e));
                }
            }
        }

        ProcessorResult {
            files_modified,
            todos_replaced,
            errors,
        }
    }

    async fn process_file(
        repo_root: &Path,
        path: &Path,
        replacements: &[(&TodoComment, &CreatedIssue)],
    ) -> Result<usize, TowlProcessorError> {
        let canonical = path
            .canonicalize()
            .map_err(|e| TowlProcessorError::FileReadError(path.to_path_buf(), e))?; // clone: owned path for error variant
        let canonical_root = repo_root
            .canonicalize()
            .map_err(|e| TowlProcessorError::FileReadError(repo_root.to_path_buf(), e))?; // clone: owned path for error variant

        if !canonical.starts_with(&canonical_root) {
            return Err(TowlProcessorError::PathOutsideRoot {
                path: path.to_path_buf(),      // clone: owned path for error variant
                root: repo_root.to_path_buf(), // clone: owned path for error variant
            });
        }

        let content = tokio::fs::read_to_string(&canonical)
            .await
            .map_err(|e| TowlProcessorError::FileReadError(canonical.clone(), e))?; // clone: need path for error variant, also used later for write

        let mut lines: Vec<String> = content.lines().map(String::from).collect();
        let total_lines = lines.len();
        let mut replaced = 0;

        for (todo, issue) in replacements {
            let line_idx = todo.line_number.checked_sub(1).ok_or_else(|| {
                TowlProcessorError::LineOutOfBounds {
                    path: path.to_path_buf(), // clone: owned path for error variant
                    line: todo.line_number,
                    total_lines,
                }
            })?;

            if line_idx >= total_lines {
                return Err(TowlProcessorError::LineOutOfBounds {
                    path: path.to_path_buf(), // clone: owned path for error variant
                    line: todo.line_number,
                    total_lines,
                });
            }

            if !issue.html_url.starts_with("https://github.com/") {
                return Err(TowlProcessorError::InvalidIssueUrl {
                    url: issue.html_url.clone(), // clone: need owned String for error variant
                });
            }

            let line = &lines[line_idx];
            let prefix = line.get(..todo.column_start).ok_or_else(|| {
                TowlProcessorError::CommentPrefixNotFound {
                    path: path.to_path_buf(), // clone: owned path for error variant
                    line: todo.line_number,
                }
            })?;
            let desc = todo.description.trim();
            if desc.is_empty() {
                lines[line_idx] = format!("{prefix}GH_ISSUE: {}", issue.html_url);
            } else {
                lines[line_idx] = format!("{prefix}GH_ISSUE: {} : {desc}", issue.html_url);
            }
            replaced += 1;
        }

        let mut new_content = lines.join("\n");
        if content.ends_with('\n') {
            new_content.push('\n');
        }

        crate::atomic_write(&canonical, new_content.as_bytes())
            .await
            .map_err(|e| TowlProcessorError::FileWriteError(canonical, e))?;

        Ok(replaced)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comment::todo::test_support::TestTodoBuilder;
    use crate::comment::todo::TodoComment;
    use crate::github::types::CreatedIssue;
    use proptest::prelude::*;
    use rstest::rstest;

    fn make_todo(
        path: &Path,
        line_number: usize,
        column_start: usize,
        original_text: &str,
        description: &str,
    ) -> TodoComment {
        TestTodoBuilder::new()
            .file_path(path)
            .line_number(line_number)
            .column_start(column_start)
            .column_end(original_text.len())
            .original_text(original_text)
            .description(description)
            .build()
    }

    fn make_issue(number: u64) -> CreatedIssue {
        CreatedIssue::new(
            number,
            format!("Issue #{number}"),
            format!("https://github.com/owner/repo/issues/{number}"),
            format!("test_{number}"),
        )
    }

    proptest! {
        #[test]
        fn prop_indentation_preserved(
            indent in " {0,20}",
            desc in "[a-zA-Z0-9 ]{1,50}",
            issue_num in 1u64..100_000,
        ) {
            let line = format!("{indent}// TODO: {desc}");
            let column_start = indent.len() + 3;

            let temp_dir = tempfile::TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.rs");
            std::fs::write(&file_path, &line).unwrap();

            let todo = make_todo(&file_path, 1, column_start, &line, &desc);
            let issue = make_issue(issue_num);

            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(Processor::replace_todos(temp_dir.path(), &[(todo, issue)]));

            prop_assert_eq!(result.errors.len(), 0);
            prop_assert_eq!(result.todos_replaced, 1);
            prop_assert_eq!(result.files_modified, 1);

            let content = std::fs::read_to_string(&file_path).unwrap();
            let expected_prefix = format!("{indent}// ");
            prop_assert!(
                content.starts_with(&expected_prefix),
                "Indent+prefix not preserved. Got: {:?}, Expected start: {:?}",
                content,
                expected_prefix
            );
            let expected_url = format!("https://github.com/owner/repo/issues/{issue_num}");
            prop_assert!(
                content.contains(&expected_url),
                "URL missing. Got: {:?}",
                content
            );
            let trimmed = desc.trim();
            if !trimmed.is_empty() {
                prop_assert!(
                    content.contains(trimmed),
                    "Description missing. Got: {:?}",
                    content
                );
            }
        }

        #[test]
        fn prop_replacement_produces_valid_line(
            desc in "[a-zA-Z0-9 ]{1,50}",
            issue_num in 1u64..100_000,
        ) {
            let line = format!("// TODO: {desc}");

            let temp_dir = tempfile::TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.rs");
            std::fs::write(&file_path, &line).unwrap();

            let todo = make_todo(&file_path, 1, 3, &line, &desc);
            let issue = make_issue(issue_num);
            let url = format!("https://github.com/owner/repo/issues/{issue_num}");

            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(Processor::replace_todos(temp_dir.path(), &[(todo, issue)]));

            prop_assert_eq!(result.errors.len(), 0);

            let content = std::fs::read_to_string(&file_path).unwrap();
            let trimmed = desc.trim();
            let expected = if trimmed.is_empty() {
                format!("// GH_ISSUE: {url}")
            } else {
                format!("// GH_ISSUE: {url} : {trimmed}")
            };
            prop_assert_eq!(content, expected);
        }
    }

    #[rstest]
    #[case("// TODO: fix", 3, "// GH_ISSUE:", "fix")]
    #[case("# TODO: fix", 2, "# GH_ISSUE:", "fix")]
    #[case("/* TODO: fix", 3, "/* GH_ISSUE:", "fix")]
    #[case("* TODO: fix", 2, "* GH_ISSUE:", "fix")]
    #[tokio::test]
    async fn test_comment_prefix_preserved(
        #[case] original: &str,
        #[case] column_start: usize,
        #[case] expected_start: &str,
        #[case] desc: &str,
    ) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        std::fs::write(&file_path, original).unwrap();

        let todo = make_todo(&file_path, 1, column_start, original, desc);
        let issue = make_issue(42);

        let result = Processor::replace_todos(temp_dir.path(), &[(todo, issue)]).await;

        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.todos_replaced, 1);

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(
            content.starts_with(expected_start),
            "Expected start: {expected_start}, got: {content}"
        );
        assert!(content.contains("github.com/owner/repo/issues/42"));
        assert!(
            content.ends_with(&format!(" : {desc}")),
            "Description missing. Got: {content}"
        );
    }

    #[rstest]
    #[case(2)]
    #[case(5)]
    #[case(10)]
    #[tokio::test]
    async fn test_multiple_todos_same_file(#[case] count: usize) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let mut lines = Vec::with_capacity(count);
        let mut pairs = Vec::with_capacity(count);

        for i in 0..count {
            let desc = format!("fix item {i}");
            let line = format!("// TODO: {desc}");
            let todo = make_todo(&file_path, i + 1, 3, &line, &desc);
            let issue_num = u64::try_from(i + 1).unwrap();
            let issue = make_issue(issue_num);
            lines.push(line);
            pairs.push((todo, issue));
        }

        std::fs::write(&file_path, lines.join("\n")).unwrap();

        let result = Processor::replace_todos(temp_dir.path(), &pairs).await;

        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.todos_replaced, count);
        assert_eq!(result.files_modified, 1);

        let content = std::fs::read_to_string(&file_path).unwrap();
        for i in 1..=count {
            assert!(
                content.contains(&format!("github.com/owner/repo/issues/{i}")),
                "Missing issue #{i} in output"
            );
        }
    }

    #[tokio::test]
    async fn test_empty_replacements() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let result = Processor::replace_todos(temp_dir.path(), &[]).await;

        assert_eq!(result.files_modified, 0);
        assert_eq!(result.todos_replaced, 0);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_line_out_of_bounds() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        std::fs::write(&file_path, "// single line").unwrap();

        let todo = make_todo(&file_path, 99, 3, "// TODO: fix", "fix");
        let issue = make_issue(1);

        let result = Processor::replace_todos(temp_dir.path(), &[(todo, issue)]).await;

        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.todos_replaced, 0);
    }

    #[tokio::test]
    async fn test_nonexistent_file_error() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let todo = make_todo(
            &temp_dir.path().join("nonexistent_towl_test.rs"),
            1,
            3,
            "// TODO: fix",
            "fix",
        );
        let issue = make_issue(1);

        let result = Processor::replace_todos(temp_dir.path(), &[(todo, issue)]).await;

        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.files_modified, 0);
    }

    #[tokio::test]
    async fn test_comment_prefix_not_found() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        std::fs::write(&file_path, "short").unwrap();

        let todo = make_todo(&file_path, 1, 100, "// TODO: fix", "fix");
        let issue = make_issue(1);

        let result = Processor::replace_todos(temp_dir.path(), &[(todo, issue)]).await;

        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.todos_replaced, 0);
    }

    #[tokio::test]
    async fn test_path_outside_root() {
        let root_dir = tempfile::TempDir::new().unwrap();
        let other_dir = tempfile::TempDir::new().unwrap();
        let file_path = other_dir.path().join("test.rs");
        std::fs::write(&file_path, "// TODO: fix").unwrap();

        let todo = make_todo(&file_path, 1, 3, "// TODO: fix", "fix");
        let issue = make_issue(1);

        let result = Processor::replace_todos(root_dir.path(), &[(todo, issue)]).await;

        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.files_modified, 0);
    }

    #[tokio::test]
    async fn test_multiple_files() {
        let temp_dir = tempfile::TempDir::new().unwrap();

        let file_a = temp_dir.path().join("a.rs");
        let file_b = temp_dir.path().join("b.rs");
        std::fs::write(&file_a, "// TODO: fix a").unwrap();
        std::fs::write(&file_b, "# TODO: fix b").unwrap();

        let todo_a = make_todo(&file_a, 1, 3, "// TODO: fix a", "fix a");
        let todo_b = make_todo(&file_b, 1, 2, "# TODO: fix b", "fix b");
        let issue_a = make_issue(1);
        let issue_b = make_issue(2);

        let result =
            Processor::replace_todos(temp_dir.path(), &[(todo_a, issue_a), (todo_b, issue_b)])
                .await;

        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.files_modified, 2);
        assert_eq!(result.todos_replaced, 2);

        let content_a = std::fs::read_to_string(&file_a).unwrap();
        let content_b = std::fs::read_to_string(&file_b).unwrap();
        assert!(content_a.contains("issues/1"));
        assert!(content_b.contains("issues/2"));
    }
}
