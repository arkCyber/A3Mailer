//! Configuration Loader for A3Mailer
//!
//! This module provides multi-source configuration loading capabilities
//! supporting TOML files, environment variables, command line arguments,
//! and remote configuration sources.

use crate::{A3MailerConfig, ConfigSource, Result, ConfigError};
use std::collections::HashMap;
use std::path::Path;
use tracing::{info, warn, error, debug};
use serde_json::Value;

/// Load configuration from a TOML file
pub async fn load_from_file(path: &Path) -> Result<A3MailerConfig> {
    info!("Loading configuration from file: {}", path.display());
    
    if !path.exists() {
        return Err(ConfigError::FileNotFound(path.to_string_lossy().to_string()));
    }
    
    let content = tokio::fs::read_to_string(path).await
        .map_err(|e| ConfigError::IoError(e.to_string()))?;
    
    let config: A3MailerConfig = toml::from_str(&content)
        .map_err(|e| ConfigError::ParseError(format!("TOML parse error: {}", e)))?;
    
    info!("Successfully loaded configuration from file: {}", path.display());
    Ok(config)
}

/// Load configuration from environment variables
pub async fn load_from_environment() -> Result<A3MailerConfig> {
    info!("Loading configuration from environment variables");
    
    let mut config = A3MailerConfig::default();
    
    // Server configuration
    if let Ok(hostname) = std::env::var("A3MAILER_HOSTNAME") {
        config.server.hostname = hostname;
    }
    
    if let Ok(bind_addresses) = std::env::var("A3MAILER_BIND_ADDRESSES") {
        config.server.bind_addresses = bind_addresses
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
    }
    
    if let Ok(max_connections) = std::env::var("A3MAILER_MAX_CONNECTIONS") {
        config.server.max_connections = max_connections.parse()
            .map_err(|e| ConfigError::ParseError(format!("Invalid max_connections: {}", e)))?;
    }
    
    if let Ok(worker_threads) = std::env::var("A3MAILER_WORKER_THREADS") {
        config.server.worker_threads = Some(worker_threads.parse()
            .map_err(|e| ConfigError::ParseError(format!("Invalid worker_threads: {}", e)))?);
    }
    
    // AI configuration
    if let Ok(ai_enabled) = std::env::var("A3MAILER_AI_ENABLED") {
        config.ai.enabled = ai_enabled.parse()
            .map_err(|e| ConfigError::ParseError(format!("Invalid ai_enabled: {}", e)))?;
    }
    
    if let Ok(model_path) = std::env::var("A3MAILER_AI_MODEL_PATH") {
        config.ai.model_path = model_path;
    }
    
    if let Ok(threat_threshold) = std::env::var("A3MAILER_AI_THREAT_THRESHOLD") {
        config.ai.threat_detection.confidence_threshold = threat_threshold.parse()
            .map_err(|e| ConfigError::ParseError(format!("Invalid threat_threshold: {}", e)))?;
    }
    
    // Web3 configuration
    if let Ok(web3_enabled) = std::env::var("A3MAILER_WEB3_ENABLED") {
        config.web3.enabled = web3_enabled.parse()
            .map_err(|e| ConfigError::ParseError(format!("Invalid web3_enabled: {}", e)))?;
    }
    
    if let Ok(blockchain_network) = std::env::var("A3MAILER_BLOCKCHAIN_NETWORK") {
        config.web3.blockchain_network = blockchain_network;
    }
    
    if let Ok(rpc_url) = std::env::var("A3MAILER_RPC_URL") {
        config.web3.rpc_url = rpc_url;
    }
    
    if let Ok(did_resolver) = std::env::var("A3MAILER_DID_RESOLVER_URL") {
        config.web3.did.resolver_url = did_resolver;
    }
    
    if let Ok(ipfs_gateway) = std::env::var("A3MAILER_IPFS_GATEWAY") {
        config.web3.ipfs.gateway_url = ipfs_gateway;
    }
    
    // Storage configuration
    if let Ok(storage_backend) = std::env::var("A3MAILER_STORAGE_BACKEND") {
        config.storage.backend = storage_backend;
    }
    
    if let Ok(connection_string) = std::env::var("A3MAILER_DATABASE_URL") {
        config.storage.connection_string = connection_string;
    }
    
    if let Ok(max_connections) = std::env::var("A3MAILER_DB_MAX_CONNECTIONS") {
        config.storage.max_connections = max_connections.parse()
            .map_err(|e| ConfigError::ParseError(format!("Invalid db_max_connections: {}", e)))?;
    }
    
    // Security configuration
    if let Ok(tls_cert) = std::env::var("A3MAILER_TLS_CERT_FILE") {
        config.server.tls.cert_file = tls_cert;
    }
    
    if let Ok(tls_key) = std::env::var("A3MAILER_TLS_KEY_FILE") {
        config.server.tls.key_file = tls_key;
    }
    
    if let Ok(tls_enabled) = std::env::var("A3MAILER_TLS_ENABLED") {
        config.server.tls.enabled = tls_enabled.parse()
            .map_err(|e| ConfigError::ParseError(format!("Invalid tls_enabled: {}", e)))?;
    }
    
    // Monitoring configuration
    if let Ok(monitoring_enabled) = std::env::var("A3MAILER_MONITORING_ENABLED") {
        config.monitoring.enabled = monitoring_enabled.parse()
            .map_err(|e| ConfigError::ParseError(format!("Invalid monitoring_enabled: {}", e)))?;
    }
    
    if let Ok(metrics_port) = std::env::var("A3MAILER_METRICS_PORT") {
        config.monitoring.metrics_port = metrics_port.parse()
            .map_err(|e| ConfigError::ParseError(format!("Invalid metrics_port: {}", e)))?;
    }
    
    // Logging configuration
    if let Ok(log_level) = std::env::var("A3MAILER_LOG_LEVEL") {
        config.logging.level = log_level;
    }
    
    if let Ok(log_format) = std::env::var("A3MAILER_LOG_FORMAT") {
        config.logging.format = log_format;
    }
    
    // Enterprise configuration
    if let Ok(license_key) = std::env::var("A3MAILER_LICENSE_KEY") {
        config.enterprise.license_key = Some(license_key);
    }
    
    info!("Successfully loaded configuration from environment variables");
    Ok(config)
}

