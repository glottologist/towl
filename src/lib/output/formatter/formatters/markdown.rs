use std::collections::HashMap;

use crate::{
    comment::todo::{TodoComment, TodoType},
    output::formatter::{error::FormatterError, Formatter},
};

pub struct MarkdownFormatter;

impl Formatter for MarkdownFormatter {
    fn format(
        &self,
        todos_map: &HashMap<&TodoType, Vec<&TodoComment>>,
        total_count: usize,
    ) -> Result<Vec<String>, FormatterError> {
        let mut output: Vec<String> = vec![];

        output.push("# TODO Comments\n\n".to_string());
        output.push(format!("Found {} TODO comments:\n\n", total_count));

        for (todo_type, todos_of_type) in todos_map {
            output.push(format!(
                "## {:?} ({} items)\n\n",
                todo_type,
                todos_of_type.len()
            ));

            for todo in todos_of_type {
                let location = format!("{}:{}", todo.file_path.display(), todo.line_number);

                if let Some(ref func_context) = todo.function_context {
                    output.push(format!(
                        "- **{}** @ `{}` (in `{}`)",
                        todo.description.trim(),
                        location,
                        func_context
                    ));
                } else {
                    output.push(format!(
                        "- **{}** @ `{}`",
                        todo.description.trim(),
                        location
                    ));
                }
            }
        }

        Ok(output)
    }
}
