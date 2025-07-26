//! Replication node implementation

use std::fmt;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Node identifier
pub type NodeId = String;

/// Replication node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationNode {
    id: NodeId,
    address: String,
    port: u16,
    status: NodeStatus,
    last_seen: DateTime<Utc>,
    priority: u8,
    tags: Vec<String>,
    version: String,
}

/// Node status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Node is healthy and reachable
    Healthy,
    /// Node is unreachable
    Unreachable,
    /// Node is degraded (high latency, errors)
    Degraded,
    /// Node is being synchronized
    Syncing,
    /// Node is offline
    Offline,
}

impl ReplicationNode {
    /// Create a new replication node
    pub fn new(
        id: NodeId,
        address: String,
        port: u16,
        priority: u8,
        tags: Vec<String>,
    ) -> Self {
        Self {
            id,
            address,
            port,
            status: NodeStatus::Offline,
            last_seen: Utc::now(),
            priority,
            tags,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Get node ID
    pub fn id(&self) -> &NodeId {
        &self.id
    }

    /// Get node address
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Get node port
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get node status
    pub fn status(&self) -> &NodeStatus {
        &self.status
    }

    /// Set node status
    pub fn set_status(&mut self, status: NodeStatus) {
        self.status = status;
        self.last_seen = Utc::now();
    }

    /// Get last seen timestamp
    pub fn last_seen(&self) -> DateTime<Utc> {
        self.last_seen
    }

    /// Update last seen timestamp
    pub fn update_last_seen(&mut self) {
        self.last_seen = Utc::now();
    }

    /// Get node priority
    pub fn priority(&self) -> u8 {
        self.priority
    }

    /// Get node tags
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Get node version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Check if node is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self.status, NodeStatus::Healthy)
    }

    /// Check if node is reachable
    pub fn is_reachable(&self) -> bool {
        !matches!(self.status, NodeStatus::Unreachable | NodeStatus::Offline)
    }

    /// Get full address (address:port)
    pub fn full_address(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }

    /// Check if node has tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(&tag.to_string())
    }
}

impl fmt::Display for ReplicationNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Node(id={}, address={}:{}, status={:?}, priority={})",
            self.id, self.address, self.port, self.status, self.priority
        )
    }
}

impl fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeStatus::Healthy => write!(f, "healthy"),
            NodeStatus::Unreachable => write!(f, "unreachable"),
            NodeStatus::Degraded => write!(f, "degraded"),
            NodeStatus::Syncing => write!(f, "syncing"),
            NodeStatus::Offline => write!(f, "offline"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = ReplicationNode::new(
            "node1".to_string(),
            "127.0.0.1".to_string(),
            8080,
            1,
            vec!["primary".to_string()],
        );

        assert_eq!(node.id(), "node1");
        assert_eq!(node.address(), "127.0.0.1");
        assert_eq!(node.port(), 8080);
        assert_eq!(node.priority(), 1);
        assert!(node.has_tag("primary"));
        assert!(!node.is_healthy());
        assert!(!node.is_reachable());
    }

    #[test]
    fn test_node_status_update() {
        let mut node = ReplicationNode::new(
            "node1".to_string(),
            "127.0.0.1".to_string(),
            8080,
            1,
            vec![],
        );

        node.set_status(NodeStatus::Healthy);
        assert!(node.is_healthy());
        assert!(node.is_reachable());
    }

    #[test]
    fn test_full_address() {
        let node = ReplicationNode::new(
            "node1".to_string(),
            "127.0.0.1".to_string(),
            8080,
            1,
            vec![],
        );

        assert_eq!(node.full_address(), "127.0.0.1:8080");
    }
}
