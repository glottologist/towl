//! GitHub issue creation from TODO comments.
//!
//! Use [`GitHubClient`] to create issues and detect duplicates. Issues are
//! deduplicated by title and by embedded TODO ID in the issue body.

pub mod client;
pub mod error;
pub mod types;

pub use client::GitHubClient;
pub use error::TowlGitHubError;
pub use types::CreatedIssue;
