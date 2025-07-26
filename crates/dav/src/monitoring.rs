/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! DAV Server Monitoring and Performance Tracking
//!
//! This module provides comprehensive monitoring capabilities for the DAV server,
//! including request metrics, performance tracking, and health monitoring.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use hyper::StatusCode;
use tracing::{debug, info, warn};

/// DAV server metrics collector
///
/// Collects and aggregates metrics for DAV operations including
/// request counts, response times, error rates, and resource usage.
#[derive(Debug, Clone)]
pub struct DavMetrics {
    inner: Arc<RwLock<DavMetricsInner>>,
}

#[derive(Debug)]
struct DavMetricsInner {
    /// Request counts by method
    request_counts: HashMap<String, u64>,
    /// Response time histograms by method
    response_times: HashMap<String, Vec<Duration>>,
    /// Error counts by status code
    error_counts: HashMap<u16, u64>,
    /// Active request count
    active_requests: u64,
    /// Total bytes transferred
    bytes_transferred: u64,
    /// Start time for metrics collection
    start_time: Instant,
}

impl DavMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(DavMetricsInner {
                request_counts: HashMap::new(),
                response_times: HashMap::new(),
                error_counts: HashMap::new(),
                active_requests: 0,
                bytes_transferred: 0,
                start_time: Instant::now(),
            })),
        }
    }

    /// Record a request start
    pub fn record_request_start(&self, method: &str) {
        if let Ok(mut inner) = self.inner.write() {
            *inner.request_counts.entry(method.to_string()).or_insert(0) += 1;
            inner.active_requests += 1;

            debug!(
                method = method,
                active_requests = inner.active_requests,
                "DAV request started"
            );
        }
    }

    /// Record a request completion
    pub fn record_request_complete(
        &self,
        method: &str,
        status_code: StatusCode,
        duration: Duration,
        bytes_transferred: u64,
    ) {
        if let Ok(mut inner) = self.inner.write() {
            // Record response time
            inner
                .response_times
                .entry(method.to_string())
                .or_insert_with(Vec::new)
                .push(duration);

            // Record error if status indicates error
            if status_code.is_client_error() || status_code.is_server_error() {
                *inner.error_counts.entry(status_code.as_u16()).or_insert(0) += 1;

                warn!(
                    method = method,
                    status_code = status_code.as_u16(),
                    duration_ms = duration.as_millis(),
                    "DAV request completed with error"
                );
            } else {
                debug!(
                    method = method,
                    status_code = status_code.as_u16(),
                    duration_ms = duration.as_millis(),
                    bytes = bytes_transferred,
                    "DAV request completed successfully"
                );
            }

            // Update counters
            inner.active_requests = inner.active_requests.saturating_sub(1);
            inner.bytes_transferred += bytes_transferred;
        }
    }

    /// Get current metrics snapshot
    pub fn get_metrics(&self) -> DavMetricsSnapshot {
        if let Ok(inner) = self.inner.read() {
            let mut method_stats = HashMap::new();

            for (method, count) in &inner.request_counts {
                let response_times = inner.response_times.get(method).cloned().unwrap_or_default();
                let avg_response_time = if !response_times.is_empty() {
                    response_times.iter().sum::<Duration>() / response_times.len() as u32
                } else {
                    Duration::ZERO
                };

                let max_response_time = response_times.iter().max().copied().unwrap_or(Duration::ZERO);
                let min_response_time = response_times.iter().min().copied().unwrap_or(Duration::ZERO);

                method_stats.insert(method.clone(), MethodStats {
                    request_count: *count,
                    avg_response_time,
                    min_response_time,
                    max_response_time,
                });
            }

            DavMetricsSnapshot {
                method_stats,
                error_counts: inner.error_counts.clone(),
                active_requests: inner.active_requests,
                bytes_transferred: inner.bytes_transferred,
                uptime: inner.start_time.elapsed(),
                total_requests: inner.request_counts.values().sum(),
                error_rate: calculate_error_rate(&inner.request_counts, &inner.error_counts),
            }
        } else {
            DavMetricsSnapshot::default()
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        if let Ok(mut inner) = self.inner.write() {
            inner.request_counts.clear();
            inner.response_times.clear();
            inner.error_counts.clear();
            inner.active_requests = 0;
            inner.bytes_transferred = 0;
            inner.start_time = Instant::now();

            info!("DAV metrics reset");
        }
    }

    /// Get health status based on metrics
    pub fn get_health_status(&self) -> DavHealthStatus {
        let metrics = self.get_metrics();

        let status = if metrics.error_rate > 0.1 {
            HealthStatus::Unhealthy
        } else if metrics.error_rate > 0.05 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        DavHealthStatus {
            status,
            active_requests: metrics.active_requests,
            error_rate: metrics.error_rate,
            uptime: metrics.uptime,
            total_requests: metrics.total_requests,
        }
    }
}

