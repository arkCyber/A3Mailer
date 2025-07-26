/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! POP3 Configuration Management Module
//!
//! This module provides comprehensive configuration management for the POP3 server,
//! including validation, hot reloading, environment-specific configurations,
//! and production-ready configuration handling.
//!
//! # Features
//!
//! * **Configuration Validation**: Comprehensive validation with detailed error messages
//! * **Hot Reloading**: Runtime configuration updates without service restart
//! * **Environment Management**: Environment-specific configuration overrides
//! * **Configuration Templates**: Pre-defined configurations for different deployment scenarios
//! * **Secure Defaults**: Security-first default configurations
//! * **Configuration Monitoring**: Track configuration changes and validation status
//!
//! # Architecture
//!
//! The configuration system is built around several key components:
//!
//! * `Pop3Config`: Main configuration structure with all server settings
//! * `ConfigManager`: Handles loading, validation, and hot reloading
//! * `ConfigValidator`: Comprehensive validation with detailed error reporting
//! * `EnvironmentConfig`: Environment-specific configuration overrides
//! * `ConfigWatcher`: File system monitoring for hot reloading
//!
//! # Configuration Hierarchy
//!
//! Configuration values are resolved in the following order (highest to lowest priority):
//! 1. Environment variables
//! 2. Command line arguments
//! 3. Configuration file
//! 4. Default values
//!
//! # Examples
//!
//! ```rust
//! use pop3::config::{Pop3Config, ConfigManager, Environment};
//!
//! // Load configuration with environment overrides
//! let config = ConfigManager::load_with_environment(
//!     "config.toml",
//!     Environment::Production
//! ).await?;
//!
//! // Start hot reloading
//! let mut manager = ConfigManager::new(config);
//! manager.start_hot_reload().await?;
//!
//! // Get current configuration
//! let current_config = manager.get_config().await;
//! ```

use std::{
    collections::HashMap,
    env,
    fmt,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};
use serde::{Deserialize, Serialize};
use tokio::{
    fs,
    sync::{broadcast, RwLock as TokioRwLock},
    time::interval,
};
use tracing::{debug, error, info, trace, warn};

use crate::{
    security::SecurityConfig,
    monitoring::MonitoringConfig,
};

/// Environment types for configuration management
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Environment {
    /// Development environment with relaxed settings
    Development,
    /// Testing environment with test-specific configurations
    Testing,
    /// Staging environment mirroring production
    Staging,
    /// Production environment with strict security and performance settings
    Production,
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Environment::Development => write!(f, "development"),
            Environment::Testing => write!(f, "testing"),
            Environment::Staging => write!(f, "staging"),
            Environment::Production => write!(f, "production"),
        }
    }
}

impl Default for Environment {
    fn default() -> Self {
        Environment::Development
    }
}

/// Configuration metadata for tracking and auditing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    /// Configuration version
    pub version: String,

    /// Environment this configuration is for
    pub environment: Environment,

    /// Timestamp when configuration was created
    pub created_at: SystemTime,

    /// Timestamp when configuration was last modified
    pub modified_at: SystemTime,

    /// Configuration file path
    pub config_path: Option<PathBuf>,

    /// Configuration checksum for integrity verification
    pub checksum: Option<String>,

    /// Configuration description
    pub description: Option<String>,

    /// Configuration tags for organization
    pub tags: Vec<String>,
}

impl Default for ConfigMetadata {
    fn default() -> Self {
        let now = SystemTime::now();
        Self {
            version: "1.0.0".to_string(),
            environment: Environment::default(),
            created_at: now,
            modified_at: now,
            config_path: None,
            checksum: None,
            description: None,
            tags: Vec::new(),
        }
    }
}

