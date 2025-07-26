/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Alert data structures and types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Critical alerts requiring immediate attention
    Critical,
    /// High priority alerts
    High,
    /// Medium priority alerts
    Medium,
    /// Warning alerts
    Warning,
    /// Informational alerts
    Info,
}

/// Alert status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertStatus {
    /// Alert is active and firing
    Firing,
    /// Alert is acknowledged but not resolved
    Acknowledged,
    /// Alert is resolved
    Resolved,
    /// Alert is suppressed
    Suppressed,
}

/// Alert context containing additional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertContext {
    /// Alert labels for grouping and routing
    pub labels: HashMap<String, String>,
    /// Alert annotations for additional information
    pub annotations: HashMap<String, String>,
    /// Source system or component
    pub source: String,
    /// Alert fingerprint for deduplication
    pub fingerprint: String,
    /// Generator URL for the alert source
    pub generator_url: Option<String>,
    /// Runbook URL for handling this alert
    pub runbook_url: Option<String>,
    /// Dashboard URL for monitoring
    pub dashboard_url: Option<String>,
}

/// Main alert structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Unique alert identifier
    pub id: Uuid,
    /// Alert title/summary
    pub title: String,
    /// Detailed alert description
    pub description: String,
    /// Alert severity level
    pub severity: AlertSeverity,
    /// Current alert status
    pub status: AlertStatus,
    /// Alert context and metadata
    pub context: AlertContext,
    /// Source component or system
    pub source: String,
    /// When the alert was first created
    pub created_at: DateTime<Utc>,
    /// When the alert was last updated
    pub updated_at: DateTime<Utc>,
    /// When the alert was last seen/occurred
    pub last_occurrence: DateTime<Utc>,
    /// When the alert was resolved (if applicable)
    pub resolved_at: Option<DateTime<Utc>>,
    /// When the alert was acknowledged (if applicable)
    pub acknowledged_at: Option<DateTime<Utc>>,
    /// Who acknowledged the alert
    pub acknowledged_by: Option<String>,
    /// Resolution note
    pub resolution_note: Option<String>,
    /// Number of times this alert has occurred
    pub count: u64,
    /// Alert timeout duration in seconds
    pub timeout: Option<u64>,
    /// Alert escalation level
    pub escalation_level: u32,
    /// Next escalation time
    pub next_escalation: Option<DateTime<Utc>>,
    /// Alert tags for categorization
    pub tags: Vec<String>,
    /// Alert priority score (calculated)
    pub priority_score: f64,
    /// Whether this alert should be silenced
    pub silenced: bool,
    /// Silence expiration time
    pub silence_expires_at: Option<DateTime<Utc>>,
    /// Related alert IDs
    pub related_alerts: Vec<Uuid>,
}

impl Alert {
    /// Create a new alert
    pub fn new(
        title: String,
        description: String,
        severity: AlertSeverity,
        source: String,
    ) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4();
        
        // Create basic context
        let mut labels = HashMap::new();
        labels.insert("severity".to_string(), severity.to_string());
        labels.insert("source".to_string(), source.clone());
        
        let context = AlertContext {
            labels,
            annotations: HashMap::new(),
            source: source.clone(),
            fingerprint: Self::generate_fingerprint(&title, &source),
            generator_url: None,
            runbook_url: None,
            dashboard_url: None,
        };

