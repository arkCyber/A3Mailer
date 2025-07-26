/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! # Load Balancer
//!
//! High-performance load balancer for Stalwart Mail Server with support for:
//!
//! - Multiple load balancing algorithms (round-robin, least connections, weighted, etc.)
//! - Health checking and automatic failover
//! - Session affinity and sticky sessions
//! - SSL termination and pass-through
//! - Real-time metrics and monitoring
//! - Dynamic configuration updates

pub mod algorithms;
pub mod backend;
pub mod config;
pub mod error;
pub mod health;
pub mod metrics;
pub mod proxy;
pub mod server;
pub mod session;
pub mod tls;

pub use algorithms::{LoadBalancingAlgorithm, LoadBalancer, LoadBalancerImpl};
pub use backend::{Backend, BackendPool, BackendStatus};
pub use config::{LoadBalancerConfig, HealthCheckConfig, SessionAffinityConfig, ServerConfig};
pub use error::{LoadBalancerError, Result};
pub use health::HealthChecker;
pub use metrics::{LoadBalancerMetrics, MetricsCollector};
pub use proxy::ProxyService;
pub use server::LoadBalancerServer;
pub use session::SessionAffinity;

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};


/// Main load balancer service
#[derive(Debug, Clone)]
pub struct LoadBalancerService {
    inner: Arc<LoadBalancerServiceInner>,
}

#[derive(Debug)]
struct LoadBalancerServiceInner {
    config: LoadBalancerConfig,
    backend_pool: BackendPool,
    load_balancer: LoadBalancerImpl,
    health_checker: HealthChecker,
    session_affinity: Option<SessionAffinity>,
    metrics: Arc<RwLock<LoadBalancerMetrics>>,
    server: Arc<RwLock<LoadBalancerServer>>,
}

impl LoadBalancerService {
    /// Create a new load balancer service
    pub async fn new(config: LoadBalancerConfig) -> Result<Self> {
        info!("Initializing load balancer service");

        // Create backend pool
        let backend_pool = BackendPool::new(&config.backends).await?;

        // Create load balancer with specified algorithm
        let algorithm = match config.algorithm.as_str() {
            "round_robin" => LoadBalancingAlgorithm::RoundRobin,
            "least_connections" => LoadBalancingAlgorithm::LeastConnections,
            "weighted_round_robin" => LoadBalancingAlgorithm::WeightedRoundRobin,
            "ip_hash" => LoadBalancingAlgorithm::IpHash,
            "random" => LoadBalancingAlgorithm::Random,
            _ => LoadBalancingAlgorithm::RoundRobin,
        };
        let load_balancer = algorithms::create_load_balancer(algorithm);

        // Create health checker
        let health_checker = HealthChecker::new(
            &config.health_check,
            Arc::new(backend_pool.clone()),
        ).await?;

        // Create session affinity if enabled
        let session_affinity = if let Some(ref affinity_config) = config.session_affinity {
            Some(SessionAffinity::new(affinity_config.clone()))
        } else {
            None
        };

        // Create metrics collector
        let metrics = Arc::new(RwLock::new(LoadBalancerMetrics::new()));

        // Create server
        let server = LoadBalancerServer::new(&config.server).await?;

        Ok(Self {
            inner: Arc::new(LoadBalancerServiceInner {
                config,
                backend_pool,
                load_balancer,
                health_checker,
                session_affinity,
                metrics,
                server: Arc::new(RwLock::new(server)),
            }),
        })
    }

    /// Start the load balancer service
    pub async fn start(&self) -> Result<()> {
        info!("Starting load balancer service");

        // Start health checker
        self.inner.health_checker.start().await?;

        // Start metrics collection
        self.start_metrics_collection().await;

        // Start the server
        let proxy_service = self.create_proxy_service();
        self.inner.server.write().await.start(Arc::new(proxy_service)).await?;

        info!("Load balancer service started successfully");
        Ok(())
    }

