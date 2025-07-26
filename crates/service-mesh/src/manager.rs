//! Service mesh manager

use crate::{ServiceMeshConfig, error::Result};

/// Service mesh manager
pub struct ServiceMeshManager {
    config: ServiceMeshConfig,
}

impl ServiceMeshManager {
    /// Create new service mesh manager
    pub async fn new(config: ServiceMeshConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// Start service mesh
    pub async fn start(&self) -> Result<()> {
        Ok(())
    }
}
