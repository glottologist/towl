//! Output formatting and writing for scan results.
//!
//! Supports JSON, CSV, TOML, Markdown, and terminal table formats. File-based
//! formats write to a path; terminal formats write to stdout.

pub mod error;
pub mod formatter;
pub mod writer;

use std::path::Path;

use error::TowlOutputError;
use formatter::{
    formatters::{
        csv::CsvFormatter, json::JsonFormatter, markdown::MarkdownFormatter, table::TableFormatter,
        toml::TomlFormatter,
    },
    FormatterImpl,
};
use writer::{
    writers::{file::FileWriter, stdout::StdoutWriter},
    WriterImpl,
};

use crate::{
    cli::OutputFormat,
    comment::todo::{TodoComment, TodoType},
};
use std::{collections::HashMap, path::PathBuf};

const TERMINAL_FORMAT_FILE_ERROR: &str = "Terminal format cannot write to file";

/// Handles formatting and writing TODO comments to various output destinations.
///
/// Supports multiple output formats (JSON, CSV, TOML, Markdown, Table) with
/// appropriate writers (file or stdout) based on format constraints.
pub struct Output {
    writer: WriterImpl,
    formatter: FormatterImpl,
}

impl Output {
    /// Creates a new output handler for the specified format and destination.
    ///
    /// # Format Constraints
    /// - `Terminal` and `Table`: Must output to stdout (`output_path` must be `None`)
    /// - `Json`, `Csv`, `Toml`, `Markdown`: Require `output_path` with matching extension
    ///
    /// # Errors
    /// Returns `TowlOutputError::InvalidOutputPath` if:
    /// - Terminal/Table format is used with a file path
    /// - File-based formats are used without an output path
    /// - File extension doesn't match the expected format
    ///
    /// # Example
    /// ```no_run
    /// use towl::output::Output;
    /// use towl::cli::OutputFormat;
    /// use std::path::PathBuf;
    ///
    /// // Terminal output (stdout)
    /// let output = Output::new(OutputFormat::Terminal, None)?;
    ///
    /// // JSON file output
    /// let output = Output::new(
    ///     OutputFormat::Json,
    ///     Some(PathBuf::from("todos.json"))
    /// )?;
    /// # Ok::<(), towl::output::error::TowlOutputError>(())
    /// ```
    pub fn new(
        output_format: OutputFormat,
        output_path: Option<PathBuf>,
    ) -> Result<Self, TowlOutputError> {
        let (formatter, writer) = match output_format {
            OutputFormat::Terminal => {
                if output_path.is_some() {
                    return Err(TowlOutputError::InvalidOutputPath(
                        TERMINAL_FORMAT_FILE_ERROR.to_string(),
                    ));
                }
                (
                    FormatterImpl::Table(TableFormatter),
                    WriterImpl::Stdout(StdoutWriter::new()),
                )
            }
            OutputFormat::Json => Self::file_output(
                output_path,
                "JSON",
                "json",
                FormatterImpl::Json(JsonFormatter),
            )?,
            OutputFormat::Csv => {
                Self::file_output(output_path, "CSV", "csv", FormatterImpl::Csv(CsvFormatter))?
            }
            OutputFormat::Toml => Self::file_output(
                output_path,
                "TOML",
                "toml",
                FormatterImpl::Toml(TomlFormatter),
            )?,
            OutputFormat::Markdown => Self::file_output(
                output_path,
                "Markdown",
                "md",
                FormatterImpl::Markdown(MarkdownFormatter),
            )?,
        };
        Ok(Self { writer, formatter })
    }

    fn file_output(
        output_path: Option<PathBuf>,
        format_name: &str,
        extension: &str,
        formatter: FormatterImpl,
    ) -> Result<(FormatterImpl, WriterImpl), TowlOutputError> {
        let path = output_path.ok_or_else(|| {
            TowlOutputError::InvalidOutputPath(format!(
                "{format_name} format requires an output file path"
            ))
        })?;
        Self::validate_file_extension(&path, extension)?;
        let writer = FileWriter::new(path).map_err(TowlOutputError::UnableToWriteTodos)?;
        Ok((formatter, WriterImpl::File(writer)))
    }

