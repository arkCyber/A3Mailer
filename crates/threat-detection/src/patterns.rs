//! Pattern Matching Module
//!
//! This module provides signature-based threat detection using pattern matching,
//! regular expressions, and known threat indicators to identify malicious content.

use crate::{
    ThreatEvent, ThreatType, ThreatSeverity,
    config::PatternMatchingConfig,
    detector::EmailContext,
    error::Result,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use chrono::Utc;
use regex::Regex;

/// Pattern matcher
///
/// Uses signature-based detection to identify known threats through
/// pattern matching, regular expressions, and threat indicators.
pub struct PatternMatcher {
    /// Configuration
    config: PatternMatchingConfig,
    /// Compiled threat patterns
    patterns: Arc<RwLock<CompiledPatterns>>,
    /// Pattern statistics
    stats: Arc<RwLock<PatternStats>>,
    /// Running state
    is_running: Arc<RwLock<bool>>,
}

/// Compiled patterns for efficient matching
#[derive(Debug, Default)]
pub struct CompiledPatterns {
    /// Malware signature patterns
    pub malware_patterns: Vec<ThreatPattern>,
    /// Phishing patterns
    pub phishing_patterns: Vec<ThreatPattern>,
    /// Spam patterns
    pub spam_patterns: Vec<ThreatPattern>,
    /// URL patterns
    pub url_patterns: Vec<ThreatPattern>,
    /// Attachment patterns
    pub attachment_patterns: Vec<ThreatPattern>,
    /// Header patterns
    pub header_patterns: Vec<ThreatPattern>,
}

/// Threat pattern definition
#[derive(Debug, Clone)]
pub struct ThreatPattern {
    /// Pattern ID
    pub id: String,
    /// Pattern name
    pub name: String,
    /// Pattern description
    pub description: String,
    /// Pattern type
    pub pattern_type: PatternType,
    /// Compiled regex
    pub regex: Regex,
    /// Threat severity
    pub severity: ThreatSeverity,
    /// Confidence score
    pub confidence: f64,
    /// Pattern metadata
    pub metadata: HashMap<String, String>,
    /// Last updated
    pub last_updated: chrono::DateTime<Utc>,
}

/// Pattern types
#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    /// Malware signature
    Malware,
    /// Phishing indicator
    Phishing,
    /// Spam pattern
    Spam,
    /// Malicious URL
    MaliciousUrl,
    /// Suspicious attachment
    SuspiciousAttachment,
    /// Header anomaly
    HeaderAnomaly,
    /// Content pattern
    ContentPattern,
}

/// Pattern matching statistics
#[derive(Debug, Clone, Default)]
pub struct PatternStats {
    /// Total patterns loaded
    pub total_patterns: usize,
    /// Patterns matched
    pub patterns_matched: u64,
    /// False positives
    pub false_positives: u64,
    /// Last pattern update
    pub last_update: Option<chrono::DateTime<Utc>>,
    /// Match performance
    pub avg_match_time_ms: f64,
}

/// Pattern match result
#[derive(Debug, Clone)]
pub struct PatternMatch {
    /// Matched pattern
    pub pattern: ThreatPattern,
    /// Match location
    pub location: MatchLocation,
    /// Matched text
    pub matched_text: String,
    /// Match confidence
    pub confidence: f64,
}

/// Match location in email
#[derive(Debug, Clone)]
pub enum MatchLocation {
    /// Subject line
    Subject,
    /// Message body
    Body,
    /// Header field
    Header(String),
    /// Attachment name
    AttachmentName(String),
    /// URL in content
    Url,
    /// Sender address
    Sender,
}

