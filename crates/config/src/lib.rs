//! # A3Mailer Configuration Management
//!
//! Production-grade configuration management for A3Mailer with support for
//! multiple configuration sources, environment variable overrides, and
//! runtime configuration updates.
//!
//! ## Features
//!
//! - **Multi-source Configuration**: TOML files, environment variables, command line
//! - **Hot Reloading**: Runtime configuration updates without restart
//! - **Validation**: Comprehensive configuration validation
//! - **Secrets Management**: Secure handling of sensitive configuration
//! - **Environment-specific**: Development, staging, production configurations
//! - **AI/Web3 Configuration**: Specialized configuration for AI and Web3 features
//!
//! ## Architecture
//!
//! The configuration system consists of:
//! - Config Loader: Multi-source configuration loading
//! - Config Validator: Configuration validation and sanitization
//! - Config Watcher: Hot reload and change detection
//! - Secrets Manager: Secure secrets handling
//!
//! ## Example
//!
//! ```rust,no_run
//! use a3mailer_config::{ConfigManager, ConfigSource};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config_manager = ConfigManager::builder()
//!         .add_source(ConfigSource::File("config.toml"))
//!         .add_source(ConfigSource::Environment)
//!         .build()
//!         .await?;
//!
//!     let config = config_manager.get_config().await?;
//!     println!("Server hostname: {}", config.server.hostname);
//!
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

pub mod loader;
pub mod validator;
pub mod watcher;
pub mod secrets;
pub mod error;

pub use error::{ConfigError, Result};

/// Main A3Mailer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A3MailerConfig {
    pub server: ServerConfig,
    pub ai: AiConfig,
    pub web3: Web3Config,
    pub storage: StorageConfig,
    pub security: SecurityConfig,
    pub monitoring: MonitoringConfig,
    pub logging: LoggingConfig,
    pub protocols: ProtocolsConfig,
    pub enterprise: EnterpriseConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub hostname: String,
    pub bind_addresses: Vec<String>,
    pub max_connections: u32,
    pub worker_threads: Option<u32>,
    pub timeout_seconds: u64,
    pub tls: TlsConfig,
}

/// AI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub enabled: bool,
    pub model_path: String,
    pub threat_detection: ThreatDetectionConfig,
    pub content_analysis: ContentAnalysisConfig,
    pub behavioral_analysis: BehavioralAnalysisConfig,
    pub performance: AiPerformanceConfig,
}

/// Web3 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Web3Config {
    pub enabled: bool,
    pub blockchain_network: String,
    pub rpc_url: String,
    pub contract_addresses: HashMap<String, String>,
    pub did: DidConfig,
    pub ipfs: IpfsConfig,
    pub smart_contracts: SmartContractsConfig,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub backend: String,
    pub connection_string: String,
    pub max_connections: u32,
    pub cache: CacheConfig,
    pub replication: ReplicationConfig,
    pub backup: BackupConfig,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub encryption: EncryptionConfig,
    pub authentication: AuthenticationConfig,
    pub authorization: AuthorizationConfig,
    pub rate_limiting: RateLimitingConfig,
    pub firewall: FirewallConfig,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub metrics_port: u16,
    pub health_checks: HealthCheckConfig,
    pub alerting: AlertingConfig,
    pub tracing: TracingConfig,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub output: Vec<LogOutput>,
    pub rotation: LogRotationConfig,
    pub structured: bool,
}

/// Protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolsConfig {
    pub smtp: SmtpConfig,
    pub imap: ImapConfig,
    pub pop3: Pop3Config,
    pub jmap: JmapConfig,
    pub caldav: CaldavConfig,
    pub carddav: CarddavConfig,
}

/// Enterprise configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseConfig {
    pub license_key: Option<String>,
    pub clustering: ClusteringConfig,
    pub compliance: ComplianceConfig,
    pub audit: AuditConfig,
    pub sso: SsoConfig,
}

