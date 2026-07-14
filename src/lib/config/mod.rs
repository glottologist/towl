//! Configuration loading, validation, and initialisation.
//!
//! Configuration is read from a `.towl.toml` file (see [`DEFAULT_CONFIG_PATH`]) and
//! can be overridden by environment variables (`TOWL_CONFIG`, `TOWL_GITHUB_TOKEN`,
//! `TOWL_GITHUB_OWNER`, `TOWL_GITHUB_REPO`, `TOWL_LLM_API_KEY`, `TOWL_LLM_PROVIDER`,
//! `TOWL_LLM_MODEL`, `TOWL_LLM_BASE_URL`).

pub(crate) mod defaults;
mod display;
pub mod error;
pub mod git;
mod newtypes;
mod types;
mod validation;

pub(crate) use newtypes::MAX_CONFIG_STRING_LENGTH;
pub use newtypes::{Owner, Repo};
pub use types::{GitHubConfig, LlmConfig, ParsingConfig, TowlConfig, DEFAULT_CONFIG_PATH};

#[cfg(test)]
pub use types::test_parsing_config;
