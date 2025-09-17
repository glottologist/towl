use crate::{
    config::error::TowlConfigError, output::error::TowlOutputError,
    scanner::error::TowlScannerError,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TowlError {
    #[error("Configuration error: {0} ")]
    Config(#[from] TowlConfigError),
    #[error("Scanning error: {0} ")]
    Scanner(#[from] TowlScannerError),
    #[error("Output error: {0} ")]
    Output(#[from] TowlOutputError),
    #[error("Generic error: {0}")]
    Generic(String),
}
