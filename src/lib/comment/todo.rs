use super::error::TowlCommentError;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoComment {
    pub id: String,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub column_start: usize,
    pub column_end: usize,
    pub todo_type: TodoType,
    pub original_text: String,
    pub description: String,
    pub context_lines: Vec<String>,
    pub function_context: Option<String>,
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
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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
        fn test_todo_type_case_insensitivity(
            keyword in prop::sample::select(vec!["TODO", "FIXME", "HACK", "NOTE", "BUG"]),
            suffix in "[a-zA-Z0-9 :]{0,20}"
        ) {
            let input = format!("{keyword}{suffix}");
            let result_upper = TodoType::try_from(input.as_str());
            prop_assert!(result_upper.is_ok());

            let lower_input = input.to_lowercase();
            let result_lower = TodoType::try_from(lower_input.as_str());
            prop_assert!(result_lower.is_ok());
        }

        #[test]
        fn test_todo_type_with_random_prefix_suffix(
            prefix in "[a-zA-Z0-9 ]*",
            todo_type in prop::sample::select(vec!["TODO", "FIXME", "HACK", "NOTE", "BUG"]),
            suffix in "[a-zA-Z0-9 :]*"
        ) {
            let input = format!("{prefix}{todo_type}{suffix}");
            let result = TodoType::try_from(input.as_str());

            prop_assert!(result.is_ok());
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

        #[test]
        fn test_whitespace_handling(
            spaces_before in " {0,5}",
            todo_type in prop::sample::select(vec!["TODO", "FIXME", "HACK", "NOTE", "BUG"]),
            spaces_after in " {0,5}"
        ) {
            let input = format!("{spaces_before}{todo_type}{spaces_after}");
            let result = TodoType::try_from(input.as_str());
            prop_assert!(result.is_ok());
        }
    }
}
