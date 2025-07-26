//! Threat Intelligence Module
//!
//! This module provides threat intelligence capabilities including
//! integration with external threat feeds, IOC (Indicators of Compromise)
//! management, and reputation services.

use crate::{
    ThreatEvent, ThreatType, ThreatSeverity,
    config::ThreatIntelligenceConfig,
    detector::EmailContext,
    error::Result,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};
use chrono::Utc;
use reqwest::Client;

// Constants to avoid string literal issues
const LOADING_MSG: &str = "Loading threat indicators";
const LOADED_MSG: &str = "Loaded {} total threat indicators";
const DOMAIN_VALUE: &str = "malicious-domain.com";
const DOMAIN_DESC: &str = "Known malware C&C domain";
const UPDATES_MSG: &str = "Starting background feed updates";
const SOURCES_KEY: &str = "intelligence_sources";
const INDICATORS_KEY: &str = "matched_indicators";
const MATCHES_KEY: &str = "intelligence_matches";
const MALWARE_TAG: &str = "malware";
const C2_TAG: &str = "c2";
const EXAMPLE_PREFIX: &str = "example";
const FEED_LOADING_MSG: &str = "Loading indicators from feed";
const FEED_ERROR_MSG: &str = "Failed to load indicators from feed";
const FEED_SUCCESS_MSG: &str = "Loaded {} indicators from feed";

/// Threat intelligence engine
///
/// Integrates with external threat intelligence sources to provide
/// real-time threat information and IOC matching.
pub struct ThreatIntelligence {
    /// Configuration
    config: ThreatIntelligenceConfig,
    /// HTTP client for API calls
    client: Client,
    /// Cached threat indicators
    indicators: Arc<RwLock<ThreatIndicatorCache>>,
    /// Intelligence statistics
    stats: Arc<RwLock<IntelligenceStats>>,
    /// Running state
    is_running: Arc<RwLock<bool>>,
}

/// Threat indicator cache
#[derive(Debug, Default)]
pub struct ThreatIndicatorCache {
    /// IP address indicators
    pub ip_indicators: HashMap<String, ThreatIndicator>,
    /// Domain indicators
    pub domain_indicators: HashMap<String, ThreatIndicator>,
    /// URL indicators
    pub url_indicators: HashMap<String, ThreatIndicator>,
    /// File hash indicators
    pub hash_indicators: HashMap<String, ThreatIndicator>,
    /// Email indicators
    pub email_indicators: HashMap<String, ThreatIndicator>,
    /// Last update timestamp
    pub last_updated: Option<chrono::DateTime<Utc>>,
}

