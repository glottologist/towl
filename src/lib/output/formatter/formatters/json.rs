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
        let mut groups = Vec::with_capacity(todos_map.len());

        for (todo_type, todos_of_type) in todos_map {
            let mut group_todos = Vec::with_capacity(todos_of_type.len());

            for todo in todos_of_type {
                let mut todo_json = json!({
                    "description": todo.description.trim(),
                    "file": todo.file_path.display().to_string(),
                    "line": todo.line_number,
                    "column_start": todo.column_start,
                    "column_end": todo.column_end,
                    "original_text": todo.original_text.trim(),
                    "context_lines": todo.context_lines
                });

                if let Some(ref func_context) = todo.function_context {
                    todo_json["function"] = json!(func_context);
                }

                group_todos.push(todo_json);
            }

            groups.push(json!({
                "type": todo_type.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::formatter::formatters::test_helpers::create_test_todo;
    use proptest::prelude::*;
    use rstest::rstest;

    #[rstest]
    #[case(vec![], 0)]
    #[case(vec![create_test_todo("Test", TodoType::Todo, None, true)], 1)]
    #[case(vec![create_test_todo("Fix", TodoType::Fixme, None, true), create_test_todo("Hack", TodoType::Hack, None, true)], 2)]
    #[case(vec![
        create_test_todo("Todo1", TodoType::Todo, None, true),
        create_test_todo("Todo2", TodoType::Todo, None, true),
        create_test_todo("Fix1", TodoType::Fixme, None, true)
    ], 3)]
    fn test_json_formatting_counts(#[case] todos: Vec<TodoComment>, #[case] expected_count: usize) {
        let formatter = JsonFormatter;
        let todos_map = crate::output::Output::group_todos_by_type(&todos);
        let result = formatter.format(&todos_map, expected_count).unwrap();

        assert_eq!(result.len(), 1);
        let parsed: serde_json::Value = serde_json::from_str(&result[0]).unwrap();
        assert_eq!(parsed["summary"]["total_todos"], expected_count);
    }

    #[rstest]
    #[case(true, true)]
    #[case(false, false)]
    fn test_function_context_inclusion(
        #[case] with_function: bool,
        #[case] should_have_function: bool,
    ) {
        let formatter = JsonFormatter;
        let todos = vec![create_test_todo(
            "Test",
            TodoType::Todo,
            if with_function {
                Some("test_function")
            } else {
                None
            },
            true,
        )];
        let todos_map = crate::output::Output::group_todos_by_type(&todos);

        let result = formatter.format(&todos_map, 1).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result[0]).unwrap();

        let has_function = parsed["groups"][0]["items"][0].get("function").is_some();
        assert_eq!(has_function, should_have_function);
    }

    #[test]
    fn test_json_structure() {
        let formatter = JsonFormatter;
        let todos = vec![
            create_test_todo("First", TodoType::Todo, Some("test_function"), true),
            create_test_todo("Second", TodoType::Fixme, None, true),
        ];
        let todos_map = crate::output::Output::group_todos_by_type(&todos);

        let result = formatter.format(&todos_map, 2).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result[0]).unwrap();

        assert!(parsed["summary"].is_object());
        assert!(parsed["groups"].is_array());
        assert_eq!(parsed["summary"]["total_todos"], 2);

        for group in parsed["groups"].as_array().unwrap() {
            assert!(group["type"].is_string());
            assert!(group["count"].is_number());
            assert!(group["items"].is_array());

            for item in group["items"].as_array().unwrap() {
                assert!(item["description"].is_string());
                assert!(item["file"].is_string());
                assert!(item["line"].is_number());
                assert!(item["column_start"].is_number());
                assert!(item["column_end"].is_number());
                assert!(item["original_text"].is_string());
                assert!(item["context_lines"].is_array());
            }
        }
    }

    proptest! {
        #[test]
        fn prop_json_output_is_valid_json(
            desc in "[a-zA-Z0-9 .,!?-]{1,50}",
            todo_type in prop::sample::select(vec![TodoType::Todo, TodoType::Fixme, TodoType::Hack, TodoType::Note, TodoType::Bug]),
        ) {
            let formatter = JsonFormatter;
            let todo = create_test_todo(&desc, todo_type, Some("test_func"), true);
            let mut todos_map = HashMap::new();
            todos_map.insert(&todo.todo_type, vec![&todo]);

            let result = formatter.format(&todos_map, 1).unwrap();
            prop_assert_eq!(result.len(), 1);

            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result[0]);
            prop_assert!(parsed.is_ok(), "Invalid JSON for description: {}", desc);

            let json = parsed.unwrap();
            prop_assert!(json["summary"].is_object());
            prop_assert!(json["groups"].is_array());
            prop_assert_eq!(json["summary"]["total_todos"].as_u64().unwrap(), 1);

            let items = &json["groups"][0]["items"];
            prop_assert!(items.is_array());
            let item = &items[0];
            prop_assert!(item["description"].is_string());
            prop_assert!(item["file"].is_string());
            prop_assert!(item["line"].is_number());
        }

    }
}
