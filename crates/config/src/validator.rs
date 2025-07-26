//! Configuration Validator for A3Mailer
//!
//! This module provides comprehensive configuration validation to ensure
//! all settings are valid, secure, and compatible with the system requirements.

use crate::{A3MailerConfig, Result, ConfigError};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use tracing::{info, warn, error, debug};
use url::Url;

/// Validation severity levels
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub severity: ValidationSeverity,
    pub field: String,
    pub message: String,
    pub suggestion: Option<String>,
}

/// Configuration validator
pub struct ConfigValidator {
    results: Vec<ValidationResult>,
    strict_mode: bool,
}

impl ConfigValidator {
    /// Create a new configuration validator
    pub fn new(strict_mode: bool) -> Self {
        Self {
            results: Vec::new(),
            strict_mode,
        }
    }

    /// Add validation result
    fn add_result(&mut self, severity: ValidationSeverity, field: &str, message: &str, suggestion: Option<&str>) {
        self.results.push(ValidationResult {
            severity,
            field: field.to_string(),
            message: message.to_string(),
            suggestion: suggestion.map(|s| s.to_string()),
        });
    }

    /// Add error result
    fn add_error(&mut self, field: &str, message: &str) {
        self.add_result(ValidationSeverity::Error, field, message, None);
    }

    /// Add warning result
    fn add_warning(&mut self, field: &str, message: &str, suggestion: Option<&str>) {
        self.add_result(ValidationSeverity::Warning, field, message, suggestion);
    }

    /// Add info result
    fn add_info(&mut self, field: &str, message: &str) {
        self.add_result(ValidationSeverity::Info, field, message, None);
    }

    /// Get validation results
    pub fn get_results(&self) -> &[ValidationResult] {
        &self.results
    }

    /// Check if validation passed (no errors)
    pub fn is_valid(&self) -> bool {
        !self.results.iter().any(|r| r.severity == ValidationSeverity::Error)
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.results.iter().filter(|r| r.severity == ValidationSeverity::Error).count()
    }

    /// Get warning count
    pub fn warning_count(&self) -> usize {
        self.results.iter().filter(|r| r.severity == ValidationSeverity::Warning).count()
    }
}

/// Validate complete configuration
pub async fn validate_config(config: &A3MailerConfig) -> Result<()> {
    info!("Validating A3Mailer configuration");
    
    let mut validator = ConfigValidator::new(true);
    
    // Validate each configuration section
    validate_server_config(&mut validator, &config.server).await;
    validate_ai_config(&mut validator, &config.ai).await;
    validate_web3_config(&mut validator, &config.web3).await;
    validate_storage_config(&mut validator, &config.storage).await;
    validate_security_config(&mut validator, &config.security).await;
    validate_monitoring_config(&mut validator, &config.monitoring).await;
    validate_logging_config(&mut validator, &config.logging).await;
    validate_protocols_config(&mut validator, &config.protocols).await;
    validate_enterprise_config(&mut validator, &config.enterprise).await;
    
    // Cross-validation checks
    validate_cross_dependencies(&mut validator, config).await;
    
    // Report results
    let error_count = validator.error_count();
    let warning_count = validator.warning_count();
    
    if error_count > 0 {
        error!("Configuration validation failed with {} errors and {} warnings", error_count, warning_count);
        
        for result in validator.get_results() {
            match result.severity {
                ValidationSeverity::Error => error!("ERROR [{}]: {}", result.field, result.message),
                ValidationSeverity::Warning => warn!("WARNING [{}]: {}", result.field, result.message),
                ValidationSeverity::Info => info!("INFO [{}]: {}", result.field, result.message),
            }
        }
        
        return Err(ConfigError::ValidationError(format!(
            "Configuration validation failed with {} errors", 
            error_count
        )));
    }
    
    if warning_count > 0 {
        warn!("Configuration validation completed with {} warnings", warning_count);
        for result in validator.get_results() {
            if result.severity == ValidationSeverity::Warning {
                warn!("WARNING [{}]: {}", result.field, result.message);
                if let Some(suggestion) = &result.suggestion {
                    warn!("  Suggestion: {}", suggestion);
                }
            }
        }
    } else {
        info!("Configuration validation completed successfully");
    }
    
    Ok(())
}