impl Default for DavMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of DAV metrics at a point in time
#[derive(Debug, Clone)]
pub struct DavMetricsSnapshot {
    /// Statistics by HTTP method
    pub method_stats: HashMap<String, MethodStats>,
    /// Error counts by status code
    pub error_counts: HashMap<u16, u64>,
    /// Currently active requests
    pub active_requests: u64,
    /// Total bytes transferred
    pub bytes_transferred: u64,
    /// Server uptime
    pub uptime: Duration,
    /// Total number of requests processed
    pub total_requests: u64,
    /// Overall error rate (0.0 to 1.0)
    pub error_rate: f64,
}

impl Default for DavMetricsSnapshot {
    fn default() -> Self {
        Self {
            method_stats: HashMap::new(),
            error_counts: HashMap::new(),
            active_requests: 0,
            bytes_transferred: 0,
            uptime: Duration::ZERO,
            total_requests: 0,
            error_rate: 0.0,
        }
    }
}

/// Statistics for a specific HTTP method
#[derive(Debug, Clone)]
pub struct MethodStats {
    /// Total number of requests for this method
    pub request_count: u64,
    /// Average response time
    pub avg_response_time: Duration,
    /// Minimum response time
    pub min_response_time: Duration,
    /// Maximum response time
    pub max_response_time: Duration,
}

/// DAV server health status
#[derive(Debug, Clone)]
pub struct DavHealthStatus {
    /// Overall health status
    pub status: HealthStatus,
    /// Number of active requests
    pub active_requests: u64,
    /// Current error rate
    pub error_rate: f64,
    /// Server uptime
    pub uptime: Duration,
    /// Total requests processed
    pub total_requests: u64,
}

/// Health status enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    /// Server is operating normally
    Healthy,
    /// Server is experiencing some issues but still functional
    Degraded,
    /// Server is experiencing significant issues
    Unhealthy,
}

/// Calculate error rate from request and error counts
fn calculate_error_rate(
    request_counts: &HashMap<String, u64>,
    error_counts: &HashMap<u16, u64>,
) -> f64 {
    let total_requests: u64 = request_counts.values().sum();
    let total_errors: u64 = error_counts.values().sum();

    if total_requests == 0 {
        0.0
    } else {
        total_errors as f64 / total_requests as f64
    }
}

/// Performance tracker for individual requests
#[derive(Debug)]
pub struct RequestTracker {
    method: String,
    start_time: Instant,
    metrics: DavMetrics,
}

impl RequestTracker {
    /// Create a new request tracker
    pub fn new(method: String, metrics: DavMetrics) -> Self {
        metrics.record_request_start(&method);

        Self {
            method,
            start_time: Instant::now(),
            metrics,
        }
    }

    /// Complete the request tracking
    pub fn complete(self, status_code: StatusCode, bytes_transferred: u64) {
        let duration = self.start_time.elapsed();
        self.metrics.record_request_complete(
            &self.method,
            status_code,
            duration,
            bytes_transferred,
        );
    }
}

