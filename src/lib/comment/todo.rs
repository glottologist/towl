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
