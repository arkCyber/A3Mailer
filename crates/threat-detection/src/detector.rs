//! Main Threat Detector Implementation
//!
//! This module provides the core threat detection engine that coordinates
//! various detection methods including anomaly detection, pattern matching,
//! behavioral analysis, and threat intelligence integration.

use crate::{
    ThreatDetectionConfig, ThreatEvent, ThreatType, ThreatSeverity,
    AnomalyDetector, PatternMatcher, BehavioralAnalyzer, ThreatIntelligence,
    error::Result,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};
use chrono::Utc;
use std::collections::HashMap;

/// Main threat detector
///
/// Coordinates multiple detection engines to provide comprehensive
/// threat detection capabilities for email security.
pub struct ThreatDetector {
    /// Configuration
    config: ThreatDetectionConfig,
    /// Anomaly detection engine
    anomaly_detector: Arc<AnomalyDetector>,
    /// Pattern matching engine
    pattern_matcher: Arc<PatternMatcher>,
    /// Behavioral analysis engine
    behavioral_analyzer: Arc<BehavioralAnalyzer>,
    /// Threat intelligence engine
    threat_intelligence: Arc<ThreatIntelligence>,
    /// Detection statistics
    stats: Arc<RwLock<DetectionStats>>,
    /// Running state
    is_running: Arc<RwLock<bool>>,
}

/// Detection statistics
#[derive(Debug, Clone, Default)]
pub struct DetectionStats {
    /// Total events analyzed
    pub total_analyzed: u64,
    /// Threats detected
    pub threats_detected: u64,
    /// False positives
    pub false_positives: u64,
    /// Analysis time statistics
    pub avg_analysis_time_ms: f64,
    /// Detection accuracy
    pub accuracy_rate: f64,
    /// Last analysis timestamp
    pub last_analysis: Option<chrono::DateTime<Utc>>,
}

/// Email analysis context
#[derive(Debug, Clone)]
pub struct EmailContext {
    /// Message ID
    pub message_id: String,
    /// Sender information
    pub sender: String,
    /// Recipients
    pub recipients: Vec<String>,
    /// Subject line
    pub subject: String,
    /// Message body
    pub body: String,
    /// Headers
    pub headers: HashMap<String, String>,
    /// Attachments
    pub attachments: Vec<AttachmentInfo>,
    /// Timestamp
    pub timestamp: chrono::DateTime<Utc>,
    /// Source IP
    pub source_ip: Option<String>,
}

/// Attachment information
#[derive(Debug, Clone)]
pub struct AttachmentInfo {
    /// Filename
    pub filename: String,
    /// Content type
    pub content_type: String,
    /// Size in bytes
    pub size: u64,
    /// Hash of content
    pub hash: String,
}

impl ThreatDetector {
    /// Create a new threat detector
    ///
    /// # Arguments
    /// * `config` - Threat detection configuration
    ///
    /// # Returns
    /// A new ThreatDetector instance
    pub async fn new(config: ThreatDetectionConfig) -> Result<Self> {
        info!("Initializing threat detector");

        // Initialize detection engines
        let anomaly_detector = Arc::new(AnomalyDetector::new(&config.anomaly).await?);
        let pattern_matcher = Arc::new(PatternMatcher::new(&config.patterns).await?);
        let behavioral_analyzer = Arc::new(BehavioralAnalyzer::new(&config.behavioral).await?);
        let threat_intelligence = Arc::new(ThreatIntelligence::new(&config.intelligence).await?);

        Ok(Self {
            config,
            anomaly_detector,
            pattern_matcher,
            behavioral_analyzer,
            threat_intelligence,
            stats: Arc::new(RwLock::new(DetectionStats::default())),
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start threat detection
    ///
    /// Initializes all detection engines and begins monitoring.
    pub async fn start_detection(&self) -> Result<()> {
        info!("Starting threat detection");

        let mut running = self.is_running.write().await;
        if *running {
            warn!("Threat detection is already running");
            return Ok(());
        }

        // Start all detection engines
        self.anomaly_detector.start().await?;
        self.pattern_matcher.start().await?;
        self.behavioral_analyzer.start().await?;
        self.threat_intelligence.start().await?;

        *running = true;
        info!("Threat detection started successfully");

        Ok(())
    }

    /// Stop threat detection
    pub async fn stop_detection(&self) -> Result<()> {
        info!("Stopping threat detection");

        let mut running = self.is_running.write().await;
        if !*running {
            warn!("Threat detection is not running");
            return Ok(());
        }

        // Stop all detection engines
        self.anomaly_detector.stop().await?;
        self.pattern_matcher.stop().await?;
        self.behavioral_analyzer.stop().await?;
        self.threat_intelligence.stop().await?;

        *running = false;
        info!("Threat detection stopped");

        Ok(())
    }

    /// Analyze an email for threats
    ///
    /// # Arguments
    /// * `email_context` - Email context to analyze
    ///
    /// # Returns
    /// Optional threat event if threats are detected
    pub async fn analyze_email(&self, email_context: &EmailContext) -> Result<Vec<ThreatEvent>> {
        let start_time = std::time::Instant::now();
        debug!("Analyzing email: {}", email_context.message_id);

        let mut threats = Vec::new();

        // Run all detection engines in parallel
        let (anomaly_result, pattern_result, behavioral_result, intelligence_result) = tokio::join!(
            self.anomaly_detector.analyze_email(email_context),
            self.pattern_matcher.analyze_email(email_context),
            self.behavioral_analyzer.analyze_email(email_context),
            self.threat_intelligence.analyze_email(email_context)
        );

        // Collect results from all engines
        if let Ok(Some(threat)) = anomaly_result {
            threats.push(threat);
        }

        if let Ok(Some(threat)) = pattern_result {
            threats.push(threat);
        }

        if let Ok(Some(threat)) = behavioral_result {
            threats.push(threat);
        }

        if let Ok(Some(threat)) = intelligence_result {
            threats.push(threat);
        }

        // Update statistics
        let analysis_time = start_time.elapsed().as_millis() as f64;
        self.update_stats(analysis_time, !threats.is_empty()).await;

        if !threats.is_empty() {
            info!("Detected {} threats in email {}", threats.len(), email_context.message_id);
        }

        Ok(threats)
    }

    /// Update detection statistics
    async fn update_stats(&self, analysis_time_ms: f64, threat_detected: bool) {
        let mut stats = self.stats.write().await;

        stats.total_analyzed += 1;
        if threat_detected {
            stats.threats_detected += 1;
        }

        // Update average analysis time
        if stats.total_analyzed == 1 {
            stats.avg_analysis_time_ms = analysis_time_ms;
        } else {
            stats.avg_analysis_time_ms =
                (stats.avg_analysis_time_ms * (stats.total_analyzed - 1) as f64 + analysis_time_ms)
                / stats.total_analyzed as f64;
        }

        // Update accuracy rate (simplified calculation)
        stats.accuracy_rate = if stats.total_analyzed > 0 {
            (stats.threats_detected as f64 / stats.total_analyzed as f64) * 100.0
        } else {
            0.0
        };

        stats.last_analysis = Some(Utc::now());
    }

    /// Get detection statistics
    pub async fn get_stats(&self) -> DetectionStats {
        self.stats.read().await.clone()
    }

    /// Check if detection is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }
}
