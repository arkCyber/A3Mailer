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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        let metrics = collector.get_metrics(0, 0);

        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 0);
        assert_eq!(metrics.active_connections, 0);
        assert_eq!(metrics.request_rate, 0.0);
        assert_eq!(metrics.error_rate, 0.0);
    }

    #[test]
    fn test_request_recording() {
        let collector = MetricsCollector::new();

        // Record some successful requests
        collector.record_request_simple(true);
        collector.record_request_simple(true);
        collector.record_request_simple(false);

        let metrics = collector.get_metrics(2, 1);

        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.successful_requests, 2);
        assert_eq!(metrics.failed_requests, 1);
        assert_eq!(metrics.backend_count, 2);
        assert_eq!(metrics.healthy_backend_count, 1);
        assert!((metrics.error_rate - 33.33).abs() < 0.1);
    }

    #[test]
    fn test_connection_tracking() {
        let collector = MetricsCollector::new();

        collector.increment_connections();
        collector.increment_connections();
        collector.increment_connections();
        collector.decrement_connections();

        let metrics = collector.get_metrics(0, 0);
        assert_eq!(metrics.active_connections, 2);
    }

    #[test]
    fn test_response_time_tracking() {
        let collector = MetricsCollector::new();

        collector.record_request(true, Some(Duration::from_millis(100)));
        collector.record_request(true, Some(Duration::from_millis(200)));
        collector.record_request(false, Some(Duration::from_millis(300)));

        let metrics = collector.get_metrics(0, 0);

        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.successful_requests, 2);
        assert_eq!(metrics.failed_requests, 1);
        assert!((metrics.average_response_time_ms - 200.0).abs() < 0.1);
    }

    #[test]
    fn test_metrics_reset() {
        let collector = MetricsCollector::new();

        collector.record_request_simple(true);
        collector.increment_connections();

        let metrics_before = collector.get_metrics(0, 0);
        assert_eq!(metrics_before.total_requests, 1);
        assert_eq!(metrics_before.active_connections, 1);

        collector.reset();

        let metrics_after = collector.get_metrics(0, 0);
        assert_eq!(metrics_after.total_requests, 0);
        assert_eq!(metrics_after.active_connections, 0);
    }

    #[test]
    fn test_load_balancer_metrics_creation() {
        let metrics = LoadBalancerMetrics::new();

        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 0);
        assert_eq!(metrics.average_response_time_ms, 0.0);
        assert_eq!(metrics.active_connections, 0);
        assert_eq!(metrics.backend_count, 0);
        assert_eq!(metrics.healthy_backend_count, 0);
        assert_eq!(metrics.request_rate, 0.0);
        assert_eq!(metrics.error_rate, 0.0);
    }

    #[test]
    fn test_backend_stats_update() {
        let mut metrics = LoadBalancerMetrics::new();

        let backend_stats = vec![
            (
                crate::Backend::new("backend1".to_string(), "127.0.0.1".to_string(), 8001, 1),
                crate::BackendStatus::Healthy,
            ),
            (
                crate::Backend::new("backend2".to_string(), "127.0.0.1".to_string(), 8002, 1),
                crate::BackendStatus::Unhealthy,
            ),
            (
                crate::Backend::new("backend3".to_string(), "127.0.0.1".to_string(), 8003, 1),
                crate::BackendStatus::Healthy,
            ),
        ];

        metrics.update_backend_stats(backend_stats);

        assert_eq!(metrics.backend_count, 3);
        assert_eq!(metrics.healthy_backend_count, 2);
    }

    #[test]
    fn test_success_rate_calculation() {
        let mut metrics = LoadBalancerMetrics::new();
        metrics.total_requests = 100;
        metrics.successful_requests = 85;
        metrics.failed_requests = 15;

        assert!((metrics.success_rate() - 85.0).abs() < 0.1);

        // Test with zero requests
        let empty_metrics = LoadBalancerMetrics::new();
        assert_eq!(empty_metrics.success_rate(), 0.0);
    }

    #[test]
    fn test_health_check() {
        let mut metrics = LoadBalancerMetrics::new();

        // Healthy case: has healthy backends and low error rate
        metrics.healthy_backend_count = 2;
        metrics.backend_count = 3;
        metrics.error_rate = 5.0;
        assert!(metrics.is_healthy());

        // Unhealthy case: no healthy backends
        metrics.healthy_backend_count = 0;
        assert!(!metrics.is_healthy());

        // Unhealthy case: high error rate
        metrics.healthy_backend_count = 2;
        metrics.error_rate = 75.0;
        assert!(!metrics.is_healthy());
    }

    #[test]
    fn test_status_summary() {
        let mut metrics = LoadBalancerMetrics::new();
        metrics.backend_count = 3;
        metrics.healthy_backend_count = 2;
        metrics.total_requests = 100;
        metrics.successful_requests = 95;
        metrics.failed_requests = 5;
        metrics.error_rate = 5.0;
        metrics.request_rate = 10.5;

        let summary = metrics.status_summary();
        assert!(summary.contains("Healthy"));
        assert!(summary.contains("2/3 backends"));
        assert!(summary.contains("95.0% success rate"));
        assert!(summary.contains("10.50 req/s"));

        // Test unhealthy status
        metrics.healthy_backend_count = 0;
        metrics.error_rate = 60.0;
        let unhealthy_summary = metrics.status_summary();
        assert!(unhealthy_summary.contains("Unhealthy"));
        assert!(unhealthy_summary.contains("60.0% error rate"));
    }

    #[test]
    fn test_metrics_serialization() {
        let metrics = LoadBalancerMetrics {
            total_requests: 1000,
            successful_requests: 950,
            failed_requests: 50,
            average_response_time_ms: 125.5,
            active_connections: 25,
            backend_count: 5,
            healthy_backend_count: 4,
            request_rate: 15.2,
            error_rate: 5.0,
        };

        // Test serialization to JSON
        let json = serde_json::to_string(&metrics).expect("Failed to serialize metrics");
        assert!(json.contains("\"total_requests\":1000"));
        assert!(json.contains("\"error_rate\":5.0"));

        // Test deserialization from JSON
        let deserialized: LoadBalancerMetrics = serde_json::from_str(&json)
            .expect("Failed to deserialize metrics");
        assert_eq!(deserialized, metrics);
    }
}
