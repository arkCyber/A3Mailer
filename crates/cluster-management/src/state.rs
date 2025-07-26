//! State module

/// State placeholder
pub struct State;

/// Cluster state
#[derive(Debug, Clone)]
pub struct ClusterState {
    pub nodes: Vec<crate::NodeInfo>,
    pub leader: Option<String>,
}

/// Cluster state manager
pub struct ClusterStateManager;
