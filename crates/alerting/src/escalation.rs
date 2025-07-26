/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Alert escalation policies and management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Escalation policy for alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationPolicy {
    /// Policy name
    pub name: String,
    /// Escalation levels
    pub levels: Vec<EscalationLevel>,
    /// Whether policy is enabled
    pub enabled: bool,
    /// Policy description
    pub description: Option<String>,
    /// Policy metadata
    pub metadata: HashMap<String, String>,
}

/// Escalation level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationLevel {
    /// Level number (0-based)
    pub level: u32,
    /// Delay before escalating to this level
    pub delay: Duration,
    /// Notification channels for this level
    pub channels: Vec<String>,
    /// Whether to repeat notifications at this level
    pub repeat: bool,
    /// Repeat interval if enabled
    pub repeat_interval: Option<Duration>,
    /// Maximum number of repeats
    pub max_repeats: Option<u32>,
    /// Level description
    pub description: Option<String>,
}

/// Escalation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationConfig {
    /// Default escalation policy
    pub default_policy: Option<String>,
    /// Escalation policies by name
    pub policies: HashMap<String, EscalationPolicy>,
    /// Global escalation settings
    pub global_settings: GlobalEscalationSettings,
}

/// Global escalation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalEscalationSettings {
    /// Enable escalation globally
    pub enabled: bool,
    /// Maximum escalation level
    pub max_level: u32,
    /// Default escalation delay
    pub default_delay: Duration,
    /// Enable escalation suppression during maintenance
    pub suppress_during_maintenance: bool,
    /// Escalation timeout (stop escalating after this time)
    pub escalation_timeout: Option<Duration>,
}

impl EscalationPolicy {
    /// Create a new escalation policy
    pub fn new(name: String) -> Self {
        Self {
            name,
            levels: Vec::new(),
            enabled: true,
            description: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Add an escalation level
    pub fn add_level(&mut self, level: EscalationLevel) {
        self.levels.push(level);
        // Sort levels by level number
        self.levels.sort_by_key(|l| l.level);
    }
    
    /// Get escalation level by number
    pub fn get_level(&self, level_num: u32) -> Option<&EscalationLevel> {
        self.levels.iter().find(|l| l.level == level_num)
    }
    
    /// Get next escalation level
    pub fn get_next_level(&self, current_level: u32) -> Option<&EscalationLevel> {
        self.levels.iter().find(|l| l.level > current_level)
    }
    
    /// Get maximum level
    pub fn max_level(&self) -> u32 {
        self.levels.iter().map(|l| l.level).max().unwrap_or(0)
    }
    
    /// Enable the policy
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    /// Disable the policy
    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

impl EscalationLevel {
    /// Create a new escalation level
    pub fn new(level: u32, delay: Duration, channels: Vec<String>) -> Self {
        Self {
            level,
            delay,
            channels,
            repeat: false,
            repeat_interval: None,
            max_repeats: None,
            description: None,
        }
    }
    
    /// Enable repeating notifications
    pub fn with_repeat(mut self, interval: Duration, max_repeats: Option<u32>) -> Self {
        self.repeat = true;
        self.repeat_interval = Some(interval);
        self.max_repeats = max_repeats;
        self
    }
    
    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
    
    /// Check if level should repeat notifications
    pub fn should_repeat(&self, repeat_count: u32) -> bool {
        if !self.repeat {
            return false;
        }
        
        if let Some(max_repeats) = self.max_repeats {
            repeat_count < max_repeats
        } else {
            true
        }
    }
}

impl Default for EscalationConfig {
    fn default() -> Self {
        Self {
            default_policy: None,
            policies: HashMap::new(),
            global_settings: GlobalEscalationSettings::default(),
        }
    }
}

impl Default for GlobalEscalationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_level: 5,
            default_delay: Duration::from_secs(300), // 5 minutes
            suppress_during_maintenance: true,
            escalation_timeout: Some(Duration::from_secs(86400)), // 24 hours
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escalation_policy_creation() {
        let policy = EscalationPolicy::new("test_policy".to_string());
        assert_eq!(policy.name, "test_policy");
        assert!(policy.enabled);
        assert!(policy.levels.is_empty());
    }

    #[test]
    fn test_escalation_levels() {
        let mut policy = EscalationPolicy::new("test_policy".to_string());
        
        let level1 = EscalationLevel::new(
            0,
            Duration::from_secs(300),
            vec!["email".to_string()],
        );
        
        let level2 = EscalationLevel::new(
            1,
            Duration::from_secs(900),
            vec!["email".to_string(), "slack".to_string()],
        );
        
        policy.add_level(level1);
        policy.add_level(level2);
        
        assert_eq!(policy.levels.len(), 2);
        assert_eq!(policy.max_level(), 1);
        
        let level = policy.get_level(0).unwrap();
        assert_eq!(level.channels.len(), 1);
        
        let next_level = policy.get_next_level(0).unwrap();
        assert_eq!(next_level.level, 1);
        assert_eq!(next_level.channels.len(), 2);
    }

    #[test]
    fn test_escalation_level_repeat() {
        let level = EscalationLevel::new(
            0,
            Duration::from_secs(300),
            vec!["email".to_string()],
        ).with_repeat(Duration::from_secs(600), Some(3));
        
        assert!(level.repeat);
        assert_eq!(level.repeat_interval, Some(Duration::from_secs(600)));
        assert_eq!(level.max_repeats, Some(3));
        
        assert!(level.should_repeat(0));
        assert!(level.should_repeat(2));
        assert!(!level.should_repeat(3));
    }

    #[test]
    fn test_global_escalation_settings() {
        let settings = GlobalEscalationSettings::default();
        assert!(settings.enabled);
        assert_eq!(settings.max_level, 5);
        assert!(settings.suppress_during_maintenance);
    }
}
