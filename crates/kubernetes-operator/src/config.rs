//! Configuration for Kubernetes operator

use serde::{Deserialize, Serialize};

/// Configuration for Kubernetes operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorConfig {
    /// Kubernetes namespace
    pub namespace: String,
    /// Operator name
    pub name: String,
}

impl Default for OperatorConfig {
    fn default() -> Self {
        Self {
            namespace: "default".to_string(),
            name: "stalwart-operator".to_string(),
        }
    }
}
