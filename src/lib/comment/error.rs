use thiserror::Error;

#[derive(Error, Debug)]
pub enum TowlCommentError {
    #[error("Cannot recognise {comment} as a valid TODO type")]
    UnknownTodoType { comment: String },
}
