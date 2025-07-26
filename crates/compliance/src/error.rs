//! Error types for compliance

use thiserror::Error;

/// Result type for compliance operations
pub type Result<T> = std::result::Result<T, ComplianceError>;

/// Errors that can occur during compliance operations
#[derive(Error, Debug)]
pub enum ComplianceError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Generic error
    #[error("Compliance error: {0}")]
    Generic(String),
}
