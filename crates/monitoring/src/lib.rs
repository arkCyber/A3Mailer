//! # A3Mailer Monitoring and Observability
//!
//! Production-grade monitoring, metrics collection, and observability
//! for A3Mailer email server with AI and Web3 integration.
//!
//! ## Features
//!
//! - **Prometheus Metrics**: Comprehensive metrics collection
//! - **Distributed Tracing**: Request tracing across services
//! - **Health Checks**: System health monitoring
//! - **Performance Monitoring**: Real-time performance metrics
//! - **AI/ML Metrics**: Machine learning model performance
//! - **Web3 Metrics**: Blockchain and DID operation metrics
//!
//! ## Architecture
//!
//! The monitoring system consists of:
//! - Metrics Collector: Prometheus-compatible metrics
//! - Health Monitor: System health checks
//! - Performance Tracker: Real-time performance monitoring
//! - Alert Manager: Threshold-based alerting
//!
//! ## Example
//!
//! ```rust,no_run
//! use a3mailer_monitoring::{MonitoringManager, MonitoringConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = MonitoringConfig::default();
//!     let monitoring = MonitoringManager::new(config).await?;
//!
//!     // Record a metric
//!     monitoring.record_email_processed("smtp").await;
//!
//!     // Check system health
//!     let health = monitoring.get_health_status().await?;
//!
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

pub mod metrics;
pub mod health;
pub mod performance;
pub mod alerts;
pub mod error;

pub use error::{MonitoringError, Result};

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub metrics_port: u16,
    pub health_check_interval: u64,
    pub performance_sampling_rate: f64,
    pub alert_thresholds: AlertThresholds,
    pub prometheus_endpoint: String,
    pub grafana_endpoint: String,
}

/// Alert threshold configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub email_processing_latency_ms: u64,
    pub ai_inference_latency_ms: u64,
    pub web3_operation_latency_ms: u64,
    pub error_rate_percent: f64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_port: 9090,
            health_check_interval: 30,
            performance_sampling_rate: 1.0,
            alert_thresholds: AlertThresholds::default(),
            prometheus_endpoint: "http://localhost:9090".to_string(),
            grafana_endpoint: "http://localhost:3000".to_string(),
        }
    }
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 80.0,
            memory_usage_percent: 85.0,
            disk_usage_percent: 90.0,
            email_processing_latency_ms: 1000,
            ai_inference_latency_ms: 10,
            web3_operation_latency_ms: 5000,
            error_rate_percent: 5.0,
        }
    }
}

/// System health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub overall_status: HealthState,
    pub components: HashMap<String, ComponentHealth>,
    pub last_updated: DateTime<Utc>,
}

/// Health state enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Component health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: HealthState,
    pub message: String,
    pub last_check: DateTime<Utc>,
    pub response_time_ms: u64,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub memory_usage_percent: f64,
    pub disk_usage_bytes: u64,
    pub disk_usage_percent: f64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub active_connections: u64,
    pub emails_processed_per_second: f64,
    pub ai_inference_latency_ms: f64,
    pub web3_operation_latency_ms: f64,
    pub timestamp: DateTime<Utc>,
}

/// Alert information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub severity: AlertSeverity,
    pub title: String,
    pub description: String,
    pub component: String,
    pub metric_name: String,
    pub current_value: f64,
    pub threshold_value: f64,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Main monitoring manager
pub struct MonitoringManager {
    config: MonitoringConfig,
    metrics_collector: Arc<RwLock<metrics::MetricsCollector>>,
    health_monitor: Arc<RwLock<health::HealthMonitor>>,
    performance_tracker: Arc<RwLock<performance::PerformanceTracker>>,
    alert_manager: Arc<RwLock<alerts::AlertManager>>,
    start_time: Instant,
}

impl MonitoringManager {
    /// Create a new monitoring manager
    pub async fn new(config: MonitoringConfig) -> Result<Self> {
        info!("Initializing monitoring and observability system");

        if !config.enabled {
            warn!("Monitoring is disabled");
        }

        // Initialize components
        let metrics_collector = Arc::new(RwLock::new(
            metrics::MetricsCollector::new(&config).await?
        ));

        let health_monitor = Arc::new(RwLock::new(
            health::HealthMonitor::new(&config).await?
        ));

        let performance_tracker = Arc::new(RwLock::new(
            performance::PerformanceTracker::new(&config).await?
        ));

        let alert_manager = Arc::new(RwLock::new(
            alerts::AlertManager::new(&config).await?
        ));

        let start_time = Instant::now();

        info!("Monitoring system initialized successfully");

        let manager = Self {
            config,
            metrics_collector,
            health_monitor,
            performance_tracker,
            alert_manager,
            start_time,
        };

        // Start background monitoring tasks
        manager.start_background_tasks().await?;

        Ok(manager)
    }

