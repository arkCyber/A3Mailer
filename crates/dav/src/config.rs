/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Configuration Management for DAV Server
//!
//! This module provides centralized configuration management for all DAV server
//! components with validation, hot-reloading, and environment-based overrides.

use std::{
    collections::HashMap,
    path::PathBuf,
    time::Duration,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn, error};

use crate::{
    async_pool::AsyncPoolConfig,
    cache::CacheConfig,
    data_access::DataAccessConfig,
    // monitoring::MonitoringConfig, // TODO: implement monitoring config
    performance::PerformanceConfig,
    security::SecurityConfig,
    router::RouterConfig,
};

/// Master configuration for the DAV server
///
/// Contains all configuration sections for different components
/// with validation and environment variable support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DavServerConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// Async pool configuration
    pub async_pool: AsyncPoolConfig,
    /// Cache configuration
    pub cache: CacheConfig,
    /// Data access configuration
    pub data_access: DataAccessConfig,
    /// Monitoring configuration
    // pub monitoring: MonitoringConfig, // TODO: implement monitoring config
    /// Performance configuration
    pub performance: PerformanceConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Router configuration
    pub router: RouterConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Feature flags
    pub features: FeatureFlags,
}

/// Server-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server bind address
    pub bind_address: String,
    /// Server port
    pub port: u16,
    /// Server name
    pub server_name: String,
    /// Maximum request size
    pub max_request_size: usize,
    /// Request timeout
    pub request_timeout: Duration,
    /// Enable TLS
    pub enable_tls: bool,
    /// TLS certificate path
    pub tls_cert_path: Option<PathBuf>,
    /// TLS private key path
    pub tls_key_path: Option<PathBuf>,
    /// Worker threads
    pub worker_threads: Option<usize>,
    /// Enable graceful shutdown
    pub enable_graceful_shutdown: bool,
    /// Graceful shutdown timeout
    pub graceful_shutdown_timeout: Duration,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 8080,
            server_name: "A3Mailer DAV Server".to_string(),
            max_request_size: 100 * 1024 * 1024, // 100MB
            request_timeout: Duration::from_secs(300), // 5 minutes
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
            worker_threads: None, // Use default (CPU cores)
            enable_graceful_shutdown: true,
            graceful_shutdown_timeout: Duration::from_secs(30),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Log format
    pub format: LogFormat,
    /// Log output
    pub output: LogOutput,
    /// Enable structured logging
    pub structured: bool,
    /// Log file path
    pub file_path: Option<PathBuf>,
    /// Log rotation
    pub rotation: LogRotation,
    /// Enable request logging
    pub enable_request_logging: bool,
    /// Enable performance logging
    pub enable_performance_logging: bool,
    /// Enable security logging
    pub enable_security_logging: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Json,
            output: LogOutput::Stdout,
            structured: true,
            file_path: None,
            rotation: LogRotation::default(),
            enable_request_logging: true,
            enable_performance_logging: true,
            enable_security_logging: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    Json,
    Text,
    Compact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogOutput {
    Stdout,
    Stderr,
    File,
    Syslog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotation {
    /// Enable log rotation
    pub enabled: bool,
    /// Maximum file size before rotation
    pub max_size: usize,
    /// Maximum number of rotated files to keep
    pub max_files: usize,
    /// Rotation interval
    pub interval: Duration,
}

impl Default for LogRotation {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size: 100 * 1024 * 1024, // 100MB
            max_files: 10,
            interval: Duration::from_secs(86400), // 24 hours
        }
    }
}