// Sub-configuration structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    pub cert_file: String,
    pub key_file: String,
    pub protocols: Vec<String>,
    pub ciphers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatDetectionConfig {
    pub enabled: bool,
    pub model_file: String,
    pub confidence_threshold: f64,
    pub update_interval: String,
    pub real_time_scanning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentAnalysisConfig {
    pub enabled: bool,
    pub nlp_model: String,
    pub sentiment_analysis: bool,
    pub language_detection: bool,
    pub content_classification: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralAnalysisConfig {
    pub enabled: bool,
    pub learning_rate: f64,
    pub anomaly_threshold: f64,
    pub training_interval: String,
    pub user_profiling: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPerformanceConfig {
    pub max_inference_time_ms: u64,
    pub batch_size: u32,
    pub gpu_enabled: bool,
    pub model_cache_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidConfig {
    pub enabled: bool,
    pub resolver_url: String,
    pub supported_methods: Vec<String>,
    pub cache_ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsConfig {
    pub enabled: bool,
    pub gateway_url: String,
    pub api_url: String,
    pub pinning_service: String,
    pub max_file_size_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartContractsConfig {
    pub enabled: bool,
    pub gas_limit: u64,
    pub gas_price: String,
    pub contract_addresses: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub backend: String,
    pub connection_string: String,
    pub ttl_seconds: u64,
    pub max_memory_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    pub enabled: bool,
    pub mode: String, // master-slave, multi-master, sharded
    pub nodes: Vec<String>,
    pub sync_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub enabled: bool,
    pub schedule: String,
    pub retention_days: u32,
    pub compression: String,
    pub encryption: bool,
    pub destinations: Vec<BackupDestination>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupDestination {
    pub name: String,
    pub backend: String, // local, s3, gcs, azure
    pub config: HashMap<String, String>,
}

// Additional configuration structures would continue here...

/// Configuration source enumeration
#[derive(Debug, Clone)]
pub enum ConfigSource {
    File(PathBuf),
    Environment,
    CommandLine(Vec<String>),
    Remote(String),
}

/// Configuration manager
pub struct ConfigManager {
    config: Arc<RwLock<A3MailerConfig>>,
    sources: Vec<ConfigSource>,
    watcher: Option<watcher::ConfigWatcher>,
    secrets_manager: secrets::SecretsManager,
    last_updated: Arc<RwLock<DateTime<Utc>>>,
}

impl ConfigManager {
    /// Create a new configuration manager builder
    pub fn builder() -> ConfigManagerBuilder {
        ConfigManagerBuilder::new()
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> Result<A3MailerConfig> {
        let config = self.config.read().await;
        Ok(config.clone())
    }

    /// Reload configuration from sources
    pub async fn reload_config(&self) -> Result<()> {
        info!("Reloading configuration from sources");
        
        let mut new_config = self.load_config_from_sources().await?;
        
        // Validate the new configuration
        validator::validate_config(&new_config).await?;
        
        // Apply secrets
        self.secrets_manager.apply_secrets(&mut new_config).await?;
        
        // Update the configuration
        {
            let mut config = self.config.write().await;
            *config = new_config;
        }
        
        // Update timestamp
        {
            let mut last_updated = self.last_updated.write().await;
            *last_updated = Utc::now();
        }
        
        info!("Configuration reloaded successfully");
        Ok(())
    }

    /// Load configuration from all sources
    async fn load_config_from_sources(&self) -> Result<A3MailerConfig> {
        let mut config = A3MailerConfig::default();
        
        for source in &self.sources {
            match source {
                ConfigSource::File(path) => {
                    let file_config = loader::load_from_file(path).await?;
                    config = loader::merge_configs(config, file_config)?;
                }
                ConfigSource::Environment => {
                    let env_config = loader::load_from_environment().await?;
                    config = loader::merge_configs(config, env_config)?;
                }
                ConfigSource::CommandLine(args) => {
                    let cli_config = loader::load_from_command_line(args).await?;
                    config = loader::merge_configs(config, cli_config)?;
                }
                ConfigSource::Remote(url) => {
                    let remote_config = loader::load_from_remote(url).await?;
                    config = loader::merge_configs(config, remote_config)?;
                }
            }
        }
        
        Ok(config)
    }

    /// Get configuration last updated timestamp
    pub async fn get_last_updated(&self) -> DateTime<Utc> {
        let last_updated = self.last_updated.read().await;
        *last_updated
    }

    /// Start configuration watching for hot reload
    pub async fn start_watching(&mut self) -> Result<()> {
        if self.watcher.is_some() {
            warn!("Configuration watcher is already running");
            return Ok(());
        }
        
        info!("Starting configuration watcher for hot reload");
        
        let watcher = watcher::ConfigWatcher::new(
            self.sources.clone(),
            Arc::clone(&self.config),
        ).await?;
        
        self.watcher = Some(watcher);
        
        info!("Configuration watcher started");
        Ok(())
    }

    /// Stop configuration watching
    pub async fn stop_watching(&mut self) -> Result<()> {
        if let Some(watcher) = self.watcher.take() {
            info!("Stopping configuration watcher");
            watcher.stop().await?;
            info!("Configuration watcher stopped");
        }
        
        Ok(())
    }
}

/// Configuration manager builder
pub struct ConfigManagerBuilder {
    sources: Vec<ConfigSource>,
}

impl ConfigManagerBuilder {
    fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Add a configuration source
    pub fn add_source(mut self, source: ConfigSource) -> Self {
        self.sources.push(source);
        self
    }

    /// Build the configuration manager
    pub async fn build(self) -> Result<ConfigManager> {
        info!("Building configuration manager with {} sources", self.sources.len());
        
        let secrets_manager = secrets::SecretsManager::new().await?;
        
        let manager = ConfigManager {
            config: Arc::new(RwLock::new(A3MailerConfig::default())),
            sources: self.sources,
            watcher: None,
            secrets_manager,
            last_updated: Arc::new(RwLock::new(Utc::now())),
        };
        
        // Load initial configuration
        manager.reload_config().await?;
        
        info!("Configuration manager built successfully");
        Ok(manager)
    }
}

impl Default for A3MailerConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            ai: AiConfig::default(),
            web3: Web3Config::default(),
            storage: StorageConfig::default(),
            security: SecurityConfig::default(),
            monitoring: MonitoringConfig::default(),
            logging: LoggingConfig::default(),
            protocols: ProtocolsConfig::default(),
            enterprise: EnterpriseConfig::default(),
        }
    }
}

// Default implementations for all config structures would continue here...

/// Initialize configuration system
pub async fn init_config(sources: Vec<ConfigSource>) -> Result<ConfigManager> {
    info!("Initializing A3Mailer configuration system");
    
    let mut builder = ConfigManager::builder();
    for source in sources {
        builder = builder.add_source(source);
    }
    
    let manager = builder.build().await?;
    
    info!("A3Mailer configuration system initialized successfully");
    Ok(manager)
}
