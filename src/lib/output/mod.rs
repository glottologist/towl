pub mod error;
pub mod formatter;
pub mod writer;

use error::TowlOutputError;
use formatter::{
    formatters::{
        csv::CsvFormatter, json::JsonFormatter, markdown::MarkdownFormatter, table::TableFormatter,
        toml::TomlFormatter,
    },
    Formatter,
};
use writer::{
    writers::{file::FileWriter, stdout::StdoutWriter},
    Writer,
};

use crate::{cli::OutputFormat, comment::todo::TodoComment};
use std::{collections::HashMap, path::PathBuf};

pub struct Output {
    writer: Box<dyn Writer>,
    formatter: Box<dyn Formatter>,
}

impl Output {
    pub fn new(
        output_format: OutputFormat,
        output_path: Option<PathBuf>,
    ) -> Result<Self, TowlOutputError> {
        let (formatter, writer): (Box<dyn Formatter>, Box<dyn Writer>) = match output_format {
            OutputFormat::Terminal => {
                if output_path.is_some() {
                    return Err(TowlOutputError::InvalidOutputPath(
                        "Terminal format cannot write to file".to_string(),
                    ));
                }
                (Box::new(MarkdownFormatter), Box::new(StdoutWriter::new()))
            }
            OutputFormat::Table => {
                if output_path.is_some() {
                    return Err(TowlOutputError::InvalidOutputPath(
                        "Table format cannot write to file".to_string(),
                    ));
                }
                (Box::new(TableFormatter), Box::new(StdoutWriter::new()))
            }
            OutputFormat::Json => {
                let path = output_path.ok_or_else(|| {
                    TowlOutputError::InvalidOutputPath(
                        "JSON format requires an output file path".to_string(),
                    )
                })?;
                Self::validate_file_extension(&path, "json")?;
                (Box::new(JsonFormatter), Box::new(FileWriter::new(path)))
            }
            OutputFormat::Csv => {
                let path = output_path.ok_or_else(|| {
                    TowlOutputError::InvalidOutputPath(
                        "CSV format requires an output file path".to_string(),
                    )
                })?;
                Self::validate_file_extension(&path, "csv")?;
                (Box::new(CsvFormatter), Box::new(FileWriter::new(path)))
            }
            OutputFormat::Toml => {
                let path = output_path.ok_or_else(|| {
                    TowlOutputError::InvalidOutputPath(
                        "TOML format requires an output file path".to_string(),
                    )
                })?;
                Self::validate_file_extension(&path, "toml")?;
                (Box::new(TomlFormatter), Box::new(FileWriter::new(path)))
            }
            OutputFormat::Markdown => {
                let path = output_path.ok_or_else(|| {
                    TowlOutputError::InvalidOutputPath(
                        "Markdown format requires an output file path".to_string(),
                    )
                })?;
                Self::validate_file_extension(&path, "md")?;
                (Box::new(MarkdownFormatter), Box::new(FileWriter::new(path)))
            }
        };
        Ok(Self { writer, formatter })
    }

    fn validate_file_extension(path: &PathBuf, expected_ext: &str) -> Result<(), TowlOutputError> {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext == expected_ext {
                Ok(())
            } else {
                Err(TowlOutputError::InvalidOutputPath(format!(
                    "File extension '{}' does not match expected extension '{}' for this format",
                    ext, expected_ext
                )))
            }
        } else {
            Err(TowlOutputError::InvalidOutputPath(format!(
                "Output file must have '{}' extension",
                expected_ext
            )))
        }
    }

    fn group_todos_by_type<'a>(
        &self,
        todos: &'a [TodoComment],
    ) -> HashMap<&'a crate::comment::todo::TodoType, Vec<&'a TodoComment>> {
        let mut todo_map = HashMap::new();
        for todo in todos {
            todo_map
                .entry(&todo.todo_type)
                .or_insert_with(Vec::new)
                .push(todo);
        }
        todo_map
    }

    pub async fn save(&self, todos: &[TodoComment]) -> Result<(), TowlOutputError> {
        let grouped_todos = self.group_todos_by_type(todos);
        let total_count = todos.len();
        let formatted = self
            .formatter
            .format(&grouped_todos, total_count)
            .map_err(TowlOutputError::UnableToFormatTodos)?;
        self.writer
            .write(formatted)
            .await
            .map_err(TowlOutputError::UnableToWriteTodos)
    }
}
