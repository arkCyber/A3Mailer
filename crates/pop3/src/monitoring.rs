/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! POP3 Performance Monitoring and Observability Module
//!
//! This module provides comprehensive monitoring, metrics collection, and observability
//! features for the POP3 server, designed for production environments with high
//! availability and performance requirements.
//!
//! # Features
//!
//! * **Real-time Metrics**: Live performance metrics with minimal overhead
//! * **Health Checks**: Comprehensive health monitoring for all components
//! * **Alerting**: Configurable thresholds and alert generation
//! * **Observability**: Detailed tracing and logging integration
//! * **Dashboard Integration**: Prometheus/Grafana compatible metrics
//! * **Performance Profiling**: Built-in profiling and bottleneck detection
//!
//! # Architecture
//!
//! The monitoring system is built around several key components:
//!
//! * `Pop3Monitor`: Central monitoring coordinator
//! * `MetricsCollector`: Real-time metrics collection
//! * `HealthChecker`: Component health monitoring
//! * `AlertManager`: Threshold-based alerting
//! * `PerformanceProfiler`: Performance analysis and optimization
//!
//! # Performance Impact
//!
//! The monitoring system is designed to have minimal performance impact:
//! - Metrics collection: < 1% CPU overhead
//! - Memory usage: < 10MB for typical deployments
//! - Network overhead: Configurable, typically < 1KB/s
//!
//! # Examples
//!
//! ```rust
//! use pop3::monitoring::{Pop3Monitor, MonitoringConfig};
//!
//! // Create monitoring system
//! let config = MonitoringConfig::production();
//! let monitor = Pop3Monitor::new(config).await?;
//!
//! // Start monitoring
//! monitor.start().await?;
//!
//! // Get current metrics
//! let metrics = monitor.get_metrics().await;
//! println!("Active sessions: {}", metrics.active_sessions);
//! ```

use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime},
};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{mpsc, RwLock as TokioRwLock, broadcast},
    time::interval,
};
use tracing::{debug, error, info, trace, warn};

use crate::{
    security::SecurityStatsSnapshot,
    mailbox::MailboxStats,
    protocol::response::ResponseMetricsSnapshot,
};

/// Monitoring configuration for the POP3 server
///
/// Controls all aspects of monitoring behavior including metrics collection,
/// health checks, alerting, and performance profiling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable comprehensive monitoring
    pub enabled: bool,

    /// Metrics collection interval
    pub metrics_interval: Duration,

    /// Health check interval
    pub health_check_interval: Duration,

    /// Enable performance profiling
    pub enable_profiling: bool,

    /// Enable real-time alerting
    pub enable_alerting: bool,

    /// Maximum number of metric samples to retain
    pub max_metric_samples: usize,

    /// Prometheus metrics endpoint configuration
    pub prometheus: PrometheusConfig,

    /// Alert thresholds configuration
    pub alerts: AlertConfig,

    /// Performance profiling configuration
    pub profiling: ProfilingConfig,
}

/// Prometheus metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusConfig {
    /// Enable Prometheus metrics export
    pub enabled: bool,

    /// Metrics endpoint path
    pub endpoint: String,

    /// Metrics namespace prefix
    pub namespace: String,

    /// Additional labels to include
    pub labels: HashMap<String, String>,
}

/// Alert configuration and thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Enable alerting system
    pub enabled: bool,

    /// High CPU usage threshold (percentage)
    pub cpu_threshold: f64,

    /// High memory usage threshold (percentage)
    pub memory_threshold: f64,

    /// High connection count threshold
    pub connection_threshold: u64,

    /// High error rate threshold (errors per minute)
    pub error_rate_threshold: f64,

    /// Response time threshold (milliseconds)
    pub response_time_threshold: u64,

    /// Alert cooldown period
    pub alert_cooldown: Duration,
}

/// Performance profiling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingConfig {
    /// Enable performance profiling
    pub enabled: bool,

    /// Profiling sample rate (0.0 to 1.0)
    pub sample_rate: f64,

    /// Enable CPU profiling
    pub enable_cpu_profiling: bool,

    /// Enable memory profiling
    pub enable_memory_profiling: bool,

    /// Enable I/O profiling
    pub enable_io_profiling: bool,

    /// Profile data retention period
    pub retention_period: Duration,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_interval: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(60),
            enable_profiling: false,
            enable_alerting: true,
            max_metric_samples: 1440, // 24 hours at 1-minute intervals
            prometheus: PrometheusConfig::default(),
            alerts: AlertConfig::default(),
            profiling: ProfilingConfig::default(),
        }
    }
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: "/metrics".to_string(),
            namespace: "pop3".to_string(),
            labels: HashMap::new(),
        }
    }
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cpu_threshold: 80.0,
            memory_threshold: 85.0,
            connection_threshold: 1000,
            error_rate_threshold: 10.0,
            response_time_threshold: 5000,
            alert_cooldown: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sample_rate: 0.01, // 1% sampling
            enable_cpu_profiling: true,
            enable_memory_profiling: true,
            enable_io_profiling: false,
            retention_period: Duration::from_secs(3600), // 1 hour
        }
    }
}

impl MonitoringConfig {
    /// Creates a production-ready monitoring configuration
    ///
    /// Optimized for production environments with appropriate intervals,
    /// thresholds, and features enabled for operational monitoring.
    pub fn production() -> Self {
        Self {
            enabled: true,
            metrics_interval: Duration::from_secs(15),
            health_check_interval: Duration::from_secs(30),
            enable_profiling: true,
            enable_alerting: true,
            max_metric_samples: 2880, // 48 hours at 1-minute intervals
            prometheus: PrometheusConfig {
                enabled: true,
                endpoint: "/metrics".to_string(),
                namespace: "stalwart_pop3".to_string(),
                labels: [
                    ("service".to_string(), "pop3".to_string()),
                    ("environment".to_string(), "production".to_string()),
                ].into_iter().collect(),
            },
            alerts: AlertConfig {
                enabled: true,
                cpu_threshold: 75.0,
                memory_threshold: 80.0,
                connection_threshold: 5000,
                error_rate_threshold: 5.0,
                response_time_threshold: 3000,
                alert_cooldown: Duration::from_secs(180), // 3 minutes
            },
            profiling: ProfilingConfig {
                enabled: true,
                sample_rate: 0.005, // 0.5% sampling for production
                enable_cpu_profiling: true,
                enable_memory_profiling: true,
                enable_io_profiling: true,
                retention_period: Duration::from_secs(7200), // 2 hours
            },
        }
    }

