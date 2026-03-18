//! Post-issue-creation processor that replaces TODO comments with issue links.
//!
//! After GitHub issues are created, [`Processor::replace_todos`] rewrites source
//! files to replace each TODO comment with a `GH_ISSUE: <issue_url>` link using atomic
//! file writes.

pub mod error;
mod types;

pub use types::{Processor, ProcessorResult};
