/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Security Audit Logging Module
//!
//! This module provides comprehensive audit logging capabilities for security
//! events, compliance requirements, and forensic analysis.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
    fmt,
};
use serde::{Serialize, Deserialize};
use tracing::{debug, info, warn, error};

/// Audit event types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuditEventType {
    /// Authentication events
    Authentication,
    /// Authorization events
    Authorization,
    /// Data access events
    DataAccess,
    /// Configuration changes
    ConfigurationChange,
    /// Security policy violations
    SecurityViolation,
    /// System events
    System,
    /// Network events
    Network,
    /// Email events
    Email,
    /// Administrative actions
    Administrative,
}

/// Audit event severity levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuditSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Audit event outcome
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuditOutcome {
    Success,
    Failure,
    Unknown,
}

/// Audit event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    pub id: String,
    /// Event timestamp (Unix timestamp)
    pub timestamp: u64,
    /// Event type
    pub event_type: AuditEventType,
    /// Event severity
    pub severity: AuditSeverity,
    /// Event outcome
    pub outcome: AuditOutcome,
    /// Source IP address
    pub source_ip: Option<String>,
    /// User ID or identifier
    pub user_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Resource accessed or affected
    pub resource: Option<String>,
    /// Action performed
    pub action: String,
    /// Event description
    pub description: String,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Request ID for correlation
    pub request_id: Option<String>,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(
        event_type: AuditEventType,
        severity: AuditSeverity,
        outcome: AuditOutcome,
        action: String,
        description: String,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp,
            event_type,
            severity,
            outcome,
            source_ip: None,
            user_id: None,
            session_id: None,
            resource: None,
            action,
            description,
            metadata: HashMap::new(),
            request_id: None,
        }
    }

    /// Set source IP address
    pub fn with_source_ip(mut self, ip: String) -> Self {
        self.source_ip = Some(ip);
        self
    }

    /// Set user ID
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Set session ID
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Set resource
    pub fn with_resource(mut self, resource: String) -> Self {
        self.resource = Some(resource);
        self
    }

    /// Set request ID
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Convert to structured log format
    pub fn to_structured_log(&self) -> String {
        format!(
            "[{}] {} {} {} {} - {} (ID: {}, IP: {}, User: {})",
            self.timestamp,
            self.event_type,
            self.severity,
            self.outcome,
            self.action,
            self.description,
            self.id,
            self.source_ip.as_deref().unwrap_or("unknown"),
            self.user_id.as_deref().unwrap_or("unknown")
        )
    }
}

impl fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditEventType::Authentication => write!(f, "AUTH"),
            AuditEventType::Authorization => write!(f, "AUTHZ"),
            AuditEventType::DataAccess => write!(f, "DATA"),
            AuditEventType::ConfigurationChange => write!(f, "CONFIG"),
            AuditEventType::SecurityViolation => write!(f, "SECURITY"),
            AuditEventType::System => write!(f, "SYSTEM"),
            AuditEventType::Network => write!(f, "NETWORK"),
            AuditEventType::Email => write!(f, "EMAIL"),
            AuditEventType::Administrative => write!(f, "ADMIN"),
        }
    }
}

impl fmt::Display for AuditSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditSeverity::Info => write!(f, "INFO"),
            AuditSeverity::Warning => write!(f, "WARN"),
            AuditSeverity::Error => write!(f, "ERROR"),
            AuditSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