/// Load configuration from command line arguments
pub async fn load_from_command_line(args: &[String]) -> Result<A3MailerConfig> {
    info!("Loading configuration from command line arguments");
    
    let mut config = A3MailerConfig::default();
    let mut i = 0;
    
    while i < args.len() {
        let arg = &args[i];
        
        match arg.as_str() {
            "--hostname" => {
                if i + 1 < args.len() {
                    config.server.hostname = args[i + 1].clone();
                    i += 1;
                }
            }
            "--port" => {
                if i + 1 < args.len() {
                    let port: u16 = args[i + 1].parse()
                        .map_err(|e| ConfigError::ParseError(format!("Invalid port: {}", e)))?;
                    config.server.bind_addresses = vec![format!("0.0.0.0:{}", port)];
                    i += 1;
                }
            }
            "--max-connections" => {
                if i + 1 < args.len() {
                    config.server.max_connections = args[i + 1].parse()
                        .map_err(|e| ConfigError::ParseError(format!("Invalid max-connections: {}", e)))?;
                    i += 1;
                }
            }
            "--ai-enabled" => {
                if i + 1 < args.len() {
                    config.ai.enabled = args[i + 1].parse()
                        .map_err(|e| ConfigError::ParseError(format!("Invalid ai-enabled: {}", e)))?;
                    i += 1;
                }
            }
            "--web3-enabled" => {
                if i + 1 < args.len() {
                    config.web3.enabled = args[i + 1].parse()
                        .map_err(|e| ConfigError::ParseError(format!("Invalid web3-enabled: {}", e)))?;
                    i += 1;
                }
            }
            "--database-url" => {
                if i + 1 < args.len() {
                    config.storage.connection_string = args[i + 1].clone();
                    i += 1;
                }
            }
            "--log-level" => {
                if i + 1 < args.len() {
                    config.logging.level = args[i + 1].clone();
                    i += 1;
                }
            }
            "--metrics-port" => {
                if i + 1 < args.len() {
                    config.monitoring.metrics_port = args[i + 1].parse()
                        .map_err(|e| ConfigError::ParseError(format!("Invalid metrics-port: {}", e)))?;
                    i += 1;
                }
            }
            "--license-key" => {
                if i + 1 < args.len() {
                    config.enterprise.license_key = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            _ => {
                // Ignore unknown arguments
                debug!("Ignoring unknown command line argument: {}", arg);
            }
        }
        
        i += 1;
    }
    
    info!("Successfully loaded configuration from command line arguments");
    Ok(config)
}

/// Load configuration from remote source
pub async fn load_from_remote(url: &str) -> Result<A3MailerConfig> {
    info!("Loading configuration from remote source: {}", url);
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| ConfigError::NetworkError(e.to_string()))?;
    
    let response = client
        .get(url)
        .header("Accept", "application/toml, application/json")
        .send()
        .await
        .map_err(|e| ConfigError::NetworkError(e.to_string()))?;
    
    if !response.status().is_success() {
        return Err(ConfigError::NetworkError(format!(
            "Remote config request failed with status: {}", 
            response.status()
        )));
    }
    
    let content_type = response.headers()
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .unwrap_or("application/toml");
    
    let content = response.text().await
        .map_err(|e| ConfigError::NetworkError(e.to_string()))?;
    
    let config = if content_type.contains("json") {
        // Parse as JSON first, then convert to config
        let json_value: Value = serde_json::from_str(&content)
            .map_err(|e| ConfigError::ParseError(format!("JSON parse error: {}", e)))?;
        
        serde_json::from_value(json_value)
            .map_err(|e| ConfigError::ParseError(format!("JSON to config conversion error: {}", e)))?
    } else {
        // Parse as TOML
        toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(format!("TOML parse error: {}", e)))?
    };
    
    info!("Successfully loaded configuration from remote source: {}", url);
    Ok(config)
}

