/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Metrics collection and reporting for alerting system

use crate::alert::{Alert, AlertSeverity, AlertStatus};
use crate::channels::{ChannelType, DeliveryResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Alerting metrics collector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingMetrics {
    /// Total alerts fired
    pub alerts_fired_total: u64,
    /// Total alerts resolved
    pub alerts_resolved_total: u64,
    /// Alerts by severity
    pub alerts_by_severity: HashMap<String, u64>,
    /// Alerts by status
    pub alerts_by_status: HashMap<String, u64>,
    /// Alerts by source
    pub alerts_by_source: HashMap<String, u64>,
    /// Current active alerts count
    pub active_alerts_count: usize,
    /// Notification delivery metrics
    pub delivery_metrics: DeliveryMetrics,
    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,
    /// Error metrics
    pub error_metrics: ErrorMetrics,
    /// Last update timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Notification delivery metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryMetrics {
    /// Total notifications sent
    pub notifications_sent_total: u64,
    /// Successful deliveries
    pub successful_deliveries: u64,
    /// Failed deliveries
    pub failed_deliveries: u64,
    /// Deliveries by channel type
    pub deliveries_by_channel: HashMap<String, ChannelDeliveryMetrics>,
    /// Average delivery time in milliseconds
    pub avg_delivery_time_ms: f64,
    /// Delivery success rate (0.0 to 1.0)
    pub success_rate: f64,
}

/// Channel-specific delivery metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDeliveryMetrics {
    /// Total deliveries attempted
    pub total_attempts: u64,
    /// Successful deliveries
    pub successful: u64,
    /// Failed deliveries
    pub failed: u64,
    /// Average delivery time in milliseconds
    pub avg_delivery_time_ms: f64,
    /// Last successful delivery
    pub last_success: Option<chrono::DateTime<chrono::Utc>>,
    /// Last failed delivery
    pub last_failure: Option<chrono::DateTime<chrono::Utc>>,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average alert processing time in milliseconds
    pub avg_processing_time_ms: f64,
    /// Peak alerts per minute
    pub peak_alerts_per_minute: u64,
    /// Current alerts per minute
    pub current_alerts_per_minute: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
}

/// Error metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    /// Total errors
    pub total_errors: u64,
    /// Errors by category
    pub errors_by_category: HashMap<String, u64>,
    /// Recent errors
    pub recent_errors: Vec<ErrorRecord>,
    /// Error rate (errors per minute)
    pub error_rate: f64,
}

/// Error record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecord {
    /// Error timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Error category
    pub category: String,
    /// Error message
    pub message: String,
    /// Error context
    pub context: HashMap<String, String>,
}

impl AlertingMetrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            alerts_fired_total: 0,
            alerts_resolved_total: 0,
            alerts_by_severity: HashMap::new(),
            alerts_by_status: HashMap::new(),
            alerts_by_source: HashMap::new(),
            active_alerts_count: 0,
            delivery_metrics: DeliveryMetrics::new(),
            performance_metrics: PerformanceMetrics::new(),
            error_metrics: ErrorMetrics::new(),
            last_updated: chrono::Utc::now(),
        }
    }
    
    /// Record an alert being fired
    pub fn record_alert_fired(&mut self, alert: &Alert) {
        self.alerts_fired_total += 1;
        
        // Update severity metrics
        let severity_key = alert.severity.to_string();
        *self.alerts_by_severity.entry(severity_key).or_insert(0) += 1;
        
        // Update status metrics
        let status_key = alert.status.to_string();
        *self.alerts_by_status.entry(status_key).or_insert(0) += 1;
        
        // Update source metrics
        *self.alerts_by_source.entry(alert.source.clone()).or_insert(0) += 1;
        
        self.last_updated = chrono::Utc::now();
    }
    
    /// Record an alert being resolved
    pub fn record_alert_resolved(&mut self, _alert_id: Uuid) {
        self.alerts_resolved_total += 1;
        self.last_updated = chrono::Utc::now();
    }
    
    /// Record a notification being sent
    pub fn record_notification_sent(&mut self, result: &DeliveryResult) {
        self.delivery_metrics.record_delivery(result);
        self.last_updated = chrono::Utc::now();
    }
    
    /// Update active alert count
    pub fn update_active_alert_count(&mut self, count: usize) {
        self.active_alerts_count = count;
        self.last_updated = chrono::Utc::now();
    }
    
    /// Record an error
    pub fn record_error(&mut self, category: &str, message: &str, context: HashMap<String, String>) {
        self.error_metrics.record_error(category, message, context);
        self.last_updated = chrono::Utc::now();
    }
    
    /// Get alert success rate
    pub fn alert_success_rate(&self) -> f64 {
        if self.alerts_fired_total == 0 {
            return 1.0;
        }
        self.alerts_resolved_total as f64 / self.alerts_fired_total as f64
    }
    
    /// Get metrics summary
    pub fn summary(&self) -> MetricsSummary {
        MetricsSummary {
            total_alerts: self.alerts_fired_total,
            active_alerts: self.active_alerts_count,
            resolved_alerts: self.alerts_resolved_total,
            success_rate: self.alert_success_rate(),
            delivery_success_rate: self.delivery_metrics.success_rate,
            avg_delivery_time_ms: self.delivery_metrics.avg_delivery_time_ms,
            error_rate: self.error_metrics.error_rate,
            uptime_seconds: self.performance_metrics.uptime_seconds,
        }
    }
}

