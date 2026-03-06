use std::path::Path;

use regex::{Regex, RegexBuilder};

use crate::{
    comment::todo::{TodoComment, TodoType},
    config::ParsingConfig,
};

use super::error::TowlParserError;

const MIN_CONTEXT_LINES: usize = 1;
const MAX_CONTEXT_LINES: usize = 50;
const FORWARD_SEARCH_LINES: usize = 3;
const MAX_PATTERN_LENGTH: usize = 256;
const REGEX_SIZE_LIMIT: usize = 262_144;
const MAX_TOTAL_PATTERNS: usize = 50;

/// Parses file content to extract TODO comments with context.
///
/// Uses configurable regex patterns to identify comments and TODO markers,
/// extracting surrounding context and function information for each match.
pub struct Parser {
    comment_patterns: Vec<Regex>,
    patterns: Vec<Pattern>,
    function_patterns: Vec<Regex>,
    context_lines: usize,
}

struct Pattern {
    regex: Regex,
    todo_type: TodoType,
}

impl Parser {
    fn build_regex(pattern: &str) -> Result<Regex, TowlParserError> {
        if pattern.len() > MAX_PATTERN_LENGTH {
            return Err(TowlParserError::PatternTooLong(
                pattern.len(),
                MAX_PATTERN_LENGTH,
            ));
        }
        RegexBuilder::new(pattern)
            .size_limit(REGEX_SIZE_LIMIT)
            .build()
            .map_err(|e| TowlParserError::InvalidRegexPattern(pattern.to_string(), e))
    }

    /// Creates a new parser from configuration.
    ///
    /// Compiles all regex patterns from the config at construction time for efficiency.
    ///
    /// # Errors
    /// Returns `TowlParserError::InvalidRegexPattern` if any pattern is malformed.
    /// Returns `TowlParserError::TooManyTotalPatterns` if total patterns exceed the budget.
    pub(crate) fn new(config: &ParsingConfig) -> Result<Self, TowlParserError> {
        let total_patterns = config
            .comment_prefixes
            .len()
            .saturating_add(config.todo_patterns.len())
            .saturating_add(config.function_patterns.len());

        if total_patterns > MAX_TOTAL_PATTERNS {
            return Err(TowlParserError::TooManyTotalPatterns {
                count: total_patterns,
                max_allowed: MAX_TOTAL_PATTERNS,
            });
        }

        let comment_patterns = config
            .comment_prefixes
            .iter()
            .map(|p| Self::build_regex(p))
            .collect::<Result<Vec<_>, _>>()?;

        let patterns = config
            .todo_patterns
            .iter()
            .map(|p| {
                let regex = Self::build_regex(p)?;
                let todo_type: TodoType = p
                    .as_str()
                    .try_into()
                    .map_err(TowlParserError::UnknownConfigPattern)?;
                Ok(Pattern { regex, todo_type })
            })
            .collect::<Result<Vec<_>, TowlParserError>>()?;

        let function_patterns = config
            .function_patterns
            .iter()
            .map(|p| Self::build_regex(p))
            .collect::<Result<Vec<_>, _>>()?;

        let context_lines = config
            .include_context_lines
            .clamp(MIN_CONTEXT_LINES, MAX_CONTEXT_LINES);

        Ok(Self {
            comment_patterns,
            patterns,
            function_patterns,
            context_lines,
        })
    }
    /// Parses file content to extract all TODO comments.
    ///
    /// Identifies comment lines using configured patterns, then searches for TODO markers
    /// within those comments. For each TODO found, extracts:
    /// - Description text
    /// - Surrounding context lines
    /// - Function context (if applicable)
    /// - Location information (line, column)
    ///
    /// # Errors
    /// Returns `TowlParserError` if TODO extraction fails (rare, defensive).
    pub(crate) fn parse(
        &self,
        path: &Path,
        content: &str,
    ) -> Result<Vec<TodoComment>, TowlParserError> {
        let mut todos = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            let is_comment = self
                .comment_patterns
                .iter()
                .any(|pattern| pattern.is_match(line));

            if !is_comment {
                continue;
            }

            for pattern in &self.patterns {
                if let Some(captures) = pattern.regex.captures(line) {
                    let todo = self.extract_todo(
                        path,
                        line,
                        line_idx + 1,
                        &captures,
                        &lines,
                        pattern.todo_type,
                    )?;
                    todos.push(todo);
                }
            }
        }

