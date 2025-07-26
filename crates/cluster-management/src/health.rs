//! Health module

/// Health placeholder
pub struct Health;

/// Health monitor
pub struct HealthMonitor;

/// Node health
#[derive(Debug, Clone)]
pub struct NodeHealth {
    pub node_id: String,
    pub is_healthy: bool,
    pub last_check: std::time::Instant,
}
