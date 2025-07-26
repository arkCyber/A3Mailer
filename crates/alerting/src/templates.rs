/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Template engine for alert formatting

use crate::alert::Alert;
use crate::error::{AlertingError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Template engine for formatting alerts
#[derive(Debug)]
pub struct TemplateEngine {
    templates: HashMap<String, AlertTemplate>,
    config: HashMap<String, String>,
}

/// Alert template configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertTemplate {
    /// Template name
    pub name: String,
    /// Template content
    pub content: String,
    /// Template type
    pub template_type: TemplateType,
    /// Template format
    pub format: TemplateFormat,
    /// Template variables
    pub variables: HashMap<String, String>,
    /// Template metadata
    pub metadata: HashMap<String, String>,
}

/// Template types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateType {
    /// Alert notification template
    Alert,
    /// Resolution notification template
    Resolution,
    /// Summary template
    Summary,
    /// Custom template
    Custom(String),
}

/// Template formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateFormat {
    /// Plain text format
    Text,
    /// HTML format
    Html,
    /// Markdown format
    Markdown,
    /// JSON format
    Json,
    /// Custom format
    Custom(String),
}

/// Template configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    /// Default templates
    pub default_templates: HashMap<String, String>,
    /// Custom templates
    pub custom_templates: HashMap<String, AlertTemplate>,
    /// Template settings
    pub settings: TemplateSettings,
}

/// Template settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSettings {
    /// Enable template caching
    pub enable_caching: bool,
    /// Cache TTL in seconds
    pub cache_ttl: u64,
    /// Maximum template size in bytes
    pub max_template_size: usize,
    /// Enable template validation
    pub enable_validation: bool,
    /// Template timeout in seconds
    pub timeout_seconds: u64,
}

impl TemplateEngine {
    /// Create a new template engine
    pub async fn new(config: &HashMap<String, String>) -> Result<Self> {
        let mut engine = Self {
            templates: HashMap::new(),
            config: config.clone(),
        };
        
        // Load default templates
        engine.load_default_templates().await?;
        
        Ok(engine)
    }
    
    /// Render an alert using a template
    pub async fn render_alert(&self, alert: &Alert, template_name: &str) -> Result<String> {
        let template = self.templates.get(template_name)
            .ok_or_else(|| AlertingError::template(format!("Template not found: {}", template_name)))?;
        
        self.render_template(template, alert).await
    }
    
    /// Render a template with alert data
    async fn render_template(&self, template: &AlertTemplate, alert: &Alert) -> Result<String> {
        match template.format {
            TemplateFormat::Text => self.render_text_template(template, alert).await,
            TemplateFormat::Html => self.render_html_template(template, alert).await,
            TemplateFormat::Markdown => self.render_markdown_template(template, alert).await,
            TemplateFormat::Json => self.render_json_template(template, alert).await,
            TemplateFormat::Custom(_) => {
                Err(AlertingError::template("Custom template formats not implemented"))
            }
        }
    }
    
    /// Render text template
    async fn render_text_template(&self, template: &AlertTemplate, alert: &Alert) -> Result<String> {
        let mut content = template.content.clone();
        
        // Simple variable substitution
        content = content.replace("{{title}}", &alert.title);
        content = content.replace("{{description}}", &alert.description);
        content = content.replace("{{severity}}", &alert.severity.to_string());
        content = content.replace("{{status}}", &alert.status.to_string());
        content = content.replace("{{source}}", &alert.source);
        content = content.replace("{{created_at}}", &alert.created_at.to_rfc3339());
        content = content.replace("{{alert_id}}", &alert.id.to_string());
        
        // Replace labels
        for (key, value) in &alert.context.labels {
            let placeholder = format!("{{{{label.{}}}}}", key);
            content = content.replace(&placeholder, value);
        }
        
        // Replace annotations
        for (key, value) in &alert.context.annotations {
            let placeholder = format!("{{{{annotation.{}}}}}", key);
            content = content.replace(&placeholder, value);
        }
        
        Ok(content)
    }
    
    /// Render HTML template
    async fn render_html_template(&self, template: &AlertTemplate, alert: &Alert) -> Result<String> {
        // For now, use the same logic as text template but with HTML escaping
        let text_content = self.render_text_template(template, alert).await?;
        
        // Simple HTML escaping
        let html_content = text_content
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("\"", "&quot;")
            .replace("'", "&#x27;");
        
        Ok(html_content)
    }
    
    /// Render Markdown template
    async fn render_markdown_template(&self, template: &AlertTemplate, alert: &Alert) -> Result<String> {
        // Use text template rendering for Markdown
        self.render_text_template(template, alert).await
    }
    
    /// Render JSON template
    async fn render_json_template(&self, _template: &AlertTemplate, alert: &Alert) -> Result<String> {
        // Return alert as JSON
        serde_json::to_string_pretty(alert)
            .map_err(|e| AlertingError::template(format!("JSON serialization failed: {}", e)))
    }
    
    /// Add a custom template
    pub fn add_template(&mut self, template: AlertTemplate) {
        self.templates.insert(template.name.clone(), template);
    }
    
    /// Remove a template
    pub fn remove_template(&mut self, name: &str) -> bool {
        self.templates.remove(name).is_some()
    }
    
