//! Regex-based parser for extracting TODO comments from source file content.
//!
//! Compiles comment-prefix and TODO-keyword patterns from [`crate::config::ParsingConfig`]
//! into a reusable [`Parser`](types::Parser) that produces [`TodoComment`](crate::comment::todo::TodoComment) values.

mod context;
pub mod error;
mod pattern;
mod types;

pub(crate) use types::*;
