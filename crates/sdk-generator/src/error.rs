//! Error types for SDK generator

use thiserror::Error;

/// Result type for generator operations
pub type Result<T> = std::result::Result<T, GeneratorError>;

/// Errors that can occur during SDK generation
#[derive(Error, Debug)]
pub enum GeneratorError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Template error
    #[error("Template error: {0}")]
    Template(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Generic error
    #[error("Generator error: {0}")]
    Generic(String),
}
