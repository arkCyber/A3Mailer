//! Node Management Module
//!
//! This module provides node information and management functionality
//! for cluster operations.

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// Node placeholder for future implementation
#[derive(Debug, Clone)]
pub struct Node;

/// Node information structure
///
/// Contains comprehensive information about a cluster node,
/// including its identity, network details, and operational status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeInfo {
    /// Unique node identifier
    pub id: String,
    /// Node network address (IP:port)
    pub address: String,
    /// Node hostname
    pub hostname: String,
    /// Node version information
    pub version: String,
    /// Node capabilities and features
    pub capabilities: Vec<String>,
    /// Node metadata (labels, annotations, etc.)
    pub metadata: std::collections::HashMap<String, String>,
    /// Node status
    pub status: NodeStatus,
    /// Last heartbeat timestamp
    pub last_heartbeat: SystemTime,
    /// Node startup time
    pub startup_time: SystemTime,
    /// Node resource information
    pub resources: NodeResources,
}

/// Node resource information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeResources {
    /// CPU cores available
    pub cpu_cores: u32,
    /// Memory in bytes
    pub memory_bytes: u64,
    /// Disk space in bytes
    pub disk_bytes: u64,
    /// Network bandwidth in bytes per second
    pub network_bandwidth: u64,
    /// Current CPU usage percentage (0-100)
    pub cpu_usage: f64,
    /// Current memory usage percentage (0-100)
    pub memory_usage: f64,
    /// Current disk usage percentage (0-100)
    pub disk_usage: f64,
}

/// Node status enumeration
///
/// Represents the current operational status of a cluster node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    /// Node is active and healthy
    Active,
    /// Node is inactive or unreachable
    Inactive,
    /// Node is in the process of joining the cluster
    Joining,
    /// Node is in the process of leaving the cluster
    Leaving,
    /// Node is in maintenance mode
    Maintenance,
    /// Node has failed and needs attention
    Failed,
}

impl NodeInfo {
    /// Create a new NodeInfo instance
    ///
    /// # Arguments
    /// * `node_config` - Node configuration (placeholder for now)
    ///
    /// # Returns
    /// A new NodeInfo instance with default values
    pub async fn new(_node_config: &str) -> crate::Result<Self> {
        let hostname = gethostname::gethostname()
            .to_string_lossy()
            .to_string();

        let now = SystemTime::now();

        Ok(Self {
            id: format!("node-{}", uuid::Uuid::new_v4()),
            address: "127.0.0.1:8080".to_string(),
            hostname,
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: vec![
                "email".to_string(),
                "clustering".to_string(),
                "load-balancing".to_string(),
            ],
            metadata: std::collections::HashMap::new(),
            status: NodeStatus::Joining,
            last_heartbeat: now,
            startup_time: now,
            resources: NodeResources::default(),
        })
    }

    /// Update node heartbeat
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = SystemTime::now();
    }

    /// Check if node is healthy based on last heartbeat
    pub fn is_healthy(&self, timeout: std::time::Duration) -> bool {
        if let Ok(elapsed) = self.last_heartbeat.elapsed() {
            elapsed < timeout
        } else {
            false
        }
    }

    /// Update node status
    pub fn update_status(&mut self, status: NodeStatus) {
        self.status = status;
        self.update_heartbeat();
    }

    /// Get node uptime
    pub fn uptime(&self) -> Option<std::time::Duration> {
        self.startup_time.elapsed().ok()
    }
}

impl Default for NodeResources {
    fn default() -> Self {
        Self {
            cpu_cores: num_cpus::get() as u32,
            memory_bytes: 8 * 1024 * 1024 * 1024, // 8GB default
            disk_bytes: 100 * 1024 * 1024 * 1024, // 100GB default
            network_bandwidth: 1_000_000_000, // 1Gbps default
            cpu_usage: 0.0,
            memory_usage: 0.0,
            disk_usage: 0.0,
        }
    }
}

impl NodeResources {
    /// Update resource usage statistics
    pub fn update_usage(&mut self, cpu: f64, memory: f64, disk: f64) {
        self.cpu_usage = cpu.clamp(0.0, 100.0);
        self.memory_usage = memory.clamp(0.0, 100.0);
        self.disk_usage = disk.clamp(0.0, 100.0);
    }

    /// Check if resources are under pressure
    pub fn is_under_pressure(&self) -> bool {
        self.cpu_usage > 80.0 || self.memory_usage > 80.0 || self.disk_usage > 90.0
    }

    /// Get resource utilization score (0-100)
    pub fn utilization_score(&self) -> f64 {
        (self.cpu_usage + self.memory_usage + self.disk_usage) / 3.0
    }
}
