//! Error types for service mesh

use thiserror::Error;

/// Result type for service mesh operations
pub type Result<T> = std::result::Result<T, ServiceMeshError>;

/// Errors that can occur during service mesh operations
#[derive(Error, Debug)]
pub enum ServiceMeshError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Generic error
    #[error("Service mesh error: {0}")]
    Generic(String),
}
