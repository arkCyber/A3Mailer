//! Alerting System for A3Mailer
//!
//! This module provides comprehensive alerting capabilities with multiple
//! notification channels, alert rules, and escalation policies.

use crate::{MonitoringConfig, Result, MonitoringError};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Alert severity levels
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl AlertSeverity {
    /// Get numeric priority for sorting
    pub fn priority(&self) -> u8 {
        match self {
            AlertSeverity::Critical => 5,
            AlertSeverity::High => 4,
            AlertSeverity::Medium => 3,
            AlertSeverity::Low => 2,
            AlertSeverity::Info => 1,
        }
    }
}

/// Alert status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertStatus {
    Firing,
    Resolved,
    Acknowledged,
    Suppressed,
}

/// Alert rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub metric_name: String,
    pub condition: AlertCondition,
    pub threshold: f64,
    pub duration_seconds: u64,
    pub severity: AlertSeverity,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub enabled: bool,
}

/// Alert condition types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
    GreaterThanOrEqual,
    LessThanOrEqual,
    PercentageIncrease,
    PercentageDecrease,
}

impl AlertCondition {
    /// Evaluate condition against current and threshold values
    pub fn evaluate(&self, current: f64, threshold: f64, previous: Option<f64>) -> bool {
        match self {
            AlertCondition::GreaterThan => current > threshold,
            AlertCondition::LessThan => current < threshold,
            AlertCondition::Equal => (current - threshold).abs() < f64::EPSILON,
            AlertCondition::NotEqual => (current - threshold).abs() > f64::EPSILON,
            AlertCondition::GreaterThanOrEqual => current >= threshold,
            AlertCondition::LessThanOrEqual => current <= threshold,
            AlertCondition::PercentageIncrease => {
                if let Some(prev) = previous {
                    if prev > 0.0 {
                        let increase = ((current - prev) / prev) * 100.0;
                        increase > threshold
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            AlertCondition::PercentageDecrease => {
                if let Some(prev) = previous {
                    if prev > 0.0 {
                        let decrease = ((prev - current) / prev) * 100.0;
                        decrease > threshold
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }
}

/// Alert instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub rule_id: String,
    pub name: String,
    pub description: String,
    pub severity: AlertSeverity,
    pub status: AlertStatus,
    pub current_value: f64,
    pub threshold: f64,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub fired_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<String>,
    pub escalation_level: u32,
    pub notification_count: u32,
}

impl Alert {
    /// Create a new alert
    pub fn new(rule: &AlertRule, current_value: f64) -> Self {
        Self {
            id: format!("alert_{}", uuid::Uuid::new_v4()),
            rule_id: rule.id.clone(),
            name: rule.name.clone(),
            description: rule.description.clone(),
            severity: rule.severity.clone(),
            status: AlertStatus::Firing,
            current_value,
            threshold: rule.threshold,
            labels: rule.labels.clone(),
            annotations: rule.annotations.clone(),
            fired_at: Utc::now(),
            resolved_at: None,
            acknowledged_at: None,
            acknowledged_by: None,
            escalation_level: 0,
            notification_count: 0,
        }
    }

    /// Mark alert as resolved
    pub fn resolve(&mut self) {
        self.status = AlertStatus::Resolved;
        self.resolved_at = Some(Utc::now());
    }

    /// Acknowledge alert
    pub fn acknowledge(&mut self, acknowledged_by: String) {
        self.status = AlertStatus::Acknowledged;
        self.acknowledged_at = Some(Utc::now());
        self.acknowledged_by = Some(acknowledged_by);
    }

    /// Get alert duration
    pub fn duration(&self) -> Duration {
        let end_time = self.resolved_at.unwrap_or_else(Utc::now);
        let duration = end_time.signed_duration_since(self.fired_at);
        Duration::from_secs(duration.num_seconds().max(0) as u64)
    }
}

/// Notification channel types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email {
        recipients: Vec<String>,
        smtp_config: EmailConfig,
    },
    Slack {
        webhook_url: String,
        channel: String,
    },
    Discord {
        webhook_url: String,
    },
    PagerDuty {
        integration_key: String,
    },
    Webhook {
        url: String,
        headers: HashMap<String, String>,
    },
    SMS {
        provider: String,
        recipients: Vec<String>,
        api_key: String,
    },
}

/// Email configuration for notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub use_tls: bool,
}

/// Escalation policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationPolicy {
    pub id: String,
    pub name: String,
    pub levels: Vec<EscalationLevel>,
    pub repeat_interval_minutes: u64,
    pub max_escalations: u32,
}

/// Escalation level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationLevel {
    pub level: u32,
    pub delay_minutes: u64,
    pub channels: Vec<String>, // Channel IDs
    pub conditions: Vec<EscalationCondition>,
}

/// Escalation conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EscalationCondition {
    TimeElapsed(u64), // minutes
    SeverityLevel(AlertSeverity),
    NotAcknowledged,
    RepeatCount(u32),
}