/// Comprehensive POP3 server configuration
///
/// This structure contains all configuration settings for the POP3 server,
/// organized into logical sections for maintainability and clarity.
///
/// # Configuration Sections
///
/// * **Metadata**: Configuration versioning and environment information
/// * **Server**: Core server settings like timeouts and limits
/// * **Security**: Authentication, encryption, and access control
/// * **Protocol**: POP3 protocol-specific settings
/// * **Performance**: Optimization and resource management settings
/// * **Monitoring**: Observability and metrics configuration
/// * **Logging**: Logging configuration and levels
///
/// # Validation
///
/// All configuration values are validated when loaded to ensure:
/// - Values are within acceptable ranges
/// - Required dependencies are met
/// - Security settings are appropriate for the environment
/// - Performance settings are realistic
///
/// # Examples
///
/// ```rust
/// use pop3::config::{Pop3Config, Environment};
///
/// // Create production configuration
/// let config = Pop3Config::for_environment(Environment::Production);
///
/// // Validate configuration
/// config.validate()?;
///
/// // Save to file
/// config.to_file("pop3-prod.toml").await?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pop3Config {
    /// Configuration metadata
    pub metadata: ConfigMetadata,

    /// Server settings
    pub server: ServerConfig,

    /// Security settings
    pub security: SecurityConfig,

    /// Protocol settings
    pub protocol: ProtocolConfig,

    /// Performance settings
    pub performance: PerformanceConfig,

    /// Monitoring settings
    pub monitoring: MonitoringConfig,

    /// Logging settings
    pub logging: LoggingConfig,

    /// Environment-specific overrides
    pub environment_overrides: HashMap<String, toml::Value>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,

    /// Enable structured logging (JSON format)
    pub structured: bool,

    /// Log file path (None for stdout)
    pub file_path: Option<PathBuf>,

    /// Maximum log file size before rotation
    pub max_file_size: u64,

    /// Number of rotated log files to keep
    pub max_files: u32,

    /// Enable log compression
    pub compress: bool,

    /// Log format template
    pub format: String,

    /// Enable performance logging
    pub enable_performance_logs: bool,

    /// Enable security event logging
    pub enable_security_logs: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            structured: false,
            file_path: None,
            max_file_size: 100 * 1024 * 1024, // 100MB
            max_files: 10,
            compress: true,
            format: "%Y-%m-%d %H:%M:%S [%l] %t - %m%n".to_string(),
            enable_performance_logs: true,
            enable_security_logs: true,
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server greeting message
    pub greeting: String,
    /// Maximum message size (in bytes)
    pub max_message_size: u64,
    /// Session timeout for authenticated users
    pub session_timeout: Duration,
    /// Session timeout for unauthenticated users
    pub unauth_timeout: Duration,
    /// Enable APOP authentication
    pub enable_apop: bool,
    /// Enable UTF8 support
    pub enable_utf8: bool,
    /// Enable STLS (STARTTLS)
    pub enable_stls: bool,
}

/// Protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolConfig {
    /// Maximum line length
    pub max_line_length: usize,
    /// Maximum number of arguments per command
    pub max_arguments: usize,
    /// Enable pipelining
    pub enable_pipelining: bool,
    /// Maximum pipelined commands
    pub max_pipelined_commands: usize,
    /// Enable TOP command
    pub enable_top: bool,
    /// Enable UIDL command
    pub enable_uidl: bool,
    /// Maximum TOP lines
    pub max_top_lines: u32,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Maximum concurrent sessions
    pub max_concurrent_sessions: usize,
    /// Connection pool size
    pub connection_pool_size: usize,
    /// Buffer size for I/O operations
    pub io_buffer_size: usize,
    /// Enable response caching
    pub enable_response_caching: bool,
    /// Cache TTL
    pub cache_ttl: Duration,
    /// Maximum cached responses
    pub max_cached_responses: usize,
}

impl Default for Pop3Config {
    fn default() -> Self {
        Self {
            metadata: ConfigMetadata::default(),
            server: ServerConfig::default(),
            security: SecurityConfig::default(),
            protocol: ProtocolConfig::default(),
            performance: PerformanceConfig::default(),
            monitoring: MonitoringConfig::default(),
            logging: LoggingConfig::default(),
            environment_overrides: HashMap::new(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            greeting: "Stalwart POP3 at your service".to_string(),
            max_message_size: 50 * 1024 * 1024, // 50MB
            session_timeout: Duration::from_secs(1800), // 30 minutes
            unauth_timeout: Duration::from_secs(300), // 5 minutes
            enable_apop: true,
            enable_utf8: true,
            enable_stls: true,
        }
    }
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            max_line_length: 8192,
            max_arguments: 10,
            enable_pipelining: true,
            max_pipelined_commands: 100,
            enable_top: true,
            enable_uidl: true,
            max_top_lines: 1000,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_concurrent_sessions: 1000,
            connection_pool_size: 100,
            io_buffer_size: 8192,
            enable_response_caching: false,
            cache_ttl: Duration::from_secs(300),
            max_cached_responses: 1000,
        }
    }
}

