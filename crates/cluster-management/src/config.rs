//! Cluster Management Configuration
//!
//! This module provides comprehensive configuration for cluster management,
//! including service discovery, health monitoring, leader election, and consensus.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Cluster configuration
///
/// Main configuration structure for cluster management functionality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Unique identifier for this node
    pub node_id: String,
    /// Name of the cluster this node belongs to
    pub cluster_name: String,
    /// Whether cluster management is enabled
    pub enabled: bool,
    /// Service discovery configuration
    pub discovery: ServiceDiscoveryConfig,
    /// Cluster state management configuration
    pub state: ClusterStateConfig,
    /// Health monitoring configuration
    pub health: HealthMonitorConfig,
    /// Leader election configuration
    pub leader_election: LeaderElectionConfig,
    /// Consensus engine configuration
    pub consensus: ConsensusConfig,
}

/// Service discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscoveryConfig {
    /// Service discovery backend type
    pub backend: ServiceDiscoveryBackend,
    /// Refresh interval for service discovery
    pub refresh_interval: Duration,
    /// Service registration TTL
    pub registration_ttl: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
}

/// Service discovery backend types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceDiscoveryBackend {
    /// Consul service discovery
    Consul {
        /// Consul server address
        address: String,
        /// Consul datacenter
        datacenter: Option<String>,
        /// Authentication token
        token: Option<String>,
    },
    /// etcd service discovery
    Etcd {
        /// etcd endpoints
        endpoints: Vec<String>,
        /// Authentication username
        username: Option<String>,
        /// Authentication password
        password: Option<String>,
    },
    /// Kubernetes service discovery
    Kubernetes {
        /// Namespace to watch
        namespace: String,
        /// Service selector labels
        selector: std::collections::HashMap<String, String>,
    },
    /// Static service discovery
    Static {
        /// Static list of nodes
        nodes: Vec<String>,
    },
}

/// Cluster state management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStateConfig {
    /// State synchronization interval
    pub sync_interval: Duration,
    /// State persistence backend
    pub persistence: StatePersistenceConfig,
    /// Maximum number of state history entries
    pub max_history: usize,
    /// State conflict resolution strategy
    pub conflict_resolution: ConflictResolutionStrategy,
}

/// State persistence configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatePersistenceConfig {
    /// In-memory persistence (not recommended for production)
    Memory,
    /// File-based persistence
    File {
        /// File path for state storage
        path: String,
        /// Backup interval
        backup_interval: Duration,
    },
    /// Database persistence
    Database {
        /// Database connection string
        connection_string: String,
        /// Table name for state storage
        table_name: String,
    },
}

/// Conflict resolution strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolutionStrategy {
    /// Last writer wins
    LastWriterWins,
    /// Timestamp-based resolution
    Timestamp,
    /// Vector clock-based resolution
    VectorClock,
    /// Custom resolution function
    Custom(String),
}

/// Health monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMonitorConfig {
    /// Health check interval
    pub check_interval: Duration,
    /// Health check timeout
    pub check_timeout: Duration,
    /// Number of failed checks before marking unhealthy
    pub failure_threshold: u32,
    /// Number of successful checks before marking healthy
    pub success_threshold: u32,
    /// Health check endpoints
    pub endpoints: Vec<HealthCheckEndpoint>,
}

/// Health check endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckEndpoint {
    /// Endpoint name
    pub name: String,
    /// Endpoint URL
    pub url: String,
    /// HTTP method
    pub method: String,
    /// Expected status code
    pub expected_status: u16,
    /// Request timeout
    pub timeout: Duration,
}

/// Leader election configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderElectionConfig {
    /// Leader election backend
    pub backend: LeaderElectionBackend,
    /// Election timeout
    pub election_timeout: Duration,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
    /// Leader lease duration
    pub lease_duration: Duration,
    /// Retry interval for failed elections
    pub retry_interval: Duration,
}