/// Alert manager
pub struct AlertManager {
    config: MonitoringConfig,
    rules: Arc<RwLock<HashMap<String, AlertRule>>>,
    active_alerts: Arc<RwLock<HashMap<String, Alert>>>,
    alert_history: Arc<RwLock<Vec<Alert>>>,
    notification_channels: Arc<RwLock<HashMap<String, NotificationChannel>>>,
    escalation_policies: Arc<RwLock<HashMap<String, EscalationPolicy>>>,
    metrics: Arc<RwLock<AlertMetrics>>,
}

/// Alert metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertMetrics {
    pub total_alerts: u64,
    pub active_alerts: u64,
    pub resolved_alerts: u64,
    pub acknowledged_alerts: u64,
    pub alerts_by_severity: HashMap<AlertSeverity, u64>,
    pub average_resolution_time_minutes: f64,
    pub notification_success_rate: f64,
    pub escalation_rate: f64,
}

impl AlertManager {
    /// Create a new alert manager
    pub async fn new(config: &MonitoringConfig) -> Result<Self> {
        info!("Initializing alert manager");

        let alert_manager = Self {
            config: config.clone(),
            rules: Arc::new(RwLock::new(HashMap::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(Vec::new())),
            notification_channels: Arc::new(RwLock::new(HashMap::new())),
            escalation_policies: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(AlertMetrics {
                total_alerts: 0,
                active_alerts: 0,
                resolved_alerts: 0,
                acknowledged_alerts: 0,
                alerts_by_severity: HashMap::new(),
                average_resolution_time_minutes: 0.0,
                notification_success_rate: 100.0,
                escalation_rate: 0.0,
            })),
        };

        // Initialize default rules and channels
        alert_manager.initialize_defaults().await?;

        info!("Alert manager initialized successfully");
        Ok(alert_manager)
    }

    /// Initialize default alert rules and notification channels
    async fn initialize_defaults(&self) -> Result<()> {
        debug!("Initializing default alert rules and channels");

        // Add default alert rules
        self.add_default_rules().await?;
        
        // Add default notification channels
        self.add_default_channels().await?;
        
        // Add default escalation policies
        self.add_default_escalation_policies().await?;

        Ok(())
    }