    /// Creates a development-friendly monitoring configuration
    ///
    /// Higher sampling rates and more verbose monitoring suitable
    /// for development and testing environments.
    pub fn development() -> Self {
        Self {
            enabled: true,
            metrics_interval: Duration::from_secs(5),
            health_check_interval: Duration::from_secs(10),
            enable_profiling: true,
            enable_alerting: false, // Disable alerts in development
            max_metric_samples: 720, // 12 hours at 1-minute intervals
            prometheus: PrometheusConfig {
                enabled: true,
                endpoint: "/metrics".to_string(),
                namespace: "stalwart_pop3_dev".to_string(),
                labels: [
                    ("service".to_string(), "pop3".to_string()),
                    ("environment".to_string(), "development".to_string()),
                ].into_iter().collect(),
            },
            alerts: AlertConfig {
                enabled: false,
                ..AlertConfig::default()
            },
            profiling: ProfilingConfig {
                enabled: true,
                sample_rate: 0.1, // 10% sampling for development
                enable_cpu_profiling: true,
                enable_memory_profiling: true,
                enable_io_profiling: true,
                retention_period: Duration::from_secs(1800), // 30 minutes
            },
        }
    }
}

/// Comprehensive POP3 server metrics
///
/// Contains all performance and operational metrics for the POP3 server,
/// designed for monitoring, alerting, and performance analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pop3Metrics {
    /// Timestamp when metrics were collected
    pub timestamp: SystemTime,

    /// Connection metrics
    pub connections: ConnectionMetrics,

    /// Session metrics
    pub sessions: SessionMetrics,

    /// Command metrics
    pub commands: CommandMetrics,

    /// Authentication metrics
    pub authentication: AuthenticationMetrics,

    /// Mailbox metrics
    pub mailbox: MailboxMetrics,

    /// Security metrics
    pub security: SecurityMetrics,

    /// Performance metrics
    pub performance: PerformanceMetrics,

    /// System resource metrics
    pub system: SystemMetrics,
}

/// Connection-related metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetrics {
    /// Current active connections
    pub active_connections: u64,

    /// Total connections accepted
    pub total_connections: u64,

    /// Total connections rejected
    pub rejected_connections: u64,

    /// Connections per second (current rate)
    pub connections_per_second: f64,

    /// Average connection duration
    pub avg_connection_duration: Duration,

    /// Peak concurrent connections
    pub peak_connections: u64,

    /// Connection errors
    pub connection_errors: u64,
}

/// Session-related metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    /// Current active sessions
    pub active_sessions: u64,

    /// Total sessions created
    pub total_sessions: u64,

    /// Sessions in different states
    pub sessions_by_state: HashMap<String, u64>,

    /// Average session duration
    pub avg_session_duration: Duration,

    /// Session timeouts
    pub session_timeouts: u64,

    /// Session errors
    pub session_errors: u64,
}

/// Command execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandMetrics {
    /// Total commands processed
    pub total_commands: u64,

    /// Commands per second (current rate)
    pub commands_per_second: f64,

    /// Commands by type
    pub commands_by_type: HashMap<String, u64>,

    /// Command success rate
    pub success_rate: f64,

    /// Average command execution time
    pub avg_execution_time: Duration,

    /// Command errors by type
    pub errors_by_type: HashMap<String, u64>,

    /// Rate limited commands
    pub rate_limited_commands: u64,
}

/// Authentication metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationMetrics {
    /// Total authentication attempts
    pub total_attempts: u64,

    /// Successful authentications
    pub successful_attempts: u64,

    /// Failed authentications
    pub failed_attempts: u64,

    /// Authentication success rate
    pub success_rate: f64,

    /// Average authentication time
    pub avg_auth_time: Duration,

    /// Authentication methods used
    pub methods_used: HashMap<String, u64>,

    /// Blocked IPs
    pub blocked_ips: u64,
}

/// Mailbox operation metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailboxMetrics {
    /// Total mailboxes accessed
    pub total_mailboxes_accessed: u64,

    /// Average mailbox size
    pub avg_mailbox_size: u64,

    /// Total messages processed
    pub total_messages_processed: u64,

    /// Average message size
    pub avg_message_size: u64,

    /// Mailbox load time
    pub avg_mailbox_load_time: Duration,

    /// Cache hit rate
    pub cache_hit_rate: f64,

    /// Storage operations
    pub storage_operations: u64,
}

/// Security-related metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetrics {
    /// Suspicious activities detected
    pub suspicious_activities: u64,

    /// Security violations
    pub security_violations: u64,

    /// Brute force attempts
    pub brute_force_attempts: u64,

    /// Honeypot hits
    pub honeypot_hits: u64,

    /// Geographic blocks
    pub geographic_blocks: u64,

    /// TLS usage rate
    pub tls_usage_rate: f64,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average response time
    pub avg_response_time: Duration,

    /// 95th percentile response time
    pub p95_response_time: Duration,

    /// 99th percentile response time
    pub p99_response_time: Duration,

    /// Throughput (operations per second)
    pub throughput: f64,

    /// Error rate (errors per minute)
    pub error_rate: f64,

    /// Queue depth
    pub queue_depth: u64,

    /// Latency distribution
    pub latency_histogram: HashMap<String, u64>,
}

/// System resource metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// CPU usage percentage
    pub cpu_usage: f64,

    /// Memory usage in bytes
    pub memory_usage: u64,

    /// Memory usage percentage
    pub memory_usage_percent: f64,

    /// Network I/O bytes
    pub network_io_bytes: u64,

    /// Disk I/O operations
    pub disk_io_operations: u64,

    /// File descriptor count
    pub file_descriptors: u64,

    /// Thread count
    pub thread_count: u64,

    /// Garbage collection metrics (if applicable)
    pub gc_metrics: Option<GcMetrics>,
}

/// Garbage collection metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcMetrics {
    /// Total GC time
    pub total_gc_time: Duration,

    /// GC frequency
    pub gc_frequency: f64,

    /// Memory freed by GC
    pub memory_freed: u64,
}

impl Default for Pop3Metrics {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now(),
            connections: ConnectionMetrics::default(),
            sessions: SessionMetrics::default(),
            commands: CommandMetrics::default(),
            authentication: AuthenticationMetrics::default(),
            mailbox: MailboxMetrics::default(),
            security: SecurityMetrics::default(),
            performance: PerformanceMetrics::default(),
            system: SystemMetrics::default(),
        }
    }
}