/// Feature flags for enabling/disabling functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Enable WebDAV support
    pub enable_webdav: bool,
    /// Enable CalDAV support
    pub enable_caldav: bool,
    /// Enable CardDAV support
    pub enable_carddav: bool,
    /// Enable principal support
    pub enable_principals: bool,
    /// Enable scheduling support
    pub enable_scheduling: bool,
    /// Enable ACL support
    pub enable_acl: bool,
    /// Enable locking support
    pub enable_locking: bool,
    /// Enable versioning support
    pub enable_versioning: bool,
    /// Enable search support
    pub enable_search: bool,
    /// Enable sync collection support
    pub enable_sync_collection: bool,
    /// Enable experimental features
    pub enable_experimental: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_webdav: true,
            enable_caldav: true,
            enable_carddav: true,
            enable_principals: true,
            enable_scheduling: true,
            enable_acl: true,
            enable_locking: true,
            enable_versioning: false, // Experimental
            enable_search: true,
            enable_sync_collection: true,
            enable_experimental: false,
        }
    }
}

impl Default for DavServerConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            async_pool: AsyncPoolConfig::default(),
            cache: CacheConfig::default(),
            data_access: DataAccessConfig::default(),
            // monitoring: MonitoringConfig::default(), // TODO: implement monitoring config
            performance: PerformanceConfig::default(),
            security: SecurityConfig::default(),
            router: RouterConfig::default(),
            logging: LoggingConfig::default(),
            features: FeatureFlags::default(),
        }
    }
}

/// Configuration manager for the DAV server
#[derive(Debug)]
pub struct ConfigManager {
    config: DavServerConfig,
    config_path: Option<PathBuf>,
    environment_overrides: HashMap<String, String>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Self {
        Self {
            config: DavServerConfig::default(),
            config_path: None,
            environment_overrides: HashMap::new(),
        }
    }

    /// Load configuration from file
    pub fn load_from_file<P: Into<PathBuf>>(mut self, path: P) -> Result<Self, ConfigError> {
        let path = path.into();

        let content = std::fs::read_to_string(&path)
            .map_err(|e| ConfigError::FileRead(path.clone(), e.to_string()))?;

        let config: DavServerConfig = match path.extension().and_then(|s| s.to_str()) {
            Some("toml") => toml::from_str(&content)
                .map_err(|e| ConfigError::ParseError("TOML".to_string(), e.to_string()))?,
            Some("yaml") | Some("yml") => serde_yaml::from_str(&content)
                .map_err(|e| ConfigError::ParseError("YAML".to_string(), e.to_string()))?,
            Some("json") => serde_json::from_str(&content)
                .map_err(|e| ConfigError::ParseError("JSON".to_string(), e.to_string()))?,
            _ => return Err(ConfigError::UnsupportedFormat(path.clone())),
        };

        self.config = config;
        self.config_path = Some(path);

        info!(
            config_path = ?self.config_path,
            "Configuration loaded from file"
        );

        Ok(self)
    }

    /// Load environment variable overrides
    pub fn load_environment_overrides(mut self) -> Self {
        for (key, value) in std::env::vars() {
            if key.starts_with("DAV_") {
                self.environment_overrides.insert(key, value);
            }
        }

        if !self.environment_overrides.is_empty() {
            info!(
                overrides = self.environment_overrides.len(),
                "Environment variable overrides loaded"
            );
        }

        self
    }

    /// Apply environment variable overrides
    pub fn apply_environment_overrides(mut self) -> Result<Self, ConfigError> {
        let overrides = self.environment_overrides.clone();
        for (key, value) in &overrides {
            self.apply_override(key, value)?;
        }

        debug!(
            applied_overrides = self.environment_overrides.len(),
            "Environment overrides applied"
        );

        Ok(self)
    }