    fn validate_file_extension(path: &Path, expected_ext: &str) -> Result<(), TowlOutputError> {
        path.extension().and_then(|e| e.to_str()).map_or_else(
            || {
                Err(TowlOutputError::InvalidOutputPath(format!(
                    "Output file must have '{expected_ext}' extension"
                )))
            },
            |ext| {
                if ext.eq_ignore_ascii_case(expected_ext) {
                    Ok(())
                } else {
                    Err(TowlOutputError::InvalidOutputPath(format!(
                        "File extension '{ext}' does not match expected extension '{expected_ext}' for this format"
                    )))
                }
            },
        )
    }

    pub(crate) fn group_todos_by_type(todos: &[TodoComment]) -> Vec<(TodoType, Vec<&TodoComment>)> {
        let mut todo_map: HashMap<TodoType, Vec<&TodoComment>> = HashMap::new();
        for todo in todos {
            todo_map.entry(todo.todo_type).or_default().push(todo);
        }

        // HashMap grouping and concurrent-scan arrival order are both
        // unordered; sort so identical scans produce identical output bytes
        let mut groups: Vec<_> = todo_map.into_iter().collect();
        groups.sort_by_key(|&(todo_type, _)| todo_type.priority());
        for (_, group) in &mut groups {
            group.sort_by(|a, b| {
                a.file_path
                    .cmp(&b.file_path)
                    .then(a.line_number.cmp(&b.line_number))
            });
        }
        groups
    }

