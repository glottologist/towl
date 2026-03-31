//! Post-issue-creation processor that replaces TODO markers with issue links.
//!
//! After GitHub issues are created, [`Processor::replace_todos`] rewrites source
//! files to replace each TODO marker with `GH_ISSUE: <issue_url> : <description>`,
//! preserving the original description text. Uses atomic file writes.

pub mod error;
mod types;

pub use types::{Processor, ProcessorResult};
