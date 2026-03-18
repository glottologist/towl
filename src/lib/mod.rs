//! # towl
//!
//! A fast CLI tool and library for scanning codebases for TODO, FIXME, HACK, NOTE,
//! and BUG comments. Supports interactive browsing via a terminal UI, multiple output
//! formats, and automatic GitHub issue creation.
//!
//! ## Library usage
//!
//! ```no_run
//! use towl::scanner::Scanner;
//! use towl::config::{ParsingConfig, TowlConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = TowlConfig::load(None)?;
//! let scanner = Scanner::new(config.parsing)?;
//! let result = scanner.scan(".".into()).await?;
//!
//! for todo in &result.todos {
//!     println!("{}: {} ({}:{})", todo.todo_type, todo.description,
//!              todo.file_path.display(), todo.line_number);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Modules
//!
//! - [`scanner`] — Directory traversal and file scanning
//! - [`config`] — Configuration loading and validation
//! - [`output`] — Formatting and writing results (JSON, CSV, Markdown, etc.)
//! - [`github`] — GitHub issue creation from TODO comments
//! - [`tui`] — Interactive terminal UI for browsing and selecting TODOs
//! - [`processor`] — Post-creation replacement of TODO comments with issue links
//! - [`comment`] — TODO comment types and data structures
//! - [`cli`] — Command-line argument parsing
//! - [`error`] — Top-level error type aggregating all module errors

pub mod cli;
pub mod comment;
pub mod config;
pub mod error;
pub mod github;
pub mod llm;
pub mod output;
pub mod parser;
pub mod processor;
pub mod scanner;
pub mod tui;

use std::path::Path;

pub(crate) const MIN_CONTEXT_LINES: usize = 1;
pub(crate) const MAX_CONTEXT_LINES: usize = 50;

/// Writes content to a file atomically via tempfile + persist.
pub(crate) async fn atomic_write(target: &Path, content: &[u8]) -> Result<(), std::io::Error> {
    use tokio::io::AsyncWriteExt;

    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let temp = tempfile::Builder::new()
        .prefix(".towl_")
        .tempfile_in(parent)?;

    let (std_file, temp_path) = temp.into_parts();
    let mut file = tokio::fs::File::from_std(std_file);

    if let Err(e) = file.write_all(content).await {
        drop(file);
        drop(temp_path);
        return Err(e);
    }

    if let Err(e) = file.flush().await {
        drop(file);
        drop(temp_path);
        return Err(e);
    }

    drop(file);

    temp_path.persist(target).map_err(|e| e.error)
}

pub(crate) fn escape_markdown(s: &str) -> String {
    let mut out = String::with_capacity(s.len().saturating_add(s.len() / 4));
    for ch in s.chars() {
        if matches!(
            ch,
            '\\' | '`' | '*' | '_' | '[' | ']' | '#' | '!' | '<' | '>' | '~' | '|'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

pub(crate) fn contains_path_traversal(path: &Path) -> bool {
    path.components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::path::PathBuf;

    proptest! {
        #[test]
        fn prop_detects_traversal_when_present(
            components in prop::collection::vec("[a-zA-Z0-9_-]{1,10}", 1..5),
            insert_pos in 0usize..5,
        ) {
            let mut parts: Vec<String> = components;
            let pos = insert_pos.min(parts.len());
            parts.insert(pos, "..".to_string());

            let path: PathBuf = parts.iter().collect();
            prop_assert!(
                contains_path_traversal(&path),
                "Should detect traversal in: {:?}",
                path
            );
        }

        #[test]
        fn prop_accepts_safe_paths(
            components in prop::collection::vec("[a-zA-Z0-9_-]{1,10}", 1..5),
        ) {
            let path: PathBuf = components.iter().collect();
            prop_assert!(
                !contains_path_traversal(&path),
                "Should accept safe path: {:?}",
                path
            );
        }
    }
}
