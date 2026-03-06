use std::collections::HashMap;
use toml::{Table, Value};

use crate::{
    comment::todo::{TodoComment, TodoType},
    output::formatter::{error::FormatterError, Formatter},
};

pub struct TomlFormatter;

impl TomlFormatter {
    fn usize_to_i64(val: usize) -> Result<i64, FormatterError> {
        i64::try_from(val).map_err(|_| FormatterError::IntegerOverflow(val))
    }

    fn build_todo_table(todo: &TodoComment) -> Result<Table, FormatterError> {
        let mut table = Table::new();
        table.insert(
            "description".to_string(),
            Value::String(todo.description.trim().to_string()),
        );
        table.insert(
            "file".to_string(),
            Value::String(todo.file_path.display().to_string()),
        );
        table.insert(
            "line".to_string(),
            Value::Integer(Self::usize_to_i64(todo.line_number)?),
        );
        table.insert(
            "column_start".to_string(),
            Value::Integer(Self::usize_to_i64(todo.column_start)?),
        );
        table.insert(
            "column_end".to_string(),
            Value::Integer(Self::usize_to_i64(todo.column_end)?),
        );
        table.insert(
            "original_text".to_string(),
            Value::String(todo.original_text.trim().to_string()),
        );
        if !todo.context_lines.is_empty() {
            let context_values: Vec<Value> = todo
                .context_lines
                .iter()
                .map(|line| Value::String(line.clone())) // clone: Value::String requires owned String
                .collect();
            table.insert("context_lines".to_string(), Value::Array(context_values));
        }
        if let Some(ref func_context) = todo.function_context {
            table.insert("function".to_string(), Value::String(func_context.clone()));
            // clone: Value::String requires owned String
        }
        Ok(table)
    }
}

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
            Value::Integer(Self::usize_to_i64(total_count)?),
        );
        summary.insert(
            "total_groups".to_string(),
            Value::Integer(Self::usize_to_i64(todos_map.len())?),
        );
        root.insert("summary".to_string(), Value::Table(summary));

        for (todo_type, todos_of_type) in todos_map {
            let type_name = todo_type.as_filter_str().to_string();
            let mut group = Table::new();

            group.insert(
                "count".to_string(),
                Value::Integer(Self::usize_to_i64(todos_of_type.len())?),
            );

            let items: Vec<Value> = todos_of_type
                .iter()
                .map(|todo| Self::build_todo_table(todo).map(Value::Table))
                .collect::<Result<_, _>>()?;

            group.insert("items".to_string(), Value::Array(items));
            root.insert(type_name, Value::Table(group));
        }

        let toml_string = toml::to_string_pretty(&root)
            .map_err(|e| FormatterError::SerializationError(e.to_string()))?;

        Ok(vec![toml_string])
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
    #[case(vec![create_test_todo("Test", TodoType::Todo, None, false)], 1)]
    #[case(vec![
        create_test_todo("One", TodoType::Todo, None, false),
        create_test_todo("Two", TodoType::Fixme, None, false)
    ], 2)]
    fn test_toml_counts(#[case] todos: Vec<TodoComment>, #[case] expected_count: usize) {
        let formatter = TomlFormatter;
        let todos_map = crate::output::Output::group_todos_by_type(&todos);

        let result = formatter.format(&todos_map, expected_count).unwrap();
        let parsed: toml::Table = toml::from_str(&result[0]).unwrap();

        assert_eq!(
            parsed["summary"]["total_todos"].as_integer(),
            i64::try_from(expected_count).ok()
        );
    }

    #[test]
    fn test_toml_structure_with_todo() {
        let formatter = TomlFormatter;
        let todo = create_test_todo("Test description", TodoType::Todo, None, true);
        let mut todos_map = HashMap::new();
        todos_map.insert(&todo.todo_type, vec![&todo]);

        let result = formatter.format(&todos_map, 1).unwrap();
        let parsed: toml::Table = toml::from_str(&result[0]).unwrap();

        assert_eq!(parsed["summary"]["total_todos"].as_integer(), Some(1));
        assert_eq!(parsed["summary"]["total_groups"].as_integer(), Some(1));

        assert!(parsed.contains_key("todo"));
        let todo_group = parsed["todo"].as_table().unwrap();
        assert_eq!(todo_group["count"].as_integer(), Some(1));

        let items = todo_group["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);

        let item = items[0].as_table().unwrap();
        assert_eq!(item["description"].as_str(), Some("Test description"));
        assert_eq!(item["file"].as_str(), Some("test.rs"));
        assert_eq!(item["line"].as_integer(), Some(42));
        assert_eq!(item["column_start"].as_integer(), Some(1));
        assert_eq!(item["column_end"].as_integer(), Some(20));

        let context = item["context_lines"].as_array().unwrap();
        assert_eq!(context.len(), 2);
    }

    #[rstest]
    #[case(TodoType::Todo, "todo")]
    #[case(TodoType::Fixme, "fixme")]
    #[case(TodoType::Hack, "hack")]
    #[case(TodoType::Note, "note")]
    #[case(TodoType::Bug, "bug")]
    fn test_toml_type_keys(#[case] todo_type: TodoType, #[case] expected_key: &str) {
        let formatter = TomlFormatter;
        let todo = create_test_todo("Test", todo_type, None, false);
        let mut todos_map = HashMap::new();
        todos_map.insert(&todo.todo_type, vec![&todo]);

        let result = formatter.format(&todos_map, 1).unwrap();
        let parsed: toml::Table = toml::from_str(&result[0]).unwrap();

        assert!(parsed.contains_key(expected_key));
    }

    #[test]
    fn test_toml_with_function_context() {
        let formatter = TomlFormatter;
        let mut todo = create_test_todo("Test", TodoType::Note, None, false);
        todo.function_context = Some("main_function".to_string());

        let mut todos_map = HashMap::new();
        todos_map.insert(&todo.todo_type, vec![&todo]);

        let result = formatter.format(&todos_map, 1).unwrap();
        let parsed: toml::Table = toml::from_str(&result[0]).unwrap();

        let items = parsed["note"]["items"].as_array().unwrap();
        let item = items[0].as_table().unwrap();
        assert_eq!(item["function"].as_str(), Some("main_function"));
    }

    proptest! {
        #[test]
        fn prop_usize_to_i64_boundary(val: usize) {
            let result = TomlFormatter::usize_to_i64(val);
            if i64::try_from(val).is_ok() {
                prop_assert!(result.is_ok());
                prop_assert_eq!(result.unwrap(), i64::try_from(val).unwrap());
            } else {
                prop_assert!(matches!(result, Err(FormatterError::IntegerOverflow(v)) if v == val));
            }
        }

        #[test]
        fn prop_toml_output_is_valid_toml(
            desc in "[a-zA-Z0-9 .,!?-]{1,50}",
            todo_type in prop::sample::select(vec![TodoType::Todo, TodoType::Fixme, TodoType::Hack, TodoType::Note, TodoType::Bug]),
        ) {
            let formatter = TomlFormatter;
            let todo = create_test_todo(&desc, todo_type, Some("test_func"), true);
            let mut todos_map = HashMap::new();
            todos_map.insert(&todo.todo_type, vec![&todo]);

            let result = formatter.format(&todos_map, 1).unwrap();
            prop_assert_eq!(result.len(), 1);

            let parsed: Result<toml::Table, _> = toml::from_str(&result[0]);
            prop_assert!(parsed.is_ok(), "Invalid TOML for description: {}", desc);

            let table = parsed.unwrap();
            prop_assert!(table.contains_key("summary"));
            prop_assert_eq!(table["summary"]["total_todos"].as_integer(), Some(1));
        }

    }
}