/// Validate server configuration
async fn validate_server_config(validator: &mut ConfigValidator, config: &crate::ServerConfig) {
    debug!("Validating server configuration");
    
    // Validate hostname
    if config.hostname.is_empty() {
        validator.add_error("server.hostname", "Hostname cannot be empty");
    } else if config.hostname.len() > 253 {
        validator.add_error("server.hostname", "Hostname is too long (max 253 characters)");
    }
    
    // Validate bind addresses
    if config.bind_addresses.is_empty() {
        validator.add_error("server.bind_addresses", "At least one bind address must be specified");
    } else {
        let mut seen_ports = HashSet::new();
        for (i, addr) in config.bind_addresses.iter().enumerate() {
            match addr.parse::<SocketAddr>() {
                Ok(socket_addr) => {
                    if !seen_ports.insert(socket_addr.port()) {
                        validator.add_warning(
                            &format!("server.bind_addresses[{}]", i),
                            &format!("Duplicate port {} detected", socket_addr.port()),
                            Some("Consider using different ports for different services")
                        );
                    }
                    
                    // Check for privileged ports
                    if socket_addr.port() < 1024 {
                        validator.add_warning(
                            &format!("server.bind_addresses[{}]", i),
                            &format!("Using privileged port {} requires root privileges", socket_addr.port()),
                            Some("Consider using ports >= 1024 for non-root operation")
                        );
                    }
                }
                Err(_) => {
                    validator.add_error(
                        &format!("server.bind_addresses[{}]", i),
                        &format!("Invalid socket address format: {}", addr)
                    );
                }
            }
        }
    }
    
    // Validate connection limits
    if config.max_connections == 0 {
        validator.add_error("server.max_connections", "Max connections must be greater than 0");
    } else if config.max_connections > 1_000_000 {
        validator.add_warning(
            "server.max_connections",
            "Very high connection limit may cause resource exhaustion",
            Some("Consider system limits and available memory")
        );
    }
    
    // Validate worker threads
    if let Some(threads) = config.worker_threads {
        if threads == 0 {
            validator.add_error("server.worker_threads", "Worker threads must be greater than 0");
        } else if threads > 1000 {
            validator.add_warning(
                "server.worker_threads",
                "Very high thread count may cause performance issues",
                Some("Consider using a reasonable number based on CPU cores")
            );
        }
    }
    
    // Validate timeout
    if config.timeout_seconds == 0 {
        validator.add_error("server.timeout_seconds", "Timeout must be greater than 0");
    } else if config.timeout_seconds > 3600 {
        validator.add_warning(
            "server.timeout_seconds",
            "Very long timeout may cause resource leaks",
            Some("Consider using a shorter timeout (e.g., 300 seconds)")
        );
    }
    
    // Validate TLS configuration
    validate_tls_config(validator, &config.tls).await;
}

/// Validate TLS configuration
async fn validate_tls_config(validator: &mut ConfigValidator, config: &crate::TlsConfig) {
    if config.enabled {
        // Validate certificate file
        if config.cert_file.is_empty() {
            validator.add_error("server.tls.cert_file", "TLS certificate file path cannot be empty when TLS is enabled");
        } else if !Path::new(&config.cert_file).exists() {
            validator.add_error("server.tls.cert_file", &format!("TLS certificate file not found: {}", config.cert_file));
        }
        
        // Validate key file
        if config.key_file.is_empty() {
            validator.add_error("server.tls.key_file", "TLS private key file path cannot be empty when TLS is enabled");
        } else if !Path::new(&config.key_file).exists() {
            validator.add_error("server.tls.key_file", &format!("TLS private key file not found: {}", config.key_file));
        }
        
        // Validate protocols
        if config.protocols.is_empty() {
            validator.add_warning(
                "server.tls.protocols",
                "No TLS protocols specified, using defaults",
                Some("Consider explicitly specifying supported TLS versions")
            );
        } else {
            for protocol in &config.protocols {
                match protocol.as_str() {
                    "TLSv1.2" | "TLSv1.3" => {
                        // Valid protocols
                    }
                    "TLSv1.0" | "TLSv1.1" => {
                        validator.add_warning(
                            "server.tls.protocols",
                            &format!("Deprecated TLS protocol: {}", protocol),
                            Some("Consider using TLSv1.2 or TLSv1.3 only")
                        );
                    }
                    _ => {
                        validator.add_error(
                            "server.tls.protocols",
                            &format!("Unknown TLS protocol: {}", protocol)
                        );
                    }
                }
            }
        }
    }
}

