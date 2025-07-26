//! Service mesh configuration

use serde::{Deserialize, Serialize};

/// Service mesh configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMeshConfig {
    pub enabled: bool,
    pub mesh_type: String,
}

impl Default for ServiceMeshConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mesh_type: "istio".to_string(),
        }
    }
}
