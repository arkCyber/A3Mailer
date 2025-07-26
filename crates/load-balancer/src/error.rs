//! Error types for load balancer

use thiserror::Error;

/// Result type for load balancer operations
pub type Result<T> = std::result::Result<T, LoadBalancerError>;

/// Errors that can occur during load balancing
#[derive(Error, Debug)]
pub enum LoadBalancerError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Config error
    #[error("Config error: {0}")]
    Config(String),

    /// Server error
    #[error("Server error: {0}")]
    Server(String),

    /// Proxy error
    #[error("Proxy error: {0}")]
    Proxy(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),

    /// Generic error
    #[error("Load balancer error: {0}")]
    Generic(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}