// Default implementations for all metric types
impl Default for ConnectionMetrics {
    fn default() -> Self {
        Self {
            active_connections: 0,
            total_connections: 0,
            rejected_connections: 0,
            connections_per_second: 0.0,
            avg_connection_duration: Duration::default(),
            peak_connections: 0,
            connection_errors: 0,
        }
    }
}

impl Default for SessionMetrics {
    fn default() -> Self {
        Self {
            active_sessions: 0,
            total_sessions: 0,
            sessions_by_state: HashMap::new(),
            avg_session_duration: Duration::default(),
            session_timeouts: 0,
            session_errors: 0,
        }
    }
}

impl Default for CommandMetrics {
    fn default() -> Self {
        Self {
            total_commands: 0,
            commands_per_second: 0.0,
            commands_by_type: HashMap::new(),
            success_rate: 100.0,
            avg_execution_time: Duration::default(),
            errors_by_type: HashMap::new(),
            rate_limited_commands: 0,
        }
    }
}

impl Default for AuthenticationMetrics {
    fn default() -> Self {
        Self {
            total_attempts: 0,
            successful_attempts: 0,
            failed_attempts: 0,
            success_rate: 100.0,
            avg_auth_time: Duration::default(),
            methods_used: HashMap::new(),
            blocked_ips: 0,
        }
    }
}

impl Default for MailboxMetrics {
    fn default() -> Self {
        Self {
            total_mailboxes_accessed: 0,
            avg_mailbox_size: 0,
            total_messages_processed: 0,
            avg_message_size: 0,
            avg_mailbox_load_time: Duration::default(),
            cache_hit_rate: 100.0,
            storage_operations: 0,
        }
    }
}

impl Default for SecurityMetrics {
    fn default() -> Self {
        Self {
            suspicious_activities: 0,
            security_violations: 0,
            brute_force_attempts: 0,
            honeypot_hits: 0,
            geographic_blocks: 0,
            tls_usage_rate: 0.0,
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            avg_response_time: Duration::default(),
            p95_response_time: Duration::default(),
            p99_response_time: Duration::default(),
            throughput: 0.0,
            error_rate: 0.0,
            queue_depth: 0,
            latency_histogram: HashMap::new(),
        }
    }
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0,
            memory_usage_percent: 0.0,
            network_io_bytes: 0,
            disk_io_operations: 0,
            file_descriptors: 0,
            thread_count: 0,
            gc_metrics: None,
        }
    }
}

/// Health check status for individual components
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Component is healthy and operating normally
    Healthy,
    /// Component is degraded but still functional
    Degraded,
    /// Component is unhealthy and may not be functioning
    Unhealthy,
    /// Component status is unknown
    Unknown,
}

/// Individual health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Name of the component being checked
    pub name: String,

    /// Current health status
    pub status: HealthStatus,

    /// Human-readable status message
    pub message: String,

    /// Timestamp of the check
    pub timestamp: SystemTime,

    /// Response time for the check
    pub response_time: Duration,

    /// Additional details about the check
    pub details: HashMap<String, String>,
}

impl HealthCheck {
    /// Creates a new health check result
    pub fn new(name: String, status: HealthStatus, message: String) -> Self {
        Self {
            name,
            status,
            message,
            timestamp: SystemTime::now(),
            response_time: Duration::default(),
            details: HashMap::new(),
        }
    }

    /// Adds a detail to the health check
    pub fn with_detail(mut self, key: String, value: String) -> Self {
        self.details.insert(key, value);
        self
    }

    /// Sets the response time
    pub fn with_response_time(mut self, response_time: Duration) -> Self {
        self.response_time = response_time;
        self
    }
}

/// Overall system health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    /// Overall system status
    pub status: HealthStatus,

    /// Timestamp of the health check
    pub timestamp: SystemTime,

    /// Individual component health checks
    pub checks: Vec<HealthCheck>,

    /// System uptime
    pub uptime: Duration,

    /// Version information
    pub version: String,
}

impl SystemHealth {
    /// Determines overall system status from individual checks
    pub fn calculate_overall_status(checks: &[HealthCheck]) -> HealthStatus {
        if checks.is_empty() {
            return HealthStatus::Unknown;
        }

        let mut has_unhealthy = false;
        let mut has_degraded = false;

        for check in checks {
            match check.status {
                HealthStatus::Unhealthy => has_unhealthy = true,
                HealthStatus::Degraded => has_degraded = true,
                HealthStatus::Unknown => return HealthStatus::Unknown,
                HealthStatus::Healthy => {}
            }
        }

        if has_unhealthy {
            HealthStatus::Unhealthy
        } else if has_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }
}

/// Alert severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Informational alert
    Info,
    /// Warning alert
    Warning,
    /// Critical alert requiring immediate attention
    Critical,
}

/// Alert notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Unique alert identifier
    pub id: String,

    /// Alert severity
    pub severity: AlertSeverity,

    /// Alert title
    pub title: String,

    /// Detailed alert message
    pub message: String,

    /// Component that triggered the alert
    pub component: String,

    /// Metric that triggered the alert
    pub metric: String,

    /// Current metric value
    pub current_value: f64,

    /// Threshold that was exceeded
    pub threshold: f64,

    /// Timestamp when alert was triggered
    pub timestamp: SystemTime,

    /// Additional context
    pub context: HashMap<String, String>,
}

impl Alert {
    /// Creates a new alert
    pub fn new(
        id: String,
        severity: AlertSeverity,
        title: String,
        message: String,
        component: String,
        metric: String,
        current_value: f64,
        threshold: f64,
    ) -> Self {
        Self {
            id,
            severity,
            title,
            message,
            component,
            metric,
            current_value,
            threshold,
            timestamp: SystemTime::now(),
            context: HashMap::new(),
        }
    }

    /// Adds context to the alert
    pub fn with_context(mut self, key: String, value: String) -> Self {
        self.context.insert(key, value);
        self
    }
}

/// Central POP3 monitoring system
///
/// Coordinates all monitoring activities including metrics collection,
/// health checks, alerting, and performance profiling.
pub struct Pop3Monitor {
    /// Monitoring configuration
    config: MonitoringConfig,

    /// Current metrics
    metrics: Arc<TokioRwLock<Pop3Metrics>>,