/// Threat indicator
///
/// Represents a single indicator of compromise with associated metadata.
#[derive(Debug, Clone)]
pub struct ThreatIndicator {
    /// Indicator ID
    pub id: String,
    /// Indicator type
    pub indicator_type: IndicatorType,
    /// Indicator value
    pub value: String,
    /// Threat type
    pub threat_type: ThreatType,
    /// Severity level
    pub severity: ThreatSeverity,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Source of the indicator
    pub source: String,
    /// Description
    pub description: String,
    /// Tags
    pub tags: Vec<String>,
    /// First seen timestamp
    pub first_seen: chrono::DateTime<Utc>,
    /// Last seen timestamp
    pub last_seen: chrono::DateTime<Utc>,
    /// Expiration timestamp
    pub expires_at: Option<chrono::DateTime<Utc>>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Types of threat indicators
#[derive(Debug, Clone, PartialEq)]
pub enum IndicatorType {
    /// IP address
    IpAddress,
    /// Domain name
    Domain,
    /// URL
    Url,
    /// File hash (MD5, SHA1, SHA256)
    FileHash,
    /// Email address
    EmailAddress,
    /// Subject line pattern
    SubjectPattern,
    /// Content pattern
    ContentPattern,
}

/// Intelligence statistics
#[derive(Debug, Clone, Default)]
pub struct IntelligenceStats {
    /// Total indicators loaded
    pub total_indicators: usize,
    /// Indicators matched
    pub indicators_matched: u64,
    /// API calls made
    pub api_calls: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Last feed update
    pub last_feed_update: Option<chrono::DateTime<Utc>>,
    /// Average lookup time
    pub avg_lookup_time_ms: f64,
}

/// Threat intelligence match result
#[derive(Debug, Clone)]
pub struct IntelligenceMatch {
    /// Matched indicator
    pub indicator: ThreatIndicator,
    /// Match context
    pub context: MatchContext,
    /// Match confidence
    pub confidence: f64,
}

/// Match context information
#[derive(Debug, Clone)]
pub enum MatchContext {
    /// Sender IP address
    SenderIp(String),
    /// Sender domain
    SenderDomain(String),
    /// URL in content
    ContentUrl(String),
    /// Attachment hash
    AttachmentHash(String),
    /// Sender email
    SenderEmail(String),
}

impl ThreatIntelligence {
    /// Create new threat intelligence engine
    ///
    /// # Arguments
    /// * `config` - Threat intelligence configuration
    ///
    /// # Returns
    /// A new ThreatIntelligence instance
    pub async fn new(config: &ThreatIntelligenceConfig) -> Result<Self> {
        info!("Initializing threat intelligence engine");

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| crate::error::ThreatDetectionError::Config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config: config.clone(),
            client,
            indicators: Arc::new(RwLock::new(ThreatIndicatorCache::default())),
            stats: Arc::new(RwLock::new(IntelligenceStats::default())),
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start threat intelligence engine
    pub async fn start(&self) -> Result<()> {
        info!("Starting threat intelligence engine");

        let mut running = self.is_running.write().await;
        if *running {
            warn!("Threat intelligence engine is already running");
            return Ok(());
        }

        // Load threat indicators
        self.load_threat_indicators().await?;

        // Start background feed updates
        self.start_feed_updates().await?;

        *running = true;
        info!("Threat intelligence engine started");

        Ok(())
    }

    /// Stop threat intelligence engine
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping threat intelligence engine");

        let mut running = self.is_running.write().await;
        if !*running {
            warn!("Threat intelligence engine is not running");
            return Ok(());
        }

        // Stop background tasks
        // TODO: Implement proper task cancellation

        *running = false;
        info!("Threat intelligence engine stopped");

        Ok(())
    }

    /// Analyze email using threat intelligence
    ///
    /// # Arguments
    /// * `email_context` - Email context to analyze
    ///
    /// # Returns
    /// Optional threat event if intelligence matches are found
    pub async fn analyze_email(&self, email_context: &EmailContext) -> Result<Option<ThreatEvent>> {
        debug!("Analyzing email with threat intelligence: {}", email_context.message_id);

        let matches = self.find_intelligence_matches(email_context).await?;

        if !matches.is_empty() {
            // Find the highest severity match
            let highest_severity_match = matches.iter()
                .max_by_key(|m| self.severity_to_score(&m.indicator.severity))
                .unwrap();

            let threat_event = ThreatEvent {
                id: format!("intel-{}", uuid::Uuid::new_v4()),
                threat_type: highest_severity_match.indicator.threat_type.clone(),
                severity: highest_severity_match.indicator.severity.clone(),
                description: format!("Threat intelligence match: {}", highest_severity_match.indicator.description),
                source: email_context.sender.clone(),
                target: Some(email_context.recipients.join(", ")),
                timestamp: Utc::now(),
                metadata: self.create_intelligence_metadata(&matches),
                confidence: highest_severity_match.confidence,
            };

            Ok(Some(threat_event))
        } else {
            Ok(None)
        }
    }

    /// Find threat intelligence matches
    async fn find_intelligence_matches(&self, email_context: &EmailContext) -> Result<Vec<IntelligenceMatch>> {
        let indicators = self.indicators.read().await;
        let mut matches = Vec::new();

        // Check sender IP
        if let Some(source_ip) = &email_context.source_ip {
            if let Some(indicator) = indicators.ip_indicators.get(source_ip) {
                matches.push(IntelligenceMatch {
                    indicator: indicator.clone(),
                    context: MatchContext::SenderIp(source_ip.clone()),
                    confidence: indicator.confidence,
                });
            }
        }

        // Check sender domain
        let sender_domain = email_context.sender.split('@').nth(1).unwrap_or("");
        if let Some(indicator) = indicators.domain_indicators.get(sender_domain) {
            matches.push(IntelligenceMatch {
                indicator: indicator.clone(),
                context: MatchContext::SenderDomain(sender_domain.to_string()),
                confidence: indicator.confidence,
            });
        }

        // Check sender email
        if let Some(indicator) = indicators.email_indicators.get(&email_context.sender) {
            matches.push(IntelligenceMatch {
                indicator: indicator.clone(),
                context: MatchContext::SenderEmail(email_context.sender.clone()),
                confidence: indicator.confidence,
            });
        }

        // Check URLs in content
        let urls = self.extract_urls(&email_context.body).await;
        for url in urls {
            if let Some(indicator) = indicators.url_indicators.get(&url) {
                matches.push(IntelligenceMatch {
                    indicator: indicator.clone(),
                    context: MatchContext::ContentUrl(url),
                    confidence: indicator.confidence,
                });
            }
        }

        // Check attachment hashes
        for attachment in &email_context.attachments {
            if let Some(indicator) = indicators.hash_indicators.get(&attachment.hash) {
                matches.push(IntelligenceMatch {
                    indicator: indicator.clone(),
                    context: MatchContext::AttachmentHash(attachment.hash.clone()),
                    confidence: indicator.confidence,
                });
            }
        }

        Ok(matches)
    }

