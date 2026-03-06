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
        output.push(format!(
            "Found {total_count} TODO comment{}:\n\n",
            if total_count == 1 { "" } else { "s" }
        ));

        for (todo_type, todos_of_type) in todos_map {
            output.push(format!(
                "## {todo_type} ({} item{})\n\n",
                todos_of_type.len(),
                if todos_of_type.len() == 1 { "" } else { "s" }
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

                if !todo.context_lines.is_empty() {
                    output.push("  ```".to_string());
                    for context_line in &todo.context_lines {
                        output.push(format!("  {context_line}"));
                    }
                    output.push("  ```".to_string());
                }
                output.push(String::new());
            }
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::formatter::formatters::test_helpers::create_test_todo;
    use rstest::rstest;

    #[rstest]
    #[case(0, "Found 0 TODO comments")]
    #[case(1, "Found 1 TODO comment")]
    #[case(10, "Found 10 TODO comments")]
    fn test_markdown_summary_count(#[case] count: usize, #[case] expected: &str) {
        let formatter = MarkdownFormatter;
        let todos_map = HashMap::new();

        let result = formatter.format(&todos_map, count).unwrap();
        assert!(result[1].contains(expected));
    }

    #[test]
    fn test_markdown_with_function_context() {
        let formatter = MarkdownFormatter;
        let todo = create_test_todo("Test", TodoType::Todo, Some("main"), false);
        let mut todos_map = HashMap::new();
        todos_map.insert(&todo.todo_type, vec![&todo]);

        let result = formatter.format(&todos_map, 1).unwrap();
        let output = result.join("\n");

        assert!(output.contains("(in `main`)"));
        assert!(output.contains("test.rs:42"));
    }

    #[test]
    fn test_markdown_without_function_context() {
        let formatter = MarkdownFormatter;
        let todo = create_test_todo("Test", TodoType::Fixme, None, false);
        let mut todos_map = HashMap::new();
        todos_map.insert(&todo.todo_type, vec![&todo]);

        let result = formatter.format(&todos_map, 1).unwrap();
        let output = result.join("\n");

        assert!(!output.contains("(in `"));
        assert!(output.contains("test.rs:42"));
    }

    #[test]
    fn test_markdown_with_context_lines() {
        let formatter = MarkdownFormatter;
        let todo = create_test_todo("Test", TodoType::Hack, None, true);
        let mut todos_map = HashMap::new();
        todos_map.insert(&todo.todo_type, vec![&todo]);

        let result = formatter.format(&todos_map, 1).unwrap();
        let output = result.join("\n");

        assert!(output.contains("```"));
        assert!(output.contains("context line 1"));
        assert!(output.contains("context line 2"));
    }

    #[rstest]
    #[case(TodoType::Todo, "## TODO")]
    #[case(TodoType::Fixme, "## FIXME")]
    #[case(TodoType::Hack, "## HACK")]
    #[case(TodoType::Note, "## NOTE")]
    #[case(TodoType::Bug, "## BUG")]
    fn test_markdown_section_headers(#[case] todo_type: TodoType, #[case] expected_header: &str) {
        let formatter = MarkdownFormatter;
        let todo = create_test_todo("Test", todo_type, None, false);
        let mut todos_map = HashMap::new();
        todos_map.insert(&todo.todo_type, vec![&todo]);

        let result = formatter.format(&todos_map, 1).unwrap();
        let output = result.join("\n");

        assert!(output.contains(expected_header));
        assert!(output.contains("(1 item)"));
    }

    #[test]
    fn test_markdown_multiple_todos() {
        let formatter = MarkdownFormatter;
        let todo1 = create_test_todo("First", TodoType::Todo, Some("func1"), true);
        let todo2 = create_test_todo("Second", TodoType::Todo, None, false);
        let todo3 = create_test_todo("Third", TodoType::Bug, Some("func3"), false);

        let mut todos_map = HashMap::new();
        todos_map.insert(&TodoType::Todo, vec![&todo1, &todo2]);
        todos_map.insert(&TodoType::Bug, vec![&todo3]);

        let result = formatter.format(&todos_map, 3).unwrap();
        let output = result.join("\n");

        assert!(output.contains("## TODO (2 items)"));
        assert!(output.contains("## BUG (1 item)"));

        assert!(output.contains("**First**"));
        assert!(output.contains("**Second**"));
        assert!(output.contains("**Third**"));

        assert!(output.contains("(in `func1`)"));
        assert!(output.contains("(in `func3`)"));
    }
}