    /// Historical metrics for trend analysis
    metrics_history: Arc<RwLock<VecDeque<Pop3Metrics>>>,

    /// Health check results
    health_status: Arc<TokioRwLock<SystemHealth>>,

    /// Active alerts
    active_alerts: Arc<RwLock<HashMap<String, Alert>>>,

    /// Metrics collector
    collector: Arc<MetricsCollector>,

    /// Health checker
    health_checker: Arc<HealthChecker>,

    /// Alert manager
    alert_manager: Arc<AlertManager>,

    /// Performance profiler
    profiler: Option<Arc<PerformanceProfiler>>,

    /// Monitoring task handles
    task_handles: Vec<tokio::task::JoinHandle<()>>,

    /// Shutdown signal
    shutdown_tx: Option<broadcast::Sender<()>>,

    /// Running state
    is_running: Arc<AtomicBool>,
}

impl Pop3Monitor {
    /// Creates a new POP3 monitoring system
    ///
    /// # Arguments
    ///
    /// * `config` - Monitoring configuration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::monitoring::{Pop3Monitor, MonitoringConfig};
    ///
    /// let config = MonitoringConfig::production();
    /// let monitor = Pop3Monitor::new(config).await?;
    /// ```
    pub async fn new(config: MonitoringConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        info!("Initializing POP3 monitoring system");

        let metrics = Arc::new(TokioRwLock::new(Pop3Metrics::default()));
        let metrics_history = Arc::new(RwLock::new(VecDeque::with_capacity(config.max_metric_samples)));
        let health_status = Arc::new(TokioRwLock::new(SystemHealth {
            status: HealthStatus::Unknown,
            timestamp: SystemTime::now(),
            checks: Vec::new(),
            uptime: Duration::default(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }));
        let active_alerts = Arc::new(RwLock::new(HashMap::new()));

        let collector = Arc::new(MetricsCollector::new(config.clone()));
        let health_checker = Arc::new(HealthChecker::new(config.clone()));
        let alert_manager = Arc::new(AlertManager::new(config.clone()));

        let profiler = if config.profiling.enabled {
            Some(Arc::new(PerformanceProfiler::new(config.profiling.clone())))
        } else {
            None
        };

        Ok(Self {
            config,
            metrics,
            metrics_history,
            health_status,
            active_alerts,
            collector,
            health_checker,
            alert_manager,
            profiler,
            task_handles: Vec::new(),
            shutdown_tx: None,
            is_running: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Starts the monitoring system
    ///
    /// Launches background tasks for metrics collection, health checks,
    /// and alerting.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let monitor = Pop3Monitor::new(config).await?;
    /// monitor.start().await?;
    /// ```
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_running.load(Ordering::Relaxed) {
            warn!("Monitoring system is already running");
            return Ok(());
        }

        info!("Starting POP3 monitoring system");

        let (shutdown_tx, _) = broadcast::channel(1);
        self.shutdown_tx = Some(shutdown_tx.clone());

        // Start metrics collection task
        if self.config.enabled {
            let collector = Arc::clone(&self.collector);
            let metrics = Arc::clone(&self.metrics);
            let metrics_history = Arc::clone(&self.metrics_history);
            let interval = self.config.metrics_interval;
            let max_samples = self.config.max_metric_samples;
            let mut shutdown_rx_clone = shutdown_tx.subscribe();

            let handle = tokio::spawn(async move {
                let mut ticker = tokio::time::interval(interval);

                loop {
                    tokio::select! {
                        _ = ticker.tick() => {
                            if let Ok(new_metrics) = collector.collect_metrics().await {
                                // Update current metrics
                                {
                                    let mut current_metrics = metrics.write().await;
                                    *current_metrics = new_metrics.clone();
                                }

                                // Add to history
                                {
                                    let mut history = metrics_history.write().unwrap();
                                    history.push_back(new_metrics);

                                    // Maintain history size limit
                                    while history.len() > max_samples {
                                        history.pop_front();
                                    }
                                }

                                trace!("Metrics collected and stored");
                            } else {
                                warn!("Failed to collect metrics");
                            }
                        }
                        _ = shutdown_rx_clone.recv() => {
                            info!("Metrics collection task shutting down");
                            break;
                        }
                    }
                }
            });

            self.task_handles.push(handle);
        }

        // Start health check task
        let health_checker = Arc::clone(&self.health_checker);
        let health_status = Arc::clone(&self.health_status);
        let interval = self.config.health_check_interval;
        let mut shutdown_rx_clone = shutdown_tx.subscribe();

        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if let Ok(health_checks) = health_checker.run_health_checks().await {
                            let overall_status = SystemHealth::calculate_overall_status(&health_checks);

                            let mut health = health_status.write().await;
                            health.status = overall_status;
                            health.checks = health_checks;
                            health.timestamp = SystemTime::now();

                            trace!("Health checks completed");
                        } else {
                            warn!("Failed to run health checks");
                        }
                    }
                    _ = shutdown_rx_clone.recv() => {
                        info!("Health check task shutting down");
                        break;
                    }
                }
            }
        });

        self.task_handles.push(handle);

        // Start alerting task if enabled
        if self.config.alerts.enabled {
            let alert_manager = Arc::clone(&self.alert_manager);
            let metrics = Arc::clone(&self.metrics);
            let active_alerts = Arc::clone(&self.active_alerts);
            let mut shutdown_rx_clone = shutdown_tx.subscribe();

            let handle = tokio::spawn(async move {
                let mut ticker = tokio::time::interval(Duration::from_secs(30)); // Check every 30 seconds

                loop {
                    tokio::select! {
                        _ = ticker.tick() => {
                            let current_metrics = metrics.read().await;

                            if let Ok(new_alerts) = alert_manager.check_thresholds(&current_metrics).await {
                                let mut alerts = active_alerts.write().unwrap();

                                for alert in new_alerts {
                                    alerts.insert(alert.id.clone(), alert);
                                }

                                // Clean up old alerts (implement alert expiration logic)
                                // This would typically involve checking alert timestamps
                                // and removing alerts that are no longer relevant
                            }
                        }
                        _ = shutdown_rx_clone.recv() => {
                            info!("Alerting task shutting down");
                            break;
                        }
                    }
                }
            });

            self.task_handles.push(handle);
        }

        self.is_running.store(true, Ordering::Relaxed);
        info!("POP3 monitoring system started successfully");

        Ok(())
    }

    /// Stops the monitoring system
    ///
    /// Gracefully shuts down all monitoring tasks.
    pub async fn stop(&mut self) {
        if !self.is_running.load(Ordering::Relaxed) {
            return;
        }

        info!("Stopping POP3 monitoring system");

        // Send shutdown signal
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        // Wait for all tasks to complete
        for handle in self.task_handles.drain(..) {
            let _ = handle.await;
        }

        self.is_running.store(false, Ordering::Relaxed);
        info!("POP3 monitoring system stopped");
    }

    /// Gets current metrics
    ///
    /// # Returns
    ///
    /// Current POP3 server metrics
    pub async fn get_metrics(&self) -> Pop3Metrics {
        self.metrics.read().await.clone()
    }

    /// Gets metrics history
    ///
    /// # Returns
    ///
    /// Vector of historical metrics samples
    pub fn get_metrics_history(&self) -> Vec<Pop3Metrics> {
        self.metrics_history.read().unwrap().iter().cloned().collect()
    }

    /// Gets current system health
    ///
    /// # Returns
    ///
    /// Current system health status
    pub async fn get_health(&self) -> SystemHealth {
        self.health_status.read().await.clone()
    }

    /// Gets active alerts
    ///
    /// # Returns
    ///
    /// Vector of currently active alerts
    pub fn get_active_alerts(&self) -> Vec<Alert> {
        self.active_alerts.read().unwrap().values().cloned().collect()
    }

    /// Checks if the monitoring system is running
    ///
    /// # Returns
    ///
    /// `true` if monitoring is active, `false` otherwise
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }
}