    /// Stop the load balancer service
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping load balancer service");

        // Stop health checker
        self.inner.health_checker.stop().await?;

        // Stop server
        self.inner.server.write().await.stop().await?;

        info!("Load balancer service stopped");
        Ok(())
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> LoadBalancerMetrics {
        self.inner.metrics.read().await.clone()
    }

    /// Get backend pool status
    pub async fn get_backend_status(&self) -> Vec<(Backend, BackendStatus)> {
        let backends = self.inner.backend_pool.get_all_status().await;
        backends
    }

    /// Add a new backend
    pub async fn add_backend(&self, backend: Backend) -> Result<()> {
        info!("Adding new backend: {:?}", backend);
        self.inner.backend_pool.add_backend(backend).await?;
        Ok(())
    }

    /// Remove a backend
    pub async fn remove_backend(&self, backend_id: &str) -> Result<bool> {
        info!("Removing backend: {}", backend_id);
        let removed = self.inner.backend_pool.remove_backend(backend_id).await?;
        if removed {
            info!("Backend removed successfully: {}", backend_id);
        } else {
            warn!("Backend not found: {}", backend_id);
        }
        Ok(removed)
    }

    /// Update backend weight
    pub async fn update_backend_weight(&self, backend_id: &str, weight: u32) -> Result<bool> {
        info!("Updating backend weight: {} -> {}", backend_id, weight);
        self.inner.backend_pool.update_weight(backend_id, weight).await?;
        Ok(true)
    }

    /// Create proxy service for handling requests
    fn create_proxy_service(&self) -> ProxyService {
        ProxyService::new(
            Arc::new(self.inner.backend_pool.clone()),
            self.inner.load_balancer.clone(),
            Arc::new(MetricsCollector::new()),
        )
    }

    /// Start metrics collection background task
    async fn start_metrics_collection(&self) {
        let metrics = self.inner.metrics.clone();
        let backend_pool = self.inner.backend_pool.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Collect backend statistics
                let backend_stats = backend_pool.get_statistics().await;

                // Convert to the format expected by metrics
                let backend_status_pairs: Vec<(Backend, BackendStatus)> = backend_stats
                    .into_iter()
                    .map(|stats| {
                        let backend = Backend::new(
                            stats.backend_id.clone(),
                            "127.0.0.1".to_string(), // TODO: Get actual address from backend
                            8080, // TODO: Get actual port from backend
                            1, // TODO: Get actual weight from backend
                        );
                        let status = stats.status.clone();
                        (backend, status)
                    })
                    .collect();

                // Update metrics
                {
                    let mut metrics_guard = metrics.write().await;
                    metrics_guard.update_backend_stats(backend_status_pairs);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_service_creation() {
        let config = LoadBalancerConfig {
            enabled: true,
            algorithm: "round_robin".to_string(),
            health_check_interval: 30,
            backends: vec![
                Backend::new(
                    "backend1".to_string(),
                    "127.0.0.1".to_string(),
                    8001,
                    1,
                ),
            ],
            server: ServerConfig {
                listen_address: "127.0.0.1".to_string(),
                listen_port: 8080,
            },
            health_check: HealthCheckConfig {
                enabled: true,
                interval_seconds: 30,
                timeout_seconds: 5,
                path: "/health".to_string(),
                expected_status: 200,
            },
            session_affinity: None,
        };

        let service = LoadBalancerService::new(config).await;
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_backend_management() {
        let config = LoadBalancerConfig::default();
        let service = LoadBalancerService::new(config).await.unwrap();

        // Add a backend
        let backend = Backend::new(
            "test_backend".to_string(),
            "127.0.0.1:9000".parse().unwrap(),
            1,
            100,
        );

        assert!(service.add_backend(backend).await.is_ok());

        // Check backend status
        let status = service.get_backend_status().await;
        assert!(!status.is_empty());

        // Remove backend
        assert!(service.remove_backend("test_backend").await.unwrap());
    }
}