/// Merge two configurations, with the second one taking precedence
pub fn merge_configs(base: A3MailerConfig, override_config: A3MailerConfig) -> Result<A3MailerConfig> {
    debug!("Merging configurations");
    
    // For now, we'll do a simple override merge
    // In a production system, this would be more sophisticated
    let mut merged = base;
    
    // Server config
    if override_config.server.hostname != "localhost" {
        merged.server.hostname = override_config.server.hostname;
    }
    if !override_config.server.bind_addresses.is_empty() {
        merged.server.bind_addresses = override_config.server.bind_addresses;
    }
    if override_config.server.max_connections != 10000 {
        merged.server.max_connections = override_config.server.max_connections;
    }
    if override_config.server.worker_threads.is_some() {
        merged.server.worker_threads = override_config.server.worker_threads;
    }
    if override_config.server.timeout_seconds != 300 {
        merged.server.timeout_seconds = override_config.server.timeout_seconds;
    }
    
    // AI config
    merged.ai.enabled = override_config.ai.enabled;
    if override_config.ai.model_path != "models/" {
        merged.ai.model_path = override_config.ai.model_path;
    }
    
    // Web3 config
    merged.web3.enabled = override_config.web3.enabled;
    if override_config.web3.blockchain_network != "ethereum" {
        merged.web3.blockchain_network = override_config.web3.blockchain_network;
    }
    if !override_config.web3.rpc_url.contains("YOUR_PROJECT_ID") {
        merged.web3.rpc_url = override_config.web3.rpc_url;
    }
    
    // Storage config
    if override_config.storage.backend != "postgresql" {
        merged.storage.backend = override_config.storage.backend;
    }
    if !override_config.storage.connection_string.contains("user:pass@localhost") {
        merged.storage.connection_string = override_config.storage.connection_string;
    }
    if override_config.storage.max_connections != 100 {
        merged.storage.max_connections = override_config.storage.max_connections;
    }
    
    // Monitoring config
    merged.monitoring.enabled = override_config.monitoring.enabled;
    if override_config.monitoring.metrics_port != 9090 {
        merged.monitoring.metrics_port = override_config.monitoring.metrics_port;
    }
    
    // Logging config
    if override_config.logging.level != "info" {
        merged.logging.level = override_config.logging.level;
    }
    if override_config.logging.format != "json" {
        merged.logging.format = override_config.logging.format;
    }
    merged.logging.structured = override_config.logging.structured;
    
    // Enterprise config
    if override_config.enterprise.license_key.is_some() {
        merged.enterprise.license_key = override_config.enterprise.license_key;
    }
    
    debug!("Configuration merge completed");
    Ok(merged)
}

/// Validate configuration file format
pub fn validate_config_file(path: &Path) -> Result<()> {
    debug!("Validating configuration file format: {}", path.display());
    
    if !path.exists() {
        return Err(ConfigError::FileNotFound(path.to_string_lossy().to_string()));
    }
    
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    
    match extension.to_lowercase().as_str() {
        "toml" => {
            // Validate TOML syntax
            let content = std::fs::read_to_string(path)
                .map_err(|e| ConfigError::IoError(e.to_string()))?;
            
            toml::from_str::<Value>(&content)
                .map_err(|e| ConfigError::ParseError(format!("Invalid TOML syntax: {}", e)))?;
        }
        "json" => {
            // Validate JSON syntax
            let content = std::fs::read_to_string(path)
                .map_err(|e| ConfigError::IoError(e.to_string()))?;
            
            serde_json::from_str::<Value>(&content)
                .map_err(|e| ConfigError::ParseError(format!("Invalid JSON syntax: {}", e)))?;
        }
        _ => {
            return Err(ConfigError::UnsupportedFormat(format!(
                "Unsupported configuration file format: {}", 
                extension
            )));
        }
    }
    
    info!("Configuration file format validation successful: {}", path.display());
    Ok(())
}