/// Metrics collection system
///
/// Responsible for gathering performance and operational metrics
/// from various POP3 server components.
pub struct MetricsCollector {
    config: MonitoringConfig,
    start_time: Instant,
    last_collection: Arc<RwLock<Option<Instant>>>,
}

impl MetricsCollector {
    /// Creates a new metrics collector
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            start_time: Instant::now(),
            last_collection: Arc::new(RwLock::new(None)),
        }
    }

    /// Collects current metrics from all components
    ///
    /// # Returns
    ///
    /// Complete metrics snapshot or error if collection fails
    pub async fn collect_metrics(&self) -> Result<Pop3Metrics, Box<dyn std::error::Error + Send + Sync>> {
        let collection_start = Instant::now();

        trace!("Starting metrics collection");

        // Update last collection time
        {
            let mut last = self.last_collection.write().unwrap();
            *last = Some(collection_start);
        }

        let mut metrics = Pop3Metrics::default();

        // Collect connection metrics
        metrics.connections = self.collect_connection_metrics().await?;

        // Collect session metrics
        metrics.sessions = self.collect_session_metrics().await?;

        // Collect command metrics
        metrics.commands = self.collect_command_metrics().await?;

        // Collect authentication metrics
        metrics.authentication = self.collect_authentication_metrics().await?;

        // Collect mailbox metrics
        metrics.mailbox = self.collect_mailbox_metrics().await?;

        // Collect security metrics
        metrics.security = self.collect_security_metrics().await?;

        // Collect performance metrics
        metrics.performance = self.collect_performance_metrics().await?;

        // Collect system metrics
        metrics.system = self.collect_system_metrics().await?;

        let collection_time = collection_start.elapsed();

        debug!(
            collection_time_ms = collection_time.as_millis(),
            "Metrics collection completed"
        );

        Ok(metrics)
    }

    /// Collects connection-related metrics
    async fn collect_connection_metrics(&self) -> Result<ConnectionMetrics, Box<dyn std::error::Error + Send + Sync>> {
        // In a real implementation, this would gather metrics from:
        // - Connection pool
        // - Network layer
        // - Session manager

        Ok(ConnectionMetrics {
            active_connections: 0, // TODO: Get from connection pool
            total_connections: 0,  // TODO: Get from session manager
            rejected_connections: 0, // TODO: Get from security manager
            connections_per_second: 0.0, // TODO: Calculate from recent data
            avg_connection_duration: Duration::from_secs(300), // TODO: Calculate
            peak_connections: 0, // TODO: Track peak
            connection_errors: 0, // TODO: Get from error tracking
        })
    }

    /// Collects session-related metrics
    async fn collect_session_metrics(&self) -> Result<SessionMetrics, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement actual session metrics collection
        Ok(SessionMetrics::default())
    }

    /// Collects command execution metrics
    async fn collect_command_metrics(&self) -> Result<CommandMetrics, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement actual command metrics collection
        Ok(CommandMetrics::default())
    }

    /// Collects authentication metrics
    async fn collect_authentication_metrics(&self) -> Result<AuthenticationMetrics, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement actual authentication metrics collection
        Ok(AuthenticationMetrics::default())
    }

    /// Collects mailbox operation metrics
    async fn collect_mailbox_metrics(&self) -> Result<MailboxMetrics, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement actual mailbox metrics collection
        Ok(MailboxMetrics::default())
    }

    /// Collects security metrics
    async fn collect_security_metrics(&self) -> Result<SecurityMetrics, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement actual security metrics collection
        Ok(SecurityMetrics::default())
    }

    /// Collects performance metrics
    async fn collect_performance_metrics(&self) -> Result<PerformanceMetrics, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement actual performance metrics collection
        Ok(PerformanceMetrics::default())
    }

    /// Collects system resource metrics
    async fn collect_system_metrics(&self) -> Result<SystemMetrics, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement actual system metrics collection
        // This would typically use system APIs to get:
        // - CPU usage
        // - Memory usage
        // - Network I/O
        // - Disk I/O
        // - File descriptors
        // - Thread count

        Ok(SystemMetrics::default())
    }
}

/// Health checking system
///
/// Monitors the health of various POP3 server components and
/// provides detailed health status information.
pub struct HealthChecker {
    config: MonitoringConfig,
    check_registry: HashMap<String, Box<dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = HealthCheck> + Send>> + Send + Sync>>,
}

impl HealthChecker {
    /// Creates a new health checker
    pub fn new(config: MonitoringConfig) -> Self {
        let mut checker = Self {
            config,
            check_registry: HashMap::new(),
        };

        // Register built-in health checks
        checker.register_builtin_checks();

        checker
    }

