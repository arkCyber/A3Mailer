//! Node module

/// Node placeholder
pub struct Node;

/// Node information
pub struct NodeInfo {
    pub id: String,
    pub address: String,
}

/// Node status
#[derive(Debug, Clone)]
pub enum NodeStatus {
    Active,
    Inactive,
    Joining,
    Leaving,
}