/// Metrics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub total_alerts: u64,
    pub active_alerts: usize,
    pub resolved_alerts: u64,
    pub success_rate: f64,
    pub delivery_success_rate: f64,
    pub avg_delivery_time_ms: f64,
    pub error_rate: f64,
    pub uptime_seconds: u64,
}

impl DeliveryMetrics {
    pub fn new() -> Self {
        Self {
            notifications_sent_total: 0,
            successful_deliveries: 0,
            failed_deliveries: 0,
            deliveries_by_channel: HashMap::new(),
            avg_delivery_time_ms: 0.0,
            success_rate: 1.0,
        }
    }
    
    pub fn record_delivery(&mut self, result: &DeliveryResult) {
        self.notifications_sent_total += 1;
        
        if result.success {
            self.successful_deliveries += 1;
        } else {
            self.failed_deliveries += 1;
        }
        
        // Update channel-specific metrics
        let channel_key = result.channel_type.to_string();
        let channel_metrics = self.deliveries_by_channel
            .entry(channel_key)
            .or_insert_with(ChannelDeliveryMetrics::new);
        
        channel_metrics.record_delivery(result);
        
        // Update overall metrics
        self.update_aggregates();
    }
    
    fn update_aggregates(&mut self) {
        if self.notifications_sent_total > 0 {
            self.success_rate = self.successful_deliveries as f64 / self.notifications_sent_total as f64;
        }
        
        // Calculate average delivery time across all channels
        let mut total_time = 0.0;
        let mut total_count = 0u64;
        
        for metrics in self.deliveries_by_channel.values() {
            total_time += metrics.avg_delivery_time_ms * metrics.total_attempts as f64;
            total_count += metrics.total_attempts;
        }
        
        if total_count > 0 {
            self.avg_delivery_time_ms = total_time / total_count as f64;
        }
    }
}

impl ChannelDeliveryMetrics {
    pub fn new() -> Self {
        Self {
            total_attempts: 0,
            successful: 0,
            failed: 0,
            avg_delivery_time_ms: 0.0,
            last_success: None,
            last_failure: None,
        }
    }
    
    pub fn record_delivery(&mut self, result: &DeliveryResult) {
        self.total_attempts += 1;
        
        if result.success {
            self.successful += 1;
            self.last_success = Some(result.timestamp);
        } else {
            self.failed += 1;
            self.last_failure = Some(result.timestamp);
        }
        
        // Update average delivery time
        let new_avg = (self.avg_delivery_time_ms * (self.total_attempts - 1) as f64 + result.duration_ms as f64) / self.total_attempts as f64;
        self.avg_delivery_time_ms = new_avg;
    }
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            avg_processing_time_ms: 0.0,
            peak_alerts_per_minute: 0,
            current_alerts_per_minute: 0,
            memory_usage_bytes: 0,
            cpu_usage_percent: 0.0,
            uptime_seconds: 0,
        }
    }
}

impl ErrorMetrics {
    pub fn new() -> Self {
        Self {
            total_errors: 0,
            errors_by_category: HashMap::new(),
            recent_errors: Vec::new(),
            error_rate: 0.0,
        }
    }
    
    pub fn record_error(&mut self, category: &str, message: &str, context: HashMap<String, String>) {
        self.total_errors += 1;
        
        *self.errors_by_category.entry(category.to_string()).or_insert(0) += 1;
        
        let error_record = ErrorRecord {
            timestamp: chrono::Utc::now(),
            category: category.to_string(),
            message: message.to_string(),
            context,
        };
        
        self.recent_errors.push(error_record);
        
        // Keep only last 100 errors
        if self.recent_errors.len() > 100 {
            self.recent_errors.remove(0);
        }
        
        // Update error rate (simplified calculation)
        self.error_rate = self.total_errors as f64 / 60.0; // errors per minute
    }
}

impl Default for AlertingMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert::AlertSeverity;

    #[test]
    fn test_metrics_creation() {
        let metrics = AlertingMetrics::new();
        assert_eq!(metrics.alerts_fired_total, 0);
        assert_eq!(metrics.active_alerts_count, 0);
        assert_eq!(metrics.alert_success_rate(), 1.0);
    }

    #[test]
    fn test_alert_recording() {
        let mut metrics = AlertingMetrics::new();
        
        let alert = crate::alert::Alert::new(
            "Test Alert".to_string(),
            "Test Description".to_string(),
            AlertSeverity::Critical,
            "test_source".to_string(),
        );
        
        metrics.record_alert_fired(&alert);
        
        assert_eq!(metrics.alerts_fired_total, 1);
        assert_eq!(metrics.alerts_by_severity.get("critical"), Some(&1));
        assert_eq!(metrics.alerts_by_source.get("test_source"), Some(&1));
    }

    #[test]
    fn test_delivery_metrics() {
        let mut metrics = DeliveryMetrics::new();
        
        let result = DeliveryResult {
            channel_type: ChannelType::Email,
            success: true,
            timestamp: chrono::Utc::now(),
            duration_ms: 100,
            error: None,
            metadata: HashMap::new(),
        };
        
        metrics.record_delivery(&result);
        
        assert_eq!(metrics.notifications_sent_total, 1);
        assert_eq!(metrics.successful_deliveries, 1);
        assert_eq!(metrics.success_rate, 1.0);
    }
}
