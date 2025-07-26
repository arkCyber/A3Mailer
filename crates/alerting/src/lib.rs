/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! # Intelligent Alerting System
//!
//! Comprehensive alerting system for Stalwart Mail Server with support for:
//!
//! - Multiple notification channels (email, SMS, Slack, Telegram, webhooks)
//! - Alert aggregation and deduplication
//! - Escalation policies and on-call schedules
//! - Template-based alert formatting
//! - Alert suppression and maintenance windows
//! - Real-time alert dashboard and API

pub mod alert;
pub mod channels;
pub mod config;
pub mod engine;
pub mod error;
pub mod escalation;
pub mod metrics;
pub mod rules;
pub mod templates;
pub mod suppression;

pub use alert::{Alert, AlertSeverity, AlertStatus, AlertContext};
pub use channels::{NotificationChannelImpl, ChannelType, DeliveryResult, ChannelConfig};
pub use config::AlertingConfig;
pub use engine::AlertingEngine;
pub use error::{AlertingError, Result};
pub use escalation::{EscalationPolicy, EscalationLevel};
pub use metrics::AlertingMetrics;
pub use rules::{AlertRule, RuleCondition, RuleAction};
pub use templates::{AlertTemplate, TemplateEngine};

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use uuid::Uuid;

/// Main alerting service
#[derive(Debug, Clone)]
pub struct AlertingService {
    inner: Arc<AlertingServiceInner>,
}

#[derive(Debug)]
struct AlertingServiceInner {
    config: AlertingConfig,
    engine: AlertingEngine,
    channels: Vec<NotificationChannelImpl>,
    template_engine: TemplateEngine,
    metrics: Arc<RwLock<AlertingMetrics>>,
    active_alerts: Arc<RwLock<std::collections::HashMap<Uuid, Alert>>>,
}

impl AlertingService {
    /// Create a new alerting service
    pub async fn new(config: AlertingConfig) -> Result<Self> {
        info!("Initializing alerting service");

        // Create alerting engine
        let engine = AlertingEngine::new(&config.engine).await?;

        // Initialize notification channels (convert from HashMap to ChannelConfig)
        let channel_configs: Vec<ChannelConfig> = config.channels.iter().map(|ch| {
            ChannelConfig {
                name: ch.get("name").unwrap_or(&"default".to_string()).clone(),
                channel_type: match ch.get("type").unwrap_or(&"webhook".to_string()).as_str() {
                    "webhook" => ChannelType::Webhook,
                    "slack" => ChannelType::Slack,
                    "email" => ChannelType::Email,
                    _ => ChannelType::Webhook,
                },
                config: ch.clone(),
                enabled: ch.get("enabled").and_then(|v| v.parse().ok()).unwrap_or(true),
                priority: ch.get("priority").and_then(|v| v.parse().ok()).unwrap_or(1),
                rate_limit: None,
                retry: channels::RetryConfig::default(),
                filter: channels::FilterConfig::default(),
            }
        }).collect();
        let channels = Self::initialize_channels(&channel_configs).await?;

        // Create template engine
        let template_engine = TemplateEngine::new(&config.templates).await?;

        // Create metrics collector
        let metrics = Arc::new(RwLock::new(AlertingMetrics::new()));

        // Initialize active alerts storage
        let active_alerts = Arc::new(RwLock::new(std::collections::HashMap::new()));

        Ok(Self {
            inner: Arc::new(AlertingServiceInner {
                config,
                engine,
                channels,
                template_engine,
                metrics,
                active_alerts,
            }),
        })
    }

    /// Start the alerting service
    pub async fn start(&self) -> Result<()> {
        info!("Starting alerting service");

        // Start the alerting engine
        self.inner.engine.start().await?;

        // Start metrics collection
        self.start_metrics_collection().await;

        // Start alert cleanup task
        self.start_alert_cleanup().await;

        info!("Alerting service started successfully");
        Ok(())
    }

    /// Stop the alerting service
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping alerting service");

        // Stop the alerting engine
        self.inner.engine.stop().await?;