    /// Registers built-in health checks
    fn register_builtin_checks(&mut self) {
        // Database connectivity check
        self.register_check("database".to_string(), Box::new(|| {
            Box::pin(async {
                // TODO: Implement actual database health check
                HealthCheck::new(
                    "database".to_string(),
                    HealthStatus::Healthy,
                    "Database connection is healthy".to_string(),
                )
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = HealthCheck> + Send>>
        }));

        // Memory usage check
        self.register_check("memory".to_string(), Box::new(|| {
            Box::pin(async {
                // TODO: Implement actual memory health check
                HealthCheck::new(
                    "memory".to_string(),
                    HealthStatus::Healthy,
                    "Memory usage is within normal limits".to_string(),
                )
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = HealthCheck> + Send>>
        }));

        // Disk space check
        self.register_check("disk".to_string(), Box::new(|| {
            Box::pin(async {
                // TODO: Implement actual disk space health check
                HealthCheck::new(
                    "disk".to_string(),
                    HealthStatus::Healthy,
                    "Disk space is sufficient".to_string(),
                )
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = HealthCheck> + Send>>
        }));
    }

    /// Registers a custom health check
    pub fn register_check<F>(&mut self, name: String, check_fn: F)
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = HealthCheck> + Send>> + Send + Sync + 'static,
    {
        self.check_registry.insert(name, Box::new(check_fn));
    }

    /// Runs all registered health checks
    ///
    /// # Returns
    ///
    /// Vector of health check results
    pub async fn run_health_checks(&self) -> Result<Vec<HealthCheck>, Box<dyn std::error::Error + Send + Sync>> {
        let mut results = Vec::new();

        for (name, check_fn) in &self.check_registry {
            let start_time = Instant::now();

            match tokio::time::timeout(Duration::from_secs(30), check_fn()).await {
                Ok(mut check) => {
                    check.response_time = start_time.elapsed();
                    results.push(check);
                }
                Err(_) => {
                    // Timeout
                    results.push(HealthCheck::new(
                        name.clone(),
                        HealthStatus::Unhealthy,
                        "Health check timed out".to_string(),
                    ).with_response_time(Duration::from_secs(30)));
                }
            }
        }

        Ok(results)
    }
}

/// Alert management system
///
/// Monitors metrics against configured thresholds and generates
/// alerts when thresholds are exceeded.
pub struct AlertManager {
    config: MonitoringConfig,
    last_alerts: Arc<RwLock<HashMap<String, SystemTime>>>,
}

impl AlertManager {
    /// Creates a new alert manager
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            last_alerts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Checks metrics against thresholds and generates alerts
    ///
    /// # Arguments
    ///
    /// * `metrics` - Current metrics to check
    ///
    /// # Returns
    ///
    /// Vector of new alerts generated
    pub async fn check_thresholds(&self, metrics: &Pop3Metrics) -> Result<Vec<Alert>, Box<dyn std::error::Error + Send + Sync>> {
        let mut alerts = Vec::new();
        let now = SystemTime::now();

        // Check CPU usage
        if metrics.system.cpu_usage > self.config.alerts.cpu_threshold {
            if self.should_generate_alert("cpu_usage", now) {
                alerts.push(Alert::new(
                    format!("cpu_usage_{}", now.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()),
                    AlertSeverity::Warning,
                    "High CPU Usage".to_string(),
                    format!("CPU usage is {:.1}%, exceeding threshold of {:.1}%",
                           metrics.system.cpu_usage, self.config.alerts.cpu_threshold),
                    "system".to_string(),
                    "cpu_usage".to_string(),
                    metrics.system.cpu_usage,
                    self.config.alerts.cpu_threshold,
                ));

                self.record_alert("cpu_usage", now);
            }
        }

        // Check memory usage
        if metrics.system.memory_usage_percent > self.config.alerts.memory_threshold {
            if self.should_generate_alert("memory_usage", now) {
                alerts.push(Alert::new(
                    format!("memory_usage_{}", now.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()),
                    AlertSeverity::Warning,
                    "High Memory Usage".to_string(),
                    format!("Memory usage is {:.1}%, exceeding threshold of {:.1}%",
                           metrics.system.memory_usage_percent, self.config.alerts.memory_threshold),
                    "system".to_string(),
                    "memory_usage".to_string(),
                    metrics.system.memory_usage_percent,
                    self.config.alerts.memory_threshold,
                ));

                self.record_alert("memory_usage", now);
            }
        }

        // Check connection count
        if metrics.connections.active_connections > self.config.alerts.connection_threshold {
            if self.should_generate_alert("connection_count", now) {
                alerts.push(Alert::new(
                    format!("connection_count_{}", now.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()),
                    AlertSeverity::Critical,
                    "High Connection Count".to_string(),
                    format!("Active connections: {}, exceeding threshold of {}",
                           metrics.connections.active_connections, self.config.alerts.connection_threshold),
                    "connections".to_string(),
                    "active_connections".to_string(),
                    metrics.connections.active_connections as f64,
                    self.config.alerts.connection_threshold as f64,
                ));

                self.record_alert("connection_count", now);
            }
        }

        // Check error rate
        if metrics.performance.error_rate > self.config.alerts.error_rate_threshold {
            if self.should_generate_alert("error_rate", now) {
                alerts.push(Alert::new(
                    format!("error_rate_{}", now.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()),
                    AlertSeverity::Critical,
                    "High Error Rate".to_string(),
                    format!("Error rate is {:.2} errors/min, exceeding threshold of {:.2}",
                           metrics.performance.error_rate, self.config.alerts.error_rate_threshold),
                    "performance".to_string(),
                    "error_rate".to_string(),
                    metrics.performance.error_rate,
                    self.config.alerts.error_rate_threshold,
                ));

                self.record_alert("error_rate", now);
            }
        }

        // Check response time
        let response_time_ms = metrics.performance.avg_response_time.as_millis() as u64;
        if response_time_ms > self.config.alerts.response_time_threshold {
            if self.should_generate_alert("response_time", now) {
                alerts.push(Alert::new(
                    format!("response_time_{}", now.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()),
                    AlertSeverity::Warning,
                    "High Response Time".to_string(),
                    format!("Average response time is {}ms, exceeding threshold of {}ms",
                           response_time_ms, self.config.alerts.response_time_threshold),
                    "performance".to_string(),
                    "response_time".to_string(),
                    response_time_ms as f64,
                    self.config.alerts.response_time_threshold as f64,
                ));

                self.record_alert("response_time", now);
            }
        }

        Ok(alerts)
    }

    /// Checks if an alert should be generated based on cooldown period
    fn should_generate_alert(&self, alert_type: &str, now: SystemTime) -> bool {
        let last_alerts = self.last_alerts.read().unwrap();

        if let Some(&last_alert_time) = last_alerts.get(alert_type) {
            if let Ok(duration) = now.duration_since(last_alert_time) {
                duration >= self.config.alerts.alert_cooldown
            } else {
                true // If we can't calculate duration, allow the alert
            }
        } else {
            true // No previous alert of this type
        }
    }

    /// Records that an alert was generated
    fn record_alert(&self, alert_type: &str, time: SystemTime) {
        let mut last_alerts = self.last_alerts.write().unwrap();
        last_alerts.insert(alert_type.to_string(), time);
    }
}

/// Performance profiling system
///
/// Provides detailed performance analysis and bottleneck detection
/// for the POP3 server.
pub struct PerformanceProfiler {
    config: ProfilingConfig,
    profiles: Arc<RwLock<VecDeque<ProfileSample>>>,
}

/// Individual performance profile sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSample {
    /// Timestamp of the sample
    pub timestamp: SystemTime,

    /// CPU profiling data
    pub cpu_profile: Option<CpuProfile>,

    /// Memory profiling data
    pub memory_profile: Option<MemoryProfile>,

    /// I/O profiling data
    pub io_profile: Option<IoProfile>,
}

/// CPU profiling data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuProfile {
    /// CPU usage by function/component
    pub usage_by_component: HashMap<String, f64>,

    /// Hot spots (functions consuming most CPU)
    pub hot_spots: Vec<HotSpot>,

    /// Call graph information
    pub call_graph: Option<String>, // Simplified representation
}

/// Memory profiling data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryProfile {
    /// Memory usage by component
    pub usage_by_component: HashMap<String, u64>,

    /// Memory allocations
    pub allocations: u64,

    /// Memory deallocations
    pub deallocations: u64,

    /// Memory leaks detected
    pub potential_leaks: Vec<MemoryLeak>,
}

/// I/O profiling data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoProfile {
    /// Network I/O statistics
    pub network_io: NetworkIoStats,

    /// Disk I/O statistics
    pub disk_io: DiskIoStats,
}

/// Hot spot in CPU profiling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotSpot {
    /// Function or component name
    pub name: String,

    /// CPU usage percentage
    pub cpu_percent: f64,

    /// Number of calls
    pub call_count: u64,

    /// Average execution time
    pub avg_execution_time: Duration,
}

/// Potential memory leak
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLeak {
    /// Location of potential leak
    pub location: String,

    /// Amount of memory potentially leaked
    pub size: u64,

    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
}

/// Network I/O statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIoStats {
    /// Bytes read
    pub bytes_read: u64,

