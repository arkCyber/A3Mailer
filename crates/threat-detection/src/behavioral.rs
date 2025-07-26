//! Behavioral Analysis Module
//!
//! This module provides behavioral analysis capabilities to detect threats
//! based on user behavior patterns, communication patterns, and deviations
//! from normal behavior.

use crate::{
    ThreatEvent, ThreatType, ThreatSeverity,
    config::BehavioralAnalysisConfig,
    detector::EmailContext,
    error::Result,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use chrono::{Utc, Duration, Timelike};

/// Behavioral analyzer
///
/// Analyzes user behavior patterns to detect anomalous activities
/// that may indicate compromised accounts or insider threats.
pub struct BehavioralAnalyzer {
    /// Configuration
    config: BehavioralAnalysisConfig,
    /// User behavior profiles
    profiles: Arc<RwLock<HashMap<String, BehaviorProfile>>>,
    /// Analysis statistics
    stats: Arc<RwLock<BehavioralStats>>,
    /// Running state
    is_running: Arc<RwLock<bool>>,
}

/// Behavior profile for a user
///
/// Contains learned patterns of normal behavior for a specific user.
#[derive(Debug, Clone)]
pub struct BehaviorProfile {
    /// User identifier
    pub user_id: String,
    /// Email sending patterns
    pub sending_patterns: SendingPatterns,
    /// Communication patterns
    pub communication_patterns: CommunicationPatterns,
    /// Content patterns
    pub content_patterns: ContentPatterns,
    /// Timing patterns
    pub timing_patterns: TimingPatterns,
    /// Profile creation time
    pub created_at: chrono::DateTime<Utc>,
    /// Last update time
    pub last_updated: chrono::DateTime<Utc>,
    /// Number of observations
    pub observation_count: u64,
}

/// Email sending patterns
#[derive(Debug, Clone, Default)]
pub struct SendingPatterns {
    /// Average emails per day
    pub avg_emails_per_day: f64,
    /// Peak sending hours
    pub peak_hours: Vec<u32>,
    /// Typical recipients
    pub frequent_recipients: HashMap<String, u32>,
    /// Subject line patterns
    pub subject_patterns: Vec<String>,
    /// Attachment usage patterns
    pub attachment_usage: AttachmentUsage,
}

/// Communication patterns
#[derive(Debug, Clone, Default)]
pub struct CommunicationPatterns {
    /// Internal vs external communication ratio
    pub internal_external_ratio: f64,
    /// Reply vs new email ratio
    pub reply_new_ratio: f64,
    /// Average response time
    pub avg_response_time: Duration,
    /// Communication network
    pub communication_network: HashMap<String, f64>,
}

/// Content patterns
#[derive(Debug, Clone, Default)]
pub struct ContentPatterns {
    /// Average email length
    pub avg_email_length: f64,
    /// Language patterns
    pub language_patterns: HashMap<String, f64>,
    /// Vocabulary patterns
    pub vocabulary_patterns: HashMap<String, f64>,
    /// Formatting patterns
    pub formatting_patterns: FormattingPatterns,
}

/// Timing patterns
#[derive(Debug, Clone, Default)]
pub struct TimingPatterns {
    /// Active hours distribution
    pub active_hours: HashMap<u32, f64>,
    /// Day of week patterns
    pub day_patterns: HashMap<u32, f64>,
    /// Seasonal patterns
    pub seasonal_patterns: HashMap<u32, f64>,
    /// Time zone consistency
    pub timezone_consistency: f64,
}

/// Attachment usage patterns
#[derive(Debug, Clone, Default)]
pub struct AttachmentUsage {
    /// Frequency of attachments
    pub attachment_frequency: f64,
    /// Common file types
    pub common_file_types: HashMap<String, u32>,
    /// Average attachment size
    pub avg_attachment_size: f64,
}

/// Formatting patterns
#[derive(Debug, Clone, Default)]
pub struct FormattingPatterns {
    /// HTML vs plain text ratio
    pub html_plain_ratio: f64,
    /// Signature patterns
    pub signature_patterns: Vec<String>,
    /// Font and style patterns
    pub style_patterns: HashMap<String, f64>,
}

/// Behavioral analysis statistics
#[derive(Debug, Clone, Default)]
pub struct BehavioralStats {
    /// Total profiles
    pub total_profiles: usize,
    /// Behavioral anomalies detected
    pub anomalies_detected: u64,
    /// Profile updates
    pub profile_updates: u64,
    /// Last analysis time
    pub last_analysis: Option<chrono::DateTime<Utc>>,
    /// Average analysis time
    pub avg_analysis_time_ms: f64,
}

/// Behavioral anomaly result
#[derive(Debug, Clone)]
pub struct BehavioralAnomaly {
    /// Anomaly type
    pub anomaly_type: BehavioralAnomalyType,
    /// Severity score
    pub severity_score: f64,
    /// Description
    pub description: String,
    /// Expected behavior
    pub expected: String,
    /// Observed behavior
    pub observed: String,
}

/// Types of behavioral anomalies
#[derive(Debug, Clone, PartialEq)]
pub enum BehavioralAnomalyType {
    /// Unusual sending volume
    SendingVolumeAnomaly,
    /// Unusual timing
    TimingAnomaly,
    /// Unusual recipients
    RecipientAnomaly,
    /// Unusual content
    ContentAnomaly,
    /// Unusual attachment behavior
    AttachmentAnomaly,
    /// Communication pattern change
    CommunicationAnomaly,
}

impl BehavioralAnalyzer {
    /// Create new behavioral analyzer
    ///
    /// # Arguments
    /// * `config` - Behavioral analysis configuration
    ///
    /// # Returns
    /// A new BehavioralAnalyzer instance
    pub async fn new(config: &BehavioralAnalysisConfig) -> Result<Self> {
        info!("Initializing behavioral analyzer");

        Ok(Self {
            config: config.clone(),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(BehavioralStats::default())),
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start behavioral analyzer
    pub async fn start(&self) -> Result<()> {
        info!("Starting behavioral analyzer");

        let mut running = self.is_running.write().await;
        if *running {
            warn!("Behavioral analyzer is already running");
            return Ok(());
        }

        // Load existing profiles
        self.load_profiles().await?;

        *running = true;
        info!("Behavioral analyzer started");

        Ok(())
    }

    /// Stop behavioral analyzer
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping behavioral analyzer");

        let mut running = self.is_running.write().await;
        if !*running {
            warn!("Behavioral analyzer is not running");
            return Ok(());
        }

        // Save profiles
        self.save_profiles().await?;

        *running = false;
        info!("Behavioral analyzer stopped");

        Ok(())
    }

    /// Analyze email for behavioral anomalies
    ///
    /// # Arguments
    /// * `email_context` - Email context to analyze
    ///
    /// # Returns
    /// Optional threat event if behavioral anomalies are detected
    pub async fn analyze_email(&self, email_context: &EmailContext) -> Result<Option<ThreatEvent>> {
        debug!("Analyzing email for behavioral anomalies: {}", email_context.message_id);

        let anomalies = self.detect_behavioral_anomalies(email_context).await?;

        if !anomalies.is_empty() {
            // Find the most severe anomaly
            let most_severe = anomalies.iter()
                .max_by(|a, b| a.severity_score.partial_cmp(&b.severity_score).unwrap())
                .unwrap();

            let threat_event = ThreatEvent {
                id: format!("behavioral-{}", uuid::Uuid::new_v4()),
                threat_type: ThreatType::BehavioralAnomaly,
                severity: self.score_to_severity(most_severe.severity_score),
                description: format!("Behavioral anomaly: {}", most_severe.description),
                source: email_context.sender.clone(),
                target: Some(email_context.recipients.join(", ")),
                timestamp: Utc::now(),
                metadata: self.create_behavioral_metadata(&anomalies),
                confidence: most_severe.severity_score,
            };

            Ok(Some(threat_event))
        } else {
            // Update user profile with normal behavior
            self.update_user_profile(email_context).await?;
            Ok(None)
        }
    }

    /// Detect behavioral anomalies
    async fn detect_behavioral_anomalies(&self, email_context: &EmailContext) -> Result<Vec<BehavioralAnomaly>> {
        let profiles = self.profiles.read().await;
        let mut anomalies = Vec::new();

        if let Some(profile) = profiles.get(&email_context.sender) {
            // Check sending volume anomaly
            if let Some(anomaly) = self.check_sending_volume_anomaly(email_context, profile).await {
                anomalies.push(anomaly);
            }

            // Check timing anomaly
            if let Some(anomaly) = self.check_timing_anomaly(email_context, profile).await {
                anomalies.push(anomaly);
            }

            // Check recipient anomaly
            if let Some(anomaly) = self.check_recipient_anomaly(email_context, profile).await {
                anomalies.push(anomaly);
            }

            // Check content anomaly
            if let Some(anomaly) = self.check_content_anomaly(email_context, profile).await {
                anomalies.push(anomaly);
            }

            // Check attachment anomaly
            if let Some(anomaly) = self.check_attachment_anomaly(email_context, profile).await {
                anomalies.push(anomaly);
            }
        } else {
            // New user - create initial profile
            self.create_initial_profile(email_context).await?;
        }

        Ok(anomalies)
    }

    /// Check sending volume anomaly
    async fn check_sending_volume_anomaly(
        &self,
        _email_context: &EmailContext,
        _profile: &BehaviorProfile
    ) -> Option<BehavioralAnomaly> {
        // TODO: Implement sending volume anomaly detection
        // - Compare current sending rate with historical patterns
        // - Consider time of day and day of week
        None
    }

    /// Check timing anomaly
    async fn check_timing_anomaly(
        &self,
        email_context: &EmailContext,
        profile: &BehaviorProfile
    ) -> Option<BehavioralAnomaly> {
        let hour = email_context.timestamp.hour();
        let expected_activity = profile.timing_patterns.active_hours.get(&hour).unwrap_or(&0.0);

        if *expected_activity < 0.1 {
            // User rarely sends emails at this hour
            Some(BehavioralAnomaly {
                anomaly_type: BehavioralAnomalyType::TimingAnomaly,
                severity_score: 0.7,
                description: "Email sent at unusual time".to_string(),
                expected: format!("Low activity at hour {}", hour),
                observed: format!("Email sent at hour {}", hour),
            })
        } else {
            None
        }
    }

    /// Check recipient anomaly
    async fn check_recipient_anomaly(
        &self,
        email_context: &EmailContext,
        profile: &BehaviorProfile
    ) -> Option<BehavioralAnomaly> {
        let mut unknown_recipients = 0;
        let total_recipients = email_context.recipients.len();

        for recipient in &email_context.recipients {
            if !profile.sending_patterns.frequent_recipients.contains_key(recipient) {
                unknown_recipients += 1;
            }
        }

        let unknown_ratio = unknown_recipients as f64 / total_recipients as f64;

        if unknown_ratio > 0.8 && total_recipients > 1 {
            // Most recipients are unknown
            Some(BehavioralAnomaly {
                anomaly_type: BehavioralAnomalyType::RecipientAnomaly,
                severity_score: 0.6,
                description: "Email sent to mostly unknown recipients".to_string(),
                expected: "Known recipients".to_string(),
                observed: format!("{:.1}% unknown recipients", unknown_ratio * 100.0),
            })
        } else {
            None
        }
    }

    /// Check content anomaly
    async fn check_content_anomaly(
        &self,
        email_context: &EmailContext,
        profile: &BehaviorProfile
    ) -> Option<BehavioralAnomaly> {
        let email_length = email_context.body.len() as f64;
        let expected_length = profile.content_patterns.avg_email_length;

        if expected_length > 0.0 {
            let length_ratio = email_length / expected_length;

            if length_ratio > 5.0 || length_ratio < 0.2 {
                // Email is much longer or shorter than usual
                Some(BehavioralAnomaly {
                    anomaly_type: BehavioralAnomalyType::ContentAnomaly,
                    severity_score: 0.4,
                    description: "Unusual email length".to_string(),
                    expected: format!("~{:.0} characters", expected_length),
                    observed: format!("{:.0} characters", email_length),
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Check attachment anomaly
    async fn check_attachment_anomaly(
        &self,
        email_context: &EmailContext,
        profile: &BehaviorProfile
    ) -> Option<BehavioralAnomaly> {
        let has_attachments = !email_context.attachments.is_empty();
        let expected_frequency = profile.sending_patterns.attachment_usage.attachment_frequency;

        if has_attachments && expected_frequency < 0.1 {
            // User rarely sends attachments
            Some(BehavioralAnomaly {
                anomaly_type: BehavioralAnomalyType::AttachmentAnomaly,
                severity_score: 0.5,
                description: "Unusual attachment usage".to_string(),
                expected: "No attachments".to_string(),
                observed: format!("{} attachments", email_context.attachments.len()),
            })
        } else {
            None
        }
    }

    /// Update user profile with new email data
    async fn update_user_profile(&self, email_context: &EmailContext) -> Result<()> {
        let mut profiles = self.profiles.write().await;

        if let Some(profile) = profiles.get_mut(&email_context.sender) {
            // Update existing profile
            profile.observation_count += 1;
            profile.last_updated = Utc::now();

            // Update patterns (simplified)
            let hour = email_context.timestamp.hour();
            *profile.timing_patterns.active_hours.entry(hour).or_insert(0.0) += 1.0;

            for recipient in &email_context.recipients {
                *profile.sending_patterns.frequent_recipients.entry(recipient.clone()).or_insert(0) += 1;
            }

            // Update content patterns
            let email_length = email_context.body.len() as f64;
            if profile.content_patterns.avg_email_length == 0.0 {
                profile.content_patterns.avg_email_length = email_length;
            } else {
                profile.content_patterns.avg_email_length =
                    (profile.content_patterns.avg_email_length * (profile.observation_count - 1) as f64 + email_length)
                    / profile.observation_count as f64;
            }

            // Update attachment patterns
            let has_attachments = !email_context.attachments.is_empty();
            if has_attachments {
                profile.sending_patterns.attachment_usage.attachment_frequency =
                    (profile.sending_patterns.attachment_usage.attachment_frequency * (profile.observation_count - 1) as f64 + 1.0)
                    / profile.observation_count as f64;
            } else {
                profile.sending_patterns.attachment_usage.attachment_frequency =
                    (profile.sending_patterns.attachment_usage.attachment_frequency * (profile.observation_count - 1) as f64)
                    / profile.observation_count as f64;
            }
        }

        Ok(())
    }

    /// Create initial profile for new user
    async fn create_initial_profile(&self, email_context: &EmailContext) -> Result<()> {
        let mut profiles = self.profiles.write().await;

        let mut profile = BehaviorProfile {
            user_id: email_context.sender.clone(),
            sending_patterns: SendingPatterns::default(),
            communication_patterns: CommunicationPatterns::default(),
            content_patterns: ContentPatterns::default(),
            timing_patterns: TimingPatterns::default(),
            created_at: Utc::now(),
            last_updated: Utc::now(),
            observation_count: 1,
        };

        // Initialize with first observation
        let hour = email_context.timestamp.hour();
        profile.timing_patterns.active_hours.insert(hour, 1.0);

        for recipient in &email_context.recipients {
            profile.sending_patterns.frequent_recipients.insert(recipient.clone(), 1);
        }

        profile.content_patterns.avg_email_length = email_context.body.len() as f64;

        if !email_context.attachments.is_empty() {
            profile.sending_patterns.attachment_usage.attachment_frequency = 1.0;
        }

        profiles.insert(email_context.sender.clone(), profile);

        info!("Created initial behavior profile for user: {}", email_context.sender);

        Ok(())
    }

    /// Convert severity score to threat severity
    fn score_to_severity(&self, score: f64) -> ThreatSeverity {
        if score >= 0.8 {
            ThreatSeverity::High
        } else if score >= 0.6 {
            ThreatSeverity::Medium
        } else {
            ThreatSeverity::Low
        }
    }

    /// Create metadata for behavioral anomalies
    fn create_behavioral_metadata(&self, anomalies: &[BehavioralAnomaly]) -> HashMap<String, serde_json::Value> {
        let mut metadata = HashMap::new();

        metadata.insert("anomaly_count".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(anomalies.len())));

        let anomaly_types: Vec<String> = anomalies.iter()
            .map(|a| format!("{:?}", a.anomaly_type))
            .collect();

        metadata.insert("anomaly_types".to_string(),
                        serde_json::Value::Array(anomaly_types.into_iter()
                            .map(serde_json::Value::String)
                            .collect()));

        metadata
    }

    /// Load behavior profiles
    async fn load_profiles(&self) -> Result<()> {
        debug!("Loading behavior profiles");
        // TODO: Load from persistent storage
        Ok(())
    }

    /// Save behavior profiles
    async fn save_profiles(&self) -> Result<()> {
        debug!("Saving behavior profiles");
        // TODO: Save to persistent storage
        Ok(())
    }

    /// Get behavioral analysis statistics
    pub async fn get_stats(&self) -> BehavioralStats {
        self.stats.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_email_context() -> EmailContext {
        EmailContext {
            sender: "test@example.com".to_string(),
            recipients: vec!["recipient@example.com".to_string()],
            subject: "Test Subject".to_string(),
            body: "Test email body".to_string(),
            headers: HashMap::new(),
            attachments: vec![],
            timestamp: chrono::Utc::now(),
            source_ip: Some("192.168.1.1".to_string()),
            message_id: "test-message-id".to_string(),
        }
    }

    #[tokio::test]
    async fn test_behavioral_analyzer_creation() {
        let config = BehavioralAnalysisConfig::default();
        let analyzer = BehavioralAnalyzer::new(&config).await.unwrap();

        let stats = analyzer.get_stats().await;
        assert_eq!(stats.total_profiles, 0);
        assert_eq!(stats.anomalies_detected, 0);
    }

    #[tokio::test]
    async fn test_analyze_email() {
        let config = BehavioralAnalysisConfig::default();
        let analyzer = BehavioralAnalyzer::new(&config).await.unwrap();

        let context = create_test_email_context();
        let result = analyzer.analyze_email(&context).await.unwrap();

        // For now, this should return None as the analysis is basic
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_user_profile() {
        let config = BehavioralAnalysisConfig::default();
        let analyzer = BehavioralAnalyzer::new(&config).await.unwrap();

        let context = create_test_email_context();
        let result = analyzer.update_user_profile(&context).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_email_analysis() {
        let config = BehavioralAnalysisConfig::default();
        let analyzer = BehavioralAnalyzer::new(&config).await.unwrap();

        // Analyze multiple emails
        for i in 0..5 {
            let mut context = create_test_email_context();
            context.sender = format!("test{}@example.com", i);
            context.message_id = format!("test-message-{}", i);

            let result = analyzer.analyze_email(&context).await.unwrap();
            // For now, should return None
            assert!(result.is_none());
        }

        let stats = analyzer.get_stats().await;
        // Check that profiles were created
        assert!(stats.total_profiles > 0);
    }

    #[tokio::test]
    async fn test_suspicious_email_patterns() {
        let config = BehavioralAnalysisConfig::default();
        let analyzer = BehavioralAnalyzer::new(&config).await.unwrap();

        // Create a suspicious email context
        let mut context = create_test_email_context();
        context.subject = "URGENT!!! CLICK NOW!!!".to_string();
        context.body = "You have won $1,000,000! Click here immediately!".to_string();
        context.sender = "noreply@suspicious-domain.com".to_string();

        let result = analyzer.analyze_email(&context).await.unwrap();
        // For now, should return None as detection is not fully implemented
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_off_hours_email() {
        let config = BehavioralAnalysisConfig::default();
        let analyzer = BehavioralAnalyzer::new(&config).await.unwrap();

        // Create an email sent at an unusual time (3 AM)
        let mut context = create_test_email_context();
        let mut timestamp = chrono::Utc::now();
        timestamp = timestamp.with_hour(3).unwrap().with_minute(0).unwrap();
        context.timestamp = timestamp;

        let result = analyzer.analyze_email(&context).await.unwrap();

        // For now, should return None
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_high_volume_sender() {
        let config = BehavioralAnalysisConfig::default();
        let analyzer = BehavioralAnalyzer::new(&config).await.unwrap();

        let context = create_test_email_context();

        // Simulate high volume by analyzing many emails from the same sender
        for _ in 0..10 {
            let _ = analyzer.analyze_email(&context).await.unwrap();
        }

        let stats = analyzer.get_stats().await;

        // Should have created profiles
        assert!(stats.total_profiles > 0);
    }
}