        Self {
            id,
            title,
            description,
            severity,
            status: AlertStatus::Firing,
            context,
            source,
            created_at: now,
            updated_at: now,
            last_occurrence: now,
            resolved_at: None,
            acknowledged_at: None,
            acknowledged_by: None,
            resolution_note: None,
            count: 1,
            timeout: None,
            escalation_level: 0,
            next_escalation: None,
            tags: Vec::new(),
            priority_score: Self::calculate_priority_score(severity),
            silenced: false,
            silence_expires_at: None,
            related_alerts: Vec::new(),
        }
    }

    /// Create an alert with custom context
    pub fn with_context(
        title: String,
        description: String,
        severity: AlertSeverity,
        context: AlertContext,
    ) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4();

        Self {
            id,
            title,
            description,
            severity,
            status: AlertStatus::Firing,
            source: context.source.clone(),
            context,
            created_at: now,
            updated_at: now,
            last_occurrence: now,
            resolved_at: None,
            acknowledged_at: None,
            acknowledged_by: None,
            resolution_note: None,
            count: 1,
            timeout: None,
            escalation_level: 0,
            next_escalation: None,
            tags: Vec::new(),
            priority_score: Self::calculate_priority_score(severity),
            silenced: false,
            silence_expires_at: None,
            related_alerts: Vec::new(),
        }
    }

    /// Acknowledge the alert
    pub fn acknowledge(&mut self, acknowledged_by: String) {
        self.status = AlertStatus::Acknowledged;
        self.acknowledged_at = Some(Utc::now());
        self.acknowledged_by = Some(acknowledged_by);
        self.updated_at = Utc::now();
    }

    /// Resolve the alert
    pub fn resolve(&mut self, resolution_note: Option<String>) {
        self.status = AlertStatus::Resolved;
        self.resolved_at = Some(Utc::now());
        self.resolution_note = resolution_note;
        self.updated_at = Utc::now();
    }

    /// Suppress the alert
    pub fn suppress(&mut self) {
        self.status = AlertStatus::Suppressed;
        self.updated_at = Utc::now();
    }

    /// Silence the alert for a duration
    pub fn silence(&mut self, duration: chrono::Duration) {
        self.silenced = true;
        self.silence_expires_at = Some(Utc::now() + duration);
        self.updated_at = Utc::now();
    }

    /// Check if alert is silenced
    pub fn is_silenced(&self) -> bool {
        if !self.silenced {
            return false;
        }

        if let Some(expires_at) = self.silence_expires_at {
            Utc::now() < expires_at
        } else {
            true
        }
    }

    /// Update alert occurrence
    pub fn update_occurrence(&mut self) {
        self.count += 1;
        self.last_occurrence = Utc::now();
        self.updated_at = Utc::now();
    }

    /// Add a label to the alert
    pub fn add_label(&mut self, key: String, value: String) {
        self.context.labels.insert(key, value);
        self.updated_at = Utc::now();
    }

    /// Add an annotation to the alert
    pub fn add_annotation(&mut self, key: String, value: String) {
        self.context.annotations.insert(key, value);
        self.updated_at = Utc::now();
    }

    /// Add a tag to the alert
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = Utc::now();
        }
    }

    /// Set escalation level
    pub fn set_escalation_level(&mut self, level: u32, next_escalation: Option<DateTime<Utc>>) {
        self.escalation_level = level;
        self.next_escalation = next_escalation;
        self.updated_at = Utc::now();
    }

    /// Calculate priority score based on severity and other factors
    fn calculate_priority_score(severity: AlertSeverity) -> f64 {
        match severity {
            AlertSeverity::Critical => 100.0,
            AlertSeverity::High => 80.0,
            AlertSeverity::Medium => 60.0,
            AlertSeverity::Warning => 40.0,
            AlertSeverity::Info => 20.0,
        }
    }

    /// Generate fingerprint for deduplication
    fn generate_fingerprint(title: &str, source: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        title.hash(&mut hasher);
        source.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Get alert age in seconds
    pub fn age_seconds(&self) -> i64 {
        (Utc::now() - self.created_at).num_seconds()
    }

    /// Check if alert has expired based on timeout
    pub fn is_expired(&self) -> bool {
        if let Some(timeout) = self.timeout {
            self.age_seconds() > timeout as i64
        } else {
            false
        }
    }

    /// Get alert duration in seconds (time since creation)
    pub fn duration_seconds(&self) -> i64 {
        match self.status {
            AlertStatus::Resolved => {
                if let Some(resolved_at) = self.resolved_at {
                    (resolved_at - self.created_at).num_seconds()
                } else {
                    self.age_seconds()
                }
            }
            _ => self.age_seconds(),
        }
    }
}

