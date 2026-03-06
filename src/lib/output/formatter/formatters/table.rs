use std::borrow::Cow;
use std::collections::HashMap;

use crate::{
    comment::todo::{TodoComment, TodoType},
    output::formatter::{error::FormatterError, Formatter},
};

pub struct TableFormatter;

impl TableFormatter {
    fn calculate_column_widths(
        todos_map: &HashMap<&TodoType, Vec<&TodoComment>>,
    ) -> (usize, usize, usize, usize, usize) {
        let mut type_width = 4;
        let mut desc_width = 11;
        let mut file_width = 4;
        let mut line_width = 4;
        let mut func_width = 8;

        for (todo_type, todos_of_type) in todos_map {
            let type_str = todo_type.to_string();
            type_width = type_width.max(type_str.len());

            for todo in todos_of_type {
                desc_width = desc_width.max(todo.description.trim().len().min(50));
                file_width = file_width.max(todo.file_path.display().to_string().len().min(40));
                line_width = line_width.max(todo.line_number.to_string().len());

                if let Some(ref func_context) = todo.function_context {
                    func_width = func_width.max(func_context.len().min(30));
                }
            }
        }

        (type_width, desc_width, file_width, line_width, func_width)
    }

    fn format_row(
        content: (&str, &str, &str, &str, &str),
        widths: (usize, usize, usize, usize, usize),
        is_header: bool,
    ) -> String {
        let (type_val, desc_val, file_val, line_val, func_val) = content;
        let (type_width, desc_width, file_width, line_width, func_width) = widths;

        let type_truncated = Self::truncate_string(type_val, type_width);
        let desc_truncated = Self::truncate_string(desc_val, desc_width);
        let file_truncated = Self::truncate_string(file_val, file_width);
        let line_truncated = Self::truncate_string(line_val, line_width);
        let func_truncated = Self::truncate_string(func_val, func_width);

        if is_header {
            format!(
                "│ {type_truncated:<type_width$} │ {desc_truncated:<desc_width$} │ {file_truncated:<file_width$} │ {line_truncated:<line_width$} │ {func_truncated:<func_width$} │"
            )
        } else {
            format!(
                "│ {type_truncated:<type_width$} │ {desc_truncated:<desc_width$} │ {file_truncated:<file_width$} │ {line_truncated:>line_width$} │ {func_truncated:<func_width$} │"
            )
        }
    }

    fn format_separator(widths: (usize, usize, usize, usize, usize), is_top: bool) -> String {
        let (type_width, desc_width, file_width, line_width, func_width) = widths;

        let left = if is_top { "┌" } else { "├" };
        let right = if is_top { "┐" } else { "┤" };
        let cross = if is_top { "┬" } else { "┼" };

        format!(
            "{}{}{}{}{}{}{}{}{}{}",
            left,
            "─".repeat(type_width + 2),
            cross,
            "─".repeat(desc_width + 2),
            cross,
            "─".repeat(file_width + 2),
            cross,
            "─".repeat(line_width + 2),
            cross,
            "─".repeat(func_width + 2) + right
        )
    }

    fn format_bottom(widths: (usize, usize, usize, usize, usize)) -> String {
        let (type_width, desc_width, file_width, line_width, func_width) = widths;

        format!(
            "└{}┴{}┴{}┴{}┴{}┘",
            "─".repeat(type_width + 2),
            "─".repeat(desc_width + 2),
            "─".repeat(file_width + 2),
            "─".repeat(line_width + 2),
            "─".repeat(func_width + 2)
        )
    }

    fn truncate_string(s: &str, max_len: usize) -> Cow<'_, str> {
        if s.chars().count() <= max_len {
            Cow::Borrowed(s)
        } else {
            let truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
            Cow::Owned(format!("{truncated}…"))
        }
    }
}