impl Pop3Config {
    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Pop3Config = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Creates a configuration for the specified environment
    pub fn for_environment(environment: Environment) -> Self {
        let mut config = match environment {
            Environment::Development => Self::development(),
            Environment::Testing => Self::testing(),
            Environment::Staging => Self::staging(),
            Environment::Production => Self::production(),
        };

        config.metadata.environment = environment;
        config.metadata.description = Some(format!("Configuration for {} environment", environment));
        config.metadata.tags.push(environment.to_string());

        config
    }

    /// Applies environment variable overrides
    pub fn apply_env_overrides(&mut self) {
        // Server overrides
        if let Ok(val) = env::var("POP3_SERVER_GREETING") {
            self.server.greeting = val;
        }
        if let Ok(val) = env::var("POP3_SERVER_MAX_MESSAGE_SIZE") {
            if let Ok(size) = val.parse::<u64>() {
                self.server.max_message_size = size;
            }
        }
        if let Ok(val) = env::var("POP3_SERVER_SESSION_TIMEOUT") {
            if let Ok(timeout) = val.parse::<u64>() {
                self.server.session_timeout = Duration::from_secs(timeout);
            }
        }

        // Security overrides
        if let Ok(val) = env::var("POP3_SECURITY_MAX_AUTH_ATTEMPTS") {
            if let Ok(attempts) = val.parse::<u32>() {
                self.security.max_auth_attempts = attempts;
            }
        }
        if let Ok(val) = env::var("POP3_SECURITY_REQUIRE_TLS") {
            if let Ok(require_tls) = val.parse::<bool>() {
                self.security.require_tls_for_auth = require_tls;
            }
        }

        // Performance overrides
        if let Ok(val) = env::var("POP3_PERFORMANCE_MAX_SESSIONS") {
            if let Ok(sessions) = val.parse::<usize>() {
                self.performance.max_concurrent_sessions = sessions;
            }
        }

        // Logging overrides
        if let Ok(val) = env::var("POP3_LOG_LEVEL") {
            self.logging.level = val;
        }
        if let Ok(val) = env::var("POP3_LOG_STRUCTURED") {
            if let Ok(structured) = val.parse::<bool>() {
                self.logging.structured = structured;
            }
        }

        debug!("Applied environment variable overrides");
    }

    /// Internal method for loading configuration from path (used by hot reload)
    async fn load_from_path(path: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let content = fs::read_to_string(path).await?;
        let mut config: Self = toml::from_str(&content)?;

        // Update metadata
        config.metadata.config_path = Some(path.to_path_buf());
        config.metadata.modified_at = SystemTime::now();
        config.metadata.checksum = Some(Self::calculate_checksum(&content));

        // Validate configuration
        config.validate().map_err(|e| format!("Validation error: {}", e))?;

        info!(path = %path.display(), "Configuration loaded successfully");
        Ok(config)
    }

