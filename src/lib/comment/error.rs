use thiserror::Error;

/// Errors produced when parsing comment text into a [`super::todo::TodoType`].
#[derive(Error, Debug)]
pub enum TowlCommentError {
    #[error("Cannot recognise {comment} as a valid TODO type")]
    UnknownTodoType { comment: String },
}
