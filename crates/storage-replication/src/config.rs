//! Configuration for storage replication

use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Configuration for storage replication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    /// Replication mode
    pub mode: ReplicationMode,
    
    /// List of replication nodes
    pub nodes: Vec<NodeConfig>,
    
    /// Replication timeout
    pub timeout: Duration,
    
    /// Batch size for replication operations
    pub batch_size: usize,
    
    /// Enable compression
    pub compression: bool,
    
    /// Encryption settings
    pub encryption: EncryptionConfig,
    
    /// Conflict resolution strategy
    pub conflict_resolution: ConflictResolutionStrategy,
    
    /// Health check interval
    pub health_check_interval: Duration,
    
    /// Maximum retry attempts
    pub max_retries: u32,
}

/// Replication mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplicationMode {
    /// Master-slave replication
    MasterSlave,
    /// Multi-master replication
    MultiMaster,
    /// Ring replication
    Ring,
}

/// Node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Node identifier
    pub id: String,
    
    /// Node address
    pub address: String,
    
    /// Node port
    pub port: u16,
    
    /// Node priority (for master election)
    pub priority: u8,
    
    /// Node tags for routing
    pub tags: Vec<String>,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Enable encryption
    pub enabled: bool,
    
    /// Encryption algorithm
    pub algorithm: EncryptionAlgorithm,
    
    /// Key derivation settings
    pub key_derivation: KeyDerivationConfig,
}

/// Encryption algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM
    Aes256Gcm,
    /// ChaCha20-Poly1305
    ChaCha20Poly1305,
}

/// Key derivation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDerivationConfig {
    /// Algorithm for key derivation
    pub algorithm: KeyDerivationAlgorithm,
    
    /// Number of iterations
    pub iterations: u32,
    
    /// Salt length
    pub salt_length: usize,
}

/// Key derivation algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyDerivationAlgorithm {
    /// PBKDF2 with SHA-256
    Pbkdf2Sha256,
    /// Argon2id
    Argon2id,
}

/// Conflict resolution strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolutionStrategy {
    /// Last write wins
    LastWriteWins,
    /// First write wins
    FirstWriteWins,
    /// Manual resolution required
    Manual,
    /// Custom resolution function
    Custom(String),
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            mode: ReplicationMode::MasterSlave,
            nodes: Vec::new(),
            timeout: Duration::from_secs(30),
            batch_size: 1000,
            compression: false,
            encryption: EncryptionConfig::default(),
            conflict_resolution: ConflictResolutionStrategy::LastWriteWins,
            health_check_interval: Duration::from_secs(10),
            max_retries: 3,
        }
    }
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            key_derivation: KeyDerivationConfig::default(),
        }
    }
}

impl Default for KeyDerivationConfig {
    fn default() -> Self {
        Self {
            algorithm: KeyDerivationAlgorithm::Pbkdf2Sha256,
            iterations: 100_000,
            salt_length: 32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ReplicationConfig::default();
        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.max_retries, 3);
        assert!(config.encryption.enabled);
    }

    #[test]
    fn test_config_serialization() {
        let config = ReplicationConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: ReplicationConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.batch_size, deserialized.batch_size);
        assert_eq!(config.max_retries, deserialized.max_retries);
    }
}