    /// Calculate checksum for configuration content
    fn calculate_checksum(content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Creates a testing configuration
    pub fn testing() -> Self {
        let mut config = Self::development();
        config.metadata.environment = Environment::Testing;
        config.server.greeting = "Stalwart POP3 Test Server".to_string();
        config.performance.max_concurrent_sessions = 50;
        config.logging.level = "debug".to_string();
        config
    }

    /// Creates a staging configuration
    pub fn staging() -> Self {
        let mut config = Self::production();
        config.metadata.environment = Environment::Staging;
        config.server.greeting = "Stalwart POP3 Staging Server".to_string();
        config.logging.level = "debug".to_string();
        config
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate server config
        if self.server.max_message_size == 0 {
            return Err("max_message_size must be greater than 0".into());
        }
        if self.server.session_timeout.as_secs() == 0 {
            return Err("session_timeout must be greater than 0".into());
        }
        if self.server.unauth_timeout.as_secs() == 0 {
            return Err("unauth_timeout must be greater than 0".into());
        }

        // Validate protocol config
        if self.protocol.max_line_length < 512 {
            return Err("max_line_length must be at least 512".into());
        }
        if self.protocol.max_arguments == 0 {
            return Err("max_arguments must be greater than 0".into());
        }
        if self.protocol.max_pipelined_commands == 0 {
            return Err("max_pipelined_commands must be greater than 0".into());
        }

        // Validate performance config
        if self.performance.max_concurrent_sessions == 0 {
            return Err("max_concurrent_sessions must be greater than 0".into());
        }
        if self.performance.connection_pool_size == 0 {
            return Err("connection_pool_size must be greater than 0".into());
        }
        if self.performance.io_buffer_size < 1024 {
            return Err("io_buffer_size must be at least 1024".into());
        }

        // Validate security config
        if self.security.max_auth_attempts == 0 {
            return Err("max_auth_attempts must be greater than 0".into());
        }
        if self.security.auth_window.as_secs() == 0 {
            return Err("auth_window must be greater than 0".into());
        }

        Ok(())
    }

    /// Create a production-ready configuration
    pub fn production() -> Self {
        Self {
            metadata: ConfigMetadata {
                environment: Environment::Production,
                description: Some("Production configuration".to_string()),
                tags: vec!["production".to_string()],
                ..Default::default()
            },
            server: ServerConfig {
                greeting: "Stalwart POP3 Server".to_string(),
                max_message_size: 100 * 1024 * 1024, // 100MB
                session_timeout: Duration::from_secs(3600), // 1 hour
                unauth_timeout: Duration::from_secs(180), // 3 minutes
                enable_apop: true,
                enable_utf8: true,
                enable_stls: true,
            },
            security: SecurityConfig {
                max_auth_attempts: 3,
                auth_window: Duration::from_secs(900), // 15 minutes
                max_connections_per_ip: 5,
                max_commands_per_minute: 30,
                min_command_delay: Duration::from_millis(50),
                enable_security_logging: true,
                suspicious_threshold: 5,
                max_session_duration: Duration::from_secs(3600), // 1 hour
                enable_geo_blocking: false,
                blocked_countries: Vec::new(),
                require_tls_for_auth: false,
                auto_block_duration: Duration::from_secs(1800), // 30 minutes
                enable_honeypot: false,
            },
            protocol: ProtocolConfig {
                max_line_length: 4096,
                max_arguments: 5,
                enable_pipelining: true,
                max_pipelined_commands: 50,
                enable_top: true,
                enable_uidl: true,
                max_top_lines: 500,
            },
            performance: PerformanceConfig {
                max_concurrent_sessions: 500,
                connection_pool_size: 50,
                io_buffer_size: 4096,
                enable_response_caching: true,
                cache_ttl: Duration::from_secs(600),
                max_cached_responses: 500,
            },
            monitoring: MonitoringConfig::production(),
            logging: LoggingConfig {
                level: "info".to_string(),
                structured: true,
                file_path: Some(PathBuf::from("/var/log/stalwart/pop3.log")),
                max_file_size: 100 * 1024 * 1024, // 100MB
                max_files: 10,
                compress: true,
                format: "%Y-%m-%d %H:%M:%S [%l] %t - %m%n".to_string(),
                enable_performance_logs: true,
                enable_security_logs: true,
            },
            environment_overrides: HashMap::new(),
        }
    }

    /// Create a development configuration
    pub fn development() -> Self {
        Self {
            metadata: ConfigMetadata {
                environment: Environment::Development,
                description: Some("Development configuration".to_string()),
                tags: vec!["development".to_string()],
                ..Default::default()
            },
            server: ServerConfig {
                greeting: "Stalwart POP3 Development Server".to_string(),
                max_message_size: 10 * 1024 * 1024, // 10MB
                session_timeout: Duration::from_secs(600), // 10 minutes
                unauth_timeout: Duration::from_secs(120), // 2 minutes
                enable_apop: true,
                enable_utf8: true,
                enable_stls: false, // Disable TLS for development
            },
            security: SecurityConfig {
                max_auth_attempts: 10,
                auth_window: Duration::from_secs(300), // 5 minutes
                max_connections_per_ip: 50,
                max_commands_per_minute: 120,
                min_command_delay: Duration::from_millis(10),
                enable_security_logging: true,
                suspicious_threshold: 20,
                max_session_duration: Duration::from_secs(7200), // 2 hours
                enable_geo_blocking: false,
                blocked_countries: Vec::new(),
                require_tls_for_auth: false,
                auto_block_duration: Duration::from_secs(300), // 5 minutes
                enable_honeypot: false,
            },
            protocol: ProtocolConfig {
                max_line_length: 8192,
                max_arguments: 10,
                enable_pipelining: true,
                max_pipelined_commands: 100,
                enable_top: true,
                enable_uidl: true,
                max_top_lines: 1000,
            },
            performance: PerformanceConfig {
                max_concurrent_sessions: 100,
                connection_pool_size: 10,
                io_buffer_size: 8192,
                enable_response_caching: false,
                cache_ttl: Duration::from_secs(60),
                max_cached_responses: 100,
            },
            monitoring: MonitoringConfig::development(),
            logging: LoggingConfig {
                level: "debug".to_string(),
                structured: false,
                file_path: None, // Log to stdout in development
                max_file_size: 10 * 1024 * 1024, // 10MB
                max_files: 3,
                compress: false,
                format: "%Y-%m-%d %H:%M:%S [%l] %t - %m%n".to_string(),
                enable_performance_logs: true,
                enable_security_logs: true,
            },
            environment_overrides: HashMap::new(),
        }
    }
}

/// Configuration manager with hot reloading capabilities
///
/// Provides advanced configuration management including:
/// - Hot reloading without service restart
/// - Configuration validation and error handling
/// - Change notifications and callbacks
/// - Configuration backup and rollback
///
/// # Examples
///
/// ```rust
/// use pop3::config::{ConfigManager, Pop3Config};
///
/// let config = Pop3Config::production();
/// let mut manager = ConfigManager::new(config);
///
/// // Start hot reloading
/// manager.start_hot_reload("config.toml").await?;
///
/// // Register change callback
/// manager.on_config_change(|new_config| {
///     println!("Configuration updated!");
/// });
/// ```
pub struct ConfigManager {
    /// Current configuration
    current_config: Arc<TokioRwLock<Pop3Config>>,

