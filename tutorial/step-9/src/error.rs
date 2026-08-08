use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)] // ConfigError rispecchia lo schema reale ma non viene mai costruita in questa demo
pub enum ZfsBackupError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Invalid skip_filesystems pattern '{pattern}': {source}")]
    InvalidPattern {
        pattern: String,
        #[source]
        source: regex::Error,
    },
}
