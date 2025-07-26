/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Notification channels for alert delivery

use crate::alert::Alert;
use crate::error::{AlertingError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use tracing::{info, warn, error, debug};

/// Notification channel types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelType {
    /// Email notifications
    Email,
    /// SMS notifications
    Sms,
    /// Slack notifications
    Slack,
    /// Discord notifications
    Discord,
    /// Telegram notifications
    Telegram,
    /// Microsoft Teams notifications
    Teams,
    /// PagerDuty notifications
    PagerDuty,
    /// Webhook notifications
    Webhook,
    /// Custom channel
    Custom(String),
}

/// Delivery result for notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryResult {
    /// Channel type that delivered the notification
    pub channel_type: ChannelType,
    /// Whether delivery was successful
    pub success: bool,
    /// Delivery timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Delivery duration in milliseconds
    pub duration_ms: u64,
    /// Error message if delivery failed
    pub error: Option<String>,
    /// Delivery metadata
    pub metadata: HashMap<String, String>,
}

/// Channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// Channel name
    pub name: String,
    /// Channel type
    pub channel_type: ChannelType,
    /// Channel configuration parameters
    pub config: HashMap<String, String>,
    /// Whether channel is enabled
    pub enabled: bool,
    /// Channel priority (higher = more important)
    pub priority: u8,
    /// Rate limiting configuration
    pub rate_limit: Option<RateLimitConfig>,
    /// Retry configuration
    pub retry: RetryConfig,
    /// Filter configuration
    pub filter: FilterConfig,
}

/// Rate limiting configuration for channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum messages per time window
    pub max_messages: u32,
    /// Time window in seconds
    pub window_seconds: u64,
    /// Burst allowance
    pub burst: u32,
}

/// Retry configuration for channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial retry delay in seconds
    pub initial_delay_seconds: u64,
    /// Maximum retry delay in seconds
    pub max_delay_seconds: u64,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Whether to use exponential backoff
    pub exponential_backoff: bool,
}

/// Filter configuration for channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    /// Minimum severity level to send
    pub min_severity: Option<String>,
    /// Maximum severity level to send
    pub max_severity: Option<String>,
    /// Source filters (regex patterns)
    pub source_filters: Vec<String>,
    /// Label filters
    pub label_filters: HashMap<String, String>,
    /// Time-based filters
    pub time_filters: Vec<TimeFilter>,
}

/// Time-based filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeFilter {
    /// Days of week (0 = Sunday, 6 = Saturday)
    pub days_of_week: Vec<u8>,
    /// Start time (HH:MM format)
    pub start_time: String,
    /// End time (HH:MM format)
    pub end_time: String,
    /// Timezone
    pub timezone: String,
}

/// Notification channel trait
#[async_trait]
pub trait NotificationChannel: Send + Sync + fmt::Debug {
    /// Get channel type
    fn channel_type(&self) -> ChannelType;

    /// Get channel name
    fn name(&self) -> &str;

    /// Check if channel should send this alert
    async fn should_send_alert(&self, alert: &Alert) -> bool;

    /// Send alert notification
    async fn send_alert(&self, alert: &Alert) -> Result<DeliveryResult>;

    /// Check if channel should send resolution notification
    async fn should_send_resolution(&self, alert: &Alert) -> bool;

    /// Send resolution notification
    async fn send_resolution(&self, alert: &Alert) -> Result<DeliveryResult>;

    /// Test channel connectivity
    async fn test_connection(&self) -> Result<()>;

    /// Get channel health status
    async fn health_check(&self) -> Result<ChannelHealth>;
}

/// Channel health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelHealth {
    /// Whether channel is healthy
    pub healthy: bool,
    /// Last successful delivery timestamp
    pub last_success: Option<chrono::DateTime<chrono::Utc>>,
    /// Last failure timestamp
    pub last_failure: Option<chrono::DateTime<chrono::Utc>>,
    /// Consecutive failure count
    pub consecutive_failures: u32,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    /// Average delivery time in milliseconds
    pub avg_delivery_time_ms: f64,
}

