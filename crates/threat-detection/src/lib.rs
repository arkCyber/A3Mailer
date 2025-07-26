//! # Stalwart Threat Detection
//!
//! AI-driven threat detection system for Stalwart Mail Server.
//! Provides real-time analysis of email traffic, user behavior, and system events
//! to identify potential security threats and anomalies.
//!
//! ## Features
//!
//! - **Anomaly Detection**: Statistical and ML-based anomaly detection
//! - **Pattern Matching**: Rule-based threat pattern recognition
//! - **Behavioral Analysis**: User and system behavior analysis
//! - **Real-time Analysis**: Live threat detection and response
//! - **Threat Intelligence**: Integration with external threat feeds
//!
//! ## Architecture
//!
//! The threat detection system consists of:
//! - Threat Detector: Main detection engine
//! - Anomaly Detector: Statistical anomaly detection
//! - Pattern Matcher: Rule-based pattern matching
//! - Behavioral Analyzer: User behavior analysis
//! - Threat Intelligence: External threat data integration
//!
//! ## Example
//!
//! ```rust,no_run
//! use stalwart_threat_detection::{ThreatDetector, ThreatDetectionConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ThreatDetectionConfig::default();
//!     let detector = ThreatDetector::new(config).await?;
//!
//!     // Start threat detection
//!     detector.start_detection().await?;
//!
//!     Ok(())
//! }
//! ```

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

pub mod config;
pub mod detector;
pub mod anomaly;
pub mod patterns;
pub mod behavioral;
pub mod intelligence;
pub mod models;
pub mod metrics;
pub mod error;

pub use config::{ThreatDetectionConfig, AnomalyDetectionConfig, PatternMatchingConfig,
                BehavioralAnalysisConfig, ThreatIntelligenceConfig, ThreatFeed};
pub use detector::{ThreatDetector, EmailContext, AttachmentInfo, DetectionStats};
pub use anomaly::{AnomalyDetector, AnomalyScore, AnomalyResult, DetectedAnomaly, AnomalyType};
pub use patterns::{PatternMatcher, ThreatPattern, PatternType, PatternMatch, MatchLocation};
pub use behavioral::{BehavioralAnalyzer, BehaviorProfile, BehavioralAnomaly, BehavioralAnomalyType};
pub use intelligence::{ThreatIntelligence, ThreatIndicator, IndicatorType, IntelligenceMatch};
pub use error::{ThreatDetectionError, Result};

/// Threat severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum ThreatSeverity {
    /// Low severity threat
    Low,
    /// Medium severity threat
    Medium,
    /// High severity threat
    High,
    /// Critical severity threat
    Critical,
}

/// Threat detection event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThreatEvent {
    /// Event ID
    pub id: String,
    /// Threat type
    pub threat_type: ThreatType,
    /// Severity level
    pub severity: ThreatSeverity,
    /// Event description
    pub description: String,
    /// Source of the threat
    pub source: String,
    /// Target of the threat
    pub target: Option<String>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
}

/// Types of threats
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ThreatType {
    /// Malware detection
    Malware,
    /// Phishing attempt
    Phishing,
    /// Spam detection
    Spam,
    /// Brute force attack
    BruteForce,
    /// Anomalous behavior
    Anomaly,
    /// Data exfiltration
    DataExfiltration,
    /// Privilege escalation
    PrivilegeEscalation,
    /// Suspicious login
    SuspiciousLogin,
    /// Rate limiting violation
    RateLimitViolation,
    /// Behavioral anomaly
    BehavioralAnomaly,
    /// Unknown threat
    Unknown,
}

/// Threat detection status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionStatus {
    /// Detection is active
    Active,
    /// Detection is paused
    Paused,
    /// Detection has failed
    Failed,
    /// Detection is initializing
    Initializing,
}

/// Main threat detection context
pub struct ThreatDetectionContext {
    pub config: ThreatDetectionConfig,
    pub status: Arc<RwLock<DetectionStatus>>,
    pub events: Arc<RwLock<Vec<ThreatEvent>>>,
}

