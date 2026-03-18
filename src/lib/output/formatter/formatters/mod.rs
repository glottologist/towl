pub(crate) mod csv;
pub(crate) mod json;
pub(crate) mod markdown;
pub(crate) mod table;
pub(crate) mod toml;

#[must_use]
pub(crate) const fn pluralize(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

#[cfg(test)]
pub mod test_helpers {
    use crate::comment::todo::test_support::TestTodoBuilder;
    use crate::comment::todo::{TodoComment, TodoType};

    #[must_use]
    pub fn create_test_todo(
        desc: &str,
        todo_type: TodoType,
        function: Option<&str>,
        with_context: bool,
    ) -> TodoComment {
        let mut builder = TestTodoBuilder::new()
            .description(desc)
            .todo_type(todo_type)
            .line_number(42)
            .column_start(1)
            .column_end(20);

        if with_context {
            builder = builder.context_lines(vec![
                "context line 1".to_string(),
                "context line 2".to_string(),
            ]);
        }

        if let Some(f) = function {
            builder = builder.function_context(f);
        }

        builder.build()
    }
}
