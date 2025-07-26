//! SDK generator configuration

use serde::{Deserialize, Serialize};

/// Generator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorConfig {
    pub output_dir: String,
    pub template_dir: Option<String>,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            output_dir: "./generated".to_string(),
            template_dir: None,
        }
    }
}