impl AlertSeverity {
    /// Get numeric value for severity (higher = more severe)
    pub fn numeric_value(&self) -> u8 {
        match self {
            Self::Critical => 5,
            Self::High => 4,
            Self::Medium => 3,
            Self::Warning => 2,
            Self::Info => 1,
        }
    }

    /// Get color code for severity
    pub fn color_code(&self) -> &'static str {
        match self {
            Self::Critical => "#FF0000", // Red
            Self::High => "#FF8000",     // Orange
            Self::Medium => "#FFFF00",   // Yellow
            Self::Warning => "#80FF00",  // Light Green
            Self::Info => "#0080FF",     // Blue
        }
    }
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Critical => write!(f, "critical"),
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
        }
    }
}

impl std::fmt::Display for AlertStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Firing => write!(f, "firing"),
            Self::Acknowledged => write!(f, "acknowledged"),
            Self::Resolved => write!(f, "resolved"),
            Self::Suppressed => write!(f, "suppressed"),
        }
    }
}

impl Default for AlertContext {
    fn default() -> Self {
        Self {
            labels: HashMap::new(),
            annotations: HashMap::new(),
            source: "unknown".to_string(),
            fingerprint: "unknown".to_string(),
            generator_url: None,
            runbook_url: None,
            dashboard_url: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_creation() {
        let alert = Alert::new(
            "Test Alert".to_string(),
            "Test Description".to_string(),
            AlertSeverity::Critical,
            "test_source".to_string(),
        );

        assert_eq!(alert.title, "Test Alert");
        assert_eq!(alert.severity, AlertSeverity::Critical);
        assert_eq!(alert.status, AlertStatus::Firing);
        assert_eq!(alert.count, 1);
        assert!(!alert.is_silenced());
    }

    #[test]
    fn test_alert_acknowledge() {
        let mut alert = Alert::new(
            "Test Alert".to_string(),
            "Test Description".to_string(),
            AlertSeverity::Warning,
            "test_source".to_string(),
        );

        alert.acknowledge("test_user".to_string());
        assert_eq!(alert.status, AlertStatus::Acknowledged);
        assert!(alert.acknowledged_at.is_some());
        assert_eq!(alert.acknowledged_by, Some("test_user".to_string()));
    }

    #[test]
    fn test_alert_resolve() {
        let mut alert = Alert::new(
            "Test Alert".to_string(),
            "Test Description".to_string(),
            AlertSeverity::High,
            "test_source".to_string(),
        );

        alert.resolve(Some("Fixed the issue".to_string()));
        assert_eq!(alert.status, AlertStatus::Resolved);
        assert!(alert.resolved_at.is_some());
        assert_eq!(alert.resolution_note, Some("Fixed the issue".to_string()));
    }

    #[test]
    fn test_alert_silence() {
        let mut alert = Alert::new(
            "Test Alert".to_string(),
            "Test Description".to_string(),
            AlertSeverity::Medium,
            "test_source".to_string(),
        );

        alert.silence(chrono::Duration::hours(1));
        assert!(alert.is_silenced());
        assert!(alert.silence_expires_at.is_some());
    }

    #[test]
    fn test_severity_ordering() {
        assert!(AlertSeverity::Critical.numeric_value() > AlertSeverity::High.numeric_value());
        assert!(AlertSeverity::High.numeric_value() > AlertSeverity::Medium.numeric_value());
        assert!(AlertSeverity::Medium.numeric_value() > AlertSeverity::Warning.numeric_value());
        assert!(AlertSeverity::Warning.numeric_value() > AlertSeverity::Info.numeric_value());
    }
}