    /// Add default alert rules
    async fn add_default_rules(&self) -> Result<()> {
        let mut rules = self.rules.write().await;

        // High memory usage alert
        rules.insert("high_memory_usage".to_string(), AlertRule {
            id: "high_memory_usage".to_string(),
            name: "High Memory Usage".to_string(),
            description: "Memory usage is above 80%".to_string(),
            metric_name: "memory_usage_percent".to_string(),
            condition: AlertCondition::GreaterThan,
            threshold: 80.0,
            duration_seconds: 300, // 5 minutes
            severity: AlertSeverity::High,
            labels: [("component".to_string(), "system".to_string())].iter().cloned().collect(),
            annotations: [("runbook".to_string(), "https://docs.a3mailer.com/alerts/memory".to_string())].iter().cloned().collect(),
            enabled: true,
        });

        // Low cache hit rate alert
        rules.insert("low_cache_hit_rate".to_string(), AlertRule {
            id: "low_cache_hit_rate".to_string(),
            name: "Low Cache Hit Rate".to_string(),
            description: "Cache hit rate is below 70%".to_string(),
            metric_name: "cache_hit_rate".to_string(),
            condition: AlertCondition::LessThan,
            threshold: 0.7,
            duration_seconds: 600, // 10 minutes
            severity: AlertSeverity::Medium,
            labels: [("component".to_string(), "cache".to_string())].iter().cloned().collect(),
            annotations: [("runbook".to_string(), "https://docs.a3mailer.com/alerts/cache".to_string())].iter().cloned().collect(),
            enabled: true,
        });

        // High error rate alert
        rules.insert("high_error_rate".to_string(), AlertRule {
            id: "high_error_rate".to_string(),
            name: "High Error Rate".to_string(),
            description: "Error rate is above 5%".to_string(),
            metric_name: "error_rate_percent".to_string(),
            condition: AlertCondition::GreaterThan,
            threshold: 5.0,
            duration_seconds: 180, // 3 minutes
            severity: AlertSeverity::Critical,
            labels: [("component".to_string(), "application".to_string())].iter().cloned().collect(),
            annotations: [("runbook".to_string(), "https://docs.a3mailer.com/alerts/errors".to_string())].iter().cloned().collect(),
            enabled: true,
        });

        // AI inference latency alert
        rules.insert("high_ai_latency".to_string(), AlertRule {
            id: "high_ai_latency".to_string(),
            name: "High AI Inference Latency".to_string(),
            description: "AI inference taking longer than 50ms".to_string(),
            metric_name: "ai_inference_duration_ms".to_string(),
            condition: AlertCondition::GreaterThan,
            threshold: 50.0,
            duration_seconds: 300, // 5 minutes
            severity: AlertSeverity::Medium,
            labels: [("component".to_string(), "ai".to_string())].iter().cloned().collect(),
            annotations: [("runbook".to_string(), "https://docs.a3mailer.com/alerts/ai".to_string())].iter().cloned().collect(),
            enabled: true,
        });

        // Web3 operation failure alert
        rules.insert("web3_operation_failures".to_string(), AlertRule {
            id: "web3_operation_failures".to_string(),
            name: "Web3 Operation Failures".to_string(),
            description: "Web3 operations failing at high rate".to_string(),
            metric_name: "web3_failure_rate_percent".to_string(),
            condition: AlertCondition::GreaterThan,
            threshold: 10.0,
            duration_seconds: 300, // 5 minutes
            severity: AlertSeverity::High,
            labels: [("component".to_string(), "web3".to_string())].iter().cloned().collect(),
            annotations: [("runbook".to_string(), "https://docs.a3mailer.com/alerts/web3".to_string())].iter().cloned().collect(),
            enabled: true,
        });

        info!("Added {} default alert rules", rules.len());
        Ok(())
    }

    /// Add default notification channels
    async fn add_default_channels(&self) -> Result<()> {
        let mut channels = self.notification_channels.write().await;

        // Email channel
        channels.insert("email_ops".to_string(), NotificationChannel::Email {
            recipients: vec!["ops@a3mailer.com".to_string()],
            smtp_config: EmailConfig {
                smtp_server: "smtp.a3mailer.com".to_string(),
                smtp_port: 587,
                username: "alerts@a3mailer.com".to_string(),
                password: "alert_password".to_string(),
                from_address: "alerts@a3mailer.com".to_string(),
                use_tls: true,
            },
        });

        // Slack channel
        channels.insert("slack_alerts".to_string(), NotificationChannel::Slack {
            webhook_url: "https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK".to_string(),
            channel: "#alerts".to_string(),
        });

        // Webhook channel
        channels.insert("webhook_monitoring".to_string(), NotificationChannel::Webhook {
            url: "https://monitoring.a3mailer.com/alerts".to_string(),
            headers: [("Authorization".to_string(), "Bearer token".to_string())].iter().cloned().collect(),
        });

        info!("Added {} default notification channels", channels.len());
        Ok(())
    }