    /// Get template by name
    pub fn get_template(&self, name: &str) -> Option<&AlertTemplate> {
        self.templates.get(name)
    }
    
    /// List all template names
    pub fn list_templates(&self) -> Vec<String> {
        self.templates.keys().cloned().collect()
    }
    
    /// Load default templates
    async fn load_default_templates(&mut self) -> Result<()> {
        // Default text template
        let default_text = AlertTemplate {
            name: "default_text".to_string(),
            content: "Alert: {{title}}\nSeverity: {{severity}}\nDescription: {{description}}\nSource: {{source}}\nTime: {{created_at}}".to_string(),
            template_type: TemplateType::Alert,
            format: TemplateFormat::Text,
            variables: HashMap::new(),
            metadata: HashMap::new(),
        };
        
        // Default HTML template
        let default_html = AlertTemplate {
            name: "default_html".to_string(),
            content: r#"
<h2>Alert: {{title}}</h2>
<p><strong>Severity:</strong> {{severity}}</p>
<p><strong>Description:</strong> {{description}}</p>
<p><strong>Source:</strong> {{source}}</p>
<p><strong>Time:</strong> {{created_at}}</p>
"#.to_string(),
            template_type: TemplateType::Alert,
            format: TemplateFormat::Html,
            variables: HashMap::new(),
            metadata: HashMap::new(),
        };
        
        // Default Markdown template
        let default_markdown = AlertTemplate {
            name: "default_markdown".to_string(),
            content: r#"
## Alert: {{title}}

**Severity:** {{severity}}
**Description:** {{description}}
**Source:** {{source}}
**Time:** {{created_at}}
"#.to_string(),
            template_type: TemplateType::Alert,
            format: TemplateFormat::Markdown,
            variables: HashMap::new(),
            metadata: HashMap::new(),
        };
        
        self.templates.insert(default_text.name.clone(), default_text);
        self.templates.insert(default_html.name.clone(), default_html);
        self.templates.insert(default_markdown.name.clone(), default_markdown);
        
        Ok(())
    }
}

impl AlertTemplate {
    /// Create a new alert template
    pub fn new(name: String, content: String, format: TemplateFormat) -> Self {
        Self {
            name,
            content,
            template_type: TemplateType::Alert,
            format,
            variables: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
    
    /// Set template type
    pub fn with_type(mut self, template_type: TemplateType) -> Self {
        self.template_type = template_type;
        self
    }
    
    /// Add a variable
    pub fn add_variable(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
    }
    
    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
    
    /// Validate template syntax
    pub fn validate(&self) -> Result<()> {
        // Basic validation - check for balanced braces
        let open_count = self.content.matches("{{").count();
        let close_count = self.content.matches("}}").count();
        
        if open_count != close_count {
            return Err(AlertingError::template("Unbalanced template braces"));
        }
        
        Ok(())
    }
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            default_templates: HashMap::new(),
            custom_templates: HashMap::new(),
            settings: TemplateSettings::default(),
        }
    }
}

impl Default for TemplateSettings {
    fn default() -> Self {
        Self {
            enable_caching: true,
            cache_ttl: 3600, // 1 hour
            max_template_size: 1024 * 1024, // 1MB
            enable_validation: true,
            timeout_seconds: 30,
        }
    }
}

impl std::fmt::Display for TemplateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alert => write!(f, "alert"),
            Self::Resolution => write!(f, "resolution"),
            Self::Summary => write!(f, "summary"),
            Self::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

impl std::fmt::Display for TemplateFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Html => write!(f, "html"),
            Self::Markdown => write!(f, "markdown"),
            Self::Json => write!(f, "json"),
            Self::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert::{Alert, AlertSeverity};

    #[tokio::test]
    async fn test_template_engine_creation() {
        let config = HashMap::new();
        let engine = TemplateEngine::new(&config).await;
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_text_template_rendering() {
        let config = HashMap::new();
        let engine = TemplateEngine::new(&config).await.unwrap();
        
        let alert = Alert::new(
            "Test Alert".to_string(),
            "Test Description".to_string(),
            AlertSeverity::Critical,
            "test_source".to_string(),
        );
        
        let result = engine.render_alert(&alert, "default_text").await;
        assert!(result.is_ok());
        
        let rendered = result.unwrap();
        assert!(rendered.contains("Test Alert"));
        assert!(rendered.contains("critical"));
        assert!(rendered.contains("test_source"));
    }

    #[test]
    fn test_template_validation() {
        let template = AlertTemplate::new(
            "test".to_string(),
            "{{title}} - {{description}}".to_string(),
            TemplateFormat::Text,
        );
        
        assert!(template.validate().is_ok());
        
        let invalid_template = AlertTemplate::new(
            "invalid".to_string(),
            "{{title} - {{description}}".to_string(),
            TemplateFormat::Text,
        );
        
        assert!(invalid_template.validate().is_err());
    }

    #[test]
    fn test_template_creation() {
        let template = AlertTemplate::new(
            "custom".to_string(),
            "Custom template: {{title}}".to_string(),
            TemplateFormat::Html,
        ).with_type(TemplateType::Resolution);
        
        assert_eq!(template.name, "custom");
        assert!(matches!(template.template_type, TemplateType::Resolution));
        assert!(matches!(template.format, TemplateFormat::Html));
    }
}