impl fmt::Display for AuditOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditOutcome::Success => write!(f, "SUCCESS"),
            AuditOutcome::Failure => write!(f, "FAILURE"),
            AuditOutcome::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Audit logger configuration
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// Enable audit logging
    pub enabled: bool,
    /// Maximum number of events to keep in memory
    pub max_events_in_memory: usize,
    /// Log to file
    pub log_to_file: bool,
    /// Log file path
    pub log_file_path: Option<String>,
    /// Log to syslog
    pub log_to_syslog: bool,
    /// Syslog facility
    pub syslog_facility: Option<String>,
    /// Minimum severity level to log
    pub min_severity: AuditSeverity,
    /// Event types to log
    pub event_types: Vec<AuditEventType>,
    /// Retention period for events
    pub retention_period: Duration,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_events_in_memory: 10000,
            log_to_file: true,
            log_file_path: Some("/var/log/stalwart/audit.log".to_string()),
            log_to_syslog: false,
            syslog_facility: Some("local0".to_string()),
            min_severity: AuditSeverity::Info,
            event_types: vec![
                AuditEventType::Authentication,
                AuditEventType::Authorization,
                AuditEventType::SecurityViolation,
                AuditEventType::ConfigurationChange,
                AuditEventType::Administrative,
            ],
            retention_period: Duration::from_secs(30 * 24 * 3600), // 30 days
        }
    }
}

/// Audit statistics
#[derive(Debug, Clone, Default)]
pub struct AuditStats {
    pub total_events: u64,
    pub events_by_type: HashMap<String, u64>,
    pub events_by_severity: HashMap<String, u64>,
    pub events_by_outcome: HashMap<String, u64>,
    pub failed_authentications: u64,
    pub security_violations: u64,
}