    /// Saves TODO comments using the configured formatter and writer.
    ///
    /// Formats the TODOs according to the output format and writes them to
    /// the configured destination (file or stdout).
    ///
    /// # Errors
    /// Returns `TowlOutputError` if formatting or writing fails.
    ///
    /// # Example
    /// ```no_run
    /// use towl::output::Output;
    /// use towl::cli::OutputFormat;
    /// use towl::comment::todo::TodoComment;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let output = Output::new(OutputFormat::Terminal, None)?;
    /// let todos: Vec<TodoComment> = vec![];
    /// output.save(&todos).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn save(&self, todos: &[TodoComment]) -> Result<(), TowlOutputError> {
        let groups = Self::group_todos_by_type(todos);
        let total_count = todos.len();
        let formatted = self
            .formatter
            .format(&groups, total_count)
            .map_err(TowlOutputError::UnableToFormatTodos)?;
        self.writer
            .write(formatted)
            .await
            .map_err(TowlOutputError::UnableToWriteTodos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::OutputFormat;
    use crate::comment::todo::TodoType;
    use crate::output::formatter::formatters::test_helpers::create_test_todo;
    use rstest::rstest;

    #[rstest]
    #[case(OutputFormat::Terminal, None, true)]
    #[case(OutputFormat::Json, Some("todos.json"), true)]
    #[case(OutputFormat::Csv, Some("todos.csv"), true)]
    #[case(OutputFormat::Toml, Some("todos.toml"), true)]
    #[case(OutputFormat::Markdown, Some("todos.md"), true)]
    #[case(OutputFormat::Terminal, Some("file.txt"), false)]
    #[case(OutputFormat::Json, None, false)]
    #[case(OutputFormat::Csv, None, false)]
    #[case(OutputFormat::Toml, None, false)]
    #[case(OutputFormat::Markdown, None, false)]
    fn test_output_new_dispatch(
        #[case] format: OutputFormat,
        #[case] path: Option<&str>,
        #[case] should_succeed: bool,
    ) {
        let output_path = path.map(PathBuf::from);
        let result = Output::new(format, output_path);

        assert_eq!(
            result.is_ok(),
            should_succeed,
            "Output::new({format:?}, {path:?}) expected success={should_succeed}, got {:?}",
            result.err()
        );
    }

    #[rstest]
    #[case("todos.json", "json", true)]
    #[case("todos.JSON", "json", true)]
    #[case("todos.Json", "json", true)]
    #[case("todos.csv", "csv", true)]
    #[case("todos.toml", "toml", true)]
    #[case("todos.md", "md", true)]
    #[case("todos.txt", "json", false)]
    #[case("todos.csv", "json", false)]
    #[case("todos", "json", false)]
    fn test_validate_file_extension(
        #[case] path: &str,
        #[case] expected_ext: &str,
        #[case] should_succeed: bool,
    ) {
        let result = Output::validate_file_extension(Path::new(path), expected_ext);
        assert_eq!(
            result.is_ok(),
            should_succeed,
            "validate_file_extension({path:?}, {expected_ext:?}) expected success={should_succeed}"
        );
    }

    #[rstest]
    #[case(vec![], 0)]
    #[case(vec![
        ("First", TodoType::Todo),
        ("Second", TodoType::Todo),
    ], 1)]
    #[case(vec![
        ("Todo", TodoType::Todo),
        ("Fix", TodoType::Fixme),
        ("Bug", TodoType::Bug),
        ("Todo2", TodoType::Todo),
    ], 3)]
    fn test_group_todos_by_type(
        #[case] inputs: Vec<(&str, TodoType)>,
        #[case] expected_groups: usize,
    ) {
        let todos: Vec<TodoComment> = inputs
            .iter()
            .map(|(desc, tt)| create_test_todo(desc, *tt, None, false))
            .collect();
        let grouped = Output::group_todos_by_type(&todos);
        assert_eq!(grouped.len(), expected_groups);
    }

    #[test]
    fn test_grouping_sorted_by_priority_then_location() {
        use crate::comment::todo::test_support::TestTodoBuilder;
        use crate::output::formatter::formatters::json::JsonFormatter;
        use crate::output::formatter::Formatter;

        let make = |todo_type, file: &str, line| {
            TestTodoBuilder::new()
                .todo_type(todo_type)
                .file_path(file)
                .line_number(line)
                .column_start(0)
                .column_end(1)
                .build()
        };
        let todos = vec![
            make(TodoType::Note, "b.rs", 5),
            make(TodoType::Bug, "z.rs", 9),
            make(TodoType::Note, "a.rs", 7),
            make(TodoType::Bug, "z.rs", 2),
        ];

        let groups = Output::group_todos_by_type(&todos);

        let order: Vec<TodoType> = groups.iter().map(|&(t, _)| t).collect();
        assert_eq!(order, vec![TodoType::Bug, TodoType::Note]);
        let bug_lines: Vec<usize> = groups[0].1.iter().map(|t| t.line_number).collect();
        assert_eq!(bug_lines, vec![2, 9]);
        let note_files: Vec<String> = groups[1]
            .1
            .iter()
            .map(|t| t.file_path.display().to_string())
            .collect();
        assert_eq!(note_files, vec!["a.rs", "b.rs"]);

        // identical bytes regardless of input order
        let reversed: Vec<TodoComment> = todos.iter().rev().cloned().collect();
        let regrouped = Output::group_todos_by_type(&reversed);
        let formatter = JsonFormatter;
        assert_eq!(
            formatter.format(&groups, todos.len()).unwrap(),
            formatter.format(&regrouped, todos.len()).unwrap()
        );
    }

    #[tokio::test]
    async fn test_save_formats_and_writes_todos() {
        let output = Output::new(OutputFormat::Terminal, None).unwrap();
        let todos = vec![
            create_test_todo("First task", TodoType::Todo, None, false),
            create_test_todo("Bug found", TodoType::Bug, None, false),
        ];

        let result = output.save(&todos).await;
        assert!(result.is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn test_validate_file_extension_non_utf8() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        // Create a path with non-UTF-8 extension bytes
        let invalid_bytes: &[u8] = b"output.\xff\xfe";
        let os_str = OsStr::from_bytes(invalid_bytes);
        let path = PathBuf::from(os_str);

        let result = Output::validate_file_extension(&path, "json");
        assert!(result.is_err(), "Non-UTF-8 extension should be rejected");
    }
}
