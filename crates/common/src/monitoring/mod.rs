/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Monitoring and Metrics Module
//!
//! This module provides comprehensive monitoring capabilities including metrics collection,
//! health checks, performance monitoring, and alerting for production environments.

pub mod metrics;
pub mod health;
pub mod alerts;
pub mod collectors;

#[cfg(test)]
mod test_utils;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use serde::{Serialize, Deserialize};
use tracing::{debug, info};

/// Monitoring configuration
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    /// Enable monitoring
    pub enabled: bool,
    /// Metrics collection interval
    pub collection_interval: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Metrics retention period
    pub retention_period: Duration,
    /// Enable Prometheus metrics export
    pub enable_prometheus: bool,
    /// Prometheus metrics port
    pub prometheus_port: u16,
    /// Enable health check endpoint
    pub enable_health_endpoint: bool,
    /// Health check endpoint path
    pub health_endpoint_path: String,
    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,
    /// Custom metrics configuration
    pub custom_metrics: Vec<CustomMetricConfig>,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(10),
            retention_period: Duration::from_secs(24 * 3600), // 24 hours
            enable_prometheus: true,
            prometheus_port: 9090,
            enable_health_endpoint: true,
            health_endpoint_path: "/health".to_string(),
            alert_thresholds: AlertThresholds::default(),
            custom_metrics: Vec::new(),
        }
    }
}

/// Alert threshold configuration
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    /// CPU usage threshold (percentage)
    pub cpu_usage_threshold: f64,
    /// Memory usage threshold (percentage)
    pub memory_usage_threshold: f64,
    /// Disk usage threshold (percentage)
    pub disk_usage_threshold: f64,
    /// Error rate threshold (percentage)
    pub error_rate_threshold: f64,
    /// Response time threshold (milliseconds)
    pub response_time_threshold: u64,
    /// Connection count threshold
    pub connection_count_threshold: u32,
    /// Queue size threshold
    pub queue_size_threshold: u32,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            cpu_usage_threshold: 80.0,
            memory_usage_threshold: 85.0,
            disk_usage_threshold: 90.0,
            error_rate_threshold: 5.0,
            response_time_threshold: 5000,
            connection_count_threshold: 1000,
            queue_size_threshold: 10000,
        }
    }
}

/// Custom metric configuration
#[derive(Debug, Clone)]
pub struct CustomMetricConfig {
    pub name: String,
    pub description: String,
    pub metric_type: MetricType,
    pub labels: Vec<String>,
}

/// Metric types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// System metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// Timestamp when metrics were collected
    pub timestamp: u64,
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// Total memory in bytes
    pub memory_total: u64,
    /// Disk usage in bytes
    pub disk_usage: u64,
    /// Total disk space in bytes
    pub disk_total: u64,
    /// Network bytes received
    pub network_rx_bytes: u64,
    /// Network bytes transmitted
    pub network_tx_bytes: u64,
    /// Load average (1 minute)
    pub load_average_1m: f64,
    /// Load average (5 minutes)
    pub load_average_5m: f64,
    /// Load average (15 minutes)
    pub load_average_15m: f64,
    /// Number of active connections
    pub active_connections: u32,
    /// Process uptime in seconds
    pub uptime: u64,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            cpu_usage: 0.0,
            memory_usage: 0,
            memory_total: 0,
            disk_usage: 0,
            disk_total: 0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            load_average_1m: 0.0,
            load_average_5m: 0.0,
            load_average_15m: 0.0,
            active_connections: 0,
            uptime: 0,
        }
    }
}

/// Application metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationMetrics {
    /// Timestamp when metrics were collected
    pub timestamp: u64,
    /// Total requests processed
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average response time in milliseconds
    pub avg_response_time: f64,
    /// 95th percentile response time
    pub p95_response_time: f64,
    /// 99th percentile response time
    pub p99_response_time: f64,
    /// Requests per second
    pub requests_per_second: f64,
    /// Active sessions
    pub active_sessions: u32,
    /// Queue sizes
    pub queue_sizes: HashMap<String, u32>,
    /// Cache hit rates
    pub cache_hit_rates: HashMap<String, f64>,
    /// Database connection pool stats
    pub db_pool_stats: DatabasePoolStats,
}

impl Default for ApplicationMetrics {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_response_time: 0.0,
            p95_response_time: 0.0,
            p99_response_time: 0.0,
            requests_per_second: 0.0,
            active_sessions: 0,
            queue_sizes: HashMap::new(),
            cache_hit_rates: HashMap::new(),
            db_pool_stats: DatabasePoolStats::default(),
        }
    }
}

