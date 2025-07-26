//! # A3Mailer Security Module
//!
//! Comprehensive security framework for A3Mailer including encryption,
//! key management, authentication, and authorization.
//!
//! ## Features
//!
//! - **Key Management**: Secure key generation, storage, and rotation
//! - **Encryption**: AES-256-GCM and ChaCha20-Poly1305 encryption
//! - **Authentication**: Multi-factor authentication and JWT tokens
//! - **Authorization**: Role-based access control (RBAC)
//! - **Audit Logging**: Comprehensive security event logging
//! - **Compliance**: GDPR, HIPAA, and SOC2 compliance features
//!
//! ## Architecture
//!
//! The security system consists of:
//! - Key Manager: Secure key lifecycle management
//! - Crypto Engine: High-performance encryption/decryption
//! - Auth Manager: Authentication and session management
//! - Access Control: Authorization and permission management
//!
//! ## Example
//!
//! ```rust,no_run
//! use a3mailer_security::{SecurityManager, SecurityConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = SecurityConfig::default();
//!     let security = SecurityManager::new(config).await?;
//!
//!     // Encrypt data
//!     let encrypted = security.encrypt("sensitive data").await?;
//!
//!     // Decrypt data
//!     let decrypted = security.decrypt(&encrypted).await?;
//!
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

pub mod keys;
pub mod crypto;
pub mod auth;
pub mod access;
pub mod audit;
pub mod error;

pub use error::{SecurityError, Result};

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub encryption: EncryptionConfig,
    pub authentication: AuthenticationConfig,
    pub authorization: AuthorizationConfig,
    pub key_management: KeyManagementConfig,
    pub audit: AuditConfig,
    pub compliance: ComplianceConfig,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub default_algorithm: String,
    pub key_size_bits: u32,
    pub enable_hardware_acceleration: bool,
    pub rotation_interval_days: u32,
    pub backup_encryption_enabled: bool,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    pub jwt_secret: String,
    pub jwt_expiry_hours: u64,
    pub mfa_enabled: bool,
    pub password_policy: PasswordPolicy,
    pub session_timeout_minutes: u64,
    pub max_login_attempts: u32,
    pub lockout_duration_minutes: u64,
}

/// Password policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordPolicy {
    pub min_length: u32,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_numbers: bool,
    pub require_special_chars: bool,
    pub max_age_days: u32,
    pub history_count: u32,
}

/// Authorization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationConfig {
    pub rbac_enabled: bool,
    pub default_role: String,
    pub admin_roles: Vec<String>,
    pub permission_cache_ttl_minutes: u64,
    pub audit_all_access: bool,
}

/// Key management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyManagementConfig {
    pub key_store_type: String, // "file", "hsm", "vault"
    pub key_store_path: String,
    pub auto_rotation_enabled: bool,
    pub rotation_schedule: String, // cron expression
    pub backup_keys_count: u32,
    pub key_derivation_iterations: u32,
}

/// Audit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub enabled: bool,
    pub log_level: String,
    pub retention_days: u32,
    pub encryption_enabled: bool,
    pub remote_logging_enabled: bool,
    pub real_time_alerts: bool,
}

/// Compliance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceConfig {
    pub gdpr_enabled: bool,
    pub hipaa_enabled: bool,
    pub soc2_enabled: bool,
    pub data_classification_enabled: bool,
    pub encryption_at_rest_required: bool,
    pub encryption_in_transit_required: bool,
}

/// Security event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEvent {
    Authentication {
        user_id: String,
        success: bool,
        method: String,
        ip_address: String,
        user_agent: String,
    },
    Authorization {
        user_id: String,
        resource: String,
        action: String,
        granted: bool,
        reason: String,
    },
    Encryption {
        operation: String, // "encrypt", "decrypt", "key_rotation"
        key_id: String,
        data_size: u64,
        algorithm: String,
    },
    KeyManagement {
        operation: String, // "generate", "rotate", "delete", "backup"
        key_id: String,
        key_type: String,
    },
    DataAccess {
        user_id: String,
        resource: String,
        operation: String,
        data_classification: String,
    },
    SecurityViolation {
        violation_type: String,
        severity: String,
        description: String,
        source_ip: String,
    },
}