    /// Start background monitoring tasks
    async fn start_background_tasks(&self) -> Result<()> {
        info!("Starting background monitoring tasks");

        // Health check task
        let health_monitor = Arc::clone(&self.health_monitor);
        let health_interval = self.config.health_check_interval;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(health_interval));
            loop {
                interval.tick().await;
                if let Err(e) = health_monitor.write().await.run_health_checks().await {
                    error!("Health check failed: {}", e);
                }
            }
        });

        // Performance monitoring task
        let performance_tracker = Arc::clone(&self.performance_tracker);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                if let Err(e) = performance_tracker.write().await.collect_metrics().await {
                    error!("Performance metrics collection failed: {}", e);
                }
            }
        });

        // Alert processing task
        let alert_manager = Arc::clone(&self.alert_manager);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                if let Err(e) = alert_manager.write().await.process_alerts().await {
                    error!("Alert processing failed: {}", e);
                }
            }
        });

        info!("Background monitoring tasks started");
        Ok(())
    }

    /// Record email processed metric
    pub async fn record_email_processed(&self, protocol: &str) -> Result<()> {
        let metrics_collector = self.metrics_collector.read().await;
        metrics_collector.increment_counter("emails_processed_total", &[("protocol", protocol)]).await?;
        Ok(())
    }

    /// Record AI inference metric
    pub async fn record_ai_inference(&self, model: &str, latency_ms: u64) -> Result<()> {
        let metrics_collector = self.metrics_collector.read().await;
        metrics_collector.record_histogram("ai_inference_duration_ms", latency_ms as f64, &[("model", model)]).await?;
        Ok(())
    }

    /// Record Web3 operation metric
    pub async fn record_web3_operation(&self, operation: &str, latency_ms: u64, success: bool) -> Result<()> {
        let metrics_collector = self.metrics_collector.read().await;
        let status = if success { "success" } else { "failure" };
        metrics_collector.record_histogram("web3_operation_duration_ms", latency_ms as f64, &[("operation", operation), ("status", status)]).await?;
        Ok(())
    }

    /// Get system health status
    pub async fn get_health_status(&self) -> Result<HealthStatus> {
        let health_monitor = self.health_monitor.read().await;
        health_monitor.get_health_status().await
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> Result<PerformanceMetrics> {
        let performance_tracker = self.performance_tracker.read().await;
        performance_tracker.get_current_metrics().await
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Result<Vec<Alert>> {
        let alert_manager = self.alert_manager.read().await;
        alert_manager.get_active_alerts().await
    }

    /// Get system uptime
    pub fn get_uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get monitoring statistics
    pub async fn get_monitoring_stats(&self) -> Result<HashMap<String, String>> {
        let mut stats = HashMap::new();
        
        stats.insert("enabled".to_string(), self.config.enabled.to_string());
        stats.insert("uptime_seconds".to_string(), self.get_uptime().as_secs().to_string());
        
        // Get component statistics
        let health_stats = self.health_monitor.read().await.get_stats().await?;
        let metrics_stats = self.metrics_collector.read().await.get_stats().await?;
        let performance_stats = self.performance_tracker.read().await.get_stats().await?;
        let alert_stats = self.alert_manager.read().await.get_stats().await?;
        
        stats.extend(health_stats);
        stats.extend(metrics_stats);
        stats.extend(performance_stats);
        stats.extend(alert_stats);
        
        Ok(stats)
    }

    /// Shutdown monitoring system
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down monitoring system");
        
        // Shutdown components
        self.metrics_collector.write().await.shutdown().await?;
        self.health_monitor.write().await.shutdown().await?;
        self.performance_tracker.write().await.shutdown().await?;
        self.alert_manager.write().await.shutdown().await?;
        
        info!("Monitoring system shutdown complete");
        Ok(())
    }
}

/// Initialize monitoring system
pub async fn init_monitoring(config: MonitoringConfig) -> Result<MonitoringManager> {
    info!("Initializing A3Mailer monitoring system");
    
    let manager = MonitoringManager::new(config).await?;
    
    info!("A3Mailer monitoring system initialized successfully");
    Ok(manager)
}
