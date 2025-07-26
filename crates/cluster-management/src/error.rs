//! Error types for cluster management

use thiserror::Error;

/// Result type for cluster operations
pub type Result<T> = std::result::Result<T, ClusterError>;

/// Errors that can occur during cluster operations
#[derive(Error, Debug)]
pub enum ClusterError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),

    /// Generic error
    #[error("Cluster error: {0}")]
    Generic(String),
}