    /// Extract URLs from text
    async fn extract_urls(&self, text: &str) -> Vec<String> {
        let url_regex = regex::Regex::new(r"https?://[^\s<>]+").unwrap();
        url_regex.find_iter(text)
            .map(|m| m.as_str().to_string())
            .collect()
    }

    /// Load threat indicators from various sources
    async fn load_threat_indicators(&self) -> Result<()> {
        // TODO: Add logging

        // Load from configured feeds
        for feed in &self.config.feeds {
            match self.load_feed_indicators(feed).await {
                Ok(_count) => {
                    // TODO: Log success properly
                }
                Err(_e) => {
                    // TODO: Log error properly
                }
            }
        }

        // Update statistics
        let indicators = self.indicators.read().await;
        let mut stats = self.stats.write().await;
        stats.total_indicators = indicators.ip_indicators.len() +
                                indicators.domain_indicators.len() +
                                indicators.url_indicators.len() +
                                indicators.hash_indicators.len() +
                                indicators.email_indicators.len();
        stats.last_feed_update = Some(Utc::now());

        tracing::info!(LOADED_MSG, stats.total_indicators);

        Ok(())
    }

    /// Load indicators from a specific feed
    async fn load_feed_indicators(&self, feed: &crate::config::ThreatFeed) -> Result<usize> {
        // TODO: Implement actual feed loading based on feed type
        // For now, return mock data
        let mut indicators = self.indicators.write().await;

        // Add some example indicators
        let example_indicator = ThreatIndicator {
            id: {
                let prefix = EXAMPLE_PREFIX;
                let uuid = uuid::Uuid::new_v4();
                let mut result = String::new();
                result.push_str(prefix);
                result.push('-');
                result.push_str(&uuid.to_string());
                result
            },
            indicator_type: IndicatorType::Domain,
            value: String::from(DOMAIN_VALUE),
            threat_type: ThreatType::Malware,
            severity: ThreatSeverity::High,
            confidence: 0.9,
            source: feed.name.clone(),
            description: String::from(DOMAIN_DESC),
            tags: vec![String::from(MALWARE_TAG), String::from(C2_TAG)],
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::days(30)),
            metadata: HashMap::new(),
        };

        indicators.domain_indicators.insert(
            example_indicator.value.clone(),
            example_indicator
        );

        Ok(1) // Return number of indicators loaded
    }

