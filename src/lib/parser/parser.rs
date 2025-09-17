use std::path::Path;

use regex::Regex;

use crate::{
    comment::todo::{TodoComment, TodoType},
    config::config::ParsingConfig,
};

use super::error::TowlParserError;

pub(crate) struct Parser {
    comment_patterns: Vec<Regex>,
    patterns: Vec<Pattern>,
    function_patterns: Vec<Regex>,
}
pub(crate) struct Pattern {
    regex: Regex,
    todo_type: TodoType,
}

impl Parser {
    pub(crate) fn new(config: &ParsingConfig) -> Result<Self, TowlParserError> {
        let mut comment_patterns = Vec::new();
        for pattern in &config.comment_prefixes {
            let regex = Regex::new(&pattern)
                .map_err(|e| TowlParserError::InvalidRegexPattern(pattern.clone().into(), e))?;
            comment_patterns.push(regex);
        }

        let num_patterns = config.todo_patterns.len();
        let mut patterns = Vec::with_capacity(num_patterns);

        for pattern in &config.todo_patterns {
            let regex = Regex::new(&pattern)
                .map_err(|e| TowlParserError::InvalidRegexPattern(pattern.clone().into(), e))?;

            let todo_type: TodoType = pattern
                .as_str()
                .try_into()
                .map_err(TowlParserError::UnknownConfigPattern)?;

            patterns.push(Pattern { regex, todo_type });
        }

        let mut function_patterns = Vec::new();
        for pattern in &config.function_patterns {
            let regex = Regex::new(&pattern)
                .map_err(|e| TowlParserError::InvalidRegexPattern(pattern.clone().into(), e))?;
            function_patterns.push(regex);
        }

        Ok(Parser {
            comment_patterns,
            patterns,
            function_patterns,
        })
    }
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
                        &pattern.todo_type,
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
        todo_type: &TodoType,
    ) -> Result<TodoComment, TowlParserError> {
        let description = if captures.len() > 1 {
            captures.get(1).map(|m| m.as_str().trim().to_string())
        } else {
            captures.get(0).map(|m| m.as_str().trim().to_string())
        }
        .unwrap_or_else(|| "No description".to_string());

        let match_start = captures.get(0).unwrap().start();
        let match_end = captures.get(0).unwrap().end();

        let context_lines = self.extract_context(all_lines, line_number - 1, 3);

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
            todo_type: todo_type.clone(),
            original_text: line.to_string(),
            description,
            context_lines,
            function_context,
        })
    }

    fn extract_context(
        &self,
        lines: &[&str],
        current_line: usize,
        context_size: usize,
    ) -> Vec<String> {
        let mut context = Vec::new();

        let start = if current_line >= context_size {
            current_line - context_size
        } else {
            0
        };

        let end = std::cmp::min(current_line + context_size + 1, lines.len());

        for i in start..end {
            if i != current_line {
                context.push(format!("{}: {}", i + 1, lines[i]));
            }
        }

        context
    }

    fn find_function_context(&self, lines: &[&str], current_line: usize) -> Option<String> {
        // LIMITATION: Only searches backwards from current line
        // May miss function context if TODO appears before function declaration
        for i in (0..=current_line).rev() {
            let line = lines[i];

            for pattern in &self.function_patterns {
                if let Some(captures) = pattern.captures(line) {
                    for j in 1..captures.len() {
                        if let Some(name) = captures.get(j) {
                            let name_str = name.as_str();
                            if !name_str.is_empty()
                                && name_str.chars().all(|c| c.is_alphanumeric() || c == '_')
                            {
                                return Some(format!("{}:{}", name_str, i + 1));
                            }
                        }
                    }
                }
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
        ParsingConfig {
            file_extensions: vec!["rs".to_string(), "py".to_string()],
            exclude_patterns: vec!["target/*".to_string()],
            include_context_lines: 3,
            comment_prefixes: vec![
                r"//".to_string(),
                r"^\s*#".to_string(),
                r"/\*".to_string(),
                r"^\s*\*".to_string(),
            ],
            todo_patterns: vec![
                r"(?i)\bTODO:\s*(.*)".to_string(),
                r"(?i)\bFIXME:\s*(.*)".to_string(),
                r"(?i)\bHACK:\s*(.*)".to_string(),
                r"(?i)\bNOTE:\s*(.*)".to_string(),
                r"(?i)\bBUG:\s*(.*)".to_string(),
            ],
            function_patterns: vec![
                r"^\s*(pub\s+)?fn\s+(\w+)".to_string(),
                r"^\s*def\s+(\w+)".to_string(),
            ],
        }
    }

    #[rstest]
    #[case("// TODO: Fix this bug", true, "Fix this bug")]
    #[case("# TODO: Python comment", true, "Python comment")]
    #[case("/* TODO: C-style comment */", true, "C-style comment */")]
    #[case("* TODO: Multi-line continuation", true, "Multi-line continuation")]
    #[case("TODO: Not in a comment", false, "")]
    #[case("let todo = \"TODO: String literal\";", false, "")]
    #[case("    // FIXME: Indented comment", true, "Indented comment")]
    #[case("//TODO: No space after prefix", true, "No space after prefix")]
    fn test_comment_todo_detection(
        #[case] line: &str,
        #[case] should_match: bool,
        #[case] expected_description: &str,
    ) {
        let config = create_test_config();
        let parser = Parser::new(&config).unwrap();
        let path = PathBuf::from("test.rs");

        let result = parser.parse(&path, line).unwrap();

        if should_match {
            assert!(!result.is_empty(), "Expected to find TODO in: {}", line);
            assert_eq!(result[0].description, expected_description);
        } else {
            assert!(result.is_empty(), "Expected no TODOs in: {}", line);
        }
    }

    #[rstest]
    #[case("// TODO: First\n// FIXME: Second\n// HACK: Third", 3)]
    #[case("Code line\n// TODO: Only this\nMore code", 1)]
    #[case("No comments here\nJust code\nNothing", 0)]
    #[case("# TODO: Python\n// TODO: Rust\n/* TODO: C */", 3)]
    fn test_multiple_todos_in_content(#[case] content: &str, #[case] expected_count: usize) {
        let config = create_test_config();
        let parser = Parser::new(&config).unwrap();
        let path = PathBuf::from("test.rs");

        let result = parser.parse(&path, content).unwrap();
        assert_eq!(result.len(), expected_count);
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
        let context = parser.extract_context(&lines, current_line, 3);

        let expected: Vec<String> = expected_context.iter().map(|s| s.to_string()).collect();
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

            let line = format!("{} {}: {}", prefix, keyword, description);
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

            let line = format!("{}// {}: {}{}", leading_ws, keyword, description, trailing_ws);
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

            let todo_line = format!("// {}: {}", keyword, description);
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
    fn test_empty_content() {
        let config = create_test_config();
        let parser = Parser::new(&config).unwrap();
        let path = PathBuf::from("test.rs");

        let result = parser.parse(&path, "").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_very_long_lines() {
        let config = create_test_config();
        let parser = Parser::new(&config).unwrap();
        let path = PathBuf::from("test.rs");

        let long_description = "a".repeat(10000);
        let content = format!("// TODO: {}", long_description);

        let result = parser.parse(&path, &content).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, long_description);
    }

    #[test]
    fn test_unicode_in_comments() {
        let config = create_test_config();
        let parser = Parser::new(&config).unwrap();
        let path = PathBuf::from("test.rs");

        let content = "// TODO: Fix unicode issue with café and señor";
        let result = parser.parse(&path, content).unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0].description.contains("café"));
        assert!(result[0].description.contains("señor"));
    }

    #[test]
    fn test_malformed_regex_patterns() {
        let mut config = create_test_config();
        config.todo_patterns = vec!["[invalid regex".to_string()];

        let result = Parser::new(&config);
        assert!(result.is_err());
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

    #[test]
    fn test_mixed_comment_styles() {
        let config = create_test_config();
        let parser = Parser::new(&config).unwrap();
        let path = PathBuf::from("test.rs");

        let content = r#"
// TODO: C++ style comment
# TODO: Python style comment  
/* TODO: C style comment */
* TODO: Multi-line continuation
"#;

        let result = parser.parse(&path, content).unwrap();
        assert_eq!(result.len(), 4);

        let descriptions: Vec<_> = result.iter().map(|t| &t.description).collect();
        assert!(descriptions.iter().any(|d| d.contains("C++ style")));
        assert!(descriptions.iter().any(|d| d.contains("Python style")));
        assert!(descriptions.iter().any(|d| d.contains("C style")));
        assert!(descriptions
            .iter()
            .any(|d| d.contains("Multi-line continuation")));
    }

    #[test]
    fn test_case_insensitive_detection() {
        let config = create_test_config();
        let parser = Parser::new(&config).unwrap();
        let path = PathBuf::from("test.rs");

        let content = r#"
// todo: lowercase
// TODO: uppercase  
// ToDo: mixed case
// FIXME: all caps
"#;

        let result = parser.parse(&path, content).unwrap();
        assert_eq!(result.len(), 4);
    }
}
