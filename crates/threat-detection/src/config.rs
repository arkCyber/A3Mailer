//! Configuration for threat detection

use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Configuration for threat detection system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatDetectionConfig {
    /// Enable anomaly detection
    pub anomaly_detection_enabled: bool,

    /// Enable pattern matching
    pub pattern_matching_enabled: bool,

    /// Enable behavioral analysis
    pub behavioral_analysis_enabled: bool,

    /// Enable threat intelligence integration
    pub threat_intelligence_enabled: bool,

    /// Anomaly detection configuration
    pub anomaly: AnomalyDetectionConfig,

    /// Pattern matching configuration
    pub patterns: PatternMatchingConfig,

    /// Behavioral analysis configuration
    pub behavioral: BehavioralAnalysisConfig,

    /// Threat intelligence configuration
    pub intelligence: ThreatIntelligenceConfig,

    /// Maximum number of events to keep in history
    pub max_events_history: usize,

    /// Detection interval
    pub detection_interval: Duration,

    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,

    /// Model update interval
    pub model_update_interval: Duration,
}

/// Anomaly detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDetectionConfig {
    /// Statistical threshold for anomaly detection
    pub statistical_threshold: f64,

    /// Anomaly threshold for threat detection
    pub anomaly_threshold: f64,

    /// Machine learning model path
    pub ml_model_path: Option<String>,

    /// Window size for statistical analysis
    pub window_size: usize,

    /// Minimum samples required for analysis
    pub min_samples: usize,

    /// Features to analyze
    pub features: Vec<String>,
}

/// Pattern matching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatchingConfig {
    /// Path to threat patterns file
    pub patterns_file: String,

    /// Enable regex patterns
    pub regex_enabled: bool,

    /// Enable signature-based detection
    pub signature_enabled: bool,

    /// Pattern update interval
    pub update_interval: Duration,
}

/// Behavioral analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralAnalysisConfig {
    /// Learning period for baseline behavior
    pub learning_period: Duration,

    /// Deviation threshold for anomalous behavior
    pub deviation_threshold: f64,

    /// User behavior features to track
    pub user_features: Vec<String>,

    /// System behavior features to track
    pub system_features: Vec<String>,

    /// Profile update interval
    pub profile_update_interval: Duration,
}

/// Threat intelligence configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatIntelligenceConfig {
    /// Threat intelligence feeds
    pub feeds: Vec<ThreatFeed>,

    /// Feed update interval
    pub update_interval: Duration,

    /// Cache duration for threat indicators
    pub cache_duration: Duration,

    /// API timeout
    pub api_timeout: Duration,
}

/// Threat feed configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatFeed {
    /// Feed name
    pub name: String,

    /// Feed URL
    pub url: String,

    /// API key (if required)
    pub api_key: Option<String>,

    /// Feed format
    pub format: ThreatFeedFormat,

    /// Feed priority
    pub priority: u8,

    /// Enable this feed
    pub enabled: bool,
}

/// Threat feed formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreatFeedFormat {
    /// STIX format
    Stix,
    /// JSON format
    Json,
    /// CSV format
    Csv,
    /// XML format
    Xml,
    /// Custom format
    Custom(String),
}

/// Alert thresholds configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// Low severity threshold
    pub low_threshold: f64,

    /// Medium severity threshold
    pub medium_threshold: f64,

    /// High severity threshold
    pub high_threshold: f64,

    /// Critical severity threshold
    pub critical_threshold: f64,
}

impl Default for ThreatDetectionConfig {
    fn default() -> Self {
        Self {
            anomaly_detection_enabled: true,
            pattern_matching_enabled: true,
            behavioral_analysis_enabled: true,
            threat_intelligence_enabled: false,
            anomaly: AnomalyDetectionConfig::default(),
            patterns: PatternMatchingConfig::default(),
            behavioral: BehavioralAnalysisConfig::default(),
            intelligence: ThreatIntelligenceConfig::default(),
            max_events_history: 10000,
            detection_interval: Duration::from_secs(60),
            alert_thresholds: AlertThresholds::default(),
            model_update_interval: Duration::from_secs(3600),
        }
    }
}

impl Default for AnomalyDetectionConfig {
    fn default() -> Self {
        Self {
            statistical_threshold: 2.0,
            anomaly_threshold: 0.7,
            ml_model_path: None,
            window_size: 100,
            min_samples: 10,
            features: vec![
                "login_frequency".to_string(),
                "email_volume".to_string(),
                "connection_patterns".to_string(),
            ],
        }
    }
}

impl Default for PatternMatchingConfig {
    fn default() -> Self {
        Self {
            patterns_file: "threat_patterns.json".to_string(),
            regex_enabled: true,
            signature_enabled: true,
            update_interval: Duration::from_secs(3600),
        }
    }
}

impl Default for BehavioralAnalysisConfig {
    fn default() -> Self {
        Self {
            learning_period: Duration::from_secs(7 * 24 * 3600), // 7 days
            deviation_threshold: 2.5,
            user_features: vec![
                "login_times".to_string(),
                "email_patterns".to_string(),
                "access_locations".to_string(),
            ],
            system_features: vec![
                "resource_usage".to_string(),
                "network_patterns".to_string(),
                "error_rates".to_string(),
            ],
            profile_update_interval: Duration::from_secs(3600),
        }
    }
}

impl Default for ThreatIntelligenceConfig {
    fn default() -> Self {
        Self {
            feeds: Vec::new(),
            update_interval: Duration::from_secs(3600),
            cache_duration: Duration::from_secs(24 * 3600),
            api_timeout: Duration::from_secs(30),
        }
    }
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            low_threshold: 0.3,
            medium_threshold: 0.6,
            high_threshold: 0.8,
            critical_threshold: 0.95,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ThreatDetectionConfig::default();
        assert!(config.anomaly_detection_enabled);
        assert!(config.pattern_matching_enabled);
        assert!(config.behavioral_analysis_enabled);
        assert!(!config.threat_intelligence_enabled);
    }

    #[test]
    fn test_config_serialization() {
        let config = ThreatDetectionConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: ThreatDetectionConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(config.max_events_history, deserialized.max_events_history);
    }
}
