/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Configuration structures for the alerting system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Main alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    /// Alerting engine configuration
    pub engine: HashMap<String, String>,

    /// Notification channels configuration
    pub channels: Vec<HashMap<String, String>>,

    /// Alert rules configuration
    pub rules: Vec<HashMap<String, String>>,

    /// Escalation policies configuration
    pub escalation: HashMap<String, String>,

    /// Alert suppression configuration
    pub suppression: HashMap<String, String>,

    /// Template configuration
    pub templates: HashMap<String, String>,

    /// Global alerting settings
    pub global: GlobalConfig,

    /// Metrics configuration
    pub metrics: MetricsConfig,

    /// Storage configuration
    pub storage: StorageConfig,

    /// Rate limiting configuration
    pub rate_limiting: RateLimitingConfig,

    /// Cleanup configuration
    pub cleanup_interval: Duration,

    /// Alert retention period
    pub retention_period: Duration,

    /// Maximum number of active alerts
    pub max_active_alerts: usize,

    /// Default alert timeout
    pub default_timeout: Option<Duration>,

    /// Enable alert deduplication
    pub enable_deduplication: bool,

    /// Deduplication window
    pub deduplication_window: Duration,

    /// Enable alert grouping
    pub enable_grouping: bool,

    /// Grouping configuration
    pub grouping: GroupingConfig,
}

/// Global alerting settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Global alert labels to add to all alerts
    pub labels: HashMap<String, String>,

    /// Global alert annotations to add to all alerts
    pub annotations: HashMap<String, String>,

    /// Default severity for alerts without explicit severity
    pub default_severity: String,

    /// Enable alert correlation
    pub enable_correlation: bool,

    /// Correlation window
    pub correlation_window: Duration,

    /// Maximum correlation distance
    pub max_correlation_distance: f64,

    /// Enable alert enrichment
    pub enable_enrichment: bool,

    /// Enrichment sources
    pub enrichment_sources: Vec<EnrichmentSource>,

    /// Enable alert validation
    pub enable_validation: bool,

    /// Validation rules
    pub validation_rules: Vec<ValidationRule>,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,

    /// Metrics collection interval
    pub collection_interval: Duration,

    /// Metrics retention period
    pub retention_period: Duration,

    /// Enable Prometheus metrics export
    pub enable_prometheus: bool,

    /// Prometheus metrics port
    pub prometheus_port: u16,

    /// Prometheus metrics path
    pub prometheus_path: String,

    /// Custom metrics labels
    pub labels: HashMap<String, String>,

    /// Enable detailed metrics
    pub enable_detailed_metrics: bool,

    /// Metrics aggregation window
    pub aggregation_window: Duration,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage backend type
    pub backend: StorageBackend,

    /// Storage connection string
    pub connection_string: String,

    /// Connection pool size
    pub pool_size: u32,

    /// Connection timeout
    pub connection_timeout: Duration,

    /// Query timeout
    pub query_timeout: Duration,

    /// Enable storage encryption
    pub enable_encryption: bool,

    /// Encryption key
    pub encryption_key: Option<String>,

    /// Enable storage compression
    pub enable_compression: bool,

    /// Compression algorithm
    pub compression_algorithm: CompressionAlgorithm,

    /// Backup configuration
    pub backup: BackupConfig,
}

/// Storage backend types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackend {
    /// In-memory storage (for testing)
    Memory,
    /// SQLite database
    Sqlite,
    /// PostgreSQL database
    PostgreSQL,
    /// MySQL database
    MySQL,
    /// Redis storage
    Redis,
    /// MongoDB storage
    MongoDB,
}

/// Compression algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// Gzip compression
    Gzip,
    /// LZ4 compression
    Lz4,
    /// Zstd compression
    Zstd,
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Enable automatic backups
    pub enabled: bool,

    /// Backup interval
    pub interval: Duration,

    /// Backup retention period
    pub retention_period: Duration,

    /// Backup storage path
    pub storage_path: String,

    /// Enable backup compression
    pub enable_compression: bool,

    /// Enable backup encryption
    pub enable_encryption: bool,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Enable rate limiting
    pub enabled: bool,

    /// Global rate limit (alerts per second)
    pub global_rate_limit: f64,

    /// Per-source rate limit
    pub per_source_rate_limit: f64,

    /// Per-channel rate limit
    pub per_channel_rate_limit: f64,

    /// Rate limit window
    pub window: Duration,

    /// Rate limit burst size
    pub burst_size: u32,

    /// Rate limit action
    pub action: RateLimitAction,
}

/// Rate limit actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateLimitAction {
    /// Drop the alert
    Drop,
    /// Queue the alert for later delivery
    Queue,
    /// Throttle the alert delivery
    Throttle,
}