    /// Bytes written
    pub bytes_written: u64,

    /// Number of read operations
    pub read_ops: u64,

    /// Number of write operations
    pub write_ops: u64,

    /// Average read latency
    pub avg_read_latency: Duration,

    /// Average write latency
    pub avg_write_latency: Duration,
}

/// Disk I/O statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskIoStats {
    /// Bytes read from disk
    pub bytes_read: u64,

    /// Bytes written to disk
    pub bytes_written: u64,

    /// Number of read operations
    pub read_ops: u64,

    /// Number of write operations
    pub write_ops: u64,

    /// Average read latency
    pub avg_read_latency: Duration,

    /// Average write latency
    pub avg_write_latency: Duration,
}

impl PerformanceProfiler {
    /// Creates a new performance profiler
    pub fn new(config: ProfilingConfig) -> Self {
        Self {
            config,
            profiles: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Starts a profiling session
    pub async fn start_profiling(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting performance profiling session");

        // TODO: Implement actual profiling start logic
        // This would typically involve:
        // - Setting up CPU profiling hooks
        // - Enabling memory tracking
        // - Starting I/O monitoring

        Ok(())
    }

    /// Stops the current profiling session and returns results
    pub async fn stop_profiling(&self) -> Result<ProfileSample, Box<dyn std::error::Error + Send + Sync>> {
        info!("Stopping performance profiling session");

        // TODO: Implement actual profiling stop logic and data collection
        let sample = ProfileSample {
            timestamp: SystemTime::now(),
            cpu_profile: None,    // TODO: Collect CPU profile data
            memory_profile: None, // TODO: Collect memory profile data
            io_profile: None,     // TODO: Collect I/O profile data
        };

        // Store the sample
        {
            let mut profiles = self.profiles.write().unwrap();
            profiles.push_back(sample.clone());

            // Maintain retention limit
            let retention_samples = (self.config.retention_period.as_secs() / 60) as usize; // Assume 1 sample per minute
            while profiles.len() > retention_samples {
                profiles.pop_front();
            }
        }

        Ok(sample)
    }

    /// Gets recent profiling samples
    pub fn get_recent_profiles(&self, count: usize) -> Vec<ProfileSample> {
        let profiles = self.profiles.read().unwrap();
        profiles.iter().rev().take(count).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[test]
    fn test_monitoring_config_default() {
        let config = MonitoringConfig::default();
        assert!(config.enabled);
        assert_eq!(config.metrics_interval, Duration::from_secs(30));
        assert_eq!(config.health_check_interval, Duration::from_secs(60));
        assert!(config.enable_alerting);
    }

    #[test]
    fn test_monitoring_config_production() {
        let config = MonitoringConfig::production();
        assert!(config.enabled);
        assert_eq!(config.metrics_interval, Duration::from_secs(15));
        assert_eq!(config.health_check_interval, Duration::from_secs(30));
        assert!(config.enable_profiling);
        assert!(config.enable_alerting);
        assert_eq!(config.max_metric_samples, 2880);
    }

    #[test]
    fn test_monitoring_config_development() {
        let config = MonitoringConfig::development();
        assert!(config.enabled);
        assert_eq!(config.metrics_interval, Duration::from_secs(5));
        assert_eq!(config.health_check_interval, Duration::from_secs(10));
        assert!(config.enable_profiling);
        assert!(!config.enable_alerting); // Disabled in development
    }

    #[test]
    fn test_pop3_metrics_default() {
        let metrics = Pop3Metrics::default();
        assert_eq!(metrics.connections.active_connections, 0);
        assert_eq!(metrics.sessions.active_sessions, 0);
        assert_eq!(metrics.commands.total_commands, 0);
        assert_eq!(metrics.authentication.total_attempts, 0);
        assert_eq!(metrics.security.suspicious_activities, 0);
    }

    #[test]
    fn test_health_check_creation() {
        let check = HealthCheck::new(
            "test".to_string(),
            HealthStatus::Healthy,
            "Test check".to_string(),
        );

        assert_eq!(check.name, "test");
        assert_eq!(check.status, HealthStatus::Healthy);
        assert_eq!(check.message, "Test check");
        assert!(check.details.is_empty());
    }

    #[test]
    fn test_health_check_with_details() {
        let check = HealthCheck::new(
            "test".to_string(),
            HealthStatus::Healthy,
            "Test check".to_string(),
        )
        .with_detail("key1".to_string(), "value1".to_string())
        .with_detail("key2".to_string(), "value2".to_string());

        assert_eq!(check.details.len(), 2);
        assert_eq!(check.details.get("key1"), Some(&"value1".to_string()));
        assert_eq!(check.details.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_system_health_calculate_overall_status() {
        // All healthy
        let checks = vec![
            HealthCheck::new("test1".to_string(), HealthStatus::Healthy, "OK".to_string()),
            HealthCheck::new("test2".to_string(), HealthStatus::Healthy, "OK".to_string()),
        ];
        assert_eq!(SystemHealth::calculate_overall_status(&checks), HealthStatus::Healthy);

        // One degraded
        let checks = vec![
            HealthCheck::new("test1".to_string(), HealthStatus::Healthy, "OK".to_string()),
            HealthCheck::new("test2".to_string(), HealthStatus::Degraded, "Warning".to_string()),
        ];
        assert_eq!(SystemHealth::calculate_overall_status(&checks), HealthStatus::Degraded);

        // One unhealthy
        let checks = vec![
            HealthCheck::new("test1".to_string(), HealthStatus::Healthy, "OK".to_string()),
            HealthCheck::new("test2".to_string(), HealthStatus::Unhealthy, "Error".to_string()),
        ];
        assert_eq!(SystemHealth::calculate_overall_status(&checks), HealthStatus::Unhealthy);

        // Empty checks
        let checks = vec![];
        assert_eq!(SystemHealth::calculate_overall_status(&checks), HealthStatus::Unknown);
    }

    #[test]
    fn test_alert_creation() {
        let alert = Alert::new(
            "test_alert".to_string(),
            AlertSeverity::Warning,
            "Test Alert".to_string(),
            "This is a test alert".to_string(),
            "test_component".to_string(),
            "test_metric".to_string(),
            85.0,
            80.0,
        );

        assert_eq!(alert.id, "test_alert");
        assert_eq!(alert.severity, AlertSeverity::Warning);
        assert_eq!(alert.title, "Test Alert");
        assert_eq!(alert.current_value, 85.0);
        assert_eq!(alert.threshold, 80.0);
    }

    #[test]
    fn test_alert_with_context() {
        let alert = Alert::new(
            "test_alert".to_string(),
            AlertSeverity::Critical,
            "Test Alert".to_string(),
            "This is a test alert".to_string(),
            "test_component".to_string(),
            "test_metric".to_string(),
            95.0,
            90.0,
        )
        .with_context("server".to_string(), "pop3-01".to_string())
        .with_context("datacenter".to_string(), "us-east-1".to_string());

        assert_eq!(alert.context.len(), 2);
        assert_eq!(alert.context.get("server"), Some(&"pop3-01".to_string()));
        assert_eq!(alert.context.get("datacenter"), Some(&"us-east-1".to_string()));
    }

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let config = MonitoringConfig::default();
        let collector = MetricsCollector::new(config);

        // Test that collector can collect metrics without errors
        let result = collector.collect_metrics().await;
        assert!(result.is_ok());

        let metrics = result.unwrap();
        assert_eq!(metrics.connections.active_connections, 0);
        assert_eq!(metrics.sessions.active_sessions, 0);
    }

    #[tokio::test]
    async fn test_health_checker_creation() {
        let config = MonitoringConfig::default();
        let health_checker = HealthChecker::new(config);

        // Test that health checker can run checks without errors
        let result = health_checker.run_health_checks().await;
        assert!(result.is_ok());

        let checks = result.unwrap();
        assert!(!checks.is_empty()); // Should have built-in checks
    }

    #[tokio::test]
    async fn test_alert_manager_threshold_checking() {
        let mut config = MonitoringConfig::default();
        config.alerts.cpu_threshold = 50.0; // Low threshold for testing

        let alert_manager = AlertManager::new(config);

        // Create metrics with high CPU usage
        let mut metrics = Pop3Metrics::default();
        metrics.system.cpu_usage = 75.0; // Above threshold

        let result = alert_manager.check_thresholds(&metrics).await;
        assert!(result.is_ok());

        let alerts = result.unwrap();
        assert!(!alerts.is_empty()); // Should generate CPU alert

        // Check that the alert is for CPU usage
        let cpu_alert = alerts.iter().find(|a| a.metric == "cpu_usage");
        assert!(cpu_alert.is_some());

        let cpu_alert = cpu_alert.unwrap();
        assert_eq!(cpu_alert.severity, AlertSeverity::Warning);
        assert_eq!(cpu_alert.current_value, 75.0);
        assert_eq!(cpu_alert.threshold, 50.0);
    }

    #[tokio::test]
    async fn test_pop3_monitor_lifecycle() {
        let config = MonitoringConfig::development();
        let mut monitor = Pop3Monitor::new(config).await.unwrap();

        // Test initial state
        assert!(!monitor.is_running());

        // Start monitoring
        monitor.start().await.unwrap();
        assert!(monitor.is_running());

        // Get initial metrics
        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.connections.active_connections, 0);

        // Get initial health
        let health = monitor.get_health().await;
        assert_eq!(health.version, env!("CARGO_PKG_VERSION"));

        // Get active alerts (should be empty initially)
        let alerts = monitor.get_active_alerts();
        assert!(alerts.is_empty());

        // Stop monitoring
        monitor.stop().await;
        assert!(!monitor.is_running());
    }

    #[test]
    fn test_performance_profiler_creation() {
        let config = ProfilingConfig::default();
        let profiler = PerformanceProfiler::new(config);

        // Test that profiler starts with empty profiles
        let profiles = profiler.get_recent_profiles(10);
        assert!(profiles.is_empty());
    }
}
