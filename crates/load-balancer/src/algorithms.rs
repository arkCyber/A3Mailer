//! Load balancing algorithms

use crate::backend::Backend;
use crate::error::{LoadBalancerError, Result};
use async_trait::async_trait;
use std::sync::Arc;

/// Load balancing algorithm types
#[derive(Debug, Clone)]
pub enum LoadBalancingAlgorithm {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    IpHash,
    Random,
}

/// Load balancer trait for selecting backends
#[async_trait]
pub trait LoadBalancer: Send + Sync {
    /// Select a backend for the request
    async fn select_backend(&self, backends: &[Arc<Backend>]) -> Result<Option<Arc<Backend>>>;

    /// Update algorithm state after request completion
    async fn update_state(&self, backend: &Backend, success: bool) -> Result<()>;
}

/// Round-robin load balancer
#[derive(Debug)]
pub struct RoundRobinBalancer {
    current: std::sync::atomic::AtomicUsize,
}

impl RoundRobinBalancer {
    pub fn new() -> Self {
        Self {
            current: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

impl Clone for RoundRobinBalancer {
    fn clone(&self) -> Self {
        Self {
            current: std::sync::atomic::AtomicUsize::new(
                self.current.load(std::sync::atomic::Ordering::Relaxed)
            ),
        }
    }
}

#[async_trait]
impl LoadBalancer for RoundRobinBalancer {
    async fn select_backend(&self, backends: &[Arc<Backend>]) -> Result<Option<Arc<Backend>>> {
        if backends.is_empty() {
            return Ok(None);
        }

        let index = self.current.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % backends.len();
        Ok(Some(backends[index].clone()))
    }

    async fn update_state(&self, _backend: &Backend, _success: bool) -> Result<()> {
        // Round-robin doesn't need state updates
        Ok(())
    }
}

/// Least connections load balancer
#[derive(Debug, Clone)]
pub struct LeastConnectionsBalancer;

impl LeastConnectionsBalancer {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LoadBalancer for LeastConnectionsBalancer {
    async fn select_backend(&self, backends: &[Arc<Backend>]) -> Result<Option<Arc<Backend>>> {
        if backends.is_empty() {
            return Ok(None);
        }

        // Find backend with least connections
        let mut best_backend = None;
        let mut min_connections = u64::MAX;

        for backend in backends {
            let connections = backend.get_active_connections().await;
            if connections < min_connections {
                min_connections = connections;
                best_backend = Some(backend.clone());
            }
        }

        Ok(best_backend)
    }

    async fn update_state(&self, _backend: &Backend, _success: bool) -> Result<()> {
        // Connection count is managed by the backend itself
        Ok(())
    }
}

/// Load balancer implementation enum
#[derive(Debug, Clone)]
pub enum LoadBalancerImpl {
    RoundRobin(RoundRobinBalancer),
    LeastConnections(LeastConnectionsBalancer),
}

impl LoadBalancerImpl {
    /// Select a backend for the request
    pub async fn select_backend(&self, backends: &[Arc<Backend>]) -> Result<Option<Arc<Backend>>> {
        match self {
            Self::RoundRobin(balancer) => balancer.select_backend(backends).await,
            Self::LeastConnections(balancer) => balancer.select_backend(backends).await,
        }
    }

    /// Update algorithm state after request completion
    pub async fn update_state(&self, backend: &Backend, success: bool) -> Result<()> {
        match self {
            Self::RoundRobin(balancer) => balancer.update_state(backend, success).await,
            Self::LeastConnections(balancer) => balancer.update_state(backend, success).await,
        }
    }
}

/// Create a load balancer instance
pub fn create_load_balancer(algorithm: LoadBalancingAlgorithm) -> LoadBalancerImpl {
    match algorithm {
        LoadBalancingAlgorithm::RoundRobin => LoadBalancerImpl::RoundRobin(RoundRobinBalancer::new()),
        LoadBalancingAlgorithm::LeastConnections => LoadBalancerImpl::LeastConnections(LeastConnectionsBalancer::new()),
        LoadBalancingAlgorithm::WeightedRoundRobin => LoadBalancerImpl::RoundRobin(RoundRobinBalancer::new()), // TODO: Implement weighted
        LoadBalancingAlgorithm::IpHash => LoadBalancerImpl::RoundRobin(RoundRobinBalancer::new()), // TODO: Implement IP hash
        LoadBalancingAlgorithm::Random => LoadBalancerImpl::RoundRobin(RoundRobinBalancer::new()), // TODO: Implement random
    }
}