/// Validate AI configuration
async fn validate_ai_config(validator: &mut ConfigValidator, config: &crate::AiConfig) {
    debug!("Validating AI configuration");
    
    if config.enabled {
        // Validate model path
        if config.model_path.is_empty() {
            validator.add_error("ai.model_path", "AI model path cannot be empty when AI is enabled");
        } else if !Path::new(&config.model_path).exists() {
            validator.add_warning(
                "ai.model_path",
                &format!("AI model path does not exist: {}", config.model_path),
                Some("Ensure the model directory exists and contains required models")
            );
        }
        
        // Validate threat detection config
        if config.threat_detection.confidence_threshold < 0.0 || config.threat_detection.confidence_threshold > 1.0 {
            validator.add_error("ai.threat_detection.confidence_threshold", "Confidence threshold must be between 0.0 and 1.0");
        }
        
        // Validate performance config
        if config.performance.max_inference_time_ms == 0 {
            validator.add_error("ai.performance.max_inference_time_ms", "Max inference time must be greater than 0");
        } else if config.performance.max_inference_time_ms > 10000 {
            validator.add_warning(
                "ai.performance.max_inference_time_ms",
                "Very long inference timeout may affect user experience",
                Some("Consider using a shorter timeout (e.g., 100ms)")
            );
        }
        
        if config.performance.batch_size == 0 {
            validator.add_error("ai.performance.batch_size", "Batch size must be greater than 0");
        }
    }
}

/// Validate Web3 configuration
async fn validate_web3_config(validator: &mut ConfigValidator, config: &crate::Web3Config) {
    debug!("Validating Web3 configuration");
    
    if config.enabled {
        // Validate blockchain network
        let valid_networks = ["ethereum", "polygon", "bsc", "avalanche", "arbitrum", "optimism"];
        if !valid_networks.contains(&config.blockchain_network.as_str()) {
            validator.add_warning(
                "web3.blockchain_network",
                &format!("Unknown blockchain network: {}", config.blockchain_network),
                Some("Consider using a well-known network name")
            );
        }
        
        // Validate RPC URL
        if config.rpc_url.is_empty() {
            validator.add_error("web3.rpc_url", "RPC URL cannot be empty when Web3 is enabled");
        } else if config.rpc_url.contains("YOUR_PROJECT_ID") {
            validator.add_error("web3.rpc_url", "RPC URL contains placeholder text, please configure with actual endpoint");
        } else {
            match Url::parse(&config.rpc_url) {
                Ok(url) => {
                    if url.scheme() != "https" && url.scheme() != "http" && url.scheme() != "wss" && url.scheme() != "ws" {
                        validator.add_error("web3.rpc_url", "RPC URL must use http, https, ws, or wss scheme");
                    }
                }
                Err(_) => {
                    validator.add_error("web3.rpc_url", "Invalid RPC URL format");
                }
            }
        }
        
        // Validate DID configuration
        if config.did.resolver_url.is_empty() {
            validator.add_warning(
                "web3.did.resolver_url",
                "DID resolver URL is empty",
                Some("Configure a DID resolver for Web3 identity features")
            );
        } else {
            match Url::parse(&config.did.resolver_url) {
                Ok(_) => {
                    // Valid URL
                }
                Err(_) => {
                    validator.add_error("web3.did.resolver_url", "Invalid DID resolver URL format");
                }
            }
        }
        
        // Validate IPFS configuration
        if config.ipfs.gateway_url.is_empty() {
            validator.add_warning(
                "web3.ipfs.gateway_url",
                "IPFS gateway URL is empty",
                Some("Configure an IPFS gateway for decentralized storage")
            );
        } else {
            match Url::parse(&config.ipfs.gateway_url) {
                Ok(_) => {
                    // Valid URL
                }
                Err(_) => {
                    validator.add_error("web3.ipfs.gateway_url", "Invalid IPFS gateway URL format");
                }
            }
        }
        
        // Validate smart contracts configuration
        if config.smart_contracts.gas_limit == 0 {
            validator.add_error("web3.smart_contracts.gas_limit", "Gas limit must be greater than 0");
        } else if config.smart_contracts.gas_limit > 10_000_000 {
            validator.add_warning(
                "web3.smart_contracts.gas_limit",
                "Very high gas limit may cause expensive transactions",
                Some("Consider using a reasonable gas limit based on contract complexity")
            );
        }
    }
}

