//! Error types for API gateway

use thiserror::Error;

/// Result type for gateway operations
pub type Result<T> = std::result::Result<T, GatewayError>;

/// Errors that can occur during gateway operations
#[derive(Error, Debug)]
pub enum GatewayError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),

    /// Generic error
    #[error("Gateway error: {0}")]
    Generic(String),
}
