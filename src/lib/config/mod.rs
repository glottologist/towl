//! Configuration loading, validation, and initialisation.
//!
//! Configuration is read from a `.towl.toml` file (see [`DEFAULT_CONFIG_PATH`]) and
//! can be overridden by environment variables (`TOWL_GITHUB_TOKEN`, `TOWL_GITHUB_OWNER`,
//! `TOWL_GITHUB_REPO`).

mod defaults;
mod display;
pub mod error;
pub mod git;
mod newtypes;
mod types;
mod validation;

pub use newtypes::{Owner, Repo};
pub use types::{GitHubConfig, ParsingConfig, TowlConfig, DEFAULT_CONFIG_PATH};

#[cfg(test)]
pub use types::test_parsing_config;