/// Leader election backend types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LeaderElectionBackend {
    /// etcd-based leader election
    Etcd {
        /// etcd endpoints
        endpoints: Vec<String>,
        /// Election key prefix
        key_prefix: String,
    },
    /// Consul-based leader election
    Consul {
        /// Consul address
        address: String,
        /// Session configuration
        session_config: ConsulSessionConfig,
    },
    /// Kubernetes-based leader election
    Kubernetes {
        /// Namespace for leader election
        namespace: String,
        /// ConfigMap name for coordination
        configmap_name: String,
    },
}

/// Consul session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsulSessionConfig {
    /// Session TTL
    pub ttl: Duration,
    /// Session behavior on node failure
    pub behavior: String,
}

/// Consensus engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Whether consensus is enabled
    pub enabled: bool,
    /// Consensus algorithm
    pub algorithm: ConsensusAlgorithm,
    /// Raft-specific configuration
    pub raft: Option<RaftConfig>,
}

/// Consensus algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusAlgorithm {
    /// Raft consensus algorithm
    Raft,
    /// PBFT consensus algorithm
    Pbft,
    /// Custom consensus algorithm
    Custom(String),
}

/// Raft consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftConfig {
    /// Election timeout range
    pub election_timeout: (Duration, Duration),
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
    /// Maximum entries per append
    pub max_append_entries: usize,
    /// Snapshot threshold
    pub snapshot_threshold: u64,
    /// Log compaction interval
    pub compaction_interval: Duration,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            node_id: "node-1".to_string(),
            cluster_name: "a3mailer-cluster".to_string(),
            enabled: false,
            discovery: ServiceDiscoveryConfig::default(),
            state: ClusterStateConfig::default(),
            health: HealthMonitorConfig::default(),
            leader_election: LeaderElectionConfig::default(),
            consensus: ConsensusConfig::default(),
        }
    }
}

impl Default for ServiceDiscoveryConfig {
    fn default() -> Self {
        Self {
            backend: ServiceDiscoveryBackend::Static {
                nodes: vec!["127.0.0.1:8080".to_string()],
            },
            refresh_interval: Duration::from_secs(30),
            registration_ttl: Duration::from_secs(60),
            health_check_interval: Duration::from_secs(10),
        }
    }
}

impl Default for ClusterStateConfig {
    fn default() -> Self {
        Self {
            sync_interval: Duration::from_secs(5),
            persistence: StatePersistenceConfig::Memory,
            max_history: 1000,
            conflict_resolution: ConflictResolutionStrategy::LastWriterWins,
        }
    }
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            check_timeout: Duration::from_secs(5),
            failure_threshold: 3,
            success_threshold: 2,
            endpoints: vec![
                HealthCheckEndpoint {
                    name: "http-health".to_string(),
                    url: "http://localhost:8080/health".to_string(),
                    method: "GET".to_string(),
                    expected_status: 200,
                    timeout: Duration::from_secs(5),
                }
            ],
        }
    }
}

impl Default for LeaderElectionConfig {
    fn default() -> Self {
        Self {
            backend: LeaderElectionBackend::Etcd {
                endpoints: vec!["http://localhost:2379".to_string()],
                key_prefix: "/a3mailer/leader".to_string(),
            },
            election_timeout: Duration::from_secs(10),
            heartbeat_interval: Duration::from_secs(2),
            lease_duration: Duration::from_secs(15),
            retry_interval: Duration::from_secs(5),
        }
    }
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: ConsensusAlgorithm::Raft,
            raft: Some(RaftConfig::default()),
        }
    }
}

impl Default for RaftConfig {
    fn default() -> Self {
        Self {
            election_timeout: (Duration::from_millis(150), Duration::from_millis(300)),
            heartbeat_interval: Duration::from_millis(50),
            max_append_entries: 100,
            snapshot_threshold: 1000,
            compaction_interval: Duration::from_secs(300),
        }
    }
}