/// Notification channel enum for concrete types
#[derive(Debug)]
pub enum NotificationChannelImpl {
    Webhook(WebhookChannel),
    Slack(SlackChannel),
}

impl NotificationChannelImpl {
    /// Get channel type
    pub fn channel_type(&self) -> ChannelType {
        match self {
            Self::Webhook(channel) => channel.channel_type(),
            Self::Slack(channel) => channel.channel_type(),
        }
    }

    /// Get channel name
    pub fn name(&self) -> &str {
        match self {
            Self::Webhook(channel) => channel.name(),
            Self::Slack(channel) => channel.name(),
        }
    }

    /// Check if channel should send this alert
    pub async fn should_send_alert(&self, alert: &Alert) -> bool {
        match self {
            Self::Webhook(channel) => channel.should_send_alert(alert).await,
            Self::Slack(channel) => channel.should_send_alert(alert).await,
        }
    }

    /// Send alert notification
    pub async fn send_alert(&self, alert: &Alert) -> Result<DeliveryResult> {
        match self {
            Self::Webhook(channel) => channel.send_alert(alert).await,
            Self::Slack(channel) => channel.send_alert(alert).await,
        }
    }

    /// Check if channel should send resolution notification
    pub async fn should_send_resolution(&self, alert: &Alert) -> bool {
        match self {
            Self::Webhook(channel) => channel.should_send_resolution(alert).await,
            Self::Slack(channel) => channel.should_send_resolution(alert).await,
        }
    }

    /// Send resolution notification
    pub async fn send_resolution(&self, alert: &Alert) -> Result<DeliveryResult> {
        match self {
            Self::Webhook(channel) => channel.send_resolution(alert).await,
            Self::Slack(channel) => channel.send_resolution(alert).await,
        }
    }

    /// Test channel connectivity
    pub async fn test_connection(&self) -> Result<()> {
        match self {
            Self::Webhook(channel) => channel.test_connection().await,
            Self::Slack(channel) => channel.test_connection().await,
        }
    }

    /// Get channel health status
    pub async fn health_check(&self) -> Result<ChannelHealth> {
        match self {
            Self::Webhook(channel) => channel.health_check().await,
            Self::Slack(channel) => channel.health_check().await,
        }
    }
}

/// Create a notification channel from configuration
pub async fn create_channel(config: &ChannelConfig) -> Result<NotificationChannelImpl> {
    info!("Creating notification channel: {} ({})", config.name, config.channel_type);

    match &config.channel_type {
        ChannelType::Webhook => {
            Ok(NotificationChannelImpl::Webhook(WebhookChannel::new(config).await?))
        }
        ChannelType::Slack => {
            Ok(NotificationChannelImpl::Slack(SlackChannel::new(config).await?))
        }
        ChannelType::Custom(name) => {
            warn!("Custom channel type not implemented: {}", name);
            Err(AlertingError::config(format!("Custom channel type not implemented: {}", name)))
        }
        _ => {
            warn!("Channel type not implemented: {:?}", config.channel_type);
            Err(AlertingError::config(format!("Channel type not implemented: {:?}", config.channel_type)))
        }
    }
}

/// Webhook notification channel
#[derive(Debug)]
pub struct WebhookChannel {
    name: String,
    url: String,
    headers: HashMap<String, String>,
    timeout_seconds: u64,
    filter: FilterConfig,
    client: reqwest::Client,
}

impl WebhookChannel {
    pub async fn new(config: &ChannelConfig) -> Result<Self> {
        let url = config.config.get("url")
            .ok_or_else(|| AlertingError::config("Webhook URL not configured"))?
            .clone();

        let timeout_seconds = config.config.get("timeout")
            .and_then(|t| t.parse().ok())
            .unwrap_or(30);

        let mut headers = HashMap::new();
        for (key, value) in &config.config {
            if key.starts_with("header_") {
                let header_name = key.strip_prefix("header_").unwrap();
                headers.insert(header_name.to_string(), value.clone());
            }
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_seconds))
            .build()
            .map_err(|e| AlertingError::network(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            name: config.name.clone(),
            url,
            headers,
            timeout_seconds,
            filter: config.filter.clone(),
            client,
        })
    }
}

