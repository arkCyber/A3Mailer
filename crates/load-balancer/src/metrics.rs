//! Load Balancer Metrics Collection
//!
//! This module provides comprehensive metrics collection for the load balancer,
//! including request statistics, response times, connection tracking, and backend health.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Load balancer metrics snapshot
///
/// Contains comprehensive metrics about the load balancer's performance,
/// including request statistics, response times, and backend health status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoadBalancerMetrics {
    /// Total number of requests processed
    pub total_requests: u64,
    /// Number of successful requests (2xx status codes)
    pub successful_requests: u64,
    /// Number of failed requests (4xx, 5xx status codes)
    pub failed_requests: u64,
    /// Average response time in milliseconds
    pub average_response_time_ms: f64,
    /// Current number of active connections
    pub active_connections: u64,
    /// Total number of configured backends
    pub backend_count: u64,
    /// Number of healthy backends
    pub healthy_backend_count: u64,
    /// Request rate (requests per second)
    pub request_rate: f64,
    /// Error rate (percentage of failed requests)
    pub error_rate: f64,
}



/// Response time tracker for calculating averages
#[derive(Debug)]
struct ResponseTimeTracker {
    total_time_ms: Arc<AtomicU64>,
    request_count: Arc<AtomicU64>,
}

impl ResponseTimeTracker {
    fn new() -> Self {
        Self {
            total_time_ms: Arc::new(AtomicU64::new(0)),
            request_count: Arc::new(AtomicU64::new(0)),
        }
    }

    fn record_response_time(&self, duration: Duration) {
        let ms = duration.as_millis() as u64;
        self.total_time_ms.fetch_add(ms, Ordering::Relaxed);
        self.request_count.fetch_add(1, Ordering::Relaxed);
    }

    fn get_average_ms(&self) -> f64 {
        let total = self.total_time_ms.load(Ordering::Relaxed);
        let count = self.request_count.load(Ordering::Relaxed);
        if count == 0 {
            0.0
        } else {
            total as f64 / count as f64
        }
    }
}

/// High-performance metrics collector with atomic operations
///
/// This collector is designed for high-throughput environments and uses
/// atomic operations to ensure thread-safety without locks.
#[derive(Debug)]
pub struct MetricsCollector {
    /// Total number of requests processed
    total_requests: Arc<AtomicU64>,
    /// Number of successful requests
    successful_requests: Arc<AtomicU64>,
    /// Number of failed requests
    failed_requests: Arc<AtomicU64>,
    /// Current number of active connections
    active_connections: Arc<AtomicU64>,
    /// Response time tracking
    response_time_tracker: ResponseTimeTracker,
    /// Timestamp when metrics collection started
    start_time: Instant,
}

impl MetricsCollector {
    /// Create new metrics collector
    ///
    /// Initializes all counters to zero and records the start time
    /// for calculating request rates.
    pub fn new() -> Self {
        info!("Initializing metrics collector");
        Self {
            total_requests: Arc::new(AtomicU64::new(0)),
            successful_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
            active_connections: Arc::new(AtomicU64::new(0)),
            response_time_tracker: ResponseTimeTracker::new(),
            start_time: Instant::now(),
        }
    }

