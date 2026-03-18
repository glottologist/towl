//! Top-level error type that unifies errors from all towl subsystems.

use crate::{
    config::error::TowlConfigError, github::error::TowlGitHubError, llm::error::TowlLlmError,
    output::error::TowlOutputError, processor::error::TowlProcessorError,
    scanner::error::TowlScannerError, tui::TowlTuiError,
};
use thiserror::Error;

/// Aggregate error type with `From` conversions for each subsystem error.
#[derive(Error, Debug)]
pub enum TowlError {
    #[error("Configuration error: {0}")]
    Config(#[from] TowlConfigError),
    #[error("Scanning error: {0}")]
    Scanner(#[from] TowlScannerError),
    #[error("Output error: {0}")]
    Output(#[from] TowlOutputError),
    #[error("GitHub error: {0}")]
    GitHub(#[from] TowlGitHubError),
    #[error("Processor error: {0}")]
    Processor(#[from] TowlProcessorError),
    #[error("TUI error: {0}")]
    Tui(#[from] TowlTuiError),
    #[error("LLM error: {0}")]
    Llm(#[from] TowlLlmError),
}
