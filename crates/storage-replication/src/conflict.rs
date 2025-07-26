//! Conflict resolution for replication

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::{error::Result, config::ConflictResolutionStrategy};

/// Conflict resolution trait
#[async_trait]
pub trait ConflictResolver: Send + Sync {
    /// Resolve a conflict between two values
    async fn resolve_conflict(&self, conflict: DataConflict) -> Result<ConflictResolution>;
}

/// Data conflict information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConflict {
    pub key: Vec<u8>,
    pub local_value: Option<Vec<u8>>,
    pub remote_value: Option<Vec<u8>>,
    pub local_timestamp: chrono::DateTime<chrono::Utc>,
    pub remote_timestamp: chrono::DateTime<chrono::Utc>,
    pub local_node_id: String,
    pub remote_node_id: String,
}

/// Conflict resolution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Use local value
    UseLocal,
    /// Use remote value
    UseRemote,
    /// Use merged value
    UseMerged(Vec<u8>),
    /// Require manual intervention
    RequireManual,
}

/// Default conflict resolver implementation
pub struct DefaultConflictResolver {
    strategy: ConflictResolutionStrategy,
}

impl DefaultConflictResolver {
    /// Create a new default conflict resolver
    pub fn new(strategy: ConflictResolutionStrategy) -> Self {
        Self { strategy }
    }
}

#[async_trait]
impl ConflictResolver for DefaultConflictResolver {
    async fn resolve_conflict(&self, conflict: DataConflict) -> Result<ConflictResolution> {
        match &self.strategy {
            ConflictResolutionStrategy::LastWriteWins => {
                if conflict.local_timestamp > conflict.remote_timestamp {
                    Ok(ConflictResolution::UseLocal)
                } else {
                    Ok(ConflictResolution::UseRemote)
                }
            }
            ConflictResolutionStrategy::FirstWriteWins => {
                if conflict.local_timestamp < conflict.remote_timestamp {
                    Ok(ConflictResolution::UseLocal)
                } else {
                    Ok(ConflictResolution::UseRemote)
                }
            }
            ConflictResolutionStrategy::Manual => {
                Ok(ConflictResolution::RequireManual)
            }
            ConflictResolutionStrategy::Custom(_) => {
                // TODO: Implement custom conflict resolution
                todo!("Custom conflict resolution implementation")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_last_write_wins() {
        let resolver = DefaultConflictResolver::new(ConflictResolutionStrategy::LastWriteWins);
        
        let now = Utc::now();
        let earlier = now - chrono::Duration::seconds(10);
        
        let conflict = DataConflict {
            key: b"test_key".to_vec(),
            local_value: Some(b"local_value".to_vec()),
            remote_value: Some(b"remote_value".to_vec()),
            local_timestamp: now,
            remote_timestamp: earlier,
            local_node_id: "node1".to_string(),
            remote_node_id: "node2".to_string(),
        };
        
        let resolution = resolver.resolve_conflict(conflict).await.unwrap();
        assert!(matches!(resolution, ConflictResolution::UseLocal));
    }

    #[tokio::test]
    async fn test_first_write_wins() {
        let resolver = DefaultConflictResolver::new(ConflictResolutionStrategy::FirstWriteWins);
        
        let now = Utc::now();
        let earlier = now - chrono::Duration::seconds(10);
        
        let conflict = DataConflict {
            key: b"test_key".to_vec(),
            local_value: Some(b"local_value".to_vec()),
            remote_value: Some(b"remote_value".to_vec()),
            local_timestamp: now,
            remote_timestamp: earlier,
            local_node_id: "node1".to_string(),
            remote_node_id: "node2".to_string(),
        };
        
        let resolution = resolver.resolve_conflict(conflict).await.unwrap();
        assert!(matches!(resolution, ConflictResolution::UseRemote));
    }
}
