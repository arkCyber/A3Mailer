//! Error types for storage replication

use thiserror::Error;

/// Result type for replication operations
pub type Result<T> = std::result::Result<T, ReplicationError>;

/// Errors that can occur during replication
#[derive(Error, Debug)]
pub enum ReplicationError {
    /// Network communication error
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Timeout error
    #[error("Operation timed out")]
    Timeout,

    /// Node not found
    #[error("Node not found: {node_id}")]
    NodeNotFound { node_id: String },

    /// Node unreachable
    #[error("Node unreachable: {node_id}")]
    NodeUnreachable { node_id: String },

    /// Conflict resolution error
    #[error("Conflict resolution failed: {reason}")]
    ConflictResolution { reason: String },

    /// Authentication error
    #[error("Authentication failed")]
    Authentication,

    /// Encryption error
    #[error("Encryption error: {0}")]
    Encryption(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    /// Replication lag too high
    #[error("Replication lag too high: {lag_ms}ms")]
    HighLag { lag_ms: u64 },

    /// Split brain scenario detected
    #[error("Split brain scenario detected")]
    SplitBrain,

    /// Quorum not available
    #[error("Quorum not available: {available}/{required}")]
    NoQuorum { available: usize, required: usize },

    /// Version mismatch
    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },

    /// Generic error
    #[error("Replication error: {0}")]
    Generic(String),
}

impl ReplicationError {
    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            ReplicationError::Network(_) => true,
            ReplicationError::Timeout => true,
            ReplicationError::NodeUnreachable { .. } => true,
            ReplicationError::NoQuorum { .. } => true,
            ReplicationError::HighLag { .. } => true,
            _ => false,
        }
    }

    /// Check if the error is critical (requires immediate attention)
    pub fn is_critical(&self) -> bool {
        match self {
            ReplicationError::SplitBrain => true,
            ReplicationError::Authentication => true,
            ReplicationError::Encryption(_) => true,
            ReplicationError::VersionMismatch { .. } => true,
            _ => false,
        }
    }

    /// Get error category for metrics
    pub fn category(&self) -> &'static str {
        match self {
            ReplicationError::Network(_) => "network",
            ReplicationError::Serialization(_) => "serialization",
            ReplicationError::Timeout => "timeout",
            ReplicationError::NodeNotFound { .. } => "node_not_found",
            ReplicationError::NodeUnreachable { .. } => "node_unreachable",
            ReplicationError::ConflictResolution { .. } => "conflict_resolution",
            ReplicationError::Authentication => "authentication",
            ReplicationError::Encryption(_) => "encryption",
            ReplicationError::Configuration(_) => "configuration",
            ReplicationError::Storage(_) => "storage",
            ReplicationError::InvalidOperation(_) => "invalid_operation",
            ReplicationError::HighLag { .. } => "high_lag",
            ReplicationError::SplitBrain => "split_brain",
            ReplicationError::NoQuorum { .. } => "no_quorum",
            ReplicationError::VersionMismatch { .. } => "version_mismatch",
            ReplicationError::Generic(_) => "generic",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_retryable() {
        assert!(ReplicationError::Timeout.is_retryable());
        assert!(!ReplicationError::Authentication.is_retryable());
    }

    #[test]
    fn test_error_critical() {
        assert!(ReplicationError::SplitBrain.is_critical());
        assert!(!ReplicationError::Timeout.is_critical());
    }

    #[test]
    fn test_error_category() {
        assert_eq!(ReplicationError::Timeout.category(), "timeout");
        assert_eq!(ReplicationError::SplitBrain.category(), "split_brain");
    }
}
