use std::collections::HashMap;

use crate::{
    comment::todo::{TodoComment, TodoType},
    output::formatter::{error::FormatterError, formatters::pluralize, Formatter},
};

const INDENTED_CODE_FENCE: &str = "  ```";

fn escape_markdown(s: &str) -> String {
    let mut out = String::with_capacity(s.len().saturating_add(s.len() / 4));
    for ch in s.chars() {
        if matches!(
            ch,
            '\\' | '`' | '*' | '_' | '[' | ']' | '#' | '!' | '<' | '>' | '~' | '|'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

pub struct MarkdownFormatter;

impl Formatter for MarkdownFormatter {
    fn format(
        &self,
        todos_map: &HashMap<&TodoType, Vec<&TodoComment>>,
        total_count: usize,
    ) -> Result<Vec<String>, FormatterError> {
        let capacity = 2 + todos_map.len() + total_count.saturating_mul(2);
        let mut output: Vec<String> = Vec::with_capacity(capacity);

        output.push("# TODO Comments\n\n".to_string()); // clone: owned String for output Vec
        output.push(format!(
            "Found {total_count} TODO comment{}:\n\n",
            pluralize(total_count)
        ));

        for (todo_type, todos_of_type) in todos_map {
            output.push(format!(
                "## {todo_type} ({} item{})\n\n",
                todos_of_type.len(),
                pluralize(todos_of_type.len())
            ));

            for todo in todos_of_type {
                let location = format!("{}:{}", todo.file_path.display(), todo.line_number);

                let escaped_desc = escape_markdown(todo.description.trim());
                if let Some(ref func_context) = todo.function_context {
                    output.push(format!(
                        "- **{escaped_desc}** @ `{location}` (in `{func_context}`)"
                    ));
                } else {
                    output.push(format!("- **{escaped_desc}** @ `{location}`"));
                }

                if !todo.context_lines.is_empty() {
                    output.push(INDENTED_CODE_FENCE.to_string()); // clone: owned String for output Vec
                    for context_line in &todo.context_lines {
                        output.push(format!("  {context_line}"));
                    }
                    output.push(INDENTED_CODE_FENCE.to_string()); // clone: owned String for output Vec
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
    use proptest::prelude::*;
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

    proptest! {
        #[test]
        fn prop_markdown_structure_valid(
            desc in "[a-zA-Z0-9 ]{1,50}",
            count in 1usize..5,
        ) {
            let formatter = MarkdownFormatter;
            let todos: Vec<_> = (0..count)
                .map(|_| create_test_todo(&desc, TodoType::Todo, None, false))
                .collect();
            let refs: Vec<&TodoComment> = todos.iter().collect();
            let mut todos_map = HashMap::new();
            todos_map.insert(&TodoType::Todo, refs);

            let result = formatter.format(&todos_map, count).unwrap();
            let output = result.join("\n");

            let expected_count = format!("Found {count} TODO comment");
            let expected_items = format!("({count} item");

            prop_assert!(output.contains("# TODO Comments"));
            prop_assert!(output.contains(&expected_count));
            prop_assert!(output.contains("## TODO"));
            prop_assert!(output.contains(&expected_items));
            for line in &result {
                if line.starts_with("- **") {
                    prop_assert!(
                        line.contains("test.rs:42"),
                        "Each item should have file:line location"
                    );
                }
            }
        }
    }
}
