//! Error types for Kubernetes operator

use thiserror::Error;

/// Result type for operator operations
pub type Result<T> = std::result::Result<T, OperatorError>;

/// Errors that can occur during operator operations
#[derive(Error, Debug)]
pub enum OperatorError {
    /// Kubernetes client error
    #[error("Kubernetes client error: {0}")]
    KubernetesClient(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Generic error
    #[error("Operator error: {0}")]
    Generic(String),
}
