use super::error::TowlCommentError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TodoType {
    Todo,
    Fixme,
    Hack,
    Note,
    Bug,
}
impl TryFrom<&str> for TodoType {
    type Error = TowlCommentError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let upper = value.to_uppercase();
        if upper.contains("TODO") {
            return Ok(TodoType::Todo);
        } else if upper.contains("FIXME") {
            return Ok(TodoType::Fixme);
        } else if upper.contains("HACK") {
            return Ok(TodoType::Hack);
        } else if upper.contains("NOTE") {
            return Ok(TodoType::Note);
        } else if upper.contains("BUG") {
            return Ok(TodoType::Bug);
        } else {
            return Err(TowlCommentError::UnknownTodoType {
                comment: value.to_owned(),
            });
        };
    }
}
impl TryFrom<String> for TodoType {
    type Error = TowlCommentError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let upper = value.to_uppercase();
        if upper.contains("TODO") {
            return Ok(TodoType::Todo);
        } else if upper.contains("FIXME") {
            return Ok(TodoType::Fixme);
        } else if upper.contains("HACK") {
            return Ok(TodoType::Hack);
        } else if upper.contains("NOTE") {
            return Ok(TodoType::Note);
        } else if upper.contains("BUG") {
            return Ok(TodoType::Bug);
        } else {
            return Err(TowlCommentError::UnknownTodoType { comment: value });
        };
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
