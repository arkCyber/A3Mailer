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
}