/// Security metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetrics {
    pub authentication_attempts: u64,
    pub authentication_failures: u64,
    pub authorization_denials: u64,
    pub encryption_operations: u64,
    pub key_rotations: u64,
    pub security_violations: u64,
    pub active_sessions: u64,
    pub failed_login_rate: f64,
    pub average_session_duration_minutes: f64,
}

/// Main security manager
pub struct SecurityManager {
    config: SecurityConfig,
    key_manager: Arc<keys::KeyManager>,
    crypto_engine: Arc<crypto::CryptoEngine>,
    auth_manager: Arc<auth::AuthManager>,
    access_control: Arc<access::AccessControl>,
    audit_logger: Arc<audit::AuditLogger>,
    metrics: Arc<RwLock<SecurityMetrics>>,
}

impl SecurityManager {
    /// Create a new security manager
    pub async fn new(config: SecurityConfig) -> Result<Self> {
        info!("Initializing security management system");

        // Initialize components
        let key_manager = Arc::new(keys::KeyManager::new(&config.key_management).await?);
        let crypto_engine = Arc::new(crypto::CryptoEngine::new(&config.encryption, key_manager.clone()).await?);
        let auth_manager = Arc::new(auth::AuthManager::new(&config.authentication).await?);
        let access_control = Arc::new(access::AccessControl::new(&config.authorization).await?);
        let audit_logger = Arc::new(audit::AuditLogger::new(&config.audit).await?);

        let metrics = Arc::new(RwLock::new(SecurityMetrics {
            authentication_attempts: 0,
            authentication_failures: 0,
            authorization_denials: 0,
            encryption_operations: 0,
            key_rotations: 0,
            security_violations: 0,
            active_sessions: 0,
            failed_login_rate: 0.0,
            average_session_duration_minutes: 0.0,
        }));

        let manager = Self {
            config,
            key_manager,
            crypto_engine,
            auth_manager,
            access_control,
            audit_logger,
            metrics,
        };

        // Start background security tasks
        manager.start_background_tasks().await?;

        info!("Security management system initialized successfully");
        Ok(manager)
    }

    /// Encrypt data
    pub async fn encrypt(&self, data: &str) -> Result<String> {
        let result = self.crypto_engine.encrypt(data.as_bytes()).await?;
        
        // Log encryption event
        self.audit_logger.log_event(SecurityEvent::Encryption {
            operation: "encrypt".to_string(),
            key_id: "default".to_string(),
            data_size: data.len() as u64,
            algorithm: self.config.encryption.default_algorithm.clone(),
        }).await?;

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.encryption_operations += 1;
        }

