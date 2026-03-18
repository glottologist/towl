//! Directory traversal and concurrent file scanning for TODO comments.
//!
//! The [`Scanner`] walks a directory tree using gitignore-aware traversal,
//! filters files by extension, and scans matching files concurrently with
//! bounded parallelism. Resource limits prevent excessive memory use on
//! large codebases.

pub mod error;
mod limits;
mod types;
mod walker;

pub use limits::ScanResult;
pub use types::Scanner;
