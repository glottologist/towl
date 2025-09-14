use crate::config::error::TowlConfigError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TowlError {
    #[error("Configuration error: {0} ")]
    Config(#[from] TowlConfigError),
}