/// Audit logger
pub struct AuditLogger {
    config: AuditConfig,
    events: Arc<RwLock<Vec<AuditEvent>>>,
    stats: Arc<RwLock<AuditStats>>,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(config: AuditConfig) -> Self {
        info!("Creating audit logger with config: {:?}", config);
        Self {
            config,
            events: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(AuditStats::default())),
        }
    }

    /// Log an audit event
    pub fn log_event(&self, event: AuditEvent) {
        if !self.config.enabled {
            return;
        }

        // Check if event type is enabled
        if !self.config.event_types.contains(&event.event_type) {
            return;
        }

        // Check minimum severity
        if !self.meets_min_severity(&event.severity) {
            return;
        }

        debug!("Logging audit event: {}", event.id);

        // Log to tracing
        match event.severity {
            AuditSeverity::Info => info!("{}", event.to_structured_log()),
            AuditSeverity::Warning => warn!("{}", event.to_structured_log()),
            AuditSeverity::Error => error!("{}", event.to_structured_log()),
            AuditSeverity::Critical => error!("CRITICAL: {}", event.to_structured_log()),
        }

        // Store in memory
        let mut events = self.events.write().unwrap();
        events.push(event.clone());

        // Limit memory usage
        if events.len() > self.config.max_events_in_memory {
            events.drain(0..1000); // Remove oldest 1000 events
        }

        // Update statistics
        self.update_stats(&event);

        // TODO: Implement file and syslog logging
        if self.config.log_to_file {
            self.log_to_file(&event);
        }

        if self.config.log_to_syslog {
            self.log_to_syslog(&event);
        }
    }

    /// Check if severity meets minimum requirement
    fn meets_min_severity(&self, severity: &AuditSeverity) -> bool {
        let severity_level = match severity {
            AuditSeverity::Info => 0,
            AuditSeverity::Warning => 1,
            AuditSeverity::Error => 2,
            AuditSeverity::Critical => 3,
        };

        let min_level = match self.config.min_severity {
            AuditSeverity::Info => 0,
            AuditSeverity::Warning => 1,
            AuditSeverity::Error => 2,
            AuditSeverity::Critical => 3,
        };

        severity_level >= min_level
    }

    /// Update audit statistics
    fn update_stats(&self, event: &AuditEvent) {
        if let Ok(mut stats) = self.stats.write() {
            stats.total_events += 1;

            // Update by type
            let type_key = event.event_type.to_string();
            *stats.events_by_type.entry(type_key).or_insert(0) += 1;

            // Update by severity
            let severity_key = event.severity.to_string();
            *stats.events_by_severity.entry(severity_key).or_insert(0) += 1;

            // Update by outcome
            let outcome_key = event.outcome.to_string();
            *stats.events_by_outcome.entry(outcome_key).or_insert(0) += 1;

            // Update specific counters
            if event.event_type == AuditEventType::Authentication && event.outcome == AuditOutcome::Failure {
                stats.failed_authentications += 1;
            }

            if event.event_type == AuditEventType::SecurityViolation {
                stats.security_violations += 1;
            }
        }
    }

    /// Log to file (placeholder implementation)
    fn log_to_file(&self, event: &AuditEvent) {
        // TODO: Implement actual file logging
        debug!("Would log to file: {}", event.to_json().unwrap_or_default());
    }

    /// Log to syslog (placeholder implementation)
    fn log_to_syslog(&self, event: &AuditEvent) {
        // TODO: Implement actual syslog logging
        debug!("Would log to syslog: {}", event.to_structured_log());
    }

    /// Get recent events
    pub fn get_recent_events(&self, limit: usize) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        events.iter().rev().take(limit).cloned().collect()
    }

    /// Get events by type
    pub fn get_events_by_type(&self, event_type: AuditEventType, limit: usize) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        events.iter()
            .rev()
            .filter(|e| e.event_type == event_type)
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get audit statistics
    pub fn get_stats(&self) -> AuditStats {
        self.stats.read().unwrap().clone()
    }

    /// Clean up old events
    pub fn cleanup_old_events(&self) {
        debug!("Cleaning up old audit events");

        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() - self.config.retention_period.as_secs();

        let mut events = self.events.write().unwrap();
        let initial_count = events.len();
        events.retain(|event| event.timestamp > cutoff_time);

        let removed_count = initial_count - events.len();
        if removed_count > 0 {
            info!("Cleaned up {} old audit events", removed_count);
        }
    }

    /// Get configuration
    pub fn get_config(&self) -> &AuditConfig {
        &self.config
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(AuditConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(
            AuditEventType::Authentication,
            AuditSeverity::Info,
            AuditOutcome::Success,
            "login".to_string(),
            "User logged in successfully".to_string(),
        )
        .with_source_ip("192.168.1.1".to_string())
        .with_user_id("user123".to_string())
        .with_metadata("method".to_string(), "password".to_string());

        assert_eq!(event.event_type, AuditEventType::Authentication);
        assert_eq!(event.severity, AuditSeverity::Info);
        assert_eq!(event.outcome, AuditOutcome::Success);
        assert_eq!(event.source_ip, Some("192.168.1.1".to_string()));
        assert_eq!(event.user_id, Some("user123".to_string()));
        assert!(event.metadata.contains_key("method"));
    }

    #[test]
    fn test_audit_logger() {
        let logger = AuditLogger::default();

        let event = AuditEvent::new(
            AuditEventType::Authentication,
            AuditSeverity::Warning,
            AuditOutcome::Failure,
            "login_failed".to_string(),
            "Failed login attempt".to_string(),
        );

        logger.log_event(event);

        let stats = logger.get_stats();
        assert_eq!(stats.total_events, 1);
        assert_eq!(stats.failed_authentications, 1);

        let recent_events = logger.get_recent_events(10);
        assert_eq!(recent_events.len(), 1);
    }

    #[test]
    fn test_severity_filtering() {
        let mut config = AuditConfig::default();
        config.min_severity = AuditSeverity::Warning;

        let logger = AuditLogger::new(config);

        // This should be filtered out
        let info_event = AuditEvent::new(
            AuditEventType::Authentication,
            AuditSeverity::Info,
            AuditOutcome::Success,
            "test".to_string(),
            "Info event".to_string(),
        );

        // This should be logged
        let warning_event = AuditEvent::new(
            AuditEventType::Authentication,
            AuditSeverity::Warning,
            AuditOutcome::Success,
            "test".to_string(),
            "Warning event".to_string(),
        );

        logger.log_event(info_event);
        logger.log_event(warning_event);

        let stats = logger.get_stats();
        assert_eq!(stats.total_events, 1); // Only warning event should be logged
    }
}