        Ok(todos)
    }

    fn extract_todo(
        &self,
        path: &Path,
        line: &str,
        line_number: usize,
        captures: &regex::Captures,
        all_lines: &[&str],
        todo_type: TodoType,
    ) -> Result<TodoComment, TowlParserError> {
        let description = if captures.len() > 1 {
            captures.get(1).map(|m| m.as_str().trim().to_string())
        } else {
            captures.get(0).map(|m| m.as_str().trim().to_string())
        }
        .unwrap_or_else(|| "No description".to_string());

        let full_match = captures.get(0).ok_or(TowlParserError::RegexGroupMissing)?;
        let match_start = full_match.start();
        let match_end = full_match.end();

        let context_lines = self.extract_context(all_lines, line_number - 1);

        let function_context = self.find_function_context(all_lines, line_number - 1);

        let id = format!(
            "{}_L{}_C{}",
            path.file_name().unwrap_or_default().to_string_lossy(),
            line_number,
            match_start
        );

        Ok(TodoComment {
            id,
            file_path: path.to_path_buf(),
            line_number,
            column_start: match_start,
            column_end: match_end,
            todo_type,
            original_text: line.to_string(),
            description,
            context_lines,
            function_context,
        })
    }

    fn extract_context(&self, lines: &[&str], current_line: usize) -> Vec<String> {
        let mut context = Vec::new();

        let start = current_line.saturating_sub(self.context_lines);

        let end = std::cmp::min(current_line + self.context_lines + 1, lines.len());

        for (i, line) in lines.iter().enumerate().take(end).skip(start) {
            if i != current_line {
                context.push(format!("{}: {}", i + 1, line));
            }
        }

        context
    }

    fn match_function_name<'a>(&self, line: &'a str) -> Option<&'a str> {
        for pattern in &self.function_patterns {
            if let Some(captures) = pattern.captures(line) {
                for j in 1..captures.len() {
                    if let Some(name) = captures.get(j) {
                        let name_str = name.as_str();
                        if !name_str.is_empty()
                            && name_str.chars().all(|c| c.is_alphanumeric() || c == '_')
                        {
                            return Some(name_str);
                        }
                    }
                }
            }
        }
        None
    }

    fn find_function_context(&self, lines: &[&str], current_line: usize) -> Option<String> {
        for i in (0..=current_line).rev() {
            if let Some(name) = self.match_function_name(lines[i]) {
                return Some(format!("{name}:{}", i + 1));
            }
        }

        let search_end = std::cmp::min(current_line + FORWARD_SEARCH_LINES + 1, lines.len());
        for (i, line) in lines
            .iter()
            .enumerate()
            .take(search_end)
            .skip(current_line + 1)
        {
            if let Some(name) = self.match_function_name(line) {
                return Some(format!("{name}:{} (below)", i + 1));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rstest::rstest;
    use std::path::PathBuf;

    fn create_test_config() -> ParsingConfig {
        crate::config::test_parsing_config()
    }

    #[rstest]
    #[case("fn test_function() {", Some("test_function:1"))]
    #[case("pub fn public_function() {", Some("public_function:1"))]
    #[case("def python_function():", Some("python_function:1"))]
    #[case("    fn indented_function() {", Some("indented_function:1"))]
    #[case("let variable = 5;", None)]
    fn test_function_context_detection(#[case] line: &str, #[case] expected_context: Option<&str>) {
        let config = create_test_config();
        let parser = Parser::new(&config).unwrap();

        let lines = vec![line];
        let context = parser.find_function_context(&lines, 0);

        match expected_context {
            Some(expected) => {
                assert_eq!(context, Some(expected.to_string()));
            }
            None => {
                assert_eq!(context, None);
            }
        }
    }

    #[rstest]
    #[case(0, vec!["2: line2", "3: line3", "4: line4"])]
    #[case(2, vec!["1: line1", "2: line2", "4: line4", "5: line5", "6: line6"])]
    #[case(4, vec!["2: line2", "3: line3", "4: line4", "6: line6"])]
    fn test_context_extraction(#[case] current_line: usize, #[case] expected_context: Vec<&str>) {
        let config = create_test_config();
        let parser = Parser::new(&config).unwrap();

        let lines = vec!["line1", "line2", "line3", "line4", "line5", "line6"];
        let context = parser.extract_context(&lines, current_line);

        let expected: Vec<String> = expected_context.iter().map(ToString::to_string).collect();
        assert_eq!(context, expected);
    }

    prop_compose! {
        fn valid_comment_prefix()(prefix in r"(//|#|\*|/\*)") -> String {
            prefix
        }
    }

    prop_compose! {
        fn valid_todo_keyword()(keyword in r"(TODO|FIXME|HACK|NOTE|BUG)") -> String {
            keyword
        }
    }

    prop_compose! {
        fn valid_description()(desc in r"[a-zA-Z0-9.,!?-][a-zA-Z0-9 .,!?-]{0,99}") -> String {
            desc
        }
    }

    proptest! {
        #[test]
        fn prop_test_comment_with_todo_always_detected(
            prefix in valid_comment_prefix(),
            keyword in valid_todo_keyword(),
            description in valid_description()
        ) {
            let config = create_test_config();
            let parser = Parser::new(&config).unwrap();
            let path = PathBuf::from("test.rs");

            let line = format!("{prefix} {keyword}: {description}");
            let result = parser.parse(&path, &line).unwrap();

            if prefix == "//" || prefix == "#" || prefix == "*" {
                prop_assert!(!result.is_empty(), "Failed to detect TODO in: {}", line);
                if !result.is_empty() {
                    let trimmed_desc = description.trim();
                    if !trimmed_desc.is_empty() {
                        prop_assert!(result[0].description.contains(trimmed_desc) || result[0].description.trim() == trimmed_desc);
                    }
                }
            }
        }

        #[test]
        fn prop_test_non_comment_todos_ignored(
            keyword in valid_todo_keyword(),
            description in valid_description()
        ) {
            let config = create_test_config();
            let parser = Parser::new(&config).unwrap();
            let path = PathBuf::from("test.rs");

            let line = format!("let {} = \"{}: {}\";", keyword.to_lowercase(), keyword, description);
            let result = parser.parse(&path, &line).unwrap();

            prop_assert!(result.is_empty(), "Incorrectly detected TODO in string: {}", line);
        }

        #[test]
        fn prop_test_whitespace_handling(
            leading_ws in r"\s*",
            trailing_ws in r"\s*",
            keyword in valid_todo_keyword(),
            description in valid_description()
        ) {
            let config = create_test_config();
            let parser = Parser::new(&config).unwrap();
            let path = PathBuf::from("test.rs");

            let line = format!("{leading_ws}// {keyword}: {description}{trailing_ws}");
            let result = parser.parse(&path, &line).unwrap();

            prop_assert!(!result.is_empty(), "Failed to detect TODO with whitespace: {}", line);
            if !result.is_empty() {
                prop_assert!(result[0].description.trim() == description.trim());
            }
        }

        #[test]
        fn prop_test_line_number_accuracy(
            lines_before in prop::collection::vec(".*", 0..10),
            keyword in valid_todo_keyword(),
            description in valid_description(),
            lines_after in prop::collection::vec(".*", 0..10)
        ) {
            let config = create_test_config();
            let parser = Parser::new(&config).unwrap();
            let path = PathBuf::from("test.rs");

            let todo_line = format!("// {keyword}: {description}");
            let expected_line_number = lines_before.len() + 1;

            let mut all_lines = lines_before;
            all_lines.push(todo_line);
            all_lines.extend(lines_after);

            let content = all_lines.join("\n");
            let result = parser.parse(&path, &content).unwrap();

            prop_assert!(!result.is_empty(), "Failed to detect TODO in multi-line content");
            if !result.is_empty() {
                prop_assert_eq!(result[0].line_number, expected_line_number);
            }
        }
    }

    #[test]
    fn test_total_pattern_budget_exceeded() {
        let mut config = create_test_config();
        config.comment_prefixes = (0..20).map(|i| format!("prefix_{i}")).collect();
        config.todo_patterns = (0..20).map(|i| format!("todo_{i}")).collect();
        config.function_patterns = (0..20).map(|i| format!("func_{i}")).collect();

        let result = Parser::new(&config);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(TowlParserError::TooManyTotalPatterns {
                count: 60,
                max_allowed: 50
            })
        ));
    }

    #[test]
    fn test_malformed_regex_patterns() {
        let mut config = create_test_config();
        config.todo_patterns = vec!["[invalid regex".to_string()];

        let result = Parser::new(&config);
        assert!(matches!(
            result,
            Err(TowlParserError::InvalidRegexPattern(..))
        ));
    }

    #[test]
    fn test_column_position_accuracy() {
        let config = create_test_config();
        let parser = Parser::new(&config).unwrap();
        let path = PathBuf::from("test.rs");

        let content = "    // TODO: Test column positions";
        let result = parser.parse(&path, content).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].column_start, 7);
        assert_eq!(result[0].column_end, content.len());
    }

    proptest! {
        #[test]
        fn prop_parser_never_panics_on_any_input(
            content in ".*"
        ) {
            let config = create_test_config();
            let parser = Parser::new(&config).unwrap();
            let path = PathBuf::from("test.rs");

            let _ = parser.parse(&path, &content);
        }

        #[test]
        fn prop_parsed_todos_have_valid_line_numbers(
            lines in prop::collection::vec("[^\n]*", 1..100)
        ) {
            let config = create_test_config();
            let parser = Parser::new(&config).unwrap();
            let path = PathBuf::from("test.rs");

            let content = lines.join("\n");
            let todos = parser.parse(&path, &content).unwrap();

            for todo in todos {
                prop_assert!(todo.line_number > 0, "Line number must be positive");
                prop_assert!(
                    todo.line_number <= lines.len(),
                    "Line number {} exceeds total lines {}",
                    todo.line_number,
                    lines.len()
                );
            }
        }

        #[test]
        fn prop_parsed_todos_have_valid_column_positions(
            prefix in "[^\n]*",
            todo_type in prop::sample::select(vec!["TODO", "FIXME", "HACK", "NOTE", "BUG"]),
            description in "[^\n]*"
        ) {
            let config = create_test_config();
            let parser = Parser::new(&config).unwrap();
            let path = PathBuf::from("test.rs");

            let line = format!("{prefix}// {todo_type}: {description}");
            let content = format!("fn main() {{\n    {line}\n}}");

            let todos = parser.parse(&path, &content).unwrap();

            for todo in &todos {
                prop_assert!(
                    todo.column_start <= todo.column_end,
                    "Column start {} must be <= column end {}",
                    todo.column_start,
                    todo.column_end
                );

                let lines: Vec<&str> = content.lines().collect();
                if todo.line_number > 0 && todo.line_number <= lines.len() {
                    let line_len = lines[todo.line_number - 1].len();
                    prop_assert!(
                        todo.column_end <= line_len,
                        "Column end {} exceeds line length {}",
                        todo.column_end,
                        line_len
                    );
                }
            }
        }

        #[test]
        fn prop_todos_preserve_original_text(
            prefix in "[^\n]*",
            todo_marker in prop::sample::select(vec!["TODO:", "FIXME:", "HACK:", "NOTE:", "BUG:"]),
            description in "[^\n]{0,100}"
        ) {
            let config = create_test_config();
            let parser = Parser::new(&config).unwrap();
            let path = PathBuf::from("test.rs");

            let original_line = format!("{prefix} // {todo_marker} {description}");
            let content = original_line.clone();

            let todos = parser.parse(&path, &content).unwrap();

            if !todos.is_empty() {
                let todo = &todos[0];
                prop_assert_eq!(
                    todo.original_text.trim(),
                    original_line.trim(),
                    "Original text should be preserved"
                );
            }
        }

        #[test]
        fn prop_multiple_todos_parsed_independently(
            num_todos in 1usize..10usize,
            base_content in "[^\n]*"
        ) {
            let config = create_test_config();
            let parser = Parser::new(&config).unwrap();
            let path = PathBuf::from("test.rs");

            let mut lines = vec![base_content.clone()];
            for i in 0..num_todos {
                lines.push(format!("// TODO: item {i}"));
                lines.push(base_content.clone());
            }

            let content = lines.join("\n");
            let todos = parser.parse(&path, &content).unwrap();

            prop_assert_eq!(
                todos.len(),
                num_todos,
                "Should find exactly {} TODOs",
                num_todos
            );

            let mut line_numbers: Vec<usize> = todos.iter().map(|t| t.line_number).collect();
            line_numbers.sort_unstable();
            line_numbers.dedup();
            prop_assert_eq!(
                line_numbers.len(),
                todos.len(),
                "Each TODO should have a unique line number"
            );
        }

        #[test]
        fn prop_parser_handles_empty_and_whitespace_lines(
            num_empty in 0usize..10usize,
            num_spaces in 0usize..10usize
        ) {
            let config = create_test_config();
            let parser = Parser::new(&config).unwrap();
            let path = PathBuf::from("test.rs");

            let mut lines = vec![];
            for _ in 0..num_empty {
                lines.push(String::new());
            }
            lines.push("// TODO: test".to_string());
            for i in 0..num_spaces {
                lines.push(" ".repeat(i));
            }

            let content = lines.join("\n");
            let result = parser.parse(&path, &content);

            prop_assert!(
                result.is_ok(),
                "Parser should handle empty/whitespace lines"
            );

            let todos = result.unwrap();
            prop_assert_eq!(todos.len(), 1, "Should find exactly one TODO");
        }

        #[test]
        fn prop_parser_respects_comment_prefixes(
            non_comment_prefix in "[^/#]*",
            todo_text in "TODO: [^\n]*"
        ) {
            prop_assume!(!non_comment_prefix.contains("//") && !non_comment_prefix.contains('#') && !non_comment_prefix.contains('*'));
            prop_assume!(!todo_text.contains("//") && !todo_text.contains("/*") && !todo_text.contains('*'));

            let config = create_test_config();
            let parser = Parser::new(&config).unwrap();
            let path = PathBuf::from("test.rs");

            let content = format!("{non_comment_prefix}{todo_text}");
            let todos = parser.parse(&path, &content).unwrap();

            if !non_comment_prefix.is_empty() {
                prop_assert!(
                    todos.is_empty(),
                    "TODO without comment prefix should not be detected"
                );
            }
        }

        #[test]
        fn prop_todo_comment_json_roundtrip(
            id in "[a-z]{10}",
            line in 1usize..10000,
            col_start in 0usize..100,
            col_end in 101usize..200,
            desc in "[a-zA-Z ]{1,100}"
        ) {
            use crate::comment::todo::{TodoComment, TodoType};

            let todo = TodoComment {
                id,
                file_path: PathBuf::from("test.rs"),
                line_number: line,
                column_start: col_start,
                column_end: col_end,
                todo_type: TodoType::Todo,
                original_text: format!("// TODO: {desc}"),
                description: desc,
                context_lines: vec!["context".to_string()],
                function_context: Some("main".to_string()),
            };

            let json = serde_json::to_string(&todo).unwrap();
            let decoded: TodoComment = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(todo, decoded);
        }

        #[test]
        fn prop_match_function_name_with_special_chars(
            prefix in "[^\\n]{0,20}",
            func_name in "[a-zA-Z_][a-zA-Z0-9_]{0,20}",
            suffix in "[^\\n]{0,20}"
        ) {
            let config = create_test_config();
            let parser = Parser::new(&config).unwrap();

            let line = format!("{prefix}fn {func_name}{suffix}");
            let result = parser.match_function_name(&line);

            // If the line starts with optional whitespace + optional "pub " + "fn " + name,
            // the function name should be matched
            if let Some(matched) = result {
                prop_assert!(
                    matched.chars().all(|c| c.is_alphanumeric() || c == '_'),
                    "Matched name should only contain alphanumeric or underscore: {:?}",
                    matched
                );
            }
        }
    }
}