#[async_trait]
impl NotificationChannel for WebhookChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Webhook
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn should_send_alert(&self, _alert: &Alert) -> bool {
        // TODO: Implement filtering logic
        true
    }

    async fn send_alert(&self, alert: &Alert) -> Result<DeliveryResult> {
        let start_time = std::time::Instant::now();

        debug!("Sending webhook notification for alert: {}", alert.id);

        let payload = serde_json::json!({
            "alert_id": alert.id,
            "title": alert.title,
            "description": alert.description,
            "severity": alert.severity,
            "status": alert.status,
            "source": alert.source,
            "created_at": alert.created_at,
            "labels": alert.context.labels,
            "annotations": alert.context.annotations
        });

        let mut request = self.client.post(&self.url)
            .json(&payload);

        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        match request.send().await {
            Ok(response) => {
                let duration = start_time.elapsed();
                let success = response.status().is_success();

                if success {
                    info!("Webhook notification sent successfully: {}", alert.id);
                } else {
                    error!("Webhook notification failed with status: {}", response.status());
                }

                Ok(DeliveryResult {
                    channel_type: ChannelType::Webhook,
                    success,
                    timestamp: chrono::Utc::now(),
                    duration_ms: duration.as_millis() as u64,
                    error: if success { None } else { Some(format!("HTTP {}", response.status())) },
                    metadata: HashMap::new(),
                })
            }
            Err(e) => {
                let duration = start_time.elapsed();
                error!("Webhook notification failed: {}", e);

                Ok(DeliveryResult {
                    channel_type: ChannelType::Webhook,
                    success: false,
                    timestamp: chrono::Utc::now(),
                    duration_ms: duration.as_millis() as u64,
                    error: Some(e.to_string()),
                    metadata: HashMap::new(),
                })
            }
        }
    }

    async fn should_send_resolution(&self, _alert: &Alert) -> bool {
        true
    }

    async fn send_resolution(&self, alert: &Alert) -> Result<DeliveryResult> {
        // Similar to send_alert but with resolution payload
        self.send_alert(alert).await
    }

    async fn test_connection(&self) -> Result<()> {
        let test_payload = serde_json::json!({
            "test": true,
            "timestamp": chrono::Utc::now()
        });

        let response = self.client.post(&self.url)
            .json(&test_payload)
            .send()
            .await
            .map_err(|e| AlertingError::network(format!("Connection test failed: {}", e)))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(AlertingError::network(format!("Connection test failed with status: {}", response.status())))
        }
    }

    async fn health_check(&self) -> Result<ChannelHealth> {
        // TODO: Implement proper health checking with metrics
        Ok(ChannelHealth {
            healthy: true,
            last_success: Some(chrono::Utc::now()),
            last_failure: None,
            consecutive_failures: 0,
            success_rate: 1.0,
            avg_delivery_time_ms: 100.0,
        })
    }
}

/// Slack notification channel
#[derive(Debug)]
pub struct SlackChannel {
    name: String,
    webhook_url: String,
    channel: Option<String>,
    username: Option<String>,
    filter: FilterConfig,
    client: reqwest::Client,
}

impl SlackChannel {
    pub async fn new(config: &ChannelConfig) -> Result<Self> {
        let webhook_url = config.config.get("webhook_url")
            .ok_or_else(|| AlertingError::config("Slack webhook URL not configured"))?
            .clone();

        let channel = config.config.get("channel").cloned();
        let username = config.config.get("username").cloned();

        let client = reqwest::Client::new();

        Ok(Self {
            name: config.name.clone(),
            webhook_url,
            channel,
            username,
            filter: config.filter.clone(),
            client,
        })
    }
}

