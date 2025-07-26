/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Alert rules and conditions

use crate::alert::Alert;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Alert rule for automated alert processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule name
    pub name: String,
    /// Rule condition expression
    pub condition: String,
    /// Actions to execute when rule matches
    pub actions: Vec<RuleAction>,
    /// Whether rule is enabled
    pub enabled: bool,
    /// Rule description
    pub description: Option<String>,
    /// Rule tags
    pub tags: Vec<String>,
    /// Rule metadata
    pub metadata: HashMap<String, String>,
}

/// Rule condition for matching alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    /// Condition expression
    pub expression: String,
    /// Condition type
    pub condition_type: ConditionType,
    /// Condition parameters
    pub parameters: HashMap<String, String>,
}

/// Types of rule conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    /// Simple field comparison
    FieldComparison,
    /// Regular expression match
    Regex,
    /// Time-based condition
    Time,
    /// Threshold condition
    Threshold,
    /// Custom condition
    Custom,
}

/// Actions to execute when rule matches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleAction {
    /// Action type
    pub action_type: ActionType,
    /// Action parameters
    pub parameters: HashMap<String, String>,
    /// Whether action is enabled
    pub enabled: bool,
}

/// Types of rule actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    /// Suppress the alert
    Suppress,
    /// Escalate the alert
    Escalate,
    /// Modify alert properties
    Modify,
    /// Route to specific channels
    Route,
    /// Add labels or annotations
    Enrich,
    /// Execute webhook
    Webhook,
    /// Custom action
    Custom(String),
}

impl AlertRule {
    /// Create a new alert rule
    pub fn new(name: String, condition: String) -> Self {
        Self {
            name,
            condition,
            actions: Vec::new(),
            enabled: true,
            description: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }
    
    /// Check if rule matches an alert
    pub fn matches(&self, alert: &Alert) -> bool {
        if !self.enabled {
            return false;
        }
        
        // Simple condition evaluation - in production this would be more sophisticated
        self.evaluate_condition(alert)
    }
    
    /// Evaluate rule condition against alert
    fn evaluate_condition(&self, alert: &Alert) -> bool {
        // Simple string-based condition evaluation
        // In production, this would use a proper expression evaluator
        
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
        
        // Default to false for unknown conditions
        false
    }
    
    /// Add an action to the rule
    pub fn add_action(&mut self, action: RuleAction) {
        self.actions.push(action);
    }
    
    /// Remove an action from the rule
    pub fn remove_action(&mut self, action_type: &ActionType) {
        self.actions.retain(|action| !std::mem::discriminant(&action.action_type).eq(&std::mem::discriminant(action_type)));
    }
    
    /// Enable the rule
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    /// Disable the rule
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    /// Add a tag to the rule
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }
    
    /// Remove a tag from the rule
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
    }
    
    /// Set metadata
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
    
    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

impl RuleAction {
    /// Create a new rule action
    pub fn new(action_type: ActionType) -> Self {
        Self {
            action_type,
            parameters: HashMap::new(),
            enabled: true,
        }
    }
    
    /// Create a suppress action
    pub fn suppress() -> Self {
        Self::new(ActionType::Suppress)
    }
    
    /// Create an escalate action
    pub fn escalate(level: u32) -> Self {
        let mut action = Self::new(ActionType::Escalate);
        action.parameters.insert("level".to_string(), level.to_string());
        action
    }
    
    /// Create a route action
    pub fn route(channels: Vec<String>) -> Self {
        let mut action = Self::new(ActionType::Route);
        action.parameters.insert("channels".to_string(), channels.join(","));
        action
    }
    
    /// Create an enrich action
    pub fn enrich(labels: HashMap<String, String>) -> Self {
        let mut action = Self::new(ActionType::Enrich);
        for (key, value) in labels {
            action.parameters.insert(format!("label_{}", key), value);
        }
        action
    }
    
    /// Set action parameter
    pub fn set_parameter(&mut self, key: String, value: String) {
        self.parameters.insert(key, value);
    }
    
    /// Get action parameter
    pub fn get_parameter(&self, key: &str) -> Option<&String> {
        self.parameters.get(key)
    }
    
    /// Enable the action
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    /// Disable the action
    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Suppress => write!(f, "suppress"),
            Self::Escalate => write!(f, "escalate"),
            Self::Modify => write!(f, "modify"),
            Self::Route => write!(f, "route"),
            Self::Enrich => write!(f, "enrich"),
            Self::Webhook => write!(f, "webhook"),
            Self::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

impl std::fmt::Display for ConditionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FieldComparison => write!(f, "field_comparison"),
            Self::Regex => write!(f, "regex"),
            Self::Time => write!(f, "time"),
            Self::Threshold => write!(f, "threshold"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert::{Alert, AlertSeverity};

    #[test]
    fn test_rule_creation() {
        let rule = AlertRule::new(
            "test_rule".to_string(),
            "severity == 'critical'".to_string(),
        );
        
        assert_eq!(rule.name, "test_rule");
        assert!(rule.enabled);
        assert!(rule.actions.is_empty());
    }

    #[test]
    fn test_rule_matching() {
        let mut rule = AlertRule::new(
            "critical_rule".to_string(),
            "critical".to_string(),
        );
        
        let alert = Alert::new(
            "Test Alert".to_string(),
            "Test Description".to_string(),
            AlertSeverity::Critical,
            "test_source".to_string(),
        );
        
        assert!(rule.matches(&alert));
        
        rule.disable();
        assert!(!rule.matches(&alert));
    }

    #[test]
    fn test_rule_actions() {
        let mut rule = AlertRule::new(
            "test_rule".to_string(),
            "test".to_string(),
        );
        
        let action = RuleAction::suppress();
        rule.add_action(action);
        
        assert_eq!(rule.actions.len(), 1);
        assert!(matches!(rule.actions[0].action_type, ActionType::Suppress));
        
        rule.remove_action(&ActionType::Suppress);
        assert!(rule.actions.is_empty());
    }

    #[test]
    fn test_action_creation() {
        let action = RuleAction::escalate(2);
        assert!(matches!(action.action_type, ActionType::Escalate));
        assert_eq!(action.get_parameter("level"), Some(&"2".to_string()));
        
        let channels = vec!["email".to_string(), "slack".to_string()];
        let route_action = RuleAction::route(channels);
        assert!(matches!(route_action.action_type, ActionType::Route));
        assert_eq!(route_action.get_parameter("channels"), Some(&"email,slack".to_string()));
    }

    #[test]
    fn test_rule_tags() {
        let mut rule = AlertRule::new(
            "test_rule".to_string(),
            "test".to_string(),
        );
        
        rule.add_tag("production".to_string());
        rule.add_tag("critical".to_string());
        
        assert_eq!(rule.tags.len(), 2);
        assert!(rule.tags.contains(&"production".to_string()));
        
        rule.remove_tag("production");
        assert_eq!(rule.tags.len(), 1);
        assert!(!rule.tags.contains(&"production".to_string()));
    }

    #[test]
    fn test_rule_metadata() {
        let mut rule = AlertRule::new(
            "test_rule".to_string(),
            "test".to_string(),
        );
        
        rule.set_metadata("owner".to_string(), "team-a".to_string());
        rule.set_metadata("priority".to_string(), "high".to_string());
        
        assert_eq!(rule.get_metadata("owner"), Some(&"team-a".to_string()));
        assert_eq!(rule.get_metadata("priority"), Some(&"high".to_string()));
        assert_eq!(rule.get_metadata("nonexistent"), None);
    }
}
