use super::error::TowlCommentError;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// The category of a TODO comment, ordered by priority (Bug=1 highest, Note=5 lowest).
///
/// Parsed from comment text via [`TryFrom<&str>`] (case-insensitive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
pub enum TodoType {
    Todo,
    Fixme,
    Hack,
    Note,
    Bug,
}
impl fmt::Display for TodoType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Todo => write!(f, "TODO"),
            Self::Fixme => write!(f, "FIXME"),
            Self::Hack => write!(f, "HACK"),
            Self::Note => write!(f, "NOTE"),
            Self::Bug => write!(f, "BUG"),
        }
    }
}

impl TodoType {
    #[must_use]
    pub const fn as_filter_str(&self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::Fixme => "fixme",
            Self::Hack => "hack",
            Self::Note => "note",
            Self::Bug => "bug",
        }
    }

    #[must_use]
    pub const fn github_label(&self) -> &'static str {
        self.as_filter_str()
    }

    #[must_use]
    pub const fn priority(&self) -> u8 {
        match self {
            Self::Bug => 1,
            Self::Fixme => 2,
            Self::Hack => 3,
            Self::Todo => 4,
            Self::Note => 5,
        }
    }
}

impl TryFrom<&str> for TodoType {
    type Error = TowlCommentError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let upper = value.to_uppercase();
        if upper.contains("TODO") {
            Ok(Self::Todo)
        } else if upper.contains("FIXME") {
            Ok(Self::Fixme)
        } else if upper.contains("HACK") {
            Ok(Self::Hack)
        } else if upper.contains("NOTE") {
            Ok(Self::Note)
        } else if upper.contains("BUG") {
            Ok(Self::Bug)
        } else {
            Err(TowlCommentError::UnknownTodoType {
                comment: value.to_owned(), // clone: need owned String for error variant
            })
        }
    }
}

/// A located TODO comment extracted from a source file.
///
/// Contains the comment text, its position within the file, surrounding context,
/// and the enclosing function name (if detected). The `id` field
/// (`{file_path}_L{line_number}`) is used for GitHub issue deduplication.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodoComment {
    /// Unique identifier: `{file_path}_L{line_number}`.
    pub id: String,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub column_start: usize,
    pub column_end: usize,
    pub todo_type: TodoType,
    /// The full original comment line as it appears in the source file.
    pub original_text: String,
    /// The extracted description text after the TODO keyword and colon.
    pub description: String,
    /// Surrounding source lines for context display.
    pub context_lines: Vec<String>,
    /// Name of the enclosing function, if detected by pattern matching.
    pub function_context: Option<String>,
    /// LLM validation analysis, populated when `--ai` flag is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub analysis: Option<crate::llm::types::AnalysisResult>,
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    pub struct TestTodoBuilder {
        todo_type: TodoType,
        file_path: PathBuf,
        line_number: usize,
        column_start: usize,
        column_end: usize,
        description: String,
        original_text: Option<String>,
        context_lines: Vec<String>,
        function_context: Option<String>,
    }

    impl TestTodoBuilder {
        pub fn new() -> Self {
            Self {
                todo_type: TodoType::Todo,
                file_path: PathBuf::from("test.rs"),
                line_number: 1,
                column_start: 0,
                column_end: 0,
                description: "test".to_string(),
                original_text: None,
                context_lines: vec![],
                function_context: None,
            }
        }

        pub fn todo_type(mut self, t: TodoType) -> Self {
            self.todo_type = t;
            self
        }

        pub fn file_path(mut self, p: impl Into<PathBuf>) -> Self {
            self.file_path = p.into();
            self
        }

        pub fn line_number(mut self, n: usize) -> Self {
            self.line_number = n;
            self
        }

        pub fn column_start(mut self, n: usize) -> Self {
            self.column_start = n;
            self
        }

        pub fn column_end(mut self, n: usize) -> Self {
            self.column_end = n;
            self
        }

        pub fn description(mut self, d: &str) -> Self {
            self.description = d.to_string();
            self
        }

        pub fn original_text(mut self, t: &str) -> Self {
            self.original_text = Some(t.to_string());
            self
        }

        pub fn context_lines(mut self, c: Vec<String>) -> Self {
            self.context_lines = c;
            self
        }

        pub fn function_context(mut self, f: &str) -> Self {
            self.function_context = Some(f.to_string());
            self
        }

        pub fn build(self) -> TodoComment {
            let original_text = self
                .original_text
                .unwrap_or_else(|| format!("// {}: {}", self.todo_type, self.description));
            TodoComment {
                id: format!("{}_L{}", self.file_path.display(), self.line_number),
                file_path: self.file_path,
                line_number: self.line_number,
                column_start: self.column_start,
                column_end: self.column_end,
                todo_type: self.todo_type,
                original_text,
                description: self.description,
                context_lines: self.context_lines,
                function_context: self.function_context,
                analysis: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rstest::rstest;

    proptest! {
        #[test]
        fn prop_unicode_handling(
            keyword in prop::sample::select(vec!["TODO", "FIXME", "HACK", "NOTE", "BUG"]),
            unicode_suffix in "[\\p{L}\\p{N}\\p{So} ]{1,30}"
        ) {
            let input = format!("{keyword}: {unicode_suffix}");
            let result = TodoType::try_from(input.as_str());
            prop_assert!(result.is_ok(), "Should handle unicode input: {}", input);
        }

        #[test]
        fn prop_try_from_accepts_valid_keywords_in_any_context(
            prefix in "[\\p{L}\\p{N}\\s]*",
            keyword in prop::sample::select(vec!["TODO", "FIXME", "HACK", "NOTE", "BUG"]),
            suffix in "[\\p{L}\\p{N}\\s:]*",
        ) {
            let input = format!("{prefix}{keyword}{suffix}");
            let result = TodoType::try_from(input.as_str());
            prop_assert!(result.is_ok(), "Should accept: {}", input);

            let lower_input = input.to_lowercase();
            let result_lower = TodoType::try_from(lower_input.as_str());
            prop_assert!(result_lower.is_ok(), "Should accept lowercase: {}", lower_input);
        }

        #[test]
        fn test_invalid_types_always_fail(
            s in prop::string::string_regex("[A-Z]{3,10}").unwrap()
                .prop_filter("Must not be a valid type", |s| {
                    !s.contains("TODO") && !s.contains("FIXME") &&
                    !s.contains("HACK") && !s.contains("NOTE") &&
                    !s.contains("BUG")
                })
        ) {
            let result = TodoType::try_from(s.as_str());
            prop_assert!(result.is_err());
        }
    }

    #[rstest]
    #[case("TODO: x", TodoType::Todo, "TODO", "todo", 4)]
    #[case("FIXME: x", TodoType::Fixme, "FIXME", "fixme", 2)]
    #[case("HACK: x", TodoType::Hack, "HACK", "hack", 3)]
    #[case("NOTE: x", TodoType::Note, "NOTE", "note", 5)]
    #[case("BUG: x", TodoType::Bug, "BUG", "bug", 1)]
    fn test_todo_type_methods(
        #[case] input: &str,
        #[case] expected_variant: TodoType,
        #[case] display: &str,
        #[case] filter_str: &str,
        #[case] priority: u8,
    ) {
        let variant = TodoType::try_from(input).unwrap();
        assert_eq!(variant, expected_variant);
        assert_eq!(format!("{variant}"), display);
        assert_eq!(variant.as_filter_str(), filter_str);
        assert_eq!(variant.github_label(), filter_str);
        assert_eq!(variant.priority(), priority);
    }
}