#[async_trait]
impl NotificationChannel for SlackChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Slack
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn should_send_alert(&self, _alert: &Alert) -> bool {
        true
    }

    async fn send_alert(&self, alert: &Alert) -> Result<DeliveryResult> {
        let start_time = std::time::Instant::now();

        let color = match alert.severity {
            crate::alert::AlertSeverity::Critical => "danger",
            crate::alert::AlertSeverity::High => "warning",
            crate::alert::AlertSeverity::Medium => "warning",
            crate::alert::AlertSeverity::Warning => "warning",
            crate::alert::AlertSeverity::Info => "good",
        };

        let mut payload = serde_json::json!({
            "attachments": [{
                "color": color,
                "title": alert.title,
                "text": alert.description,
                "fields": [
                    {
                        "title": "Severity",
                        "value": alert.severity.to_string(),
                        "short": true
                    },
                    {
                        "title": "Source",
                        "value": alert.source,
                        "short": true
                    }
                ],
                "ts": alert.created_at.timestamp()
            }]
        });

        if let Some(channel) = &self.channel {
            payload["channel"] = serde_json::Value::String(channel.clone());
        }

        if let Some(username) = &self.username {
            payload["username"] = serde_json::Value::String(username.clone());
        }

        match self.client.post(&self.webhook_url)
            .json(&payload)
            .send()
            .await
        {
            Ok(response) => {
                let duration = start_time.elapsed();
                let success = response.status().is_success();

                Ok(DeliveryResult {
                    channel_type: ChannelType::Slack,
                    success,
                    timestamp: chrono::Utc::now(),
                    duration_ms: duration.as_millis() as u64,
                    error: if success { None } else { Some(format!("HTTP {}", response.status())) },
                    metadata: HashMap::new(),
                })
            }
            Err(e) => {
                let duration = start_time.elapsed();

                Ok(DeliveryResult {
                    channel_type: ChannelType::Slack,
                    success: false,
                    timestamp: chrono::Utc::now(),
                    duration_ms: duration.as_millis() as u64,
                    error: Some(e.to_string()),
                    metadata: HashMap::new(),
                })
            }
        }
    }

    async fn should_send_resolution(&self, _alert: &Alert) -> bool {
        true
    }

    async fn send_resolution(&self, alert: &Alert) -> Result<DeliveryResult> {
        // Similar implementation for resolution notifications
        self.send_alert(alert).await
    }

    async fn test_connection(&self) -> Result<()> {
        let test_payload = serde_json::json!({
            "text": "Test message from Stalwart Alerting"
        });

        let response = self.client.post(&self.webhook_url)
            .json(&test_payload)
            .send()
            .await
            .map_err(|e| AlertingError::network(format!("Connection test failed: {}", e)))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(AlertingError::network(format!("Connection test failed with status: {}", response.status())))
        }
    }

    async fn health_check(&self) -> Result<ChannelHealth> {
        Ok(ChannelHealth {
            healthy: true,
            last_success: Some(chrono::Utc::now()),
            last_failure: None,
            consecutive_failures: 0,
            success_rate: 1.0,
            avg_delivery_time_ms: 150.0,
        })
    }
}

impl fmt::Display for ChannelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Email => write!(f, "email"),
            Self::Sms => write!(f, "sms"),
            Self::Slack => write!(f, "slack"),
            Self::Discord => write!(f, "discord"),
            Self::Telegram => write!(f, "telegram"),
            Self::Teams => write!(f, "teams"),
            Self::PagerDuty => write!(f, "pagerduty"),
            Self::Webhook => write!(f, "webhook"),
            Self::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_seconds: 1,
            max_delay_seconds: 300,
            backoff_multiplier: 2.0,
            exponential_backoff: true,
        }
    }
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            min_severity: None,
            max_severity: None,
            source_filters: Vec::new(),
            label_filters: HashMap::new(),
            time_filters: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_type_display() {
        assert_eq!(ChannelType::Email.to_string(), "email");
        assert_eq!(ChannelType::Slack.to_string(), "slack");
        assert_eq!(ChannelType::Custom("test".to_string()).to_string(), "custom:test");
    }

    #[test]
    fn test_delivery_result_creation() {
        let result = DeliveryResult {
            channel_type: ChannelType::Webhook,
            success: true,
            timestamp: chrono::Utc::now(),
            duration_ms: 100,
            error: None,
            metadata: HashMap::new(),
        };

        assert!(result.success);
        assert_eq!(result.duration_ms, 100);
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert!(config.exponential_backoff);
    }
}
