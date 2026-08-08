use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ZfsBackupError {
    #[error("Configuration error: {0}")]
    ConfigError(String),
}