/// Database connection pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabasePoolStats {
    /// Total connections in pool
    pub total_connections: u32,
    /// Active connections
    pub active_connections: u32,
    /// Idle connections
    pub idle_connections: u32,
    /// Connection wait time in milliseconds
    pub avg_wait_time: f64,
    /// Connection errors
    pub connection_errors: u64,
}

impl Default for DatabasePoolStats {
    fn default() -> Self {
        Self {
            total_connections: 0,
            active_connections: 0,
            idle_connections: 0,
            avg_wait_time: 0.0,
            connection_errors: 0,
        }
    }
}

/// Health status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
            HealthStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Component name
    pub component: String,
    /// Health status
    pub status: HealthStatus,
    /// Status message
    pub message: String,
    /// Check timestamp
    pub timestamp: u64,
    /// Response time in milliseconds
    pub response_time: u64,
    /// Additional details
    pub details: HashMap<String, String>,
}

impl HealthCheck {
    /// Create a new health check result
    pub fn new(component: String, status: HealthStatus, message: String) -> Self {
        Self {
            component,
            status,
            message,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            response_time: 0,
            details: HashMap::new(),
        }
    }

    /// Add response time
    pub fn with_response_time(mut self, response_time: Duration) -> Self {
        self.response_time = response_time.as_millis() as u64;
        self
    }

    /// Add detail
    pub fn with_detail(mut self, key: String, value: String) -> Self {
        self.details.insert(key, value);
        self
    }
}

/// Main monitoring manager
pub struct MonitoringManager {
    config: MonitoringConfig,
    system_metrics: Arc<RwLock<Vec<SystemMetrics>>>,
    app_metrics: Arc<RwLock<Vec<ApplicationMetrics>>>,
    health_checks: Arc<RwLock<Vec<HealthCheck>>>,
    custom_metrics: Arc<RwLock<HashMap<String, f64>>>,
    start_time: Instant,
}