impl Drop for RequestTracker {
    fn drop(&mut self) {
        // If the tracker is dropped without calling complete(), record it as an internal error
        let duration = self.start_time.elapsed();
        self.metrics.record_request_complete(
            &self.method,
            StatusCode::INTERNAL_SERVER_ERROR,
            duration,
            0,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_metrics_collection() {
        let metrics = DavMetrics::new();

        // Record some requests
        metrics.record_request_start("GET");
        metrics.record_request_complete(
            "GET",
            StatusCode::OK,
            Duration::from_millis(100),
            1024,
        );

        metrics.record_request_start("PUT");
        metrics.record_request_complete(
            "PUT",
            StatusCode::CREATED,
            Duration::from_millis(200),
            2048,
        );

        metrics.record_request_start("DELETE");
        metrics.record_request_complete(
            "DELETE",
            StatusCode::NOT_FOUND,
            Duration::from_millis(50),
            0,
        );

        let snapshot = metrics.get_metrics();

        // Verify metrics
        assert_eq!(snapshot.total_requests, 3);
        assert_eq!(snapshot.bytes_transferred, 3072);
        assert_eq!(snapshot.active_requests, 0);
        assert!(snapshot.error_rate > 0.0); // One error out of three requests

        // Check method stats
        assert!(snapshot.method_stats.contains_key("GET"));
        assert!(snapshot.method_stats.contains_key("PUT"));
        assert!(snapshot.method_stats.contains_key("DELETE"));

        let get_stats = &snapshot.method_stats["GET"];
        assert_eq!(get_stats.request_count, 1);
        assert_eq!(get_stats.avg_response_time, Duration::from_millis(100));

        // Check error counts
        assert_eq!(snapshot.error_counts.get(&404), Some(&1));
    }

    #[test]
    fn test_request_tracker() {
        let metrics = DavMetrics::new();

        {
            let tracker = RequestTracker::new("PROPFIND".to_string(), metrics.clone());
            thread::sleep(Duration::from_millis(10));
            tracker.complete(StatusCode::MULTI_STATUS, 512);
        }

        let snapshot = metrics.get_metrics();
        assert_eq!(snapshot.total_requests, 1);
        assert!(snapshot.method_stats.contains_key("PROPFIND"));
        assert_eq!(snapshot.bytes_transferred, 512);
    }

    #[test]
    fn test_health_status() {
        let metrics = DavMetrics::new();

        // Initially healthy
        let health = metrics.get_health_status();
        assert_eq!(health.status, HealthStatus::Healthy);

        // Add some errors to make it degraded
        for _ in 0..6 {
            metrics.record_request_start("GET");
            metrics.record_request_complete(
                "GET",
                StatusCode::OK,
                Duration::from_millis(100),
                1024,
            );
        }

        for _ in 0..4 {
            metrics.record_request_start("GET");
            metrics.record_request_complete(
                "GET",
                StatusCode::INTERNAL_SERVER_ERROR,
                Duration::from_millis(100),
                0,
            );
        }

        let health = metrics.get_health_status();
        // With 4 errors out of 10 requests, error rate is 0.4 which is > 0.1, so should be Unhealthy
        assert_eq!(health.status, HealthStatus::Unhealthy);
        assert_eq!(health.total_requests, 10);
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = DavMetrics::new();

        // Add some data
        metrics.record_request_start("GET");
        metrics.record_request_complete(
            "GET",
            StatusCode::OK,
            Duration::from_millis(100),
            1024,
        );

        let snapshot_before = metrics.get_metrics();
        assert_eq!(snapshot_before.total_requests, 1);

        // Reset metrics
        metrics.reset();

        let snapshot_after = metrics.get_metrics();
        assert_eq!(snapshot_after.total_requests, 0);
        assert_eq!(snapshot_after.bytes_transferred, 0);
        assert!(snapshot_after.method_stats.is_empty());
    }

    #[test]
    fn test_concurrent_metrics() {
        let metrics = DavMetrics::new();
        let mut handles = vec![];

        // Spawn multiple threads to record metrics concurrently
        for i in 0..10 {
            let metrics_clone = metrics.clone();
            let handle = thread::spawn(move || {
                for j in 0..10 {
                    metrics_clone.record_request_start("GET");
                    thread::sleep(Duration::from_millis(1));
                    metrics_clone.record_request_complete(
                        "GET",
                        if (i + j) % 5 == 0 { StatusCode::INTERNAL_SERVER_ERROR } else { StatusCode::OK },
                        Duration::from_millis(10 + i * j),
                        1024,
                    );
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        let snapshot = metrics.get_metrics();
        assert_eq!(snapshot.total_requests, 100);
        assert!(snapshot.error_rate > 0.0);
        assert_eq!(snapshot.active_requests, 0);
    }
}
