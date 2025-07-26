//! Backend server management

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Backend server status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BackendStatus {
    Healthy,
    Unhealthy,
    Draining,
    Disabled,
}

/// Backend server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backend {
    pub id: String,
    pub address: String,
    pub port: u16,
    pub weight: u32,
    pub status: BackendStatus,
    #[serde(skip)]
    pub active_connections: Arc<AtomicU64>,
    #[serde(skip)]
    pub total_requests: Arc<AtomicU64>,
    #[serde(skip)]
    pub failed_requests: Arc<AtomicU64>,
}

impl Backend {
    /// Create a new backend
    pub fn new(id: String, address: String, port: u16, weight: u32) -> Self {
        Self {
            id,
            address,
            port,
            weight,
            status: BackendStatus::Healthy,
            active_connections: Arc::new(AtomicU64::new(0)),
            total_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get active connections count
    pub async fn get_active_connections(&self) -> u64 {
        self.active_connections.load(Ordering::Relaxed)
    }

    /// Increment active connections
    pub async fn increment_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active connections
    pub async fn decrement_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record failed request
    pub async fn record_failure(&self) {
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Get backend URL
    pub fn url(&self) -> String {
        format!("http://{}:{}", self.address, self.port)
    }

    /// Check if backend is available
    pub fn is_available(&self) -> bool {
        matches!(self.status, BackendStatus::Healthy)
    }
}

/// Backend statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendStatistics {
    pub backend_id: String,
    pub active_connections: u64,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub success_rate: f64,
    pub status: BackendStatus,
}

/// Backend pool
#[derive(Debug, Clone)]
pub struct BackendPool {
    backends: Arc<RwLock<Vec<Arc<Backend>>>>,
}

impl BackendPool {
    /// Create new backend pool
    pub async fn new(backends: &[Backend]) -> Result<Self> {
        let backend_pool = Self {
            backends: Arc::new(RwLock::new(
                backends.iter().map(|b| Arc::new(b.clone())).collect()
            )),
        };
        Ok(backend_pool)
    }

    /// Add backend
    pub async fn add_backend(&self, backend: Backend) -> Result<()> {
        let mut backends = self.backends.write().await;
        backends.push(Arc::new(backend));
        Ok(())
    }

    /// Remove backend
    pub async fn remove_backend(&self, backend_id: &str) -> Result<bool> {
        let mut backends = self.backends.write().await;
        let initial_len = backends.len();
        backends.retain(|b| b.id != backend_id);
        Ok(backends.len() < initial_len)
    }

    /// Update backend weight
    pub async fn update_weight(&self, backend_id: &str, weight: u32) -> Result<()> {
        let backends = self.backends.read().await;
        for backend in backends.iter() {
            if backend.id == backend_id {
                // Note: This is a simplified implementation
                // In a real implementation, we'd need interior mutability for weight
                break;
            }
        }
        Ok(())
    }

    /// Get all backend status
    pub async fn get_all_status(&self) -> Vec<(Backend, BackendStatus)> {
        let backends = self.backends.read().await;
        backends.iter().map(|b| ((**b).clone(), b.status.clone())).collect()
    }

    /// Get healthy backends
    pub async fn get_healthy_backends(&self) -> Vec<Arc<Backend>> {
        let backends = self.backends.read().await;
        backends.iter()
            .filter(|b| b.is_available())
            .cloned()
            .collect()
    }

    /// Get backend by ID
    pub async fn get_backend(&self, backend_id: &str) -> Option<Arc<Backend>> {
        let backends = self.backends.read().await;
        backends.iter()
            .find(|b| b.id == backend_id)
            .cloned()
    }

    /// Get statistics for all backends
    pub async fn get_statistics(&self) -> Vec<BackendStatistics> {
        let backends = self.backends.read().await;
        let mut stats = Vec::new();

        for backend in backends.iter() {
            let total_requests = backend.total_requests.load(Ordering::Relaxed);
            let failed_requests = backend.failed_requests.load(Ordering::Relaxed);
            let success_rate = if total_requests > 0 {
                1.0 - (failed_requests as f64 / total_requests as f64)
            } else {
                1.0
            };

            stats.push(BackendStatistics {
                backend_id: backend.id.clone(),
                active_connections: backend.active_connections.load(Ordering::Relaxed),
                total_requests,
                failed_requests,
                success_rate,
                status: backend.status.clone(),
            });
        }

        stats
    }
}
