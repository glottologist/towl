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
        let mut type_width = 4; // "Type"
        let mut desc_width = 11; // "Description"
        let mut file_width = 4; // "File"
        let mut line_width = 4; // "Line"
        let mut func_width = 8; // "Function"

        for (todo_type, todos_of_type) in todos_map {
            let type_str = format!("{:?}", todo_type);
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
                "│ {:<width_type$} │ {:<width_desc$} │ {:<width_file$} │ {:<width_line$} │ {:<width_func$} │",
                type_truncated, desc_truncated, file_truncated, line_truncated, func_truncated,
                width_type = type_width,
                width_desc = desc_width,
                width_file = file_width,
                width_line = line_width,
                width_func = func_width
            )
        } else {
            format!(
                "│ {:<width_type$} │ {:<width_desc$} │ {:<width_file$} │ {:>width_line$} │ {:<width_func$} │",
                type_truncated, desc_truncated, file_truncated, line_truncated, func_truncated,
                width_type = type_width,
                width_desc = desc_width,
                width_file = file_width,
                width_line = line_width,
                width_func = func_width
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

    fn truncate_string(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}…", &s[0..max_len.saturating_sub(1)])
        }
    }
}

impl Formatter for TableFormatter {
    fn format(
        &self,
        todos_map: &HashMap<&TodoType, Vec<&TodoComment>>,
        total_count: usize,
    ) -> Result<Vec<String>, FormatterError> {
        let mut output = Vec::new();

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
        output.push("".to_string());

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
                let type_str = format!("{:?}", todo_type);
                let file_str = todo.file_path.display().to_string();
                let line_str = todo.line_number.to_string();
                let func_str = todo
                    .function_context
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("");

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
