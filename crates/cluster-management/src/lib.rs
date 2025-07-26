/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! # Cluster Management
//!
//! Distributed cluster management and coordination for Stalwart Mail Server with:
//!
//! - Node discovery and registration
//! - Cluster state synchronization
//! - Leader election and consensus
//! - Health monitoring and failure detection
//! - Load distribution and rebalancing
//! - Configuration propagation

pub mod config;
pub mod consensus;
pub mod discovery;
pub mod error;
pub mod health;
pub mod leader;
pub mod metrics;
pub mod node;
pub mod state;
pub mod sync;

pub use config::ClusterConfig;
pub use consensus::{ConsensusEngine, ConsensusState};
pub use discovery::{ServiceDiscovery, DiscoveryBackend};
pub use error::{ClusterError, Result};
pub use health::{HealthMonitor, NodeHealth};
pub use leader::{LeaderElection, LeadershipState};
pub use metrics::ClusterMetrics;
pub use node::{Node, NodeInfo, NodeStatus};
pub use state::{ClusterState, ClusterStateManager};

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use uuid::Uuid;

/// Main cluster management service
#[derive(Debug, Clone)]
pub struct ClusterManager {
    inner: Arc<ClusterManagerInner>,
}

#[derive(Debug)]
struct ClusterManagerInner {
    config: ClusterConfig,
    node_info: NodeInfo,
    cluster_state: ClusterStateManager,
    service_discovery: Box<dyn ServiceDiscovery>,
    health_monitor: HealthMonitor,
    leader_election: LeaderElection,
    consensus_engine: Option<ConsensusEngine>,
    metrics: Arc<RwLock<ClusterMetrics>>,
}

impl ClusterManager {
    /// Create a new cluster manager
    pub async fn new(config: ClusterConfig) -> Result<Self> {
        info!("Initializing cluster manager");

        // Create node information
        let node_info = NodeInfo::new(&config.node).await?;

        // Create cluster state manager
        let cluster_state = ClusterStateManager::new(&config.state).await?;

        // Initialize service discovery
        let service_discovery = discovery::create_backend(&config.discovery).await?;

        // Create health monitor
        let health_monitor = HealthMonitor::new(&config.health).await?;

        // Create leader election
        let leader_election = LeaderElection::new(&config.leader_election, node_info.clone()).await?;

        // Create consensus engine if enabled
        let consensus_engine = if config.consensus.enabled {
            Some(ConsensusEngine::new(&config.consensus, node_info.clone()).await?)
        } else {
            None
        };

        // Create metrics collector
        let metrics = Arc::new(RwLock::new(ClusterMetrics::new()));

        Ok(Self {
            inner: Arc::new(ClusterManagerInner {
                config,
                node_info,
                cluster_state,
                service_discovery,
                health_monitor,
                leader_election,
                consensus_engine,
                metrics,
            }),
        })
    }

    /// Start the cluster manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting cluster manager for node: {}", self.inner.node_info.id);

        // Register this node with service discovery
        self.inner.service_discovery.register_node(&self.inner.node_info).await?;

        // Start health monitoring
        self.inner.health_monitor.start().await?;

        // Start leader election
        self.inner.leader_election.start().await?;

        // Start consensus engine if enabled
        if let Some(ref consensus) = self.inner.consensus_engine {
            consensus.start().await?;
        }

        // Start cluster state synchronization
        self.inner.cluster_state.start().await?;

        // Start background tasks
        self.start_background_tasks().await;

