/*!
 * Configuration Management Module
 * 
 * This module provides configuration management for integration tests,
 * including test environment setup, server configuration, and test parameters.
 * 
 * Features:
 * - Test configuration loading and validation
 * - Environment-specific configurations
 * - Configuration templates
 * - Dynamic configuration generation
 * - Configuration validation
 * 
 * Author: Stalwart Labs Ltd.
 * Created: 2024-07-26
 */

use std::time::Duration;
use std::collections::HashMap;
use std::path::Path;
use tracing::{info, warn, error, debug};
use serde::{Serialize, Deserialize};

use crate::{TestConfig, ServerConfig, ExecutionConfig, UserConfig, EmailConfig, PerformanceConfig, Result};

/// Configuration manager for integration tests
pub struct ConfigManager {
    base_config: TestConfig,
    environment_configs: HashMap<String, TestConfig>,
}

/// Environment types for testing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Environment {
    Development,
    Testing,
    Staging,
    Production,
    Local,
}

/// Configuration template for different scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigTemplate {
    pub name: String,
    pub description: String,
    pub config: TestConfig,
}

/// Configuration validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Self {
        info!("Initializing configuration manager");
        
        Self {
            base_config: TestConfig::default(),
            environment_configs: HashMap::new(),
        }
    }

    /// Load configuration from file
    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        info!("Loading configuration from: {:?}", path);
        
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        
        let config: TestConfig = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;
        
        self.base_config = config;
        Ok(())
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        info!("Saving configuration to: {:?}", path);
        
        let content = toml::to_string_pretty(&self.base_config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        
        std::fs::write(path, content)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
        
        Ok(())
    }

    /// Get configuration for specific environment
    pub fn get_config(&self, environment: &Environment) -> TestConfig {
        if let Some(env_config) = self.environment_configs.get(&format!("{:?}", environment)) {
            env_config.clone()
        } else {
            self.base_config.clone()
        }
    }

    /// Set configuration for specific environment
    pub fn set_environment_config(&mut self, environment: Environment, config: TestConfig) {
        info!("Setting configuration for environment: {:?}", environment);
        self.environment_configs.insert(format!("{:?}", environment), config);
    }

    /// Validate configuration
    pub fn validate_config(&self, config: &TestConfig) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate server configuration
        if config.server.host.is_empty() {
            errors.push("Server host is empty".to_string());
        }

        if config.server.smtp_port == 0 {
            errors.push("SMTP port is invalid".to_string());
        }

        if config.server.imap_port == 0 {
            errors.push("IMAP port is invalid".to_string());
        }

        if config.server.pop3_port == 0 {
            errors.push("POP3 port is invalid".to_string());
        }

        if config.server.jmap_port == 0 {
            errors.push("JMAP port is invalid".to_string());
        }

        // Validate execution configuration
        if config.execution.concurrency == 0 {
            errors.push("Concurrency must be greater than 0".to_string());
        }

        if config.execution.concurrency > 1000 {
            warnings.push("High concurrency may cause resource issues".to_string());
        }

        if config.execution.timeout.as_secs() == 0 {
            errors.push("Timeout must be greater than 0".to_string());
        }

        // Validate user configuration
        if config.users.count == 0 {
            errors.push("User count must be greater than 0".to_string());
        }

        if config.users.domain.is_empty() {
            errors.push("User domain is empty".to_string());
        }

        if config.users.default_password.len() < 8 {
            warnings.push("Default password is shorter than 8 characters".to_string());
        }

        // Validate email configuration
        if config.email.max_size == 0 {
            errors.push("Maximum email size must be greater than 0".to_string());
        }

        if config.email.max_attachment_size > config.email.max_size {
            errors.push("Maximum attachment size cannot exceed maximum email size".to_string());
        }

        // Validate performance configuration
        if config.performance.target_eps <= 0.0 {
            errors.push("Target emails per second must be greater than 0".to_string());
        }

        if config.performance.max_connections == 0 {
            errors.push("Maximum connections must be greater than 0".to_string());
        }

        ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Generate configuration template for specific scenario
    pub fn generate_template(&self, scenario: &str) -> ConfigTemplate {
        match scenario {
            "basic" => self.generate_basic_template(),
            "stress" => self.generate_stress_template(),
            "corporate" => self.generate_corporate_template(),
            "development" => self.generate_development_template(),
            _ => self.generate_default_template(),
        }
    }

    /// Generate basic testing template
    fn generate_basic_template(&self) -> ConfigTemplate {
        let config = TestConfig {
            server: ServerConfig {
                host: "localhost".to_string(),
                smtp_port: 587,
                imap_port: 143,
                pop3_port: 110,
                jmap_port: 8080,
                http_port: 8080,
                use_tls: false,
                admin_user: "admin".to_string(),
                admin_password: "password".to_string(),
            },
            execution: ExecutionConfig {
                max_duration: Duration::from_secs(1800), // 30 minutes
                concurrency: 5,
                timeout: Duration::from_secs(30),
                retry_attempts: 3,
                retry_delay: Duration::from_secs(1),
            },
            users: UserConfig {
                count: 10,
                domain: "test.local".to_string(),
                default_password: "testpass123".to_string(),
                quota: 100 * 1024 * 1024, // 100MB
            },
            email: EmailConfig {
                bulk_count: 50,
                max_size: 5 * 1024 * 1024, // 5MB
                include_attachments: false,
                max_attachment_size: 1 * 1024 * 1024, // 1MB
            },
            performance: PerformanceConfig {
                target_eps: 10.0,
                max_connections: 50,
                stress_duration: Duration::from_secs(300), // 5 minutes
                ramp_up_duration: Duration::from_secs(30),
            },
        };

        ConfigTemplate {
            name: "Basic Testing".to_string(),
            description: "Basic configuration for simple integration testing".to_string(),
            config,
        }
    }

    /// Generate stress testing template
    fn generate_stress_template(&self) -> ConfigTemplate {
        let config = TestConfig {
            server: ServerConfig {
                host: "localhost".to_string(),
                smtp_port: 587,
                imap_port: 143,
                pop3_port: 110,
                jmap_port: 8080,
                http_port: 8080,
                use_tls: false,
                admin_user: "admin".to_string(),
                admin_password: "password".to_string(),
            },
            execution: ExecutionConfig {
                max_duration: Duration::from_secs(7200), // 2 hours
                concurrency: 100,
                timeout: Duration::from_secs(60),
                retry_attempts: 5,
                retry_delay: Duration::from_secs(2),
            },
            users: UserConfig {
                count: 1000,
                domain: "stress.test".to_string(),
                default_password: "stresstest123".to_string(),
                quota: 1024 * 1024 * 1024, // 1GB
            },
            email: EmailConfig {
                bulk_count: 10000,
                max_size: 25 * 1024 * 1024, // 25MB
                include_attachments: true,
                max_attachment_size: 10 * 1024 * 1024, // 10MB
            },
            performance: PerformanceConfig {
                target_eps: 1000.0,
                max_connections: 2000,
                stress_duration: Duration::from_secs(3600), // 1 hour
                ramp_up_duration: Duration::from_secs(300), // 5 minutes
            },
        };

        ConfigTemplate {
            name: "Stress Testing".to_string(),
            description: "High-load configuration for stress and performance testing".to_string(),
            config,
        }
    }

    /// Generate corporate environment template
    fn generate_corporate_template(&self) -> ConfigTemplate {
        let config = TestConfig {
            server: ServerConfig {
                host: "mail.company.com".to_string(),
                smtp_port: 587,
                imap_port: 993,
                pop3_port: 995,
                jmap_port: 443,
                http_port: 443,
                use_tls: true,
                admin_user: "admin".to_string(),
                admin_password: "SecurePassword123!".to_string(),
            },
            execution: ExecutionConfig {
                max_duration: Duration::from_secs(3600), // 1 hour
                concurrency: 50,
                timeout: Duration::from_secs(45),
                retry_attempts: 3,
                retry_delay: Duration::from_secs(2),
            },
            users: UserConfig {
                count: 500,
                domain: "company.com".to_string(),
                default_password: "CorporatePass123!".to_string(),
                quota: 2 * 1024 * 1024 * 1024, // 2GB
            },
            email: EmailConfig {
                bulk_count: 1000,
                max_size: 50 * 1024 * 1024, // 50MB
                include_attachments: true,
                max_attachment_size: 25 * 1024 * 1024, // 25MB
            },
            performance: PerformanceConfig {
                target_eps: 100.0,
                max_connections: 500,
                stress_duration: Duration::from_secs(1800), // 30 minutes
                ramp_up_duration: Duration::from_secs(120), // 2 minutes
            },
        };

        ConfigTemplate {
            name: "Corporate Environment".to_string(),
            description: "Configuration for corporate email environment testing".to_string(),
            config,
        }
    }

    /// Generate development template
    fn generate_development_template(&self) -> ConfigTemplate {
        let config = TestConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                smtp_port: 2525,
                imap_port: 1143,
                pop3_port: 1110,
                jmap_port: 8080,
                http_port: 8080,
                use_tls: false,
                admin_user: "dev".to_string(),
                admin_password: "dev".to_string(),
            },
            execution: ExecutionConfig {
                max_duration: Duration::from_secs(600), // 10 minutes
                concurrency: 2,
                timeout: Duration::from_secs(15),
                retry_attempts: 1,
                retry_delay: Duration::from_secs(1),
            },
            users: UserConfig {
                count: 5,
                domain: "dev.local".to_string(),
                default_password: "dev123".to_string(),
                quota: 10 * 1024 * 1024, // 10MB
            },
            email: EmailConfig {
                bulk_count: 10,
                max_size: 1 * 1024 * 1024, // 1MB
                include_attachments: false,
                max_attachment_size: 512 * 1024, // 512KB
            },
            performance: PerformanceConfig {
                target_eps: 1.0,
                max_connections: 10,
                stress_duration: Duration::from_secs(60), // 1 minute
                ramp_up_duration: Duration::from_secs(10),
            },
        };

        ConfigTemplate {
            name: "Development".to_string(),
            description: "Lightweight configuration for development and debugging".to_string(),
            config,
        }
    }

    /// Generate default template
    fn generate_default_template(&self) -> ConfigTemplate {
        ConfigTemplate {
            name: "Default".to_string(),
            description: "Default configuration template".to_string(),
            config: TestConfig::default(),
        }
    }

    /// Merge configurations (overlay second config over first)
    pub fn merge_configs(&self, base: &TestConfig, overlay: &TestConfig) -> TestConfig {
        // For simplicity, we'll just return the overlay config
        // In a real implementation, you might want to merge specific fields
        overlay.clone()
    }

    /// Get all available templates
    pub fn get_available_templates(&self) -> Vec<String> {
        vec![
            "basic".to_string(),
            "stress".to_string(),
            "corporate".to_string(),
            "development".to_string(),
        ]
    }

    /// Create configuration from environment variables
    pub fn from_environment() -> Result<TestConfig> {
        let mut config = TestConfig::default();

        // Server configuration from environment
        if let Ok(host) = std::env::var("STALWART_TEST_HOST") {
            config.server.host = host;
        }

        if let Ok(smtp_port) = std::env::var("STALWART_TEST_SMTP_PORT") {
            config.server.smtp_port = smtp_port.parse()
                .map_err(|e| format!("Invalid SMTP port: {}", e))?;
        }

        if let Ok(imap_port) = std::env::var("STALWART_TEST_IMAP_PORT") {
            config.server.imap_port = imap_port.parse()
                .map_err(|e| format!("Invalid IMAP port: {}", e))?;
        }

        if let Ok(pop3_port) = std::env::var("STALWART_TEST_POP3_PORT") {
            config.server.pop3_port = pop3_port.parse()
                .map_err(|e| format!("Invalid POP3 port: {}", e))?;
        }

        if let Ok(jmap_port) = std::env::var("STALWART_TEST_JMAP_PORT") {
            config.server.jmap_port = jmap_port.parse()
                .map_err(|e| format!("Invalid JMAP port: {}", e))?;
        }

        if let Ok(use_tls) = std::env::var("STALWART_TEST_USE_TLS") {
            config.server.use_tls = use_tls.parse()
                .map_err(|e| format!("Invalid TLS setting: {}", e))?;
        }

        // User configuration from environment
        if let Ok(domain) = std::env::var("STALWART_TEST_DOMAIN") {
            config.users.domain = domain;
        }

        if let Ok(user_count) = std::env::var("STALWART_TEST_USER_COUNT") {
            config.users.count = user_count.parse()
                .map_err(|e| format!("Invalid user count: {}", e))?;
        }

        // Execution configuration from environment
        if let Ok(concurrency) = std::env::var("STALWART_TEST_CONCURRENCY") {
            config.execution.concurrency = concurrency.parse()
                .map_err(|e| format!("Invalid concurrency: {}", e))?;
        }

        if let Ok(timeout) = std::env::var("STALWART_TEST_TIMEOUT") {
            let timeout_secs: u64 = timeout.parse()
                .map_err(|e| format!("Invalid timeout: {}", e))?;
            config.execution.timeout = Duration::from_secs(timeout_secs);
        }

        Ok(config)
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_manager_creation() {
        let manager = ConfigManager::new();
        assert_eq!(manager.base_config.server.host, "localhost");
    }

    #[test]
    fn test_config_validation() {
        let manager = ConfigManager::new();
        let config = TestConfig::default();
        
        let result = manager.validate_config(&config);
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_invalid_config_validation() {
        let manager = ConfigManager::new();
        let mut config = TestConfig::default();
        config.server.host = "".to_string(); // Invalid empty host
        config.execution.concurrency = 0; // Invalid zero concurrency
        
        let result = manager.validate_config(&config);
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_template_generation() {
        let manager = ConfigManager::new();
        
        let basic_template = manager.generate_template("basic");
        assert_eq!(basic_template.name, "Basic Testing");
        assert_eq!(basic_template.config.users.count, 10);
        
        let stress_template = manager.generate_template("stress");
        assert_eq!(stress_template.name, "Stress Testing");
        assert_eq!(stress_template.config.users.count, 1000);
    }

    #[test]
    fn test_config_file_operations() {
        let mut manager = ConfigManager::new();
        let temp_file = NamedTempFile::new().unwrap();
        
        // Save config to file
        manager.save_to_file(temp_file.path()).unwrap();
        
        // Load config from file
        manager.load_from_file(temp_file.path()).unwrap();
        
        assert_eq!(manager.base_config.server.host, "localhost");
    }

    #[test]
    fn test_environment_config() {
        let mut manager = ConfigManager::new();
        let mut dev_config = TestConfig::default();
        dev_config.server.host = "dev.example.com".to_string();
        
        manager.set_environment_config(Environment::Development, dev_config);
        
        let retrieved_config = manager.get_config(&Environment::Development);
        assert_eq!(retrieved_config.server.host, "dev.example.com");
        
        let default_config = manager.get_config(&Environment::Production);
        assert_eq!(default_config.server.host, "localhost");
    }

    #[test]
    fn test_available_templates() {
        let manager = ConfigManager::new();
        let templates = manager.get_available_templates();
        
        assert!(templates.contains(&"basic".to_string()));
        assert!(templates.contains(&"stress".to_string()));
        assert!(templates.contains(&"corporate".to_string()));
        assert!(templates.contains(&"development".to_string()));
    }
}
