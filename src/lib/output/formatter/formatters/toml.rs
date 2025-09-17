use std::collections::HashMap;
use toml::{Table, Value};

use crate::{
    comment::todo::{TodoComment, TodoType},
    output::formatter::{error::FormatterError, Formatter},
};

pub struct TomlFormatter;

impl Formatter for TomlFormatter {
    fn format(
        &self,
        todos_map: &HashMap<&TodoType, Vec<&TodoComment>>,
        total_count: usize,
    ) -> Result<Vec<String>, FormatterError> {
        let mut root = Table::new();

        let mut summary = Table::new();
        summary.insert(
            "total_todos".to_string(),
            Value::Integer(total_count as i64),
        );
        summary.insert(
            "total_groups".to_string(),
            Value::Integer(todos_map.len() as i64),
        );
        root.insert("summary".to_string(), Value::Table(summary));

        for (todo_type, todos_of_type) in todos_map {
            let type_name = format!("{:?}", todo_type).to_lowercase();
            let mut group = Table::new();

            group.insert(
                "count".to_string(),
                Value::Integer(todos_of_type.len() as i64),
            );

            let mut items = Vec::new();
            for todo in todos_of_type {
                let mut todo_table = Table::new();

                todo_table.insert(
                    "description".to_string(),
                    Value::String(todo.description.trim().to_string()),
                );
                todo_table.insert(
                    "file".to_string(),
                    Value::String(todo.file_path.display().to_string()),
                );
                todo_table.insert("line".to_string(), Value::Integer(todo.line_number as i64));
                todo_table.insert(
                    "column_start".to_string(),
                    Value::Integer(todo.column_start as i64),
                );
                todo_table.insert(
                    "column_end".to_string(),
                    Value::Integer(todo.column_end as i64),
                );
                todo_table.insert(
                    "original_text".to_string(),
                    Value::String(todo.original_text.trim().to_string()),
                );

                if let Some(ref func_context) = todo.function_context {
                    todo_table.insert("function".to_string(), Value::String(func_context.clone()));
                }

                items.push(Value::Table(todo_table));
            }

            group.insert("items".to_string(), Value::Array(items));
            root.insert(type_name, Value::Table(group));
        }

        let toml_string = toml::to_string_pretty(&root)
            .map_err(|e| FormatterError::SerializationError(e.to_string()))?;

        Ok(vec![toml_string])
    }
}