impl MonitoringManager {
    /// Create a new monitoring manager
    pub fn new(config: MonitoringConfig) -> Self {
        info!("Initializing monitoring manager with config: {:?}", config);
        Self {
            config,
            system_metrics: Arc::new(RwLock::new(Vec::new())),
            app_metrics: Arc::new(RwLock::new(Vec::new())),
            health_checks: Arc::new(RwLock::new(Vec::new())),
            custom_metrics: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    /// Record system metrics
    pub fn record_system_metrics(&self, metrics: SystemMetrics) {
        if !self.config.enabled {
            return;
        }

        debug!("Recording system metrics: CPU: {:.1}%, Memory: {} MB",
               metrics.cpu_usage, metrics.memory_usage / 1024 / 1024);

        let mut system_metrics = self.system_metrics.write().unwrap();
        system_metrics.push(metrics);

        // Keep only recent metrics
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() - self.config.retention_period.as_secs();

        system_metrics.retain(|m| m.timestamp > cutoff_time);
    }

    /// Record application metrics
    pub fn record_app_metrics(&self, metrics: ApplicationMetrics) {
        if !self.config.enabled {
            return;
        }

        debug!("Recording app metrics: RPS: {:.1}, Avg RT: {:.1}ms",
               metrics.requests_per_second, metrics.avg_response_time);

        let mut app_metrics = self.app_metrics.write().unwrap();
        app_metrics.push(metrics);

        // Keep only recent metrics
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() - self.config.retention_period.as_secs();

        app_metrics.retain(|m| m.timestamp > cutoff_time);
    }

    /// Record health check result
    pub fn record_health_check(&self, health_check: HealthCheck) {
        if !self.config.enabled {
            return;
        }

        debug!("Recording health check: {} - {}",
               health_check.component, health_check.status);

        let mut health_checks = self.health_checks.write().unwrap();
        health_checks.push(health_check);

        // Keep only recent health checks
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() - self.config.retention_period.as_secs();

        health_checks.retain(|h| h.timestamp > cutoff_time);
    }

    /// Set custom metric value
    pub fn set_custom_metric(&self, name: String, value: f64) {
        if !self.config.enabled {
            return;
        }

        debug!("Setting custom metric: {} = {}", name, value);
        let mut custom_metrics = self.custom_metrics.write().unwrap();
        custom_metrics.insert(name, value);
    }

    /// Get latest system metrics
    pub fn get_latest_system_metrics(&self) -> Option<SystemMetrics> {
        let system_metrics = self.system_metrics.read().unwrap();
        system_metrics.last().cloned()
    }

    /// Get latest application metrics
    pub fn get_latest_app_metrics(&self) -> Option<ApplicationMetrics> {
        let app_metrics = self.app_metrics.read().unwrap();
        app_metrics.last().cloned()
    }

    /// Get latest health checks
    pub fn get_latest_health_checks(&self) -> Vec<HealthCheck> {
        let health_checks = self.health_checks.read().unwrap();
        health_checks.iter().rev().take(10).cloned().collect()
    }

    /// Get overall health status
    pub fn get_overall_health_status(&self) -> HealthStatus {
        let health_checks = self.health_checks.read().unwrap();

        if health_checks.is_empty() {
            return HealthStatus::Unknown;
        }

        // Get latest health check for each component
        let mut latest_checks: HashMap<String, &HealthCheck> = HashMap::new();
        for check in health_checks.iter().rev() {
            latest_checks.entry(check.component.clone()).or_insert(check);
        }

        let mut healthy_count = 0;
        let mut degraded_count = 0;
        let mut unhealthy_count = 0;

        for check in latest_checks.values() {
            match check.status {
                HealthStatus::Healthy => healthy_count += 1,
                HealthStatus::Degraded => degraded_count += 1,
                HealthStatus::Unhealthy => unhealthy_count += 1,
                HealthStatus::Unknown => {}
            }
        }

        if unhealthy_count > 0 {
            HealthStatus::Unhealthy
        } else if degraded_count > 0 {
            HealthStatus::Degraded
        } else if healthy_count > 0 {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unknown
        }
    }

    /// Get uptime in seconds
    pub fn get_uptime(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Get configuration
    pub fn get_config(&self) -> &MonitoringConfig {
        &self.config
    }

    /// Cleanup old data
    pub fn cleanup(&self) {
        debug!("Cleaning up old monitoring data");

        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() - self.config.retention_period.as_secs();

        // Clean up system metrics
        let mut system_metrics = self.system_metrics.write().unwrap();
        let initial_count = system_metrics.len();
        system_metrics.retain(|m| m.timestamp > cutoff_time);
        let removed_count = initial_count - system_metrics.len();

        if removed_count > 0 {
            info!("Cleaned up {} old system metrics", removed_count);
        }

        // Clean up application metrics
        let mut app_metrics = self.app_metrics.write().unwrap();
        let initial_count = app_metrics.len();
        app_metrics.retain(|m| m.timestamp > cutoff_time);
        let removed_count = initial_count - app_metrics.len();

        if removed_count > 0 {
            info!("Cleaned up {} old application metrics", removed_count);
        }

        // Clean up health checks
        let mut health_checks = self.health_checks.write().unwrap();
        let initial_count = health_checks.len();
        health_checks.retain(|h| h.timestamp > cutoff_time);
        let removed_count = initial_count - health_checks.len();

        if removed_count > 0 {
            info!("Cleaned up {} old health checks", removed_count);
        }
    }
}

impl Default for MonitoringManager {
    fn default() -> Self {
        Self::new(MonitoringConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitoring_manager_creation() {
        let manager = MonitoringManager::default();
        assert!(manager.get_config().enabled);
        assert_eq!(manager.get_uptime(), 0); // Should be very small
    }

    #[test]
    fn test_system_metrics_recording() {
        let manager = MonitoringManager::default();

        let metrics = SystemMetrics {
            cpu_usage: 50.0,
            memory_usage: 1024 * 1024 * 1024, // 1GB
            ..Default::default()
        };

        manager.record_system_metrics(metrics.clone());

        let latest = manager.get_latest_system_metrics().unwrap();
        assert_eq!(latest.cpu_usage, 50.0);
        assert_eq!(latest.memory_usage, 1024 * 1024 * 1024);
    }

    #[test]
    fn test_health_status_calculation() {
        let manager = MonitoringManager::default();

        // Initially unknown
        assert_eq!(manager.get_overall_health_status(), HealthStatus::Unknown);

        // Add healthy check
        let healthy_check = HealthCheck::new(
            "database".to_string(),
            HealthStatus::Healthy,
            "Database is responding".to_string(),
        );
        manager.record_health_check(healthy_check);

        assert_eq!(manager.get_overall_health_status(), HealthStatus::Healthy);

        // Add unhealthy check
        let unhealthy_check = HealthCheck::new(
            "cache".to_string(),
            HealthStatus::Unhealthy,
            "Cache is down".to_string(),
        );
        manager.record_health_check(unhealthy_check);

        assert_eq!(manager.get_overall_health_status(), HealthStatus::Unhealthy);
    }

    #[test]
    fn test_custom_metrics() {
        let manager = MonitoringManager::default();

        manager.set_custom_metric("custom.counter".to_string(), 42.0);

        let custom_metrics = manager.custom_metrics.read().unwrap();
        assert_eq!(custom_metrics.get("custom.counter"), Some(&42.0));
    }
}
