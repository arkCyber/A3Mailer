//! Cluster configuration

use serde::{Deserialize, Serialize};

/// Cluster configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub node_id: String,
    pub cluster_name: String,
    pub enabled: bool,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            node_id: "node-1".to_string(),
            cluster_name: "stalwart-cluster".to_string(),
            enabled: false,
        }
    }
}
