/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! HTTP Performance Optimizations Module
//!
//! This module contains various performance optimizations for HTTP operations,
//! including connection pooling, response caching, request batching, and
//! other techniques to improve throughput and reduce latency.

pub mod cache;
pub mod connection_pool;

pub use cache::{HttpCache, CacheConfig, CacheKey, CachedResponse, CacheStats};
pub use connection_pool::{ConnectionPool, PoolConfig, ConnectionStats, PoolError};

use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Performance metrics for HTTP operations
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Total number of requests processed
    pub total_requests: u64,
    /// Total number of successful requests
    pub successful_requests: u64,
    /// Total number of failed requests
    pub failed_requests: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Minimum response time in milliseconds
    pub min_response_time_ms: f64,
    /// Maximum response time in milliseconds
    pub max_response_time_ms: f64,
    /// Total bytes sent
    pub total_bytes_sent: u64,
    /// Total bytes received
    pub total_bytes_received: u64,
    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f64,
    /// Connection reuse rate (0.0 to 1.0)
    pub connection_reuse_rate: f64,
}

impl PerformanceMetrics {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.successful_requests as f64 / self.total_requests as f64
        }
    }

    /// Calculate throughput (requests per second)
    pub fn throughput(&self, duration: Duration) -> f64 {
        if duration.as_secs_f64() == 0.0 {
            0.0
        } else {
            self.total_requests as f64 / duration.as_secs_f64()
        }
    }

    /// Calculate bandwidth utilization (bytes per second)
    pub fn bandwidth_utilization(&self, duration: Duration) -> (f64, f64) {
        let duration_secs = duration.as_secs_f64();
        if duration_secs == 0.0 {
            (0.0, 0.0)
        } else {
            let bytes_sent_per_sec = self.total_bytes_sent as f64 / duration_secs;
            let bytes_received_per_sec = self.total_bytes_received as f64 / duration_secs;
            (bytes_sent_per_sec, bytes_received_per_sec)
        }
    }
}

/// Performance monitor for tracking HTTP operations
pub struct PerformanceMonitor {
    start_time: Instant,
    metrics: PerformanceMetrics,
    response_times: Vec<f64>,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new() -> Self {
        info!("Starting HTTP performance monitoring");
        Self {
            start_time: Instant::now(),
            metrics: PerformanceMetrics::default(),
            response_times: Vec::new(),
        }
    }

    /// Record a successful request
    pub fn record_success(&mut self, response_time: Duration, bytes_sent: u64, bytes_received: u64) {
        let response_time_ms = response_time.as_secs_f64() * 1000.0;

        self.metrics.total_requests += 1;
        self.metrics.successful_requests += 1;
        self.metrics.total_bytes_sent += bytes_sent;
        self.metrics.total_bytes_received += bytes_received;

        self.response_times.push(response_time_ms);
        self.update_response_time_stats();

        debug!("Recorded successful request: {}ms, {} bytes sent, {} bytes received",
               response_time_ms, bytes_sent, bytes_received);
    }

    /// Record a failed request
    pub fn record_failure(&mut self, response_time: Duration) {
        let response_time_ms = response_time.as_secs_f64() * 1000.0;

        self.metrics.total_requests += 1;
        self.metrics.failed_requests += 1;

        self.response_times.push(response_time_ms);
        self.update_response_time_stats();

        debug!("Recorded failed request: {}ms", response_time_ms);
    }

    /// Update cache metrics
    pub fn update_cache_metrics(&mut self, cache_stats: &CacheStats) {
        self.metrics.cache_hit_rate = cache_stats.hit_rate();
        debug!("Updated cache hit rate: {:.2}%", self.metrics.cache_hit_rate * 100.0);
    }

    /// Update connection pool metrics
    pub fn update_connection_metrics(&mut self, connection_stats: &ConnectionStats) {
        if connection_stats.total_connections > 0 {
            self.metrics.connection_reuse_rate =
                connection_stats.connection_reuses as f64 / connection_stats.total_connections as f64;
        }
        debug!("Updated connection reuse rate: {:.2}%", self.metrics.connection_reuse_rate * 100.0);
    }

