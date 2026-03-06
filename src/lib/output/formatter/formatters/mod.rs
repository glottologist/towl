pub(crate) mod csv;
pub(crate) mod json;
pub(crate) mod markdown;
pub(crate) mod table;
pub(crate) mod toml;

#[cfg(test)]
pub mod test_helpers {
    use crate::comment::todo::{TodoComment, TodoType};
    use std::path::PathBuf;

    pub fn create_test_todo(
        desc: &str,
        todo_type: TodoType,
        function: Option<&str>,
        with_context: bool,
    ) -> TodoComment {
        TodoComment {
            id: format!("test-{desc}"),
            file_path: PathBuf::from("test.rs"),
            line_number: 42,
            column_start: 1,
            column_end: 20,
            todo_type,
            original_text: format!("// {todo_type}: {desc}"),
            description: desc.to_string(),
            context_lines: if with_context {
                vec!["context line 1".to_string(), "context line 2".to_string()]
            } else {
                vec![]
            },
            function_context: function.map(ToString::to_string),
        }
    }
}