    /// Add default escalation policies
    async fn add_default_escalation_policies(&self) -> Result<()> {
        let mut policies = self.escalation_policies.write().await;

        // Standard escalation policy
        policies.insert("standard".to_string(), EscalationPolicy {
            id: "standard".to_string(),
            name: "Standard Escalation".to_string(),
            levels: vec![
                EscalationLevel {
                    level: 1,
                    delay_minutes: 0,
                    channels: vec!["slack_alerts".to_string()],
                    conditions: vec![EscalationCondition::SeverityLevel(AlertSeverity::Medium)],
                },
                EscalationLevel {
                    level: 2,
                    delay_minutes: 15,
                    channels: vec!["email_ops".to_string()],
                    conditions: vec![EscalationCondition::NotAcknowledged],
                },
                EscalationLevel {
                    level: 3,
                    delay_minutes: 30,
                    channels: vec!["webhook_monitoring".to_string()],
                    conditions: vec![EscalationCondition::TimeElapsed(30)],
                },
            ],
            repeat_interval_minutes: 60,
            max_escalations: 3,
        });

        // Critical escalation policy
        policies.insert("critical".to_string(), EscalationPolicy {
            id: "critical".to_string(),
            name: "Critical Escalation".to_string(),
            levels: vec![
                EscalationLevel {
                    level: 1,
                    delay_minutes: 0,
                    channels: vec!["slack_alerts".to_string(), "email_ops".to_string()],
                    conditions: vec![EscalationCondition::SeverityLevel(AlertSeverity::Critical)],
                },
                EscalationLevel {
                    level: 2,
                    delay_minutes: 5,
                    channels: vec!["webhook_monitoring".to_string()],
                    conditions: vec![EscalationCondition::NotAcknowledged],
                },
            ],
            repeat_interval_minutes: 15,
            max_escalations: 10,
        });

        info!("Added {} default escalation policies", policies.len());
        Ok(())
    }

    /// Evaluate metric against alert rules
    pub async fn evaluate_metric(&self, metric_name: &str, current_value: f64, previous_value: Option<f64>) -> Result<()> {
        let rules = self.rules.read().await;
        
        for rule in rules.values() {
            if rule.enabled && rule.metric_name == metric_name {
                let condition_met = rule.condition.evaluate(current_value, rule.threshold, previous_value);
                
                if condition_met {
                    self.handle_alert_condition(rule, current_value).await?;
                } else {
                    self.handle_alert_resolution(rule).await?;
                }
            }
        }
        
        Ok(())
    }

    /// Handle alert condition being met
    async fn handle_alert_condition(&self, rule: &AlertRule, current_value: f64) -> Result<()> {
        let mut active_alerts = self.active_alerts.write().await;
        
        // Check if alert already exists
        if !active_alerts.contains_key(&rule.id) {
            let alert = Alert::new(rule, current_value);
            info!("Firing new alert: {} ({})", alert.name, alert.severity.priority());
            
            // Send notifications
            self.send_alert_notifications(&alert).await?;
            
            active_alerts.insert(rule.id.clone(), alert);
            
            // Update metrics
            self.update_alert_metrics().await;
        }
        
        Ok(())
    }

    /// Handle alert resolution
    async fn handle_alert_resolution(&self, rule: &AlertRule) -> Result<()> {
        let mut active_alerts = self.active_alerts.write().await;
        
        if let Some(mut alert) = active_alerts.remove(&rule.id) {
            alert.resolve();
            info!("Resolved alert: {} (duration: {:?})", alert.name, alert.duration());
            
            // Send resolution notification
            self.send_resolution_notification(&alert).await?;
            
            // Move to history
            let mut history = self.alert_history.write().await;
            history.push(alert);
            
            // Update metrics
            self.update_alert_metrics().await;
        }
        
        Ok(())
    }

    /// Send alert notifications
    async fn send_alert_notifications(&self, alert: &Alert) -> Result<()> {
        debug!("Sending notifications for alert: {}", alert.name);
        
        let channels = self.notification_channels.read().await;
        
        // For now, we'll simulate sending notifications
        // In a real implementation, this would send actual notifications
        for (channel_id, channel) in channels.iter() {
            match self.send_notification(channel, alert, false).await {
                Ok(_) => debug!("Sent notification via channel: {}", channel_id),
                Err(e) => warn!("Failed to send notification via {}: {}", channel_id, e),
            }
        }
        
        Ok(())
    }

    /// Send resolution notification
    async fn send_resolution_notification(&self, alert: &Alert) -> Result<()> {
        debug!("Sending resolution notification for alert: {}", alert.name);
        
        let channels = self.notification_channels.read().await;
        
        for (channel_id, channel) in channels.iter() {
            match self.send_notification(channel, alert, true).await {
                Ok(_) => debug!("Sent resolution notification via channel: {}", channel_id),
                Err(e) => warn!("Failed to send resolution notification via {}: {}", channel_id, e),
            }
        }
        
        Ok(())
    }

