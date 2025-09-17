use serde_json::json;
use std::collections::HashMap;

use crate::{
    comment::todo::{TodoComment, TodoType},
    output::formatter::{error::FormatterError, Formatter},
};

pub struct JsonFormatter;

impl Formatter for JsonFormatter {
    fn format(
        &self,
        todos_map: &HashMap<&TodoType, Vec<&TodoComment>>,
        total_count: usize,
    ) -> Result<Vec<String>, FormatterError> {
        let mut groups = Vec::new();

        for (todo_type, todos_of_type) in todos_map {
            let mut group_todos = Vec::new();

            for todo in todos_of_type {
                let mut todo_json = json!({
                    "description": todo.description.trim(),
                    "file": todo.file_path.display().to_string(),
                    "line": todo.line_number,
                    "column_start": todo.column_start,
                    "column_end": todo.column_end,
                    "original_text": todo.original_text.trim()
                });

                if let Some(ref func_context) = todo.function_context {
                    todo_json["function"] = json!(func_context);
                }

                group_todos.push(todo_json);
            }

            groups.push(json!({
                "type": format!("{:?}", todo_type),
                "count": todos_of_type.len(),
                "items": group_todos
            }));
        }

        let result = json!({
            "summary": {
                "total_todos": total_count,
                "total_groups": groups.len()
            },
            "groups": groups
        });

        let json_string = serde_json::to_string_pretty(&result)
            .map_err(|e| FormatterError::SerializationError(e.to_string()))?;

        Ok(vec![json_string])
    }
}