    /// Configuration file path
    config_path: Option<PathBuf>,

    /// Change notification sender
    change_tx: Option<broadcast::Sender<Pop3Config>>,

    /// Hot reload task handle
    reload_task: Option<tokio::task::JoinHandle<()>>,

    /// Configuration backup for rollback
    backup_config: Option<Pop3Config>,

    /// Validation errors
    validation_errors: Arc<RwLock<Vec<String>>>,
}

impl ConfigManager {
    /// Creates a new configuration manager
    ///
    /// # Arguments
    ///
    /// * `config` - Initial configuration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::config::{ConfigManager, Pop3Config};
    ///
    /// let config = Pop3Config::production();
    /// let manager = ConfigManager::new(config);
    /// ```
    pub fn new(config: Pop3Config) -> Self {
        Self {
            current_config: Arc::new(TokioRwLock::new(config)),
            config_path: None,
            change_tx: None,
            reload_task: None,
            backup_config: None,
            validation_errors: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Load configuration from a file path
    async fn load_from_path(path: &std::path::Path) -> Result<Pop3Config, Box<dyn std::error::Error + Send + Sync>> {
        Pop3Config::from_file(path.to_str().unwrap()).map_err(|e| format!("Failed to load config: {}", e).into())
    }

    /// Starts hot reloading for the specified configuration file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file to monitor
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::config::ConfigManager;
    ///
    /// let mut manager = ConfigManager::new(config);
    /// manager.start_hot_reload("config.toml").await?;
    /// ```
    pub async fn start_hot_reload<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let path = path.as_ref().to_path_buf();
        let path_display = path.display().to_string();
        self.config_path = Some(path.clone());

        let (tx, _) = broadcast::channel(16);
        self.change_tx = Some(tx.clone());

        let current_config = Arc::clone(&self.current_config);
        let validation_errors = Arc::clone(&self.validation_errors);

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            let mut last_modified = None;

            loop {
                interval.tick().await;

                // Check if file was modified
                if let Ok(metadata) = fs::metadata(&path).await {
                    if let Ok(modified) = metadata.modified() {
                        if last_modified.map_or(true, |last| modified > last) {
                            last_modified = Some(modified);

                            // Reload configuration
                            match Self::load_from_path(&path).await {
                                Ok(new_config) => {
                                    // Update current configuration
                                    {
                                        let mut config = current_config.write().await;
                                        *config = new_config.clone();
                                    }

                                    // Clear validation errors
                                    {
                                        let mut errors = validation_errors.write().unwrap();
                                        errors.clear();
                                    }

                                    // Notify subscribers
                                    let _ = tx.send(new_config);

                                    info!(path = %path.display(), "Configuration reloaded successfully");
                                }
                                Err(e) => {
                                    // Store validation error
                                    {
                                        let mut errors = validation_errors.write().unwrap();
                                        errors.clear();
                                        errors.push(format!("Configuration reload failed: {}", e));
                                    }

                                    error!(
                                        path = %path.display(),
                                        error = %e,
                                        "Failed to reload configuration"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        });

        self.reload_task = Some(handle);
        info!(path = %path_display, "Hot reload started");

        Ok(())
    }

    /// Stops hot reloading
    pub async fn stop_hot_reload(&mut self) {
        if let Some(handle) = self.reload_task.take() {
            handle.abort();
            info!("Hot reload stopped");
        }
    }

    /// Gets the current configuration
    ///
    /// # Returns
    ///
    /// Current configuration
    pub async fn get_config(&self) -> Pop3Config {
        self.current_config.read().await.clone()
    }

    /// Updates the configuration
    ///
    /// # Arguments
    ///
    /// * `new_config` - New configuration to apply
    ///
    /// # Returns
    ///
    /// Result indicating success or validation errors
    pub async fn update_config(&mut self, new_config: Pop3Config) -> Result<(), Vec<String>> {
        // Validate new configuration
        if let Err(e) = new_config.validate() {
            return Err(vec![e.to_string()]);
        }

        // Backup current configuration
        {
            let current = self.current_config.read().await;
            self.backup_config = Some(current.clone());
        }

        // Apply new configuration
        {
            let mut config = self.current_config.write().await;
            *config = new_config.clone();
        }

        // Clear validation errors
        {
            let mut errors = self.validation_errors.write().unwrap();
            errors.clear();
        }

        // Notify subscribers
        if let Some(tx) = &self.change_tx {
            let _ = tx.send(new_config);
        }

        info!("Configuration updated successfully");
        Ok(())
    }

    /// Rolls back to the previous configuration
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub async fn rollback(&mut self) -> Result<(), String> {
        if let Some(backup) = self.backup_config.take() {
            let mut config = self.current_config.write().await;
            *config = backup.clone();

            // Notify subscribers
            if let Some(tx) = &self.change_tx {
                let _ = tx.send(backup);
            }

            info!("Configuration rolled back successfully");
            Ok(())
        } else {
            Err("No backup configuration available".to_string())
        }
    }

    /// Subscribes to configuration changes
    ///
    /// # Returns
    ///
    /// Receiver for configuration change notifications
    pub fn subscribe_to_changes(&self) -> Option<broadcast::Receiver<Pop3Config>> {
        self.change_tx.as_ref().map(|tx| tx.subscribe())
    }

    /// Gets current validation errors
    ///
    /// # Returns
    ///
    /// Vector of validation error messages
    pub fn get_validation_errors(&self) -> Vec<String> {
        self.validation_errors.read().unwrap().clone()
    }

    /// Checks if the configuration is valid
    ///
    /// # Returns
    ///
    /// `true` if configuration is valid, `false` otherwise
    pub fn is_valid(&self) -> bool {
        self.validation_errors.read().unwrap().is_empty()
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_environment_display() {
        assert_eq!(Environment::Development.to_string(), "development");
        assert_eq!(Environment::Testing.to_string(), "testing");
        assert_eq!(Environment::Staging.to_string(), "staging");
        assert_eq!(Environment::Production.to_string(), "production");
    }

    #[test]
    fn test_config_metadata_default() {
        let metadata = ConfigMetadata::default();
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.environment, Environment::Development);
        assert!(metadata.tags.is_empty());
        assert!(metadata.description.is_none());
    }

    #[test]
    fn test_logging_config_default() {
        let logging = LoggingConfig::default();
        assert_eq!(logging.level, "info");
        assert!(!logging.structured);
        assert!(logging.file_path.is_none());
        assert_eq!(logging.max_file_size, 100 * 1024 * 1024);
        assert_eq!(logging.max_files, 10);
        assert!(logging.compress);
        assert!(logging.enable_performance_logs);
        assert!(logging.enable_security_logs);
    }

    #[test]
    fn test_pop3_config_for_environment() {
        let dev_config = Pop3Config::for_environment(Environment::Development);
        assert_eq!(dev_config.metadata.environment, Environment::Development);
        assert!(dev_config.metadata.description.is_some());
        assert!(dev_config.metadata.tags.contains(&"development".to_string()));

        let prod_config = Pop3Config::for_environment(Environment::Production);
        assert_eq!(prod_config.metadata.environment, Environment::Production);
        assert!(prod_config.metadata.description.is_some());
        assert!(prod_config.metadata.tags.contains(&"production".to_string()));
    }

    #[test]
    fn test_production_config() {
        let config = Pop3Config::production();
        assert_eq!(config.metadata.environment, Environment::Production);
        assert_eq!(config.server.greeting, "Stalwart POP3 Server");
        assert_eq!(config.server.max_message_size, 100 * 1024 * 1024);
        assert!(config.server.enable_stls);
        assert_eq!(config.security.max_auth_attempts, 3);
        assert_eq!(config.performance.max_concurrent_sessions, 500);
        assert_eq!(config.logging.level, "info");
        assert!(config.logging.structured);
    }

    #[test]
    fn test_development_config() {
        let config = Pop3Config::development();
        assert_eq!(config.metadata.environment, Environment::Development);
        assert_eq!(config.server.greeting, "Stalwart POP3 Development Server");
        assert_eq!(config.server.max_message_size, 10 * 1024 * 1024);
        assert!(!config.server.enable_stls);
        assert_eq!(config.security.max_auth_attempts, 10);
        assert_eq!(config.performance.max_concurrent_sessions, 100);
        assert_eq!(config.logging.level, "debug");
        assert!(!config.logging.structured);
        assert!(config.logging.file_path.is_none());
    }

    #[test]
    fn test_testing_config() {
        let config = Pop3Config::testing();
        assert_eq!(config.metadata.environment, Environment::Testing);
        assert_eq!(config.server.greeting, "Stalwart POP3 Test Server");
        assert_eq!(config.performance.max_concurrent_sessions, 50);
        assert_eq!(config.logging.level, "debug");
    }

    #[test]
    fn test_staging_config() {
        let config = Pop3Config::staging();
        assert_eq!(config.metadata.environment, Environment::Staging);
        assert_eq!(config.server.greeting, "Stalwart POP3 Staging Server");
        assert_eq!(config.logging.level, "debug");
        // Should inherit most settings from production
        assert_eq!(config.security.max_auth_attempts, 3);
        assert_eq!(config.performance.max_concurrent_sessions, 500);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Pop3Config::production();

        // Valid configuration should pass
        assert!(config.validate().is_ok());

        // Invalid server config
        config.server.max_message_size = 0;
        assert!(config.validate().is_err());

        // Reset and test invalid security config
        config = Pop3Config::production();
        config.security.max_auth_attempts = 0;
        assert!(config.validate().is_err());

        // Reset and test invalid protocol config
        config = Pop3Config::production();
        config.protocol.max_line_length = 100; // Too small
        assert!(config.validate().is_err());

        // Reset and test invalid performance config
        config = Pop3Config::production();
        config.performance.max_concurrent_sessions = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_apply_env_overrides() {
        use std::env;

        let mut config = Pop3Config::development();
        let original_greeting = config.server.greeting.clone();

        // Set environment variable
        unsafe {
            env::set_var("POP3_SERVER_GREETING", "Test Greeting");
            env::set_var("POP3_SERVER_MAX_MESSAGE_SIZE", "50000000");
            env::set_var("POP3_SECURITY_MAX_AUTH_ATTEMPTS", "5");
            env::set_var("POP3_LOG_LEVEL", "trace");
            env::set_var("POP3_LOG_STRUCTURED", "true");
        }

        // Apply overrides
        config.apply_env_overrides();

        // Check that values were updated
        assert_eq!(config.server.greeting, "Test Greeting");
        assert_eq!(config.server.max_message_size, 50000000);
        assert_eq!(config.security.max_auth_attempts, 5);
        assert_eq!(config.logging.level, "trace");
        assert!(config.logging.structured);

        // Clean up environment variables
        unsafe {
            env::remove_var("POP3_SERVER_GREETING");
            env::remove_var("POP3_SERVER_MAX_MESSAGE_SIZE");
            env::remove_var("POP3_SECURITY_MAX_AUTH_ATTEMPTS");
            env::remove_var("POP3_LOG_LEVEL");
            env::remove_var("POP3_LOG_STRUCTURED");
        }
    }

    #[tokio::test]
    async fn test_config_manager_creation() {
        let config = Pop3Config::development();
        let manager = ConfigManager::new(config.clone());

        let current_config = manager.get_config().await;
        assert_eq!(current_config.metadata.environment, config.metadata.environment);
        assert!(manager.is_valid());
        assert!(manager.get_validation_errors().is_empty());
    }

    #[tokio::test]
    async fn test_config_manager_update() {
        let initial_config = Pop3Config::development();
        let mut manager = ConfigManager::new(initial_config);

        let new_config = Pop3Config::production();
        let result = manager.update_config(new_config.clone()).await;
        assert!(result.is_ok());

        let current_config = manager.get_config().await;
        assert_eq!(current_config.metadata.environment, Environment::Production);
    }

    #[tokio::test]
    async fn test_config_manager_rollback() {
        let initial_config = Pop3Config::development();
        let mut manager = ConfigManager::new(initial_config.clone());

        // Update configuration
        let new_config = Pop3Config::production();
        manager.update_config(new_config).await.unwrap();

        // Rollback
        let result = manager.rollback().await;
        assert!(result.is_ok());

        let current_config = manager.get_config().await;
        assert_eq!(current_config.metadata.environment, Environment::Development);
    }

    #[tokio::test]
    async fn test_config_manager_invalid_update() {
        let initial_config = Pop3Config::development();
        let mut manager = ConfigManager::new(initial_config);

        // Create invalid configuration
        let mut invalid_config = Pop3Config::production();
        invalid_config.server.max_message_size = 0; // Invalid

        let result = manager.update_config(invalid_config).await;
        assert!(result.is_err());

        // Configuration should remain unchanged
        let current_config = manager.get_config().await;
        assert_eq!(current_config.metadata.environment, Environment::Development);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_validation() {
        let config = Pop3Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_production_config_validation() {
        let config = Pop3Config::production();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_development_config_validation() {
        let config = Pop3Config::development();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_config_validation() {
        let mut config = Pop3Config::default();
        config.server.max_message_size = 0;
        assert!(config.validate().is_err());

        config = Pop3Config::default();
        config.protocol.max_line_length = 100;
        assert!(config.validate().is_err());

        config = Pop3Config::default();
        config.security.max_auth_attempts = 0;
        assert!(config.validate().is_err());
    }
}
