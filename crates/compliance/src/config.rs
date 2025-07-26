//! Configuration for compliance

use serde::{Deserialize, Serialize};
use crate::ComplianceFramework;

/// Configuration for compliance system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceConfig {
    /// Enabled compliance frameworks
    pub enabled_frameworks: Vec<ComplianceFramework>,
    /// Maximum violations to keep in history
    pub max_violations_history: usize,
}

impl Default for ComplianceConfig {
    fn default() -> Self {
        Self {
            enabled_frameworks: vec![ComplianceFramework::GDPR],
            max_violations_history: 1000,
        }
    }
}
