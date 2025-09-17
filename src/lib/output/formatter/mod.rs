pub mod error;
pub mod formatters;

use crate::comment::todo::{TodoComment, TodoType};
use error::FormatterError;
use std::collections::HashMap;

pub trait Formatter {
    fn format(
        &self,
        todos: &HashMap<&TodoType, Vec<&TodoComment>>,
        total_count: usize,
    ) -> Result<Vec<String>, FormatterError>;
}
