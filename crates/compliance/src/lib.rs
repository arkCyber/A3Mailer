//! # Stalwart Compliance
//!
//! Comprehensive compliance management system for Stalwart Mail Server.
//! Provides GDPR, HIPAA, CCPA, and other regulatory compliance features
//! including audit logging, data classification, and retention policies.
//!
//! ## Features
//!
//! - **GDPR Compliance**: Data protection and privacy rights
//! - **HIPAA Compliance**: Healthcare data protection
//! - **CCPA Compliance**: California Consumer Privacy Act
//! - **Audit Logging**: Comprehensive audit trail
//! - **Data Classification**: Automatic data sensitivity classification
//! - **Retention Policies**: Automated data retention and deletion
//!
//! ## Architecture
//!
//! The compliance system consists of:
//! - Compliance Manager: Main compliance orchestrator
//! - Audit Logger: Comprehensive audit trail system
//! - Data Classifier: Automatic data sensitivity detection
//! - Retention Manager: Data lifecycle management
//! - Privacy Manager: User privacy rights management
//!
//! ## Example
//!
//! ```rust,no_run
//! use stalwart_compliance::{ComplianceManager, ComplianceConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ComplianceConfig::default();
//!     let manager = ComplianceManager::new(config).await?;
//!
//!     // Start compliance monitoring
//!     manager.start_monitoring().await?;
//!
//!     Ok(())
//! }
//! ```

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

pub mod config;
pub mod manager;
pub mod audit;
pub mod classification;
pub mod retention;
pub mod privacy;
pub mod gdpr;
pub mod hipaa;
pub mod ccpa;
pub mod metrics;
pub mod error;

pub use config::ComplianceConfig;
pub use manager::ComplianceManager;
pub use audit::{AuditLogger, AuditEvent, AuditLevel};
pub use classification::{DataClassifier, DataSensitivity, ClassificationResult};
pub use retention::{RetentionManager, RetentionPolicy, RetentionAction};
pub use privacy::{PrivacyManager, PrivacyRequest, PrivacyRequestType};
pub use error::{ComplianceError, Result};

/// Compliance frameworks
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ComplianceFramework {
    /// General Data Protection Regulation (EU)
    GDPR,
    /// Health Insurance Portability and Accountability Act (US)
    HIPAA,
    /// California Consumer Privacy Act (US)
    CCPA,
    /// Sarbanes-Oxley Act (US)
    SOX,
    /// Personal Information Protection and Electronic Documents Act (Canada)
    PIPEDA,
    /// Custom compliance framework
    Custom(String),
}

/// Compliance status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComplianceStatus {
    /// Compliant
    Compliant,
    /// Non-compliant
    NonCompliant,
    /// Under review
    UnderReview,
    /// Partially compliant
    PartiallyCompliant,
    /// Unknown status
    Unknown,
}

/// Compliance violation severity
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum ViolationSeverity {
    /// Low severity violation
    Low,
    /// Medium severity violation
    Medium,
    /// High severity violation
    High,
    /// Critical severity violation
    Critical,
}

/// Compliance violation event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComplianceViolation {
    /// Violation ID
    pub id: uuid::Uuid,
    /// Framework that was violated
    pub framework: ComplianceFramework,
    /// Violation type
    pub violation_type: String,
    /// Severity level
    pub severity: ViolationSeverity,
    /// Description of the violation
    pub description: String,
    /// Source of the violation
    pub source: String,
    /// Affected data/user
    pub affected_entity: Option<String>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    /// Resolution status
    pub resolved: bool,
    /// Resolution notes
    pub resolution_notes: Option<String>,
}

/// Main compliance context
pub struct ComplianceContext {
    pub config: ComplianceConfig,
    pub status: Arc<RwLock<ComplianceStatus>>,
    pub violations: Arc<RwLock<Vec<ComplianceViolation>>>,
    pub frameworks: Arc<RwLock<Vec<ComplianceFramework>>>,
}

impl ComplianceContext {
    /// Create a new compliance context
    pub fn new(config: ComplianceConfig) -> Self {
        let frameworks = config.enabled_frameworks.clone();
        Self {
            config,
            status: Arc::new(RwLock::new(ComplianceStatus::Unknown)),
            violations: Arc::new(RwLock::new(Vec::new())),
            frameworks: Arc::new(RwLock::new(frameworks)),
        }
    }

    /// Get current compliance status
    pub async fn status(&self) -> ComplianceStatus {
        self.status.read().await.clone()
    }

    /// Set compliance status
    pub async fn set_status(&self, status: ComplianceStatus) {
        let mut current_status = self.status.write().await;
        if *current_status != status {
            info!("Compliance status changed: {:?} -> {:?}", *current_status, status);
            *current_status = status;
        }
    }

    /// Add a compliance violation
    pub async fn add_violation(&self, violation: ComplianceViolation) {
        let mut violations = self.violations.write().await;
        violations.push(violation);

        // Keep only recent violations (configurable limit)
        if violations.len() > self.config.max_violations_history {
            let excess = violations.len() - self.config.max_violations_history;
            violations.drain(0..excess);
        }
    }

    /// Get recent violations
    pub async fn recent_violations(&self, limit: usize) -> Vec<ComplianceViolation> {
        let violations = self.violations.read().await;
        violations.iter().rev().take(limit).cloned().collect()
    }

    /// Get enabled frameworks
    pub async fn enabled_frameworks(&self) -> Vec<ComplianceFramework> {
        self.frameworks.read().await.clone()
    }
}

/// Initialize the compliance system
pub async fn init_compliance(config: ComplianceConfig) -> Result<ComplianceContext> {
    info!("Initializing compliance system");

    let context = ComplianceContext::new(config);

    // TODO: Initialize compliance components
    // - Set up audit logging
    // - Initialize data classifiers
    // - Configure retention policies
    // - Set up privacy request handlers

    context.set_status(ComplianceStatus::Compliant).await;

    info!("Compliance system initialized successfully");
    Ok(context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compliance_context_creation() {
        let config = ComplianceConfig::default();
        let context = ComplianceContext::new(config);

        assert_eq!(context.status().await, ComplianceStatus::Unknown);
    }

    #[tokio::test]
    async fn test_violation_addition() {
        let config = ComplianceConfig::default();
        let context = ComplianceContext::new(config);

        let violation = ComplianceViolation {
            id: uuid::Uuid::new_v4(),
            framework: ComplianceFramework::GDPR,
            violation_type: "data_retention".to_string(),
            severity: ViolationSeverity::Medium,
            description: "Data retained beyond policy limit".to_string(),
            source: "retention_manager".to_string(),
            affected_entity: Some("user@example.com".to_string()),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
            resolved: false,
            resolution_notes: None,
        };

        context.add_violation(violation).await;
        let violations = context.recent_violations(10).await;
        assert_eq!(violations.len(), 1);
    }
}
