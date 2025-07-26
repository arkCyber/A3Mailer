//! API gateway implementation

use crate::{GatewayConfig, error::Result};

/// API gateway
pub struct ApiGateway {
    config: GatewayConfig,
}

impl ApiGateway {
    /// Create new API gateway
    pub async fn new(config: GatewayConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// Start gateway
    pub async fn start(&self) -> Result<()> {
        // TODO: Implement gateway startup
        Ok(())
    }
}