    /// Send notification via specific channel
    async fn send_notification(&self, channel: &NotificationChannel, alert: &Alert, is_resolution: bool) -> Result<()> {
        match channel {
            NotificationChannel::Email { recipients, smtp_config } => {
                self.send_email_notification(recipients, smtp_config, alert, is_resolution).await
            }
            NotificationChannel::Slack { webhook_url, channel } => {
                self.send_slack_notification(webhook_url, channel, alert, is_resolution).await
            }
            NotificationChannel::Discord { webhook_url } => {
                self.send_discord_notification(webhook_url, alert, is_resolution).await
            }
            NotificationChannel::Webhook { url, headers } => {
                self.send_webhook_notification(url, headers, alert, is_resolution).await
            }
            _ => {
                debug!("Notification channel not implemented yet");
                Ok(())
            }
        }
    }

    /// Send email notification
    async fn send_email_notification(&self, _recipients: &[String], _smtp_config: &EmailConfig, alert: &Alert, is_resolution: bool) -> Result<()> {
        // Simulate email sending
        let action = if is_resolution { "RESOLVED" } else { "FIRING" };
        debug!("EMAIL: [{}] {} - {}", action, alert.name, alert.description);
        Ok(())
    }

    /// Send Slack notification
    async fn send_slack_notification(&self, _webhook_url: &str, _channel: &str, alert: &Alert, is_resolution: bool) -> Result<()> {
        // Simulate Slack notification
        let action = if is_resolution { "✅ RESOLVED" } else { "🚨 FIRING" };
        debug!("SLACK: {} {} - {}", action, alert.name, alert.description);
        Ok(())
    }

    /// Send Discord notification
    async fn send_discord_notification(&self, _webhook_url: &str, alert: &Alert, is_resolution: bool) -> Result<()> {
        // Simulate Discord notification
        let action = if is_resolution { "✅ RESOLVED" } else { "🚨 FIRING" };
        debug!("DISCORD: {} {} - {}", action, alert.name, alert.description);
        Ok(())
    }

    /// Send webhook notification
    async fn send_webhook_notification(&self, _url: &str, _headers: &HashMap<String, String>, alert: &Alert, is_resolution: bool) -> Result<()> {
        // Simulate webhook notification
        let action = if is_resolution { "resolved" } else { "firing" };
        debug!("WEBHOOK: {} {} - {}", action, alert.name, alert.description);
        Ok(())
    }

    /// Update alert metrics
    async fn update_alert_metrics(&self) {
        let active_alerts = self.active_alerts.read().await;
        let history = self.alert_history.read().await;
        let mut metrics = self.metrics.write().await;
        
        metrics.active_alerts = active_alerts.len() as u64;
        metrics.resolved_alerts = history.len() as u64;
        metrics.total_alerts = metrics.active_alerts + metrics.resolved_alerts;
        
        // Calculate average resolution time
        if !history.is_empty() {
            let total_duration: Duration = history.iter()
                .map(|alert| alert.duration())
                .sum();
            metrics.average_resolution_time_minutes = total_duration.as_secs() as f64 / 60.0 / history.len() as f64;
        }
        
        // Update alerts by severity
        metrics.alerts_by_severity.clear();
        for alert in active_alerts.values() {
            *metrics.alerts_by_severity.entry(alert.severity.clone()).or_insert(0) += 1;
        }
    }

    /// Get alert statistics
    pub async fn get_alert_metrics(&self) -> AlertMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Acknowledge alert
    pub async fn acknowledge_alert(&self, alert_id: &str, acknowledged_by: String) -> Result<bool> {
        let mut active_alerts = self.active_alerts.write().await;
        
        if let Some(alert) = active_alerts.get_mut(alert_id) {
            alert.acknowledge(acknowledged_by);
            info!("Alert acknowledged: {}", alert.name);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let active_alerts = self.active_alerts.read().await;
        active_alerts.values().cloned().collect()
    }

    /// Shutdown alert manager
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down alert manager");
        
        // Resolve all active alerts
        let mut active_alerts = self.active_alerts.write().await;
        for alert in active_alerts.values_mut() {
            alert.resolve();
        }
        active_alerts.clear();
        
        info!("Alert manager shutdown complete");
        Ok(())
    }
}
