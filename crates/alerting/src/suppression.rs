/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Alert suppression and silencing

use crate::alert::Alert;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Alert suppression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppressionConfig {
    /// Suppression rules
    pub rules: Vec<SuppressionRule>,
    /// Global suppression settings
    pub global_settings: GlobalSuppressionSettings,
    /// Maintenance windows
    pub maintenance_windows: Vec<MaintenanceWindow>,
}

/// Suppression rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppressionRule {
    /// Rule name
    pub name: String,
    /// Rule condition
    pub condition: String,
    /// Suppression duration
    pub duration: Option<Duration>,
    /// Whether rule is enabled
    pub enabled: bool,
    /// Rule description
    pub description: Option<String>,
    /// Rule metadata
    pub metadata: HashMap<String, String>,
}

/// Global suppression settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSuppressionSettings {
    /// Enable suppression globally
    pub enabled: bool,
    /// Default suppression duration
    pub default_duration: Duration,
    /// Maximum suppression duration
    pub max_duration: Duration,
    /// Enable automatic suppression cleanup
    pub auto_cleanup: bool,
    /// Cleanup interval
    pub cleanup_interval: Duration,
}

/// Maintenance window for suppressing alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceWindow {
    /// Window name
    pub name: String,
    /// Start time
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// End time
    pub end_time: chrono::DateTime<chrono::Utc>,
    /// Whether window is active
    pub active: bool,
    /// Window description
    pub description: Option<String>,
    /// Affected sources (empty means all)
    pub affected_sources: Vec<String>,
    /// Affected severities (empty means all)
    pub affected_severities: Vec<String>,
    /// Window metadata
    pub metadata: HashMap<String, String>,
}

impl SuppressionRule {
    /// Create a new suppression rule
    pub fn new(name: String, condition: String) -> Self {
        Self {
            name,
            condition,
            duration: None,
            enabled: true,
            description: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Set suppression duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }
    
    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
    
    /// Check if rule matches an alert
    pub fn matches(&self, alert: &Alert) -> bool {
        if !self.enabled {
            return false;
        }
        
        // Simple condition evaluation
        self.evaluate_condition(alert)
    }
    
    /// Evaluate rule condition
    fn evaluate_condition(&self, alert: &Alert) -> bool {
        // Simple string-based condition evaluation
        if self.condition.contains("severity") {
            let severity_str = alert.severity.to_string();
            return self.condition.contains(&severity_str);
        }
        
        if self.condition.contains("source") {
            return self.condition.contains(&alert.source);
        }
        
        if self.condition.contains("title") {
            return self.condition.contains(&alert.title);
        }
        
        false
    }
    
    /// Enable the rule
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    /// Disable the rule
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    /// Set metadata
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}

impl MaintenanceWindow {
    /// Create a new maintenance window
    pub fn new(
        name: String,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            name,
            start_time,
            end_time,
            active: true,
            description: None,
            affected_sources: Vec::new(),
            affected_severities: Vec::new(),
            metadata: HashMap::new(),
        }
    }
    
    /// Check if window is currently active
    pub fn is_active(&self) -> bool {
        if !self.active {
            return false;
        }
        
        let now = chrono::Utc::now();
        now >= self.start_time && now <= self.end_time
    }
    
    /// Check if alert should be suppressed by this window
    pub fn should_suppress(&self, alert: &Alert) -> bool {
        if !self.is_active() {
            return false;
        }
        
        // Check affected sources
        if !self.affected_sources.is_empty() && !self.affected_sources.contains(&alert.source) {
            return false;
        }
        
        // Check affected severities
        if !self.affected_severities.is_empty() {
            let severity_str = alert.severity.to_string();
            if !self.affected_severities.contains(&severity_str) {
                return false;
            }
        }
        
        true
    }
    
    /// Set affected sources
    pub fn with_sources(mut self, sources: Vec<String>) -> Self {
        self.affected_sources = sources;
        self
    }
    
    /// Set affected severities
    pub fn with_severities(mut self, severities: Vec<String>) -> Self {
        self.affected_severities = severities;
        self
    }
    
    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
    
    /// Activate the window
    pub fn activate(&mut self) {
        self.active = true;
    }
    
    /// Deactivate the window
    pub fn deactivate(&mut self) {
        self.active = false;
    }
    
    /// Extend the window
    pub fn extend(&mut self, duration: Duration) {
        self.end_time = self.end_time + chrono::Duration::from_std(duration).unwrap_or_default();
    }
    
    /// Get window duration
    pub fn duration(&self) -> chrono::Duration {
        self.end_time - self.start_time
    }
    
    /// Get remaining time
    pub fn remaining_time(&self) -> Option<chrono::Duration> {
        let now = chrono::Utc::now();
        if now < self.end_time {
            Some(self.end_time - now)
        } else {
            None
        }
    }
}

impl Default for SuppressionConfig {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            global_settings: GlobalSuppressionSettings::default(),
            maintenance_windows: Vec::new(),
        }
    }
}

