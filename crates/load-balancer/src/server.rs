//! Server implementation

use crate::{config::ServerConfig, error::Result, proxy::ProxyService};
use hyper::{body::Incoming, Request, Response};
use hyper_util::rt::TokioIo;
use hyper_util::server::conn::auto::Builder;
use tower::Service;
use http_body_util::Full;
use bytes::Bytes;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Load balancer server
#[derive(Debug)]
pub struct LoadBalancerServer {
    config: ServerConfig,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl LoadBalancerServer {
    /// Create new load balancer server
    pub async fn new(config: &ServerConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
            shutdown_tx: None,
        })
    }

    /// Start the server
    pub async fn start(&mut self, _proxy_service: Arc<ProxyService>) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.config.listen_address, self.config.listen_port)
            .parse()
            .map_err(|e| crate::error::LoadBalancerError::Config(format!("Invalid address: {}", e)))?;

        let (shutdown_tx, _shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        // TODO: Implement actual server with hyper 1.0 API
        // For now, just bind to the address to verify it works
        let _listener = tokio::net::TcpListener::bind(&addr).await
            .map_err(|e| crate::error::LoadBalancerError::Server(format!("Failed to bind: {}", e)))?;

        tracing::info!("Load balancer server listening on {}", addr);

        // TODO: Accept connections and handle requests
        // This is a placeholder implementation

        Ok(())
    }

    /// Stop the server
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        Ok(())
    }
}