/// Alert grouping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupingConfig {
    /// Grouping keys (labels to group by)
    pub group_by: Vec<String>,

    /// Grouping window
    pub group_window: Duration,

    /// Maximum group size
    pub max_group_size: usize,

    /// Group timeout
    pub group_timeout: Duration,

    /// Enable group notifications
    pub enable_group_notifications: bool,

    /// Group notification template
    pub group_notification_template: Option<String>,
}

/// Alert enrichment source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichmentSource {
    /// Source name
    pub name: String,

    /// Source type
    pub source_type: EnrichmentSourceType,

    /// Source configuration
    pub config: HashMap<String, String>,

    /// Enable caching
    pub enable_caching: bool,

    /// Cache TTL
    pub cache_ttl: Duration,
}

/// Enrichment source types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnrichmentSourceType {
    /// HTTP API source
    Http,
    /// Database source
    Database,
    /// File source
    File,
    /// External service
    External,
}

/// Alert validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    /// Rule name
    pub name: String,

    /// Rule condition
    pub condition: String,

    /// Rule action
    pub action: ValidationAction,

    /// Rule message
    pub message: String,
}

/// Validation actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationAction {
    /// Accept the alert
    Accept,
    /// Reject the alert
    Reject,
    /// Modify the alert
    Modify,
    /// Warn about the alert
    Warn,
}

impl Default for AlertingConfig {
    fn default() -> Self {
        Self {
            engine: HashMap::new(),
            channels: Vec::new(),
            rules: Vec::new(),
            escalation: HashMap::new(),
            suppression: HashMap::new(),
            templates: HashMap::new(),
            global: GlobalConfig::default(),
            metrics: MetricsConfig::default(),
            storage: StorageConfig::default(),
            rate_limiting: RateLimitingConfig::default(),
            cleanup_interval: Duration::from_secs(3600), // 1 hour
            retention_period: Duration::from_secs(86400 * 7), // 7 days
            max_active_alerts: 10000,
            default_timeout: Some(Duration::from_secs(3600)), // 1 hour
            enable_deduplication: true,
            deduplication_window: Duration::from_secs(300), // 5 minutes
            enable_grouping: false,
            grouping: GroupingConfig::default(),
        }
    }
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            labels: HashMap::new(),
            annotations: HashMap::new(),
            default_severity: "warning".to_string(),
            enable_correlation: false,
            correlation_window: Duration::from_secs(300),
            max_correlation_distance: 0.8,
            enable_enrichment: false,
            enrichment_sources: Vec::new(),
            enable_validation: true,
            validation_rules: Vec::new(),
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval: Duration::from_secs(60),
            retention_period: Duration::from_secs(86400 * 30), // 30 days
            enable_prometheus: false,
            prometheus_port: 9090,
            prometheus_path: "/metrics".to_string(),
            labels: HashMap::new(),
            enable_detailed_metrics: false,
            aggregation_window: Duration::from_secs(300),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: StorageBackend::Memory,
            connection_string: "memory://".to_string(),
            pool_size: 10,
            connection_timeout: Duration::from_secs(30),
            query_timeout: Duration::from_secs(60),
            enable_encryption: false,
            encryption_key: None,
            enable_compression: false,
            compression_algorithm: CompressionAlgorithm::None,
            backup: BackupConfig::default(),
        }
    }
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval: Duration::from_secs(86400), // 24 hours
            retention_period: Duration::from_secs(86400 * 30), // 30 days
            storage_path: "/var/lib/stalwart/alerting/backups".to_string(),
            enable_compression: true,
            enable_encryption: false,
        }
    }
}

impl Default for RateLimitingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            global_rate_limit: 100.0, // 100 alerts per second
            per_source_rate_limit: 10.0, // 10 alerts per second per source
            per_channel_rate_limit: 5.0, // 5 alerts per second per channel
            window: Duration::from_secs(60),
            burst_size: 10,
            action: RateLimitAction::Queue,
        }
    }
}

impl Default for GroupingConfig {
    fn default() -> Self {
        Self {
            group_by: vec!["alertname".to_string(), "severity".to_string()],
            group_window: Duration::from_secs(300), // 5 minutes
            max_group_size: 100,
            group_timeout: Duration::from_secs(3600), // 1 hour
            enable_group_notifications: false,
            group_notification_template: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AlertingConfig::default();
        assert!(config.enable_deduplication);
        assert_eq!(config.max_active_alerts, 10000);
        assert!(config.default_timeout.is_some());
    }

    #[test]
    fn test_storage_backend_serialization() {
        let backend = StorageBackend::PostgreSQL;
        let serialized = serde_json::to_string(&backend).unwrap();
        let deserialized: StorageBackend = serde_json::from_str(&serialized).unwrap();
        assert!(matches!(deserialized, StorageBackend::PostgreSQL));
    }

    #[test]
    fn test_rate_limiting_config() {
        let config = RateLimitingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.global_rate_limit, 100.0);
        assert!(matches!(config.action, RateLimitAction::Queue));
    }
}