    /// Validate the configuration
    pub fn validate(self) -> Result<Self, ConfigError> {
        // Validate server configuration
        if self.config.server.port == 0 {
            return Err(ConfigError::ValidationError("Server port cannot be 0".to_string()));
        }

        if self.config.server.bind_address.is_empty() {
            return Err(ConfigError::ValidationError("Server bind address cannot be empty".to_string()));
        }

        // Validate TLS configuration
        if self.config.server.enable_tls {
            if self.config.server.tls_cert_path.is_none() {
                return Err(ConfigError::ValidationError("TLS certificate path required when TLS is enabled".to_string()));
            }
            if self.config.server.tls_key_path.is_none() {
                return Err(ConfigError::ValidationError("TLS private key path required when TLS is enabled".to_string()));
            }
        }

        // Validate async pool configuration
        if self.config.async_pool.max_concurrent_requests == 0 {
            return Err(ConfigError::ValidationError("Max concurrent requests cannot be 0".to_string()));
        }

        if self.config.async_pool.worker_count == 0 {
            return Err(ConfigError::ValidationError("Worker count cannot be 0".to_string()));
        }

        // Validate cache configuration
        if self.config.cache.l1_size == 0 {
            return Err(ConfigError::ValidationError("L1 cache size cannot be 0".to_string()));
        }

        // Validate data access configuration
        if self.config.data_access.max_connections == 0 {
            return Err(ConfigError::ValidationError("Max database connections cannot be 0".to_string()));
        }

        if self.config.data_access.min_connections > self.config.data_access.max_connections {
            return Err(ConfigError::ValidationError("Min connections cannot be greater than max connections".to_string()));
        }

        info!("Configuration validation completed successfully");

        Ok(self)
    }

    /// Get the final configuration
    pub fn build(self) -> DavServerConfig {
        self.config
    }

    /// Get a reference to the current configuration
    pub fn config(&self) -> &DavServerConfig {
        &self.config
    }

    /// Hot reload configuration from file
    pub fn reload(&mut self) -> Result<(), ConfigError> {
        if let Some(ref path) = self.config_path {
            let new_manager = ConfigManager::new()
                .load_from_file(path.clone())?
                .load_environment_overrides()
                .apply_environment_overrides()?
                .validate()?;

            self.config = new_manager.config;

            info!("Configuration reloaded successfully");
            Ok(())
        } else {
            Err(ConfigError::NoConfigFile)
        }
    }

    fn apply_override(&mut self, key: &str, value: &str) -> Result<(), ConfigError> {
        match key {
            "DAV_SERVER_PORT" => {
                self.config.server.port = value.parse()
                    .map_err(|_| ConfigError::InvalidOverride(key.to_string(), value.to_string()))?;
            }
            "DAV_SERVER_BIND_ADDRESS" => {
                self.config.server.bind_address = value.to_string();
            }
            "DAV_SERVER_NAME" => {
                self.config.server.server_name = value.to_string();
            }
            "DAV_ENABLE_TLS" => {
                self.config.server.enable_tls = value.parse()
                    .map_err(|_| ConfigError::InvalidOverride(key.to_string(), value.to_string()))?;
            }
            "DAV_MAX_CONCURRENT_REQUESTS" => {
                self.config.async_pool.max_concurrent_requests = value.parse()
                    .map_err(|_| ConfigError::InvalidOverride(key.to_string(), value.to_string()))?;
            }
            "DAV_WORKER_COUNT" => {
                self.config.async_pool.worker_count = value.parse()
                    .map_err(|_| ConfigError::InvalidOverride(key.to_string(), value.to_string()))?;
            }
            "DAV_LOG_LEVEL" => {
                self.config.logging.level = value.to_string();
            }
            "DAV_ENABLE_CACHE" => {
                // This would set cache enable flag if it existed
                debug!(key = key, value = value, "Cache override applied");
            }
            _ => {
                warn!(key = key, value = value, "Unknown environment override");
            }
        }

        Ok(())
    }
}