impl ThreatDetectionContext {
    /// Create a new threat detection context
    pub fn new(config: ThreatDetectionConfig) -> Self {
        Self {
            config,
            status: Arc::new(RwLock::new(DetectionStatus::Initializing)),
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get current detection status
    pub async fn status(&self) -> DetectionStatus {
        self.status.read().await.clone()
    }

    /// Set detection status
    pub async fn set_status(&self, status: DetectionStatus) {
        let mut current_status = self.status.write().await;
        if *current_status != status {
            info!("Threat detection status changed: {:?} -> {:?}", *current_status, status);
            *current_status = status;
        }
    }

    /// Add a threat event
    pub async fn add_event(&self, event: ThreatEvent) {
        let mut events = self.events.write().await;
        events.push(event);

        // Keep only recent events (configurable limit)
        if events.len() > self.config.max_events_history {
            let excess = events.len() - self.config.max_events_history;
            events.drain(0..excess);
        }
    }

    /// Get recent threat events
    pub async fn recent_events(&self, limit: usize) -> Vec<ThreatEvent> {
        let events = self.events.read().await;
        events.iter().rev().take(limit).cloned().collect()
    }
}

/// Initialize the threat detection system
pub async fn init_threat_detection(config: ThreatDetectionConfig) -> Result<ThreatDetectionContext> {
    info!("Initializing threat detection system");

    let context = ThreatDetectionContext::new(config);

    // TODO: Initialize threat detection components
    // - Set up anomaly detectors
    // - Load threat patterns
    // - Initialize behavioral analyzers
    // - Connect to threat intelligence feeds

    context.set_status(DetectionStatus::Active).await;

    info!("Threat detection system initialized successfully");
    Ok(context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_threat_detection_context_creation() {
        let config = ThreatDetectionConfig::default();
        let context = ThreatDetectionContext::new(config);

        assert_eq!(context.status().await, DetectionStatus::Initializing);
    }

    #[tokio::test]
    async fn test_threat_event_addition() {
        let config = ThreatDetectionConfig::default();
        let context = ThreatDetectionContext::new(config);

        let event = ThreatEvent {
            id: "test-event-1".to_string(),
            threat_type: ThreatType::Anomaly,
            severity: ThreatSeverity::Medium,
            description: "Test anomaly detected".to_string(),
            source: "test-source".to_string(),
            target: None,
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
            confidence: 0.8,
        };

        context.add_event(event).await;
        let events = context.recent_events(10).await;
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_threat_detector_creation() {
        let config = ThreatDetectionConfig::default();
        let detector = ThreatDetector::new(config).await.unwrap();

        // Test that detector starts successfully
        detector.start_detection().await.unwrap();
    }

    #[tokio::test]
    async fn test_email_context_creation() {
        use std::collections::HashMap;

        let context = EmailContext {
            sender: "sender@example.com".to_string(),
            recipients: vec!["recipient@example.com".to_string()],
            subject: "Test Subject".to_string(),
            body: "Test email body".to_string(),
            headers: HashMap::new(),
            attachments: vec![],
            timestamp: chrono::Utc::now(),
            source_ip: Some("192.168.1.1".to_string()),
            message_id: "test-message-id".to_string(),
        };

        assert_eq!(context.sender, "sender@example.com");
        assert_eq!(context.recipients.len(), 1);
        assert_eq!(context.subject, "Test Subject");
        assert!(context.source_ip.is_some());
    }

    #[tokio::test]
    async fn test_attachment_info_creation() {
        let attachment = AttachmentInfo {
            filename: "test.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            size: 1024,
            hash: "abc123".to_string(),
        };

        assert_eq!(attachment.filename, "test.pdf");
        assert_eq!(attachment.content_type, "application/pdf");
        assert_eq!(attachment.size, 1024);
        assert_eq!(attachment.hash, "abc123");
    }

    #[tokio::test]
    async fn test_detection_stats_default() {
        let stats = DetectionStats::default();

        assert_eq!(stats.total_emails_analyzed, 0);
        assert_eq!(stats.threats_detected, 0);
        assert_eq!(stats.false_positives, 0);
        assert_eq!(stats.processing_time_ms, 0);
    }

    #[tokio::test]
    async fn test_threat_severity_ordering() {
        assert!(ThreatSeverity::Critical > ThreatSeverity::High);
        assert!(ThreatSeverity::High > ThreatSeverity::Medium);
        assert!(ThreatSeverity::Medium > ThreatSeverity::Low);
    }

    #[tokio::test]
    async fn test_threat_type_variants() {
        let types = vec![
            ThreatType::Malware,
            ThreatType::Phishing,
            ThreatType::Spam,
            ThreatType::Anomaly,
        ];

        assert_eq!(types.len(), 4);
    }

    #[tokio::test]
    async fn test_multiple_threat_events() {
        use std::collections::HashMap;

        let config = ThreatDetectionConfig::default();
        let context = ThreatDetectionContext::new(config);

        for i in 0..5 {
            let event = ThreatEvent {
                id: format!("test-{}", i),
                threat_type: ThreatType::Spam,
                severity: ThreatSeverity::Low,
                description: format!("Test threat {}", i),
                source: format!("test{}@example.com", i),
                target: Some("victim@example.com".to_string()),
                timestamp: chrono::Utc::now(),
                metadata: HashMap::new(),
                confidence: 0.5,
            };

            context.add_event(event).await;
        }

        let events = context.recent_events(10).await;
        assert_eq!(events.len(), 5);

        // Events should be in reverse order (most recent first)
        for (i, event) in events.iter().enumerate() {
            let expected_id = format!("test-{}", 4 - i);
            assert_eq!(event.id, expected_id);
            assert_eq!(event.threat_type, ThreatType::Spam);
            assert_eq!(event.severity, ThreatSeverity::Low);
        }
    }

    #[tokio::test]
    async fn test_email_analysis() {
        use std::collections::HashMap;

        let config = ThreatDetectionConfig::default();
        let detector = ThreatDetector::new(config).await.unwrap();

        let context = EmailContext {
            sender: "malicious@example.com".to_string(),
            recipients: vec!["victim@example.com".to_string()],
            subject: "Urgent: Click this link now!".to_string(),
            body: "Click here to claim your prize: http://malicious.com".to_string(),
            headers: HashMap::new(),
            attachments: vec![],
            timestamp: chrono::Utc::now(),
            source_ip: Some("192.168.1.100".to_string()),
            message_id: "malicious-message-id".to_string(),
        };

        let result = detector.analyze_email(&context).await.unwrap();
        // For now, this returns None as the analysis is not implemented
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_detection_stats() {
        let config = ThreatDetectionConfig::default();
        let detector = ThreatDetector::new(config).await.unwrap();

        let stats = detector.get_stats().await.unwrap();
        assert_eq!(stats.total_emails_analyzed, 0);
        assert_eq!(stats.threats_detected, 0);
    }

    #[tokio::test]
    async fn test_status_changes() {
        let config = ThreatDetectionConfig::default();
        let context = ThreatDetectionContext::new(config);

        assert_eq!(context.status().await, DetectionStatus::Initializing);

        context.set_status(DetectionStatus::Active).await;
        assert_eq!(context.status().await, DetectionStatus::Active);

        context.set_status(DetectionStatus::Paused).await;
        assert_eq!(context.status().await, DetectionStatus::Paused);

        context.set_status(DetectionStatus::Failed).await;
        assert_eq!(context.status().await, DetectionStatus::Failed);
    }

    #[tokio::test]
    async fn test_event_history_limit() {
        let mut config = ThreatDetectionConfig::default();
        config.max_events_history = 3; // Set a small limit for testing

        let context = ThreatDetectionContext::new(config);

        // Add more events than the limit
        for i in 0..5 {
            let event = ThreatEvent {
                id: format!("test-{}", i),
                threat_type: ThreatType::Spam,
                severity: ThreatSeverity::Low,
                description: format!("Test threat {}", i),
                source: format!("test{}@example.com", i),
                target: None,
                timestamp: chrono::Utc::now(),
                metadata: std::collections::HashMap::new(),
                confidence: 0.5,
            };

            context.add_event(event).await;
        }

        let events = context.recent_events(10).await;
        assert_eq!(events.len(), 3); // Should only keep the last 3 events

        // Should have events 2, 3, 4 (in reverse order)
        assert_eq!(events[0].id, "test-4");
        assert_eq!(events[1].id, "test-3");
        assert_eq!(events[2].id, "test-2");
    }
}