        info!("Cluster manager started successfully");
        Ok(())
    }

    /// Stop the cluster manager
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping cluster manager");

        // Stop consensus engine
        if let Some(ref consensus) = self.inner.consensus_engine {
            consensus.stop().await?;
        }

        // Stop leader election
        self.inner.leader_election.stop().await?;

        // Stop health monitoring
        self.inner.health_monitor.stop().await?;

        // Stop cluster state manager
        self.inner.cluster_state.stop().await?;

        // Deregister from service discovery
        self.inner.service_discovery.deregister_node(&self.inner.node_info.id).await?;

        info!("Cluster manager stopped");
        Ok(())
    }

    /// Get current cluster state
    pub async fn get_cluster_state(&self) -> ClusterState {
        self.inner.cluster_state.get_state().await
    }

    /// Get all nodes in the cluster
    pub async fn get_nodes(&self) -> Result<Vec<NodeInfo>> {
        self.inner.service_discovery.discover_nodes().await
    }

    /// Get current node information
    pub fn get_node_info(&self) -> &NodeInfo {
        &self.inner.node_info
    }

    /// Check if this node is the cluster leader
    pub async fn is_leader(&self) -> bool {
        self.inner.leader_election.is_leader().await
    }

    /// Get current leader information
    pub async fn get_leader(&self) -> Option<NodeInfo> {
        self.inner.leader_election.get_leader().await
    }

    /// Get cluster metrics
    pub async fn get_metrics(&self) -> ClusterMetrics {
        self.inner.metrics.read().await.clone()
    }

    /// Add a new node to the cluster
    pub async fn add_node(&self, node: NodeInfo) -> Result<()> {
        info!("Adding node to cluster: {}", node.id);
        
        // Register with service discovery
        self.inner.service_discovery.register_node(&node).await?;
        
        // Update cluster state
        self.inner.cluster_state.add_node(node).await?;
        
        Ok(())
    }

    /// Remove a node from the cluster
    pub async fn remove_node(&self, node_id: &Uuid) -> Result<bool> {
        info!("Removing node from cluster: {}", node_id);
        
        // Deregister from service discovery
        self.inner.service_discovery.deregister_node(node_id).await?;
        
        // Update cluster state
        let removed = self.inner.cluster_state.remove_node(node_id).await?;
        
        if removed {
            info!("Node removed successfully: {}", node_id);
        } else {
            warn!("Node not found for removal: {}", node_id);
        }
        
        Ok(removed)
    }

    /// Get health status of all nodes
    pub async fn get_node_health(&self) -> Result<Vec<(NodeInfo, NodeHealth)>> {
        self.inner.health_monitor.get_all_health().await
    }

    /// Trigger cluster rebalancing
    pub async fn rebalance_cluster(&self) -> Result<()> {
        info!("Triggering cluster rebalancing");
        
        if !self.is_leader().await {
            return Err(ClusterError::NotLeader);
        }
        
        // TODO: Implement rebalancing logic
        
        Ok(())
    }

    /// Propagate configuration to all nodes
    pub async fn propagate_config(&self, config_data: Vec<u8>) -> Result<()> {
        info!("Propagating configuration to cluster");
        
        if !self.is_leader().await {
            return Err(ClusterError::NotLeader);
        }
        
        // TODO: Implement configuration propagation
        
        Ok(())
    }

    /// Start background tasks
    async fn start_background_tasks(&self) {
        // Start metrics collection
        self.start_metrics_collection().await;
        
        // Start node discovery refresh
        self.start_discovery_refresh().await;
        
        // Start cluster state sync
        self.start_state_sync().await;
    }

    /// Start metrics collection background task
    async fn start_metrics_collection(&self) {
        let metrics = self.inner.metrics.clone();
        let cluster_state = self.inner.cluster_state.clone();
        let health_monitor = self.inner.health_monitor.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Collect cluster statistics
                let state = cluster_state.get_state().await;
                let health_stats = health_monitor.get_statistics().await;
                
                // Update metrics
                {
                    let mut metrics_guard = metrics.write().await;
                    metrics_guard.update_cluster_stats(&state);
                    metrics_guard.update_health_stats(&health_stats);
                }
            }
        });
    }

    /// Start service discovery refresh task
    async fn start_discovery_refresh(&self) {
        let service_discovery = self.inner.service_discovery.clone();
        let cluster_state = self.inner.cluster_state.clone();
        let refresh_interval = self.inner.config.discovery.refresh_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(refresh_interval);
            
            loop {
                interval.tick().await;
                
                // Discover nodes
                match service_discovery.discover_nodes().await {
                    Ok(nodes) => {
                        // Update cluster state with discovered nodes
                        if let Err(e) = cluster_state.update_discovered_nodes(nodes).await {
                            error!("Failed to update discovered nodes: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Node discovery failed: {}", e);
                    }
                }
            }
        });
    }

    /// Start cluster state synchronization task
    async fn start_state_sync(&self) {
        let cluster_state = self.inner.cluster_state.clone();
        let sync_interval = self.inner.config.state.sync_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(sync_interval);
            
            loop {
                interval.tick().await;
                
                // Synchronize cluster state
                if let Err(e) = cluster_state.sync_state().await {
                    error!("Cluster state sync failed: {}", e);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cluster_manager_creation() {
        let config = ClusterConfig::default();
        let manager = ClusterManager::new(config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_node_management() {
        let config = ClusterConfig::default();
        let manager = ClusterManager::new(config).await.unwrap();

        // Get initial node info
        let node_info = manager.get_node_info().clone();
        assert!(!node_info.id.is_nil());

        // Test cluster state
        let state = manager.get_cluster_state().await;
        assert!(state.nodes.is_empty()); // Initially empty
    }
}