/// Configuration error types
#[derive(Debug, Clone)]
pub enum ConfigError {
    FileRead(PathBuf, String),
    ParseError(String, String),
    UnsupportedFormat(PathBuf),
    ValidationError(String),
    InvalidOverride(String, String),
    NoConfigFile,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileRead(path, err) => write!(f, "Failed to read config file {:?}: {}", path, err),
            Self::ParseError(format, err) => write!(f, "Failed to parse {} config: {}", format, err),
            Self::UnsupportedFormat(path) => write!(f, "Unsupported config format: {:?}", path),
            Self::ValidationError(msg) => write!(f, "Configuration validation error: {}", msg),
            Self::InvalidOverride(key, value) => write!(f, "Invalid environment override {}={}", key, value),
            Self::NoConfigFile => write!(f, "No configuration file loaded"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = DavServerConfig::default();

        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.bind_address, "0.0.0.0");
        assert!(!config.server.enable_tls);
        assert!(config.features.enable_webdav);
        assert!(config.features.enable_caldav);
        assert!(config.features.enable_carddav);
    }

    #[test]
    fn test_config_validation() {
        let mut config = DavServerConfig::default();
        config.server.port = 0;

        let manager = ConfigManager {
            config,
            config_path: None,
            environment_overrides: HashMap::new(),
        };

        assert!(manager.validate().is_err());
    }

    #[test]
    fn test_environment_overrides() {
        env::set_var("DAV_SERVER_PORT", "9090");
        env::set_var("DAV_SERVER_NAME", "Test Server");

        let manager = ConfigManager::new()
            .load_environment_overrides()
            .apply_environment_overrides()
            .unwrap();

        assert_eq!(manager.config.server.port, 9090);
        assert_eq!(manager.config.server.server_name, "Test Server");

        // Cleanup
        env::remove_var("DAV_SERVER_PORT");
        env::remove_var("DAV_SERVER_NAME");
    }

    #[test]
    fn test_feature_flags() {
        let config = DavServerConfig::default();

        assert!(config.features.enable_webdav);
        assert!(config.features.enable_caldav);
        assert!(config.features.enable_carddav);
        assert!(config.features.enable_principals);
        assert!(config.features.enable_scheduling);
        assert!(config.features.enable_acl);
        assert!(config.features.enable_locking);
        assert!(!config.features.enable_versioning); // Experimental
        assert!(!config.features.enable_experimental);
    }

    #[test]
    fn test_config_manager_creation() {
        let manager = ConfigManager::new();

        assert!(manager.config_path.is_none());
        assert!(manager.environment_overrides.is_empty());
        assert_eq!(manager.config.server.port, 8080);
    }

    #[test]
    fn test_config_builder_pattern() {
        let config = ConfigManager::new()
            .load_environment_overrides()
            .apply_environment_overrides()
            .unwrap()
            .validate()
            .unwrap()
            .build();

        assert_eq!(config.server.port, 8080);
        assert!(config.features.enable_webdav);
    }

    #[test]
    fn test_server_config_defaults() {
        let config = ServerConfig::default();

        assert_eq!(config.bind_address, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.server_name, "A3Mailer DAV Server");
        assert_eq!(config.max_request_size, 100 * 1024 * 1024);
        assert!(!config.enable_tls);
        assert!(config.enable_graceful_shutdown);
    }

    #[test]
    fn test_logging_config_defaults() {
        let config = LoggingConfig::default();

        assert_eq!(config.level, "info");
        assert!(matches!(config.format, LogFormat::Json));
        assert!(matches!(config.output, LogOutput::Stdout));
        assert!(config.structured);
        assert!(config.enable_request_logging);
        assert!(config.enable_performance_logging);
        assert!(config.enable_security_logging);
    }

    #[test]
    fn test_log_rotation_defaults() {
        let rotation = LogRotation::default();

        assert!(rotation.enabled);
        assert_eq!(rotation.max_size, 100 * 1024 * 1024);
        assert_eq!(rotation.max_files, 10);
        assert_eq!(rotation.interval, Duration::from_secs(86400));
    }

    #[test]
    fn test_config_validation_tls() {
        let mut config = DavServerConfig::default();
        config.server.enable_tls = true;
        // Missing TLS cert and key paths

        let manager = ConfigManager {
            config,
            config_path: None,
            environment_overrides: HashMap::new(),
        };

        let result = manager.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("TLS certificate"));
    }

    #[test]
    fn test_config_validation_async_pool() {
        let mut config = DavServerConfig::default();
        config.async_pool.max_concurrent_requests = 0;

        let manager = ConfigManager {
            config,
            config_path: None,
            environment_overrides: HashMap::new(),
        };

        let result = manager.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Max concurrent requests"));
    }

    #[test]
    fn test_config_validation_database_connections() {
        let mut config = DavServerConfig::default();
        config.data_access.min_connections = 10;
        config.data_access.max_connections = 5; // Invalid: min > max

        let manager = ConfigManager {
            config,
            config_path: None,
            environment_overrides: HashMap::new(),
        };

        let result = manager.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Min connections cannot be greater"));
    }

    #[test]
    fn test_apply_override_server_port() {
        let mut manager = ConfigManager::new();

        let result = manager.apply_override("DAV_SERVER_PORT", "9090");
        assert!(result.is_ok());
        assert_eq!(manager.config.server.port, 9090);
    }

    #[test]
    fn test_apply_override_invalid_port() {
        let mut manager = ConfigManager::new();

        let result = manager.apply_override("DAV_SERVER_PORT", "invalid");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::InvalidOverride(_, _)));
    }

    #[test]
    fn test_apply_override_bind_address() {
        let mut manager = ConfigManager::new();

        let result = manager.apply_override("DAV_SERVER_BIND_ADDRESS", "127.0.0.1");
        assert!(result.is_ok());
        assert_eq!(manager.config.server.bind_address, "127.0.0.1");
    }

    #[test]
    fn test_apply_override_server_name() {
        let mut manager = ConfigManager::new();

        let result = manager.apply_override("DAV_SERVER_NAME", "Custom DAV Server");
        assert!(result.is_ok());
        assert_eq!(manager.config.server.server_name, "Custom DAV Server");
    }

    #[test]
    fn test_apply_override_enable_tls() {
        let mut manager = ConfigManager::new();

        let result = manager.apply_override("DAV_ENABLE_TLS", "true");
        assert!(result.is_ok());
        assert!(manager.config.server.enable_tls);

        let result = manager.apply_override("DAV_ENABLE_TLS", "false");
        assert!(result.is_ok());
        assert!(!manager.config.server.enable_tls);
    }

    #[test]
    fn test_apply_override_max_concurrent_requests() {
        let mut manager = ConfigManager::new();

        let result = manager.apply_override("DAV_MAX_CONCURRENT_REQUESTS", "5000");
        assert!(result.is_ok());
        assert_eq!(manager.config.async_pool.max_concurrent_requests, 5000);
    }

    #[test]
    fn test_apply_override_worker_count() {
        let mut manager = ConfigManager::new();

        let result = manager.apply_override("DAV_WORKER_COUNT", "8");
        assert!(result.is_ok());
        assert_eq!(manager.config.async_pool.worker_count, 8);
    }

    #[test]
    fn test_apply_override_log_level() {
        let mut manager = ConfigManager::new();

        let result = manager.apply_override("DAV_LOG_LEVEL", "debug");
        assert!(result.is_ok());
        assert_eq!(manager.config.logging.level, "debug");
    }

    #[test]
    fn test_apply_override_unknown_key() {
        let mut manager = ConfigManager::new();

        // Unknown keys should not cause errors, just warnings
        let result = manager.apply_override("DAV_UNKNOWN_KEY", "value");
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_error_display() {
        let error = ConfigError::ValidationError("Test error".to_string());
        assert_eq!(error.to_string(), "Configuration validation error: Test error");

        let error = ConfigError::InvalidOverride("KEY".to_string(), "value".to_string());
        assert_eq!(error.to_string(), "Invalid environment override KEY=value");

        let error = ConfigError::NoConfigFile;
        assert_eq!(error.to_string(), "No configuration file loaded");
    }

    #[test]
    fn test_reload_without_config_file() {
        let mut manager = ConfigManager::new();

        let result = manager.reload();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::NoConfigFile));
    }
}