    /// Start background feed updates
    async fn start_feed_updates(&self) -> Result<()> {
        // TODO: Implement periodic feed updates
        Ok(())
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

    /// Create metadata for intelligence matches
    fn create_intelligence_metadata(&self, matches: &[IntelligenceMatch]) -> HashMap<String, serde_json::Value> {
        let mut metadata = HashMap::new();

        metadata.insert(String::from(MATCHES_KEY),
                        serde_json::Value::Number(serde_json::Number::from(matches.len())));

        let indicator_ids: Vec<String> = matches.iter()
            .map(|m| m.indicator.id.clone())
            .collect();

        metadata.insert(String::from(INDICATORS_KEY),
                        serde_json::Value::Array(indicator_ids.into_iter()
                            .map(serde_json::Value::String)
                            .collect()));

        let sources: Vec<String> = matches.iter()
            .map(|m| m.indicator.source.clone())
            .collect();

        metadata.insert(String::from(SOURCES_KEY),
                        serde_json::Value::Array(sources.into_iter()
                            .map(serde_json::Value::String)
                            .collect()));

        metadata
    }

    /// Get intelligence statistics
    pub async fn get_stats(&self) -> IntelligenceStats {
        self.stats.read().await.clone()
    }

    /// Add custom threat indicator
    pub async fn add_indicator(&self, indicator: ThreatIndicator) -> Result<()> {
        let mut indicators = self.indicators.write().await;

        match indicator.indicator_type {
            IndicatorType::IpAddress => {
                indicators.ip_indicators.insert(indicator.value.clone(), indicator);
            }
            IndicatorType::Domain => {
                indicators.domain_indicators.insert(indicator.value.clone(), indicator);
            }
            IndicatorType::Url => {
                indicators.url_indicators.insert(indicator.value.clone(), indicator);
            }
            IndicatorType::FileHash => {
                indicators.hash_indicators.insert(indicator.value.clone(), indicator);
            }
            IndicatorType::EmailAddress => {
                indicators.email_indicators.insert(indicator.value.clone(), indicator);
            }
            _ => {
                // Handle other types as needed
            }
        }

        Ok(())
    }

    /// Remove threat indicator
    pub async fn remove_indicator(&self, indicator_type: IndicatorType, value: &str) -> Result<bool> {
        let mut indicators = self.indicators.write().await;

        let removed = match indicator_type {
            IndicatorType::IpAddress => indicators.ip_indicators.remove(value).is_some(),
            IndicatorType::Domain => indicators.domain_indicators.remove(value).is_some(),
            IndicatorType::Url => indicators.url_indicators.remove(value).is_some(),
            IndicatorType::FileHash => indicators.hash_indicators.remove(value).is_some(),
            IndicatorType::EmailAddress => indicators.email_indicators.remove(value).is_some(),
            _ => false,
        };

        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_config() -> ThreatIntelligenceConfig {
        ThreatIntelligenceConfig {
            feeds: vec![],
            update_interval: std::time::Duration::from_secs(3600),
            cache_duration: std::time::Duration::from_secs(1800),
            api_timeout: std::time::Duration::from_secs(30),
        }
    }

    fn create_test_email_context() -> EmailContext {
        EmailContext {
            sender: "test@example.com".to_string(),
            recipients: vec!["recipient@example.com".to_string()],
            subject: "Test Subject".to_string(),
            body: "Test email body with URL: http://example.com".to_string(),
            headers: HashMap::new(),
            attachments: vec![],
            timestamp: chrono::Utc::now(),
            source_ip: Some("192.168.1.1".to_string()),
            message_id: "test-message-id".to_string(),
        }
    }

    fn create_test_indicator() -> ThreatIndicator {
        ThreatIndicator {
            id: "test-indicator-1".to_string(),
            indicator_type: IndicatorType::Domain,
            value: "malicious.com".to_string(),
            threat_type: ThreatType::Malware,
            severity: ThreatSeverity::High,
            confidence: 0.9,
            description: "Known malware domain".to_string(),
            source: "test-feed".to_string(),
            tags: vec!["malware".to_string(), "c2".to_string()],
            first_seen: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
            expires_at: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_threat_intelligence_creation() {
        let config = create_test_config();
        let intelligence = ThreatIntelligence::new(&config).await.unwrap();

        let stats = intelligence.get_stats().await;
        assert_eq!(stats.total_indicators, 0);
    }

    #[tokio::test]
    async fn test_add_indicator() {
        let config = create_test_config();
        let intelligence = ThreatIntelligence::new(&config).await.unwrap();

        let indicator = create_test_indicator();
        let result = intelligence.add_indicator(indicator.clone()).await;

        assert!(result.is_ok());

        let stats = intelligence.get_stats().await;
        assert_eq!(stats.total_indicators, 1);
    }

    #[tokio::test]
    async fn test_remove_indicator() {
        let config = create_test_config();
        let intelligence = ThreatIntelligence::new(&config).await.unwrap();

        let indicator = create_test_indicator();
        intelligence.add_indicator(indicator.clone()).await.unwrap();

        let removed = intelligence.remove_indicator(indicator.indicator_type, &indicator.value).await.unwrap();
        assert!(removed);

        let stats = intelligence.get_stats().await;
        assert_eq!(stats.total_indicators, 0);
    }

    #[tokio::test]
    async fn test_analyze_email() {
        let config = create_test_config();
        let intelligence = ThreatIntelligence::new(&config).await.unwrap();

        // Add a malicious domain indicator
        let mut indicator = create_test_indicator();
        indicator.value = "example.com".to_string(); // Match the URL in test email
        intelligence.add_indicator(indicator).await.unwrap();

        let context = create_test_email_context();
        let result = intelligence.analyze_email(&context).await.unwrap();

        // Should detect the threat
        assert!(result.is_some());
        let threat_event = result.unwrap();
        assert_eq!(threat_event.threat_type, ThreatType::Malware);
        assert_eq!(threat_event.severity, ThreatSeverity::High);
    }

    #[tokio::test]
    async fn test_analyze_email_no_threats() {
        let config = create_test_config();
        let intelligence = ThreatIntelligence::new(&config).await.unwrap();

        let context = create_test_email_context();
        let result = intelligence.analyze_email(&context).await.unwrap();

        // Should not detect any threats
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_extract_urls() {
        let config = create_test_config();
        let intelligence = ThreatIntelligence::new(&config).await.unwrap();

        let text = "Visit https://example.com and http://test.org for more info";
        let urls = intelligence.extract_urls(text).await;

        assert_eq!(urls.len(), 2);
        assert!(urls.contains(&"https://example.com".to_string()));
        assert!(urls.contains(&"http://test.org".to_string()));
    }

    #[tokio::test]
    async fn test_extract_urls_only() {
        let config = create_test_config();
        let intelligence = ThreatIntelligence::new(&config).await.unwrap();

        let text = "Contact admin@example.com or support@test.org";
        let urls = intelligence.extract_urls(text).await;

        // Should not extract email addresses as URLs
        assert_eq!(urls.len(), 0);
    }

    #[tokio::test]
    async fn test_multiple_indicators() {
        let config = create_test_config();
        let intelligence = ThreatIntelligence::new(&config).await.unwrap();

        // Add multiple indicators
        for i in 0..5 {
            let mut indicator = create_test_indicator();
            indicator.id = format!("test-indicator-{}", i);
            indicator.value = format!("malicious{}.com", i);
            intelligence.add_indicator(indicator).await.unwrap();
        }

        let stats = intelligence.get_stats().await;
        assert_eq!(stats.total_indicators, 5);
    }

    #[tokio::test]
    async fn test_different_indicator_types() {
        let config = create_test_config();
        let intelligence = ThreatIntelligence::new(&config).await.unwrap();

        // Add indicators of different types
        let types_and_values = vec![
            (IndicatorType::Domain, "malicious.com"),
            (IndicatorType::IpAddress, "192.168.1.100"),
            (IndicatorType::Url, "http://malicious.com/path"),
            (IndicatorType::FileHash, "abc123def456"),
            (IndicatorType::EmailAddress, "malicious@example.com"),
        ];

        for (indicator_type, value) in types_and_values {
            let mut indicator = create_test_indicator();
            indicator.indicator_type = indicator_type;
            indicator.value = value.to_string();
            intelligence.add_indicator(indicator).await.unwrap();
        }

        let stats = intelligence.get_stats().await;
        assert_eq!(stats.total_indicators, 5);
    }

    #[tokio::test]
    async fn test_ip_address_detection() {
        let config = create_test_config();
        let intelligence = ThreatIntelligence::new(&config).await.unwrap();

        // Add malicious IP indicator
        let mut indicator = create_test_indicator();
        indicator.indicator_type = IndicatorType::IpAddress;
        indicator.value = "192.168.1.1".to_string(); // Match source IP in test email
        intelligence.add_indicator(indicator).await.unwrap();

        let context = create_test_email_context();
        let result = intelligence.analyze_email(&context).await.unwrap();

        // Should detect the threat from IP
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_email_address_detection() {
        let config = create_test_config();
        let intelligence = ThreatIntelligence::new(&config).await.unwrap();

        // Add malicious email indicator
        let mut indicator = create_test_indicator();
        indicator.indicator_type = IndicatorType::EmailAddress;
        indicator.value = "test@example.com".to_string(); // Match sender in test email
        intelligence.add_indicator(indicator).await.unwrap();

        let context = create_test_email_context();
        let result = intelligence.analyze_email(&context).await.unwrap();

        // Should detect the threat from email address
        assert!(result.is_some());
    }
}