        Ok(result)
    }

    /// Decrypt data
    pub async fn decrypt(&self, encrypted_data: &str) -> Result<String> {
        let result = self.crypto_engine.decrypt(encrypted_data).await?;
        
        // Log decryption event
        self.audit_logger.log_event(SecurityEvent::Encryption {
            operation: "decrypt".to_string(),
            key_id: "default".to_string(),
            data_size: encrypted_data.len() as u64,
            algorithm: self.config.encryption.default_algorithm.clone(),
        }).await?;

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.encryption_operations += 1;
        }

        Ok(result)
    }

    /// Authenticate user
    pub async fn authenticate(&self, username: &str, password: &str, ip_address: &str, user_agent: &str) -> Result<auth::AuthToken> {
        let start_time = std::time::Instant::now();
        
        let result = self.auth_manager.authenticate(username, password).await;
        
        let success = result.is_ok();
        
        // Log authentication event
        self.audit_logger.log_event(SecurityEvent::Authentication {
            user_id: username.to_string(),
            success,
            method: "password".to_string(),
            ip_address: ip_address.to_string(),
            user_agent: user_agent.to_string(),
        }).await?;

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.authentication_attempts += 1;
            if !success {
                metrics.authentication_failures += 1;
            }
            metrics.failed_login_rate = (metrics.authentication_failures as f64 / metrics.authentication_attempts as f64) * 100.0;
        }

        result
    }

    /// Check authorization
    pub async fn authorize(&self, user_id: &str, resource: &str, action: &str) -> Result<bool> {
        let result = self.access_control.check_permission(user_id, resource, action).await?;
        
        // Log authorization event
        self.audit_logger.log_event(SecurityEvent::Authorization {
            user_id: user_id.to_string(),
            resource: resource.to_string(),
            action: action.to_string(),
            granted: result,
            reason: if result { "permission_granted".to_string() } else { "permission_denied".to_string() },
        }).await?;

        // Update metrics
        if !result {
            let mut metrics = self.metrics.write().await;
            metrics.authorization_denials += 1;
        }

        Ok(result)
    }

    /// Generate new encryption key
    pub async fn generate_key(&self, key_type: &str) -> Result<String> {
        let key_id = self.key_manager.generate_key(key_type).await?;
        
        // Log key generation event
        self.audit_logger.log_event(SecurityEvent::KeyManagement {
            operation: "generate".to_string(),
            key_id: key_id.clone(),
            key_type: key_type.to_string(),
        }).await?;

        Ok(key_id)
    }

    /// Rotate encryption keys
    pub async fn rotate_keys(&self) -> Result<()> {
        info!("Starting key rotation");
        
        let rotated_keys = self.key_manager.rotate_keys().await?;
        
        for key_id in rotated_keys {
            // Log key rotation event
            self.audit_logger.log_event(SecurityEvent::KeyManagement {
                operation: "rotate".to_string(),
                key_id: key_id.clone(),
                key_type: "encryption".to_string(),
            }).await?;
        }

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.key_rotations += 1;
        }

        info!("Key rotation completed");
        Ok(())
    }

    /// Log security violation
    pub async fn log_security_violation(&self, violation_type: &str, severity: &str, description: &str, source_ip: &str) -> Result<()> {
        warn!("Security violation detected: {} - {}", violation_type, description);
        
        // Log security violation event
        self.audit_logger.log_event(SecurityEvent::SecurityViolation {
            violation_type: violation_type.to_string(),
            severity: severity.to_string(),
            description: description.to_string(),
            source_ip: source_ip.to_string(),
        }).await?;

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.security_violations += 1;
        }

        Ok(())
    }

    /// Get security metrics
    pub async fn get_security_metrics(&self) -> SecurityMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Start background security tasks
    async fn start_background_tasks(&self) -> Result<()> {
        info!("Starting background security tasks");

        // Key rotation task
        if self.config.key_management.auto_rotation_enabled {
            let key_manager = Arc::clone(&self.key_manager);
            let audit_logger = Arc::clone(&self.audit_logger);
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(24 * 60 * 60)); // Daily check
                loop {
                    interval.tick().await;
                    
                    if let Err(e) = key_manager.check_and_rotate_keys().await {
                        error!("Failed to check/rotate keys: {}", e);
                    }
                }
            });
        }

        // Session cleanup task
        let auth_manager = Arc::clone(&self.auth_manager);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5 * 60)); // Every 5 minutes
            loop {
                interval.tick().await;
                
                if let Err(e) = auth_manager.cleanup_expired_sessions().await {
                    error!("Failed to cleanup expired sessions: {}", e);
                }
            }
        });

        // Security monitoring task
        let audit_logger = Arc::clone(&self.audit_logger);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Every minute
            loop {
                interval.tick().await;
                
                if let Err(e) = audit_logger.process_security_events().await {
                    error!("Failed to process security events: {}", e);
                }
            }
        });

        info!("Background security tasks started");
        Ok(())
    }

    /// Validate security configuration
    pub fn validate_config(&self) -> Result<()> {
        // Validate encryption configuration
        if self.config.encryption.key_size_bits < 256 {
            return Err(SecurityError::ConfigError("Key size must be at least 256 bits".to_string()));
        }

        // Validate password policy
        if self.config.authentication.password_policy.min_length < 8 {
            return Err(SecurityError::ConfigError("Minimum password length must be at least 8".to_string()));
        }

        // Validate JWT configuration
        if self.config.authentication.jwt_secret.len() < 32 {
            return Err(SecurityError::ConfigError("JWT secret must be at least 32 characters".to_string()));
        }

        info!("Security configuration validation passed");
        Ok(())
    }

    /// Get security status
    pub async fn get_security_status(&self) -> Result<HashMap<String, String>> {
        let mut status = HashMap::new();
        
        let metrics = self.get_security_metrics().await;
        
        status.insert("encryption_operations".to_string(), metrics.encryption_operations.to_string());
        status.insert("active_sessions".to_string(), metrics.active_sessions.to_string());
        status.insert("failed_login_rate".to_string(), format!("{:.2}%", metrics.failed_login_rate));
        status.insert("security_violations".to_string(), metrics.security_violations.to_string());
        status.insert("key_rotations".to_string(), metrics.key_rotations.to_string());
        
        // Add component status
        status.insert("key_manager_status".to_string(), self.key_manager.get_status().await?);
        status.insert("crypto_engine_status".to_string(), self.crypto_engine.get_status().await?);
        status.insert("auth_manager_status".to_string(), self.auth_manager.get_status().await?);
        status.insert("access_control_status".to_string(), self.access_control.get_status().await?);
        
        Ok(status)
    }

    /// Shutdown security manager
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down security management system");
        
        // Shutdown components
        self.key_manager.shutdown().await?;
        self.crypto_engine.shutdown().await?;
        self.auth_manager.shutdown().await?;
        self.access_control.shutdown().await?;
        self.audit_logger.shutdown().await?;
        
        info!("Security management system shutdown complete");
        Ok(())
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            encryption: EncryptionConfig {
                default_algorithm: "AES-256-GCM".to_string(),
                key_size_bits: 256,
                enable_hardware_acceleration: true,
                rotation_interval_days: 90,
                backup_encryption_enabled: true,
            },
            authentication: AuthenticationConfig {
                jwt_secret: "your-super-secret-jwt-key-change-this-in-production".to_string(),
                jwt_expiry_hours: 24,
                mfa_enabled: false,
                password_policy: PasswordPolicy {
                    min_length: 12,
                    require_uppercase: true,
                    require_lowercase: true,
                    require_numbers: true,
                    require_special_chars: true,
                    max_age_days: 90,
                    history_count: 5,
                },
                session_timeout_minutes: 60,
                max_login_attempts: 5,
                lockout_duration_minutes: 15,
            },
            authorization: AuthorizationConfig {
                rbac_enabled: true,
                default_role: "user".to_string(),
                admin_roles: vec!["admin".to_string(), "superuser".to_string()],
                permission_cache_ttl_minutes: 30,
                audit_all_access: true,
            },
            key_management: KeyManagementConfig {
                key_store_type: "file".to_string(),
                key_store_path: "keys/".to_string(),
                auto_rotation_enabled: true,
                rotation_schedule: "0 2 * * 0".to_string(), // Weekly on Sunday at 2 AM
                backup_keys_count: 3,
                key_derivation_iterations: 100000,
            },
            audit: AuditConfig {
                enabled: true,
                log_level: "info".to_string(),
                retention_days: 365,
                encryption_enabled: true,
                remote_logging_enabled: false,
                real_time_alerts: true,
            },
            compliance: ComplianceConfig {
                gdpr_enabled: true,
                hipaa_enabled: false,
                soc2_enabled: true,
                data_classification_enabled: true,
                encryption_at_rest_required: true,
                encryption_in_transit_required: true,
            },
        }
    }
}

/// Initialize security management system
pub async fn init_security(config: SecurityConfig) -> Result<SecurityManager> {
    info!("Initializing A3Mailer security management system");
    
    let manager = SecurityManager::new(config).await?;
    
    // Validate configuration
    manager.validate_config()?;
    
    info!("A3Mailer security management system initialized successfully");
    Ok(manager)
}