impl PatternMatcher {
    /// Create new pattern matcher
    ///
    /// # Arguments
    /// * `config` - Pattern matching configuration
    ///
    /// # Returns
    /// A new PatternMatcher instance
    pub async fn new(config: &PatternMatchingConfig) -> Result<Self> {
        info!("Initializing pattern matcher");

        Ok(Self {
            config: config.clone(),
            patterns: Arc::new(RwLock::new(CompiledPatterns::default())),
            stats: Arc::new(RwLock::new(PatternStats::default())),
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start pattern matcher
    pub async fn start(&self) -> Result<()> {
        info!("Starting pattern matcher");

        let mut running = self.is_running.write().await;
        if *running {
            warn!("Pattern matcher is already running");
            return Ok(());
        }

        // Load and compile patterns
        self.load_patterns().await?;

        *running = true;
        info!("Pattern matcher started");

        Ok(())
    }

    /// Stop pattern matcher
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping pattern matcher");

        let mut running = self.is_running.write().await;
        if !*running {
            warn!("Pattern matcher is not running");
            return Ok(());
        }

        *running = false;
        info!("Pattern matcher stopped");

        Ok(())
    }

    /// Analyze email for pattern matches
    ///
    /// # Arguments
    /// * `email_context` - Email context to analyze
    ///
    /// # Returns
    /// Optional threat event if patterns are matched
    pub async fn analyze_email(&self, email_context: &EmailContext) -> Result<Option<ThreatEvent>> {
        debug!("Analyzing email for pattern matches: {}", email_context.message_id);

        let matches = self.find_pattern_matches(email_context).await?;

        if !matches.is_empty() {
            // Find the highest severity match
            let highest_severity_match = matches.iter()
                .max_by_key(|m| self.severity_to_score(&m.pattern.severity))
                .unwrap();

            let threat_event = ThreatEvent {
                id: format!("pattern-{}", uuid::Uuid::new_v4()),
                threat_type: self.pattern_type_to_threat_type(&highest_severity_match.pattern.pattern_type),
                severity: highest_severity_match.pattern.severity.clone(),
                description: format!("Pattern match: {}", highest_severity_match.pattern.description),
                source: email_context.sender.clone(),
                target: Some(email_context.recipients.join(", ")),
                timestamp: Utc::now(),
                metadata: self.create_pattern_metadata(&matches),
                confidence: highest_severity_match.confidence,
            };

            Ok(Some(threat_event))
        } else {
            Ok(None)
        }
    }

    /// Find pattern matches in email
    async fn find_pattern_matches(&self, email_context: &EmailContext) -> Result<Vec<PatternMatch>> {
        let patterns = self.patterns.read().await;
        let mut matches = Vec::new();

        // Check subject line
        matches.extend(self.check_patterns_in_text(
            &email_context.subject,
            &patterns.phishing_patterns,
            MatchLocation::Subject
        ).await);

        matches.extend(self.check_patterns_in_text(
            &email_context.subject,
            &patterns.spam_patterns,
            MatchLocation::Subject
        ).await);

        // Check message body
        matches.extend(self.check_patterns_in_text(
            &email_context.body,
            &patterns.malware_patterns,
            MatchLocation::Body
        ).await);

        matches.extend(self.check_patterns_in_text(
            &email_context.body,
            &patterns.phishing_patterns,
            MatchLocation::Body
        ).await);

        matches.extend(self.check_patterns_in_text(
            &email_context.body,
            &patterns.url_patterns,
            MatchLocation::Url
        ).await);

        // Check headers
        for (header_name, header_value) in &email_context.headers {
            matches.extend(self.check_patterns_in_text(
                header_value,
                &patterns.header_patterns,
                MatchLocation::Header(header_name.clone())
            ).await);
        }

        // Check attachments
        for attachment in &email_context.attachments {
            matches.extend(self.check_patterns_in_text(
                &attachment.filename,
                &patterns.attachment_patterns,
                MatchLocation::AttachmentName(attachment.filename.clone())
            ).await);
        }

        // Check sender
        matches.extend(self.check_patterns_in_text(
            &email_context.sender,
            &patterns.phishing_patterns,
            MatchLocation::Sender
        ).await);

        Ok(matches)
    }

    /// Check patterns in text
    async fn check_patterns_in_text(
        &self,
        text: &str,
        patterns: &[ThreatPattern],
        location: MatchLocation
    ) -> Vec<PatternMatch> {
        let mut matches = Vec::new();

        for pattern in patterns {
            if let Some(regex_match) = pattern.regex.find(text) {
                matches.push(PatternMatch {
                    pattern: pattern.clone(),
                    location: location.clone(),
                    matched_text: regex_match.as_str().to_string(),
                    confidence: pattern.confidence,
                });
            }
        }

        matches
    }

    /// Load threat patterns
    async fn load_patterns(&self) -> Result<()> {
        info!("Loading threat patterns");

        let mut patterns = self.patterns.write().await;

        // Load default patterns
        patterns.malware_patterns = self.load_malware_patterns().await?;
        patterns.phishing_patterns = self.load_phishing_patterns().await?;
        patterns.spam_patterns = self.load_spam_patterns().await?;
        patterns.url_patterns = self.load_url_patterns().await?;
        patterns.attachment_patterns = self.load_attachment_patterns().await?;
        patterns.header_patterns = self.load_header_patterns().await?;

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_patterns = patterns.malware_patterns.len() +
                              patterns.phishing_patterns.len() +
                              patterns.spam_patterns.len() +
                              patterns.url_patterns.len() +
                              patterns.attachment_patterns.len() +
                              patterns.header_patterns.len();
        stats.last_update = Some(Utc::now());

        info!("Loaded {} threat patterns", stats.total_patterns);

        Ok(())
    }

    /// Load malware patterns
    async fn load_malware_patterns(&self) -> Result<Vec<ThreatPattern>> {
        let mut patterns = Vec::new();

        // Example malware patterns
        patterns.push(ThreatPattern {
            id: "malware-001".to_string(),
            name: "Executable attachment".to_string(),
            description: "Suspicious executable file attachment".to_string(),
            pattern_type: PatternType::Malware,
            regex: Regex::new(r"(?i)\.(exe|scr|bat|cmd|com|pif)$").unwrap(),
            severity: ThreatSeverity::High,
            confidence: 0.8,
            metadata: HashMap::new(),
            last_updated: Utc::now(),
        });

        patterns.push(ThreatPattern {
            id: "malware-002".to_string(),
            name: "Macro document".to_string(),
            description: "Document with potential malicious macros".to_string(),
            pattern_type: PatternType::Malware,
            regex: Regex::new(r"(?i)\.(doc|docm|xls|xlsm|ppt|pptm)$").unwrap(),
            severity: ThreatSeverity::Medium,
            confidence: 0.6,
            metadata: HashMap::new(),
            last_updated: Utc::now(),
        });

        Ok(patterns)
    }

    /// Load phishing patterns
    async fn load_phishing_patterns(&self) -> Result<Vec<ThreatPattern>> {
        let mut patterns = Vec::new();

        // Example phishing patterns
        patterns.push(ThreatPattern {
            id: "phish-001".to_string(),
            name: "Urgent action required".to_string(),
            description: "Phishing email with urgency tactics".to_string(),
            pattern_type: PatternType::Phishing,
            regex: Regex::new(r"(?i)(urgent|immediate|action required|verify account|suspended)").unwrap(),
            severity: ThreatSeverity::High,
            confidence: 0.7,
            metadata: HashMap::new(),
            last_updated: Utc::now(),
        });

        patterns.push(ThreatPattern {
            id: "phish-002".to_string(),
            name: "Suspicious URL".to_string(),
            description: "URL with suspicious characteristics".to_string(),
            pattern_type: PatternType::Phishing,
            regex: Regex::new(r"(?i)https?://[a-z0-9-]+\.(tk|ml|ga|cf)/").unwrap(),
            severity: ThreatSeverity::Medium,
            confidence: 0.6,
            metadata: HashMap::new(),
            last_updated: Utc::now(),
        });

        Ok(patterns)
    }

    /// Load spam patterns
    async fn load_spam_patterns(&self) -> Result<Vec<ThreatPattern>> {
        let mut patterns = Vec::new();

        // Example spam patterns
        patterns.push(ThreatPattern {
            id: "spam-001".to_string(),
            name: "Money offer".to_string(),
            description: "Spam email offering money".to_string(),
            pattern_type: PatternType::Spam,
            regex: Regex::new(r"(?i)(make money|earn \$|free money|lottery winner)").unwrap(),
            severity: ThreatSeverity::Low,
            confidence: 0.8,
            metadata: HashMap::new(),
            last_updated: Utc::now(),
        });

        Ok(patterns)
    }

    /// Load URL patterns
    async fn load_url_patterns(&self) -> Result<Vec<ThreatPattern>> {
        let mut patterns = Vec::new();

        // Example URL patterns
        patterns.push(ThreatPattern {
            id: "url-001".to_string(),
            name: "Shortened URL".to_string(),
            description: "Potentially malicious shortened URL".to_string(),
            pattern_type: PatternType::MaliciousUrl,
            regex: Regex::new(r"(?i)https?://(bit\.ly|tinyurl\.com|t\.co|goo\.gl)/").unwrap(),
            severity: ThreatSeverity::Medium,
            confidence: 0.5,
            metadata: HashMap::new(),
            last_updated: Utc::now(),
        });

        Ok(patterns)
    }

    /// Load attachment patterns
    async fn load_attachment_patterns(&self) -> Result<Vec<ThreatPattern>> {
        let mut patterns = Vec::new();

        // Example attachment patterns
        patterns.push(ThreatPattern {
            id: "attach-001".to_string(),
            name: "Double extension".to_string(),
            description: "File with double extension".to_string(),
            pattern_type: PatternType::SuspiciousAttachment,
            regex: Regex::new(r"(?i)\.[a-z]{2,4}\.(exe|scr|bat|cmd)$").unwrap(),
            severity: ThreatSeverity::High,
            confidence: 0.9,
            metadata: HashMap::new(),
            last_updated: Utc::now(),
        });

        Ok(patterns)
    }

    /// Load header patterns
    async fn load_header_patterns(&self) -> Result<Vec<ThreatPattern>> {
        let mut patterns = Vec::new();

        // Example header patterns
        patterns.push(ThreatPattern {
            id: "header-001".to_string(),
            name: "Spoofed sender".to_string(),
            description: "Potentially spoofed sender address".to_string(),
            pattern_type: PatternType::HeaderAnomaly,
            regex: Regex::new(r"(?i)(noreply|no-reply)@(paypal|amazon|microsoft|google)\.com").unwrap(),
            severity: ThreatSeverity::Medium,
            confidence: 0.7,
            metadata: HashMap::new(),
            last_updated: Utc::now(),
        });

        Ok(patterns)
    }

    /// Convert pattern type to threat type
    fn pattern_type_to_threat_type(&self, pattern_type: &PatternType) -> ThreatType {
        match pattern_type {
            PatternType::Malware => ThreatType::Malware,
            PatternType::Phishing => ThreatType::Phishing,
            PatternType::Spam => ThreatType::Spam,
            PatternType::MaliciousUrl => ThreatType::Phishing,
            PatternType::SuspiciousAttachment => ThreatType::Malware,
            PatternType::HeaderAnomaly => ThreatType::Phishing,
            PatternType::ContentPattern => ThreatType::Spam,
        }
    }

    /// Convert severity to numeric score for comparison
    fn severity_to_score(&self, severity: &ThreatSeverity) -> u8 {
        match severity {
            ThreatSeverity::Critical => 4,
            ThreatSeverity::High => 3,
            ThreatSeverity::Medium => 2,
            ThreatSeverity::Low => 1,
        }
    }

    /// Create metadata for pattern matches
    fn create_pattern_metadata(&self, matches: &[PatternMatch]) -> HashMap<String, serde_json::Value> {
        let mut metadata = HashMap::new();

        metadata.insert("pattern_matches".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(matches.len())));

        let pattern_ids: Vec<String> = matches.iter()
            .map(|m| m.pattern.id.clone())
            .collect();

        metadata.insert("matched_patterns".to_string(),
                        serde_json::Value::Array(pattern_ids.into_iter()
                            .map(serde_json::Value::String)
                            .collect()));

        metadata
    }

    /// Get pattern statistics
    pub async fn get_stats(&self) -> PatternStats {
        self.stats.read().await.clone()
    }
}