/// Validate storage configuration
async fn validate_storage_config(validator: &mut ConfigValidator, config: &crate::StorageConfig) {
    debug!("Validating storage configuration");
    
    // Validate backend
    let valid_backends = ["postgresql", "mysql", "sqlite", "mongodb"];
    if !valid_backends.contains(&config.backend.as_str()) {
        validator.add_warning(
            "storage.backend",
            &format!("Unknown storage backend: {}", config.backend),
            Some("Consider using a supported backend: postgresql, mysql, sqlite, mongodb")
        );
    }
    
    // Validate connection string
    if config.connection_string.is_empty() {
        validator.add_error("storage.connection_string", "Database connection string cannot be empty");
    } else if config.connection_string.contains("user:pass@localhost") {
        validator.add_warning(
            "storage.connection_string",
            "Using default database credentials",
            Some("Configure proper database credentials for production")
        );
    }
    
    // Validate connection pool
    if config.max_connections == 0 {
        validator.add_error("storage.max_connections", "Max database connections must be greater than 0");
    } else if config.max_connections > 1000 {
        validator.add_warning(
            "storage.max_connections",
            "Very high connection pool size may exhaust database resources",
            Some("Consider using a reasonable connection pool size (e.g., 100)")
        );
    }
}

/// Validate security configuration
async fn validate_security_config(validator: &mut ConfigValidator, _config: &crate::SecurityConfig) {
    debug!("Validating security configuration");
    
    // Security validation would be implemented here
    // For now, we'll add a placeholder
    validator.add_info("security", "Security configuration validation not yet implemented");
}

/// Validate monitoring configuration
async fn validate_monitoring_config(validator: &mut ConfigValidator, config: &crate::MonitoringConfig) {
    debug!("Validating monitoring configuration");
    
    // Validate metrics port
    if config.metrics_port == 0 {
        validator.add_error("monitoring.metrics_port", "Metrics port cannot be 0");
    } else if config.metrics_port < 1024 {
        validator.add_warning(
            "monitoring.metrics_port",
            "Using privileged port for metrics",
            Some("Consider using a port >= 1024")
        );
    }
}

/// Validate logging configuration
async fn validate_logging_config(validator: &mut ConfigValidator, config: &crate::LoggingConfig) {
    debug!("Validating logging configuration");
    
    // Validate log level
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    if !valid_levels.contains(&config.level.to_lowercase().as_str()) {
        validator.add_error(
            "logging.level",
            &format!("Invalid log level: {}", config.level)
        );
    }
    
    // Validate log format
    let valid_formats = ["json", "text", "compact"];
    if !valid_formats.contains(&config.format.to_lowercase().as_str()) {
        validator.add_warning(
            "logging.format",
            &format!("Unknown log format: {}", config.format),
            Some("Consider using: json, text, or compact")
        );
    }
}

/// Validate protocols configuration
async fn validate_protocols_config(validator: &mut ConfigValidator, _config: &crate::ProtocolsConfig) {
    debug!("Validating protocols configuration");
    
    // Protocol validation would be implemented here
    validator.add_info("protocols", "Protocol configuration validation not yet implemented");
}

/// Validate enterprise configuration
async fn validate_enterprise_config(validator: &mut ConfigValidator, config: &crate::EnterpriseConfig) {
    debug!("Validating enterprise configuration");
    
    // Validate license key format if present
    if let Some(license_key) = &config.license_key {
        if license_key.len() < 32 {
            validator.add_warning(
                "enterprise.license_key",
                "License key appears to be too short",
                Some("Ensure you have a valid enterprise license key")
            );
        }
    }
}

/// Validate cross-dependencies between configuration sections
async fn validate_cross_dependencies(validator: &mut ConfigValidator, config: &A3MailerConfig) {
    debug!("Validating cross-dependencies");
    
    // Check AI and Web3 dependencies
    if config.ai.enabled && config.web3.enabled {
        validator.add_info("cross_validation", "AI and Web3 features are both enabled - full A3Mailer functionality available");
    } else if !config.ai.enabled && !config.web3.enabled {
        validator.add_warning(
            "cross_validation",
            "Both AI and Web3 features are disabled",
            Some("Consider enabling at least one advanced feature for enhanced functionality")
        );
    }
    
    // Check TLS and security
    if !config.server.tls.enabled {
        validator.add_warning(
            "cross_validation",
            "TLS is disabled - connections will not be encrypted",
            Some("Enable TLS for production deployments")
        );
    }
    
    // Check monitoring and logging
    if !config.monitoring.enabled {
        validator.add_warning(
            "cross_validation",
            "Monitoring is disabled",
            Some("Enable monitoring for production observability")
        );
    }
}
