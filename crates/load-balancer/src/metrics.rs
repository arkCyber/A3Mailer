//! Metrics collection

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Load balancer metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: f64,
    pub active_connections: u64,
    pub backend_count: u64,
    pub healthy_backend_count: u64,
}



/// Metrics collector
#[derive(Debug)]
pub struct MetricsCollector {
    total_requests: Arc<AtomicU64>,
    successful_requests: Arc<AtomicU64>,
    failed_requests: Arc<AtomicU64>,
    active_connections: Arc<AtomicU64>,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            total_requests: Arc::new(AtomicU64::new(0)),
            successful_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
            active_connections: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a request
    pub fn record_request(&self, success: bool) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        if success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Increment active connections
    pub fn increment_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active connections
    pub fn decrement_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get current metrics
    pub fn get_metrics(&self, backend_count: u64, healthy_backend_count: u64) -> LoadBalancerMetrics {
        let total = self.total_requests.load(Ordering::Relaxed);
        let successful = self.successful_requests.load(Ordering::Relaxed);
        let failed = self.failed_requests.load(Ordering::Relaxed);

        LoadBalancerMetrics {
            total_requests: total,
            successful_requests: successful,
            failed_requests: failed,
            average_response_time_ms: 0.0, // TODO: Implement response time tracking
            active_connections: self.active_connections.load(Ordering::Relaxed),
            backend_count,
            healthy_backend_count,
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadBalancerMetrics {
    /// Create new metrics with default values
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_response_time_ms: 0.0,
            active_connections: 0,
            backend_count: 0,
            healthy_backend_count: 0,
        }
    }

    /// Update backend statistics
    pub fn update_backend_stats(&mut self, _backend_stats: Vec<crate::backend::BackendStatistics>) {
        // TODO: Implement backend stats update
    }
}

impl Default for LoadBalancerMetrics {
    fn default() -> Self {
        Self::new()
    }
}
