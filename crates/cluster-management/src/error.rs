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

    /// Configuration error (alias for compatibility)
    #[error("Config error: {0}")]
    Config(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Health check error
    #[error("Health check error: {0}")]
    Health(String),

    /// Leadership error
    #[error("Not a leader")]
    NotLeader,

    /// Generic error
    #[error("Cluster error: {0}")]
    Generic(String),
}