    /// Record a request with its outcome and response time
    ///
    /// # Arguments
    /// * `success` - Whether the request was successful (2xx status code)
    /// * `response_time` - Optional response time duration
    pub fn record_request(&self, success: bool, response_time: Option<Duration>) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        if success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
            debug!("Recorded successful request");
        } else {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
            debug!("Recorded failed request");
        }

        if let Some(duration) = response_time {
            self.response_time_tracker.record_response_time(duration);
        }
    }

    /// Record a request with just success/failure status
    ///
    /// This is a convenience method for when response time is not available.
    pub fn record_request_simple(&self, success: bool) {
        self.record_request(success, None);
    }

    /// Increment active connections counter
    ///
    /// Should be called when a new connection is established.
    pub fn increment_connections(&self) {
        let new_count = self.active_connections.fetch_add(1, Ordering::Relaxed) + 1;
        debug!("Active connections incremented to {}", new_count);
    }

    /// Decrement active connections counter
    ///
    /// Should be called when a connection is closed.
    pub fn decrement_connections(&self) {
        let prev_count = self.active_connections.fetch_sub(1, Ordering::Relaxed);
        if prev_count > 0 {
            debug!("Active connections decremented to {}", prev_count - 1);
        }
    }

    /// Get current metrics snapshot
    ///
    /// # Arguments
    /// * `backend_count` - Total number of configured backends
    /// * `healthy_backend_count` - Number of healthy backends
    ///
    /// # Returns
    /// A complete metrics snapshot including calculated rates and averages
    pub fn get_metrics(&self, backend_count: u64, healthy_backend_count: u64) -> LoadBalancerMetrics {
        let total = self.total_requests.load(Ordering::Relaxed);
        let successful = self.successful_requests.load(Ordering::Relaxed);
        let failed = self.failed_requests.load(Ordering::Relaxed);
        let active = self.active_connections.load(Ordering::Relaxed);

        // Calculate request rate (requests per second)
        let elapsed_secs = self.start_time.elapsed().as_secs_f64();
        let request_rate = if elapsed_secs > 0.0 {
            total as f64 / elapsed_secs
        } else {
            0.0
        };

        // Calculate error rate (percentage)
        let error_rate = if total > 0 {
            (failed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let average_response_time = self.response_time_tracker.get_average_ms();

        debug!(
            "Metrics snapshot: total={}, successful={}, failed={}, active={}, rate={:.2}/s, error_rate={:.2}%",
            total, successful, failed, active, request_rate, error_rate
        );

        LoadBalancerMetrics {
            total_requests: total,
            successful_requests: successful,
            failed_requests: failed,
            average_response_time_ms: average_response_time,
            active_connections: active,
            backend_count,
            healthy_backend_count,
            request_rate,
            error_rate,
        }
    }

    /// Reset all metrics to zero
    ///
    /// This method is useful for testing or when restarting metrics collection.
    pub fn reset(&self) {
        info!("Resetting all metrics to zero");
        self.total_requests.store(0, Ordering::Relaxed);
        self.successful_requests.store(0, Ordering::Relaxed);
        self.failed_requests.store(0, Ordering::Relaxed);
        self.active_connections.store(0, Ordering::Relaxed);
        self.response_time_tracker.total_time_ms.store(0, Ordering::Relaxed);
        self.response_time_tracker.request_count.store(0, Ordering::Relaxed);
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadBalancerMetrics {
    /// Create new metrics with default values
    ///
    /// All counters and rates are initialized to zero.
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_response_time_ms: 0.0,
            active_connections: 0,
            backend_count: 0,
            healthy_backend_count: 0,
            request_rate: 0.0,
            error_rate: 0.0,
        }
    }

    /// Update backend statistics
    ///
    /// Updates the backend count and healthy backend count based on
    /// the current status of all backends.
    ///
    /// # Arguments
    /// * `backend_stats` - Vector of backend statistics
    pub fn update_backend_stats(&mut self, backend_stats: Vec<(crate::Backend, crate::BackendStatus)>) {
        self.backend_count = backend_stats.len() as u64;
        self.healthy_backend_count = backend_stats
            .iter()
            .filter(|(_, status)| matches!(status, crate::BackendStatus::Healthy))
            .count() as u64;

        debug!(
            "Updated backend stats: total={}, healthy={}",
            self.backend_count, self.healthy_backend_count
        );
    }

    /// Get success rate as a percentage
    ///
    /// # Returns
    /// Success rate as a percentage (0.0 to 100.0)
    pub fn success_rate(&self) -> f64 {
        if self.total_requests > 0 {
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Check if the load balancer is healthy
    ///
    /// A load balancer is considered healthy if:
    /// - At least one backend is healthy
    /// - Error rate is below 50%
    ///
    /// # Returns
    /// True if the load balancer is healthy
    pub fn is_healthy(&self) -> bool {
        self.healthy_backend_count > 0 && self.error_rate < 50.0
    }

    /// Get a human-readable status summary
    ///
    /// # Returns
    /// A string describing the current status
    pub fn status_summary(&self) -> String {
        if self.is_healthy() {
            format!(
                "Healthy: {}/{} backends, {:.1}% success rate, {:.2} req/s",
                self.healthy_backend_count,
                self.backend_count,
                self.success_rate(),
                self.request_rate
            )
        } else {
            format!(
                "Unhealthy: {}/{} backends, {:.1}% error rate",
                self.healthy_backend_count,
                self.backend_count,
                self.error_rate
            )
        }
    }
}

impl Default for LoadBalancerMetrics {
    fn default() -> Self {
        Self::new()
    }
}
