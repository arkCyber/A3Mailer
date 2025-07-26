/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Error types for backup and restore operations

use thiserror::Error;

/// Result type for backup operations
pub type Result<T> = std::result::Result<T, BackupError>;

/// Backup-specific errors
#[derive(Error, Debug)]
pub enum BackupError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Compression error: {0}")]
    CompressionError(String),
    
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Backup not found: {0}")]
    BackupNotFound(String),
    
    #[error("Invalid backup format: {0}")]
    InvalidFormat(String),
    
    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },
    
    #[error("Backup operation cancelled")]
    Cancelled,
    
    #[error("Insufficient storage space: need {needed} bytes, have {available} bytes")]
    InsufficientSpace { needed: u64, available: u64 },
}

/// Restore-specific errors
#[derive(Error, Debug)]
pub enum RestoreError {
    #[error("Backup error: {0}")]
    BackupError(#[from] BackupError),
    
    #[error("Restore target not found: {0}")]
    TargetNotFound(String),
    
    #[error("Restore target not writable: {0}")]
    TargetNotWritable(String),
    
    #[error("Restore operation cancelled")]
    Cancelled,
    
    #[error("Partial restore failure: {successful} successful, {failed} failed")]
    PartialFailure { successful: usize, failed: usize },
}

impl From<RestoreError> for BackupError {
    fn from(err: RestoreError) -> Self {
        match err {
            RestoreError::BackupError(backup_err) => backup_err,
            _ => BackupError::StorageError(err.to_string()),
        }
    }
}
