//! Proxy implementation

use crate::backend::{Backend, BackendPool};
use crate::algorithms::LoadBalancerImpl;
use crate::error::{LoadBalancerError, Result};
use crate::metrics::MetricsCollector;
use std::sync::Arc;
use hyper::{body::Incoming, Request, Response, StatusCode};
use http_body_util::Full;
use bytes::Bytes;

/// Proxy service for handling requests
#[derive(Debug)]
pub struct ProxyService {
    backend_pool: Arc<BackendPool>,
    load_balancer: LoadBalancerImpl,
    metrics: Arc<MetricsCollector>,
}

impl ProxyService {
    /// Create new proxy service
    pub fn new(
        backend_pool: Arc<BackendPool>,
        load_balancer: LoadBalancerImpl,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        Self {
            backend_pool,
            load_balancer,
            metrics,
        }
    }

    /// Handle incoming request
    pub async fn handle_request(&self, req: Request<Incoming>) -> Result<Response<Full<Bytes>>> {
        // Get healthy backends
        let backends = self.backend_pool.get_healthy_backends().await;

        if backends.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(Full::new(Bytes::from("No healthy backends available")))
                .unwrap());
        }

        // Select backend using load balancing algorithm
        let backend = match self.load_balancer.select_backend(&backends).await? {
            Some(backend) => backend,
            None => {
                return Ok(Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(Full::new(Bytes::from("No backend selected")))
                    .unwrap());
            }
        };

        // Increment connection count
        backend.increment_connections().await;
        self.metrics.increment_connections();

        // Forward request to backend
        let result = self.forward_request(req, &backend).await;

        // Decrement connection count
        backend.decrement_connections().await;
        self.metrics.decrement_connections();

        // Record metrics
        let success = result.is_ok();
        self.metrics.record_request_simple(success);

        if !success {
            backend.record_failure().await;
        }

        // Update load balancer state
        let _ = self.load_balancer.update_state(&backend, success).await;

        result
    }

    /// Forward request to backend
    async fn forward_request(&self, _req: Request<Incoming>, backend: &Backend) -> Result<Response<Full<Bytes>>> {
        // TODO: Implement actual HTTP forwarding using hyper client
        // For now, return a simple response
        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::from(format!("Forwarded to backend: {}", backend.url()))))
            .map_err(|e| LoadBalancerError::Internal(format!("Failed to build response: {}", e)))?)
    }
}
