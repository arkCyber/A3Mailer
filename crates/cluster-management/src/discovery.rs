//! Discovery module

use crate::error::Result;

/// Discovery placeholder
pub struct Discovery;

/// Service discovery trait
pub trait ServiceDiscovery {
    /// Discover services
    async fn discover(&self) -> Result<Vec<String>>;
}

/// Discovery backend
pub struct DiscoveryBackend;

/// Create discovery backend
pub async fn create_backend(_config: &crate::ClusterConfig) -> Result<Discovery> {
    Ok(Discovery)
}
