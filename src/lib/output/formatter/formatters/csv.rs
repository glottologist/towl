use std::borrow::Cow;
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
        let total_rows = todos_map.values().map(Vec::len).sum::<usize>();
        let mut output = Vec::with_capacity(total_rows.saturating_add(1));

        output.push(
            "Type,Description,File,Line,Column Start,Column End,Function,Original Text,Context Lines".to_string(),
        );

        for (todo_type, todos_of_type) in todos_map {
            let type_str = todo_type.to_string();
            for todo in todos_of_type {
                let func_field = todo
                    .function_context
                    .as_deref()
                    .map_or(Cow::Borrowed(""), escape_csv_field);
                let context_str = todo.context_lines.join(" | ");

                let row = format!(
                    "{},{},{},{},{},{},{},{},{}",
                    escape_csv_field(&type_str),
                    escape_csv_field(todo.description.trim()),
                    escape_csv_field(&todo.file_path.display().to_string()),
                    todo.line_number,
                    todo.column_start,
                    todo.column_end,
                    func_field,
                    escape_csv_field(todo.original_text.trim()),
                    escape_csv_field(&context_str),
                );

                output.push(row);
            }
        }

        Ok(output)
    }
}

fn escape_csv_field(field: &str) -> Cow<'_, str> {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        Cow::Owned(format!("\"{}\"", field.replace('"', "\"\"")))
    } else {
        Cow::Borrowed(field)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comment::todo::TodoComment;
    use crate::output::formatter::formatters::test_helpers::create_test_todo;
    use proptest::prelude::*;
    use rstest::rstest;
    use std::path::PathBuf;

    #[test]
    fn test_csv_header() {
        let formatter = CsvFormatter;
        let todos_map = HashMap::new();

        let result = formatter.format(&todos_map, 0).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "Type,Description,File,Line,Column Start,Column End,Function,Original Text,Context Lines");
    }

    #[test]
    fn test_csv_single_todo() {
        let formatter = CsvFormatter;
        let todo = create_test_todo("Test description", TodoType::Todo, Some("main"), true);
        let mut todos_map = HashMap::new();
        todos_map.insert(&todo.todo_type, vec![&todo]);

        let result = formatter.format(&todos_map, 1).unwrap();
        assert_eq!(result.len(), 2);

        let row = &result[1];
        assert!(row.contains("TODO"));
        assert!(row.contains("Test description"));
        assert!(row.contains("test.rs"));
        assert!(row.contains("42"));
        assert!(row.contains("main"));
    }

    #[rstest]
    #[case(vec![], 1)] // Empty map, just header
    #[case(vec![create_test_todo("One", TodoType::Todo, None, false)], 2)] // One todo
    #[case(vec![
        create_test_todo("One", TodoType::Todo, None, false),
        create_test_todo("Two", TodoType::Fixme, None, false)
    ], 3)] // Two todos
    fn test_csv_row_count(#[case] todos: Vec<TodoComment>, #[case] expected_rows: usize) {
        let formatter = CsvFormatter;
        let todos_map = crate::output::Output::group_todos_by_type(&todos);

        let result = formatter.format(&todos_map, todos.len()).unwrap();
        assert_eq!(result.len(), expected_rows);
    }

    #[test]
    fn test_csv_with_special_characters() {
        let formatter = CsvFormatter;
        let todo = TodoComment {
            id: "test-special".to_string(),
            file_path: PathBuf::from("path/with,comma.rs"),
            line_number: 10,
            column_start: 1,
            column_end: 50,
            todo_type: TodoType::Bug,
            original_text: "// BUG: Fix \"quotes\" and, commas".to_string(),
            description: "Fix \"quotes\" and, commas".to_string(),
            context_lines: vec!["line with, comma".to_string()],
            function_context: Some("func,with,commas".to_string()),
        };

        let mut todos_map = HashMap::new();
        todos_map.insert(&todo.todo_type, vec![&todo]);

        let result = formatter.format(&todos_map, 1).unwrap();
        assert_eq!(result.len(), 2);

        let row = &result[1];
        assert!(row.contains("\"path/with,comma.rs\""));
        assert!(row.contains("\"Fix \"\"quotes\"\" and, commas\""));
        assert!(row.contains("\"func,with,commas\""));
    }

    #[test]
    fn test_csv_without_function_context() {
        let formatter = CsvFormatter;
        let todo = create_test_todo("No function", TodoType::Note, None, false);
        let mut todos_map = HashMap::new();
        todos_map.insert(&todo.todo_type, vec![&todo]);

        let result = formatter.format(&todos_map, 1).unwrap();
        let row = &result[1];

        let parts: Vec<&str> = row.split(',').collect();
        assert!(parts.len() >= 9);
        assert_eq!(parts[6], "");
    }

    proptest! {
        #[test]
        fn prop_csv_escape_roundtrip(field in ".*") {
            let escaped = escape_csv_field(&field);
            let unescaped = unescape_csv_field(&escaped);
            prop_assert_eq!(&field, &unescaped, "Roundtrip failed for: {:?}", field);
        }

        #[test]
        fn prop_csv_escape_quotes_special_chars(field in ".*") {
            let escaped = escape_csv_field(&field);
            if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
                prop_assert!(escaped.starts_with('"'), "Escaped field should be quoted: {:?}", escaped);
                prop_assert!(escaped.ends_with('"'), "Escaped field should end with quote: {:?}", escaped);
            }
        }

        #[test]
        fn prop_csv_escape_no_bare_quotes_inside(field in ".*") {
            let escaped = escape_csv_field(&field);
            if escaped.starts_with('"') && escaped.len() >= 2 {
                let inner = &escaped[1..escaped.len() - 1];
                let mut chars = inner.chars().peekable();
                while let Some(c) = chars.next() {
                    if c == '"' {
                        prop_assert_eq!(
                            chars.peek(),
                            Some(&'"'),
                            "Bare double-quote found in escaped CSV field: {:?}",
                            escaped
                        );
                        chars.next();
                    }
                }
            }
        }
    }

    fn unescape_csv_field(field: &str) -> String {
        if field.starts_with('"') && field.ends_with('"') && field.len() >= 2 {
            field[1..field.len() - 1].replace("\"\"", "\"")
        } else {
            field.to_string()
        }
    }
}
