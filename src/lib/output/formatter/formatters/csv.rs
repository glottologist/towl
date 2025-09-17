use std::collections::HashMap;

use crate::{
    comment::todo::{TodoComment, TodoType},
    output::formatter::{error::FormatterError, Formatter},
};

pub struct CsvFormatter;

impl Formatter for CsvFormatter {
    fn format(
        &self,
        todos_map: &HashMap<&TodoType, Vec<&TodoComment>>,
        _total_count: usize,
    ) -> Result<Vec<String>, FormatterError> {
        let mut output = Vec::new();

        output.push(
            "Type,Description,File,Line,Column Start,Column End,Function,Original Text".to_string(),
        );

        for (todo_type, todos_of_type) in todos_map {
            for todo in todos_of_type {
                let mut row = Vec::new();

                row.push(escape_csv_field(&format!("{:?}", todo_type)));
                row.push(escape_csv_field(todo.description.trim()));
                row.push(escape_csv_field(&todo.file_path.display().to_string()));
                row.push(todo.line_number.to_string());
                row.push(todo.column_start.to_string());
                row.push(todo.column_end.to_string());

                if let Some(ref func_context) = todo.function_context {
                    row.push(escape_csv_field(func_context));
                } else {
                    row.push("".to_string());
                }

                row.push(escape_csv_field(todo.original_text.trim()));

                output.push(row.join(","));
            }
        }

        Ok(output)
    }
}

fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}
