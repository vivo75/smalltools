use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum ZfsBackupError {
    #[error("SSH error: {0}")]
    SshError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("ZFS command failed: {0}")]
    ZfsCommandFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Invalid snapshot state: {0}")]
    InvalidSnapshot(String),

    #[error("Backup failed: {0}")]
    BackupFailed(String),

    #[error("Remote command execution failed: {0}")]
    RemoteCommandFailed(String),
}

#[allow(dead_code)]
pub type ZfsResult<T> = Result<T, ZfsBackupError>;
