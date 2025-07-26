//! Error types for threat detection

use thiserror::Error;

/// Result type for threat detection operations
pub type Result<T> = std::result::Result<T, ThreatDetectionError>;

/// Errors that can occur during threat detection
#[derive(Error, Debug)]
pub enum ThreatDetectionError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Config error
    #[error("Config error: {0}")]
    Config(String),

    /// Model loading error
    #[error("Model loading error: {0}")]
    ModelLoading(String),

    /// Analysis error
    #[error("Analysis error: {0}")]
    Analysis(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Generic error
    #[error("Threat detection error: {0}")]
    Generic(String),
}