        info!("Alerting service stopped");
        Ok(())
    }

    /// Fire an alert
    pub async fn fire_alert(&self, alert: Alert) -> Result<Uuid> {
        info!("Firing alert: {}", alert.title);

        // Check if alert should be suppressed
        if self.inner.engine.should_suppress(&alert).await? {
            info!("Alert suppressed: {}", alert.title);
            return Ok(alert.id);
        }

        // Check for existing similar alerts (deduplication)
        if let Some(existing_id) = self.find_similar_alert(&alert).await? {
            info!("Alert deduplicated with existing alert: {}", existing_id);
            self.update_alert_count(existing_id).await?;
            return Ok(existing_id);
        }

        // Store the alert
        {
            let mut active_alerts = self.inner.active_alerts.write().await;
            active_alerts.insert(alert.id, alert.clone());
        }

        // Process the alert through the engine
        self.inner.engine.process_alert(alert.clone()).await?;

        // Send notifications
        self.send_notifications(&alert).await?;

        // Update metrics
        {
            let mut metrics = self.inner.metrics.write().await;
            metrics.record_alert_fired(&alert);
        }

        info!("Alert fired successfully: {}", alert.id);
        Ok(alert.id)
    }

    /// Resolve an alert
    pub async fn resolve_alert(&self, alert_id: Uuid, resolution_note: Option<String>) -> Result<bool> {
        info!("Resolving alert: {}", alert_id);

        let mut resolved = false;

        // Update alert status
        {
            let mut active_alerts = self.inner.active_alerts.write().await;
            if let Some(alert) = active_alerts.get_mut(&alert_id) {
                alert.status = AlertStatus::Resolved;
                alert.resolved_at = Some(chrono::Utc::now());
                alert.resolution_note = resolution_note;
                resolved = true;
            }
        }

        if resolved {
            // Send resolution notifications
            if let Some(alert) = self.get_alert(alert_id).await? {
                self.send_resolution_notifications(&alert).await?;
            }

            // Update metrics
            {
                let mut metrics = self.inner.metrics.write().await;
                metrics.record_alert_resolved(alert_id);
            }

            info!("Alert resolved successfully: {}", alert_id);
        } else {
            warn!("Alert not found for resolution: {}", alert_id);
        }

        Ok(resolved)
    }

    /// Get an alert by ID
    pub async fn get_alert(&self, alert_id: Uuid) -> Result<Option<Alert>> {
        let active_alerts = self.inner.active_alerts.read().await;
        Ok(active_alerts.get(&alert_id).cloned())
    }

    /// List active alerts
    pub async fn list_active_alerts(&self) -> Result<Vec<Alert>> {
        let active_alerts = self.inner.active_alerts.read().await;
        Ok(active_alerts.values().cloned().collect())
    }

    /// Get alerting metrics
    pub async fn get_metrics(&self) -> AlertingMetrics {
        self.inner.metrics.read().await.clone()
    }

    /// Add a new alert rule
    pub async fn add_rule(&self, rule: AlertRule) -> Result<()> {
        info!("Adding alert rule: {}", rule.name);
        self.inner.engine.add_rule(rule).await
    }

    /// Remove an alert rule
    pub async fn remove_rule(&self, rule_name: &str) -> Result<bool> {
        info!("Removing alert rule: {}", rule_name);
        self.inner.engine.remove_rule(rule_name).await
    }

    /// Initialize notification channels from configuration
    async fn initialize_channels(
        channel_configs: &[ChannelConfig],
    ) -> Result<Vec<NotificationChannelImpl>> {
        let mut channels = Vec::new();

        for config in channel_configs {
            let channel = channels::create_channel(config).await?;
            channels.push(channel);
        }

        info!("Initialized {} notification channels", channels.len());
        Ok(channels)
    }

    /// Send notifications for an alert
    async fn send_notifications(&self, alert: &Alert) -> Result<()> {
        let mut delivery_results = Vec::new();

        for channel in &self.inner.channels {
            if channel.should_send_alert(alert).await {
                match channel.send_alert(alert).await {
                    Ok(result) => {
                        delivery_results.push(result);
                        info!("Alert sent via {}: {}", channel.channel_type(), alert.id);
                    }
                    Err(e) => {
                        error!("Failed to send alert via {}: {}", channel.channel_type(), e);
                    }
                }
            }
        }

        // Update metrics with delivery results
        {
            let mut metrics = self.inner.metrics.write().await;
            for result in delivery_results {
                metrics.record_notification_sent(&result);
            }
        }

        Ok(())
    }

    /// Send resolution notifications
    async fn send_resolution_notifications(&self, alert: &Alert) -> Result<()> {
        for channel in &self.inner.channels {
            if channel.should_send_resolution(alert).await {
                match channel.send_resolution(alert).await {
                    Ok(_) => {
                        info!("Resolution sent via {}: {}", channel.channel_type(), alert.id);
                    }
                    Err(e) => {
                        error!("Failed to send resolution via {}: {}", channel.channel_type(), e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Find similar alert for deduplication
    async fn find_similar_alert(&self, alert: &Alert) -> Result<Option<Uuid>> {
        let active_alerts = self.inner.active_alerts.read().await;

        for (id, existing_alert) in active_alerts.iter() {
            if self.alerts_are_similar(alert, existing_alert) {
                return Ok(Some(*id));
            }
        }

        Ok(None)
    }

    /// Check if two alerts are similar for deduplication
    fn alerts_are_similar(&self, alert1: &Alert, alert2: &Alert) -> bool {
        // Simple similarity check - can be made more sophisticated
        alert1.title == alert2.title &&
        alert1.severity == alert2.severity &&
        alert1.source == alert2.source
    }

    /// Update alert count for deduplicated alerts
    async fn update_alert_count(&self, alert_id: Uuid) -> Result<()> {
        let mut active_alerts = self.inner.active_alerts.write().await;
        if let Some(alert) = active_alerts.get_mut(&alert_id) {
            alert.count += 1;
            alert.last_occurrence = chrono::Utc::now();
        }
        Ok(())
    }

    /// Start metrics collection background task
    async fn start_metrics_collection(&self) {
        let metrics = self.inner.metrics.clone();
        let active_alerts = self.inner.active_alerts.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

            loop {
                interval.tick().await;

                // Update active alert count
                let active_count = active_alerts.read().await.len();

                {
                    let mut metrics_guard = metrics.write().await;
                    metrics_guard.update_active_alert_count(active_count);
                }
            }
        });
    }

    /// Start alert cleanup background task
    async fn start_alert_cleanup(&self) {
        let active_alerts = self.inner.active_alerts.clone();
        let cleanup_interval = self.inner.config.cleanup_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);

            loop {
                interval.tick().await;

                // Clean up resolved alerts older than retention period
                let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(24);

                {
                    let mut alerts = active_alerts.write().await;
                    alerts.retain(|_, alert| {
                        alert.status != AlertStatus::Resolved ||
                        alert.resolved_at.map_or(true, |t| t > cutoff_time)
                    });
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_creation() {
        let config = AlertingConfig::default();
        let service = AlertingService::new(config).await;
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_alert_lifecycle() {
        let config = AlertingConfig::default();
        let service = AlertingService::new(config).await.unwrap();

        // Create a test alert
        let alert = Alert::new(
            "Test Alert".to_string(),
            "This is a test alert".to_string(),
            AlertSeverity::Warning,
            "test_source".to_string(),
        );

        let alert_id = alert.id;

        // Fire the alert
        let fired_id = service.fire_alert(alert).await.unwrap();
        assert_eq!(fired_id, alert_id);

        // Check that alert is active
        let active_alerts = service.list_active_alerts().await.unwrap();
        assert_eq!(active_alerts.len(), 1);

        // Resolve the alert
        let resolved = service.resolve_alert(alert_id, Some("Test resolution".to_string())).await.unwrap();
        assert!(resolved);

        // Check alert status
        let alert = service.get_alert(alert_id).await.unwrap().unwrap();
        assert_eq!(alert.status, AlertStatus::Resolved);
    }
}