impl Default for GlobalSuppressionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            default_duration: Duration::from_secs(3600), // 1 hour
            max_duration: Duration::from_secs(86400 * 7), // 7 days
            auto_cleanup: true,
            cleanup_interval: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Suppression manager for handling alert suppression
#[derive(Debug)]
pub struct SuppressionManager {
    config: SuppressionConfig,
}

impl SuppressionManager {
    /// Create a new suppression manager
    pub fn new(config: SuppressionConfig) -> Self {
        Self { config }
    }
    
    /// Check if an alert should be suppressed
    pub fn should_suppress(&self, alert: &Alert) -> bool {
        if !self.config.global_settings.enabled {
            return false;
        }
        
        // Check suppression rules
        for rule in &self.config.rules {
            if rule.matches(alert) {
                return true;
            }
        }
        
        // Check maintenance windows
        for window in &self.config.maintenance_windows {
            if window.should_suppress(alert) {
                return true;
            }
        }
        
        false
    }
    
    /// Add a suppression rule
    pub fn add_rule(&mut self, rule: SuppressionRule) {
        self.config.rules.push(rule);
    }
    
    /// Remove a suppression rule
    pub fn remove_rule(&mut self, name: &str) -> bool {
        let initial_len = self.config.rules.len();
        self.config.rules.retain(|rule| rule.name != name);
        self.config.rules.len() < initial_len
    }
    
    /// Add a maintenance window
    pub fn add_maintenance_window(&mut self, window: MaintenanceWindow) {
        self.config.maintenance_windows.push(window);
    }
    
    /// Remove a maintenance window
    pub fn remove_maintenance_window(&mut self, name: &str) -> bool {
        let initial_len = self.config.maintenance_windows.len();
        self.config.maintenance_windows.retain(|window| window.name != name);
        self.config.maintenance_windows.len() < initial_len
    }
    
    /// Get active maintenance windows
    pub fn get_active_windows(&self) -> Vec<&MaintenanceWindow> {
        self.config.maintenance_windows
            .iter()
            .filter(|window| window.is_active())
            .collect()
    }
    
    /// Clean up expired windows and rules
    pub fn cleanup(&mut self) {
        let now = chrono::Utc::now();
        
        // Remove expired maintenance windows
        self.config.maintenance_windows.retain(|window| {
            window.end_time > now || window.active
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert::{Alert, AlertSeverity};

    #[test]
    fn test_suppression_rule_creation() {
        let rule = SuppressionRule::new(
            "test_rule".to_string(),
            "severity == 'info'".to_string(),
        );
        
        assert_eq!(rule.name, "test_rule");
        assert!(rule.enabled);
        assert!(rule.duration.is_none());
    }

    #[test]
    fn test_suppression_rule_matching() {
        let rule = SuppressionRule::new(
            "info_rule".to_string(),
            "info".to_string(),
        );
        
        let alert = Alert::new(
            "Test Alert".to_string(),
            "Test Description".to_string(),
            AlertSeverity::Info,
            "test_source".to_string(),
        );
        
        assert!(rule.matches(&alert));
        
        let critical_alert = Alert::new(
            "Critical Alert".to_string(),
            "Critical Description".to_string(),
            AlertSeverity::Critical,
            "test_source".to_string(),
        );
        
        assert!(!rule.matches(&critical_alert));
    }

    #[test]
    fn test_maintenance_window() {
        let start = chrono::Utc::now() - chrono::Duration::minutes(30);
        let end = chrono::Utc::now() + chrono::Duration::minutes(30);
        
        let window = MaintenanceWindow::new(
            "test_window".to_string(),
            start,
            end,
        );
        
        assert!(window.is_active());
        assert!(window.remaining_time().is_some());
        
        let alert = Alert::new(
            "Test Alert".to_string(),
            "Test Description".to_string(),
            AlertSeverity::Warning,
            "test_source".to_string(),
        );
        
        assert!(window.should_suppress(&alert));
    }

    #[test]
    fn test_suppression_manager() {
        let mut manager = SuppressionManager::new(SuppressionConfig::default());
        
        let rule = SuppressionRule::new(
            "test_rule".to_string(),
            "info".to_string(),
        );
        
        manager.add_rule(rule);
        
        let info_alert = Alert::new(
            "Info Alert".to_string(),
            "Info Description".to_string(),
            AlertSeverity::Info,
            "test_source".to_string(),
        );
        
        assert!(manager.should_suppress(&info_alert));
        
        let critical_alert = Alert::new(
            "Critical Alert".to_string(),
            "Critical Description".to_string(),
            AlertSeverity::Critical,
            "test_source".to_string(),
        );
        
        assert!(!manager.should_suppress(&critical_alert));
        
        // Test rule removal
        let removed = manager.remove_rule("test_rule");
        assert!(removed);
        assert!(!manager.should_suppress(&info_alert));
    }
}