    /// Get current performance metrics
    pub fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.clone()
    }

    /// Get uptime duration
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Reset all metrics
    pub fn reset(&mut self) {
        info!("Resetting performance metrics");
        self.start_time = Instant::now();
        self.metrics = PerformanceMetrics::default();
        self.response_times.clear();
    }

    /// Generate performance report
    pub fn generate_report(&self) -> String {
        let uptime = self.uptime();
        let metrics = &self.metrics;

        format!(
            "HTTP Performance Report\n\
             ======================\n\
             Uptime: {:.2} seconds\n\
             Total Requests: {}\n\
             Successful Requests: {}\n\
             Failed Requests: {}\n\
             Success Rate: {:.2}%\n\
             Average Response Time: {:.2}ms\n\
             Min Response Time: {:.2}ms\n\
             Max Response Time: {:.2}ms\n\
             Throughput: {:.2} req/sec\n\
             Total Bytes Sent: {} bytes\n\
             Total Bytes Received: {} bytes\n\
             Cache Hit Rate: {:.2}%\n\
             Connection Reuse Rate: {:.2}%\n",
            uptime.as_secs_f64(),
            metrics.total_requests,
            metrics.successful_requests,
            metrics.failed_requests,
            metrics.success_rate() * 100.0,
            metrics.avg_response_time_ms,
            metrics.min_response_time_ms,
            metrics.max_response_time_ms,
            metrics.throughput(uptime),
            metrics.total_bytes_sent,
            metrics.total_bytes_received,
            metrics.cache_hit_rate * 100.0,
            metrics.connection_reuse_rate * 100.0
        )
    }

    // Private helper methods

    fn update_response_time_stats(&mut self) {
        if self.response_times.is_empty() {
            return;
        }

        let sum: f64 = self.response_times.iter().sum();
        self.metrics.avg_response_time_ms = sum / self.response_times.len() as f64;

        self.metrics.min_response_time_ms = self.response_times.iter()
            .copied()
            .fold(f64::INFINITY, f64::min);

        self.metrics.max_response_time_ms = self.response_times.iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance optimization recommendations
pub struct PerformanceOptimizer;

impl PerformanceOptimizer {
    /// Analyze metrics and provide optimization recommendations
    pub fn analyze_and_recommend(metrics: &PerformanceMetrics) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Check success rate
        if metrics.success_rate() < 0.95 {
            recommendations.push(format!(
                "Low success rate ({:.1}%). Consider implementing retry logic and better error handling.",
                metrics.success_rate() * 100.0
            ));
        }

        // Check response times
        if metrics.avg_response_time_ms > 1000.0 {
            recommendations.push(format!(
                "High average response time ({:.1}ms). Consider optimizing request processing or adding caching.",
                metrics.avg_response_time_ms
            ));
        }

        // Check cache hit rate
        if metrics.cache_hit_rate < 0.5 {
            recommendations.push(format!(
                "Low cache hit rate ({:.1}%). Consider adjusting cache TTL or improving cache key strategy.",
                metrics.cache_hit_rate * 100.0
            ));
        }

        // Check connection reuse
        if metrics.connection_reuse_rate < 0.7 {
            recommendations.push(format!(
                "Low connection reuse rate ({:.1}%). Consider increasing connection pool size or TTL.",
                metrics.connection_reuse_rate * 100.0
            ));
        }

        // Check for high variance in response times
        let variance_threshold = metrics.avg_response_time_ms * 2.0;
        if metrics.max_response_time_ms > variance_threshold {
            recommendations.push(format!(
                "High response time variance (max: {:.1}ms vs avg: {:.1}ms). Consider load balancing or request queuing.",
                metrics.max_response_time_ms,
                metrics.avg_response_time_ms
            ));
        }

        if recommendations.is_empty() {
            recommendations.push("Performance metrics look good! No specific recommendations at this time.".to_string());
        }

        recommendations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_monitor() {
        let mut monitor = PerformanceMonitor::new();

        // Record some requests
        monitor.record_success(Duration::from_millis(100), 1024, 2048);
        monitor.record_success(Duration::from_millis(200), 512, 1024);
        monitor.record_failure(Duration::from_millis(500));

        let metrics = monitor.get_metrics();
        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.successful_requests, 2);
        assert_eq!(metrics.failed_requests, 1);
        assert!((metrics.success_rate() - 0.6667).abs() < 0.001);
        assert_eq!(metrics.total_bytes_sent, 1536);
        assert_eq!(metrics.total_bytes_received, 3072);
    }

    #[test]
    fn test_performance_optimizer() {
        let metrics = PerformanceMetrics {
            total_requests: 100,
            successful_requests: 90,
            avg_response_time_ms: 1500.0,
            cache_hit_rate: 0.3,
            connection_reuse_rate: 0.5,
            ..Default::default()
        };

        let recommendations = PerformanceOptimizer::analyze_and_recommend(&metrics);
        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|r| r.contains("response time")));
        assert!(recommendations.iter().any(|r| r.contains("cache hit rate")));
        assert!(recommendations.iter().any(|r| r.contains("connection reuse")));
    }
}
