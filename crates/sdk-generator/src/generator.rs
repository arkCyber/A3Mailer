//! SDK generator implementation

use crate::{GeneratorConfig, error::Result};

/// SDK generator
pub struct SdkGenerator {
    config: GeneratorConfig,
}

impl SdkGenerator {
    /// Create new SDK generator
    pub async fn new(config: GeneratorConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// Generate SDK
    pub async fn generate_sdk(&self, _language: crate::Language, _output_dir: &str) -> Result<()> {
        // TODO: Implement SDK generation
        Ok(())
    }
}