impl Formatter for TableFormatter {
    fn format(
        &self,
        todos_map: &HashMap<&TodoType, Vec<&TodoComment>>,
        total_count: usize,
    ) -> Result<Vec<String>, FormatterError> {
        let mut output = Vec::with_capacity(total_count.saturating_add(5));

        if total_count == 0 {
            output.push("No TODO comments found.".to_string());
            return Ok(output);
        }

        output.push(format!(
            "Found {} TODO comment{} in {} group{}",
            total_count,
            if total_count == 1 { "" } else { "s" },
            todos_map.len(),
            if todos_map.len() == 1 { "" } else { "s" }
        ));
        output.push(String::new());

        let widths = Self::calculate_column_widths(todos_map);

        output.push(Self::format_separator(widths, true));
        output.push(Self::format_row(
            ("Type", "Description", "File", "Line", "Function"),
            widths,
            true,
        ));
        output.push(Self::format_separator(widths, false));

        for (todo_type, todos_of_type) in todos_map {
            for todo in todos_of_type {
                let type_str = todo_type.to_string();
                let file_str = todo.file_path.display().to_string();
                let line_str = todo.line_number.to_string();
                let func_str = todo.function_context.as_deref().unwrap_or("");

                output.push(Self::format_row(
                    (
                        &type_str,
                        todo.description.trim(),
                        &file_str,
                        &line_str,
                        func_str,
                    ),
                    widths,
                    false,
                ));
            }
        }

        output.push(Self::format_bottom(widths));

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::formatter::formatters::test_helpers::create_test_todo;
    use proptest::prelude::*;

    #[test]
    fn test_empty_todos() {
        let formatter = TableFormatter;
        let todos_map = HashMap::new();

        let result = formatter.format(&todos_map, 0).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "No TODO comments found.");
    }

    #[test]
    fn test_single_todo_formatting() {
        let formatter = TableFormatter;
        let todo = create_test_todo("Test description", TodoType::Todo, Some("main"), false);
        let mut todos_map = HashMap::new();
        todos_map.insert(&todo.todo_type, vec![&todo]);

        let result = formatter.format(&todos_map, 1).unwrap();
        let output = result.join("\n");

        assert!(output.contains("Found 1 TODO comment in 1 group"));
        assert!(output.contains("Type"));
        assert!(output.contains("Description"));
        assert!(output.contains("File"));
        assert!(output.contains("Line"));
        assert!(output.contains("Function"));
        assert!(output.contains("TODO"));
        assert!(output.contains("Test description"));
        assert!(output.contains("test.rs"));
        assert!(output.contains("42"));
        assert!(output.contains("main"));
    }

    #[test]
    fn test_multiple_types() {
        let formatter = TableFormatter;
        let todo1 = create_test_todo("Fix bug", TodoType::Todo, None, false);
        let todo2 = create_test_todo("Broken", TodoType::Bug, None, false);

        let mut todos_map = HashMap::new();
        todos_map.insert(&TodoType::Todo, vec![&todo1]);
        todos_map.insert(&TodoType::Bug, vec![&todo2]);

        let result = formatter.format(&todos_map, 2).unwrap();
        let output = result.join("\n");

        assert!(output.contains("Found 2 TODO comments in 2 groups"));
        assert!(output.contains("TODO"));
        assert!(output.contains("BUG"));
    }

    #[test]
    fn test_truncation() {
        let long_str = "a".repeat(100);
        let truncated = TableFormatter::truncate_string(&long_str, 10);
        assert_eq!(truncated.chars().count(), 10);
        assert!(truncated.ends_with('…'));
    }

    proptest! {
        #[test]
        fn prop_truncate_string_respects_max_len(
            s in ".{0,200}",
            max_len in 1usize..50
        ) {
            let truncated = TableFormatter::truncate_string(&s, max_len);
            prop_assert!(
                truncated.chars().count() <= max_len,
                "Truncated string ({} chars) exceeds max_len ({})",
                truncated.chars().count(),
                max_len
            );
        }
    }
}
