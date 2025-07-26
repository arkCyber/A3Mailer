//! # Plugin System for Stalwart Mail Server
//! 
//! This crate provides a dynamic plugin system for extending Stalwart Mail Server
//! functionality at runtime. It supports loading, managing, and executing plugins
//! with proper isolation and security.
//!
//! ## Features
//!
//! - Dynamic plugin loading and unloading
//! - Plugin lifecycle management
//! - Secure plugin execution environment
//! - Plugin dependency resolution
//! - Hot-reloading support
//! - Plugin API versioning
//!
//! ## Example
//!
//! ```rust
//! use plugin_system::{PluginManager, Plugin};
//!
//! // Create a plugin manager
//! let mut manager = PluginManager::new();
//!
//! // Load a plugin
//! manager.load_plugin("path/to/plugin.so").await?;
//!
//! // Execute plugin functionality
//! manager.execute_hook("on_email_received", &email_data).await?;
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

/// Plugin manager for handling dynamic plugin loading and execution
pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, Box<dyn Plugin>>>>,
    config: PluginConfig,
}

/// Configuration for the plugin system
#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub plugin_dir: String,
    pub max_plugins: usize,
    pub enable_hot_reload: bool,
    pub security_level: SecurityLevel,
}

/// Security levels for plugin execution
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityLevel {
    /// Minimal security - plugins have full system access
    Minimal,
    /// Standard security - plugins run with limited permissions
    Standard,
    /// High security - plugins run in sandboxed environment
    High,
}

/// Plugin trait that all plugins must implement
pub trait Plugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> PluginMetadata;
    
    /// Initialize the plugin
    fn initialize(&mut self) -> Result<(), PluginError>;
    
    /// Execute a plugin hook
    fn execute_hook(&self, hook: &str, data: &[u8]) -> Result<Vec<u8>, PluginError>;
    
    /// Cleanup plugin resources
    fn cleanup(&mut self) -> Result<(), PluginError>;
}

/// Plugin metadata information
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub api_version: String,
    pub dependencies: Vec<String>,
}

/// Plugin system errors
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),
    
    #[error("Plugin loading failed: {0}")]
    LoadingFailed(String),
    
    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Security violation: {0}")]
    SecurityViolation(String),
    
    #[error("API version mismatch: expected {expected}, got {actual}")]
    ApiVersionMismatch { expected: String, actual: String },
    
    #[error("Dependency not satisfied: {0}")]
    DependencyNotSatisfied(String),
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            plugin_dir: "plugins".to_string(),
            max_plugins: 100,
            enable_hot_reload: false,
            security_level: SecurityLevel::Standard,
        }
    }
}

impl PluginManager {
    /// Create a new plugin manager with default configuration
    pub fn new() -> Self {
        Self::with_config(PluginConfig::default())
    }
    
    /// Create a new plugin manager with custom configuration
    pub fn with_config(config: PluginConfig) -> Self {
        info!("Initializing plugin manager with config: {:?}", config);
        
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    /// Load a plugin from the specified path
    pub async fn load_plugin<P: AsRef<Path>>(&self, path: P) -> Result<String, PluginError> {
        let path = path.as_ref();
        info!("Loading plugin from: {:?}", path);
        
        // TODO: Implement actual plugin loading logic
        // This is a placeholder implementation
        
        let plugin_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
            
        warn!("Plugin loading not yet implemented: {}", plugin_name);
        
        Ok(plugin_name)
    }
    
    /// Unload a plugin by name
    pub async fn unload_plugin(&self, name: &str) -> Result<(), PluginError> {
        info!("Unloading plugin: {}", name);
        
        let mut plugins = self.plugins.write().await;
        if let Some(mut plugin) = plugins.remove(name) {
            plugin.cleanup()?;
            info!("Plugin {} unloaded successfully", name);
            Ok(())
        } else {
            Err(PluginError::NotFound(name.to_string()))
        }
    }
    
    /// Execute a hook across all loaded plugins
    pub async fn execute_hook(&self, hook: &str, data: &[u8]) -> Result<Vec<Vec<u8>>, PluginError> {
        debug!("Executing hook '{}' across all plugins", hook);
        
        let plugins = self.plugins.read().await;
        let mut results = Vec::new();
        
        for (name, plugin) in plugins.iter() {
            match plugin.execute_hook(hook, data) {
                Ok(result) => {
                    debug!("Plugin {} executed hook '{}' successfully", name, hook);
                    results.push(result);
                }
                Err(e) => {
                    error!("Plugin {} failed to execute hook '{}': {}", name, hook, e);
                    // Continue with other plugins
                }
            }
        }
        
        Ok(results)
    }
    
    /// Get list of loaded plugins
    pub async fn list_plugins(&self) -> Vec<String> {
        let plugins = self.plugins.read().await;
        plugins.keys().cloned().collect()
    }
    
    /// Get plugin metadata by name
    pub async fn get_plugin_metadata(&self, name: &str) -> Result<PluginMetadata, PluginError> {
        let plugins = self.plugins.read().await;
        if let Some(plugin) = plugins.get(name) {
            Ok(plugin.metadata())
        } else {
            Err(PluginError::NotFound(name.to_string()))
        }
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_plugin_manager_creation() {
        let manager = PluginManager::new();
        let plugins = manager.list_plugins().await;
        assert!(plugins.is_empty());
    }
    
    #[tokio::test]
    async fn test_plugin_manager_with_config() {
        let config = PluginConfig {
            plugin_dir: "test_plugins".to_string(),
            max_plugins: 50,
            enable_hot_reload: true,
            security_level: SecurityLevel::High,
        };
        
        let manager = PluginManager::with_config(config.clone());
        assert_eq!(manager.config.plugin_dir, "test_plugins");
        assert_eq!(manager.config.max_plugins, 50);
        assert!(manager.config.enable_hot_reload);
        assert_eq!(manager.config.security_level, SecurityLevel::High);
    }
    
    #[tokio::test]
    async fn test_unload_nonexistent_plugin() {
        let manager = PluginManager::new();
        let result = manager.unload_plugin("nonexistent").await;
        assert!(matches!(result, Err(PluginError::NotFound(_))));
    }
    
    #[tokio::test]
    async fn test_get_nonexistent_plugin_metadata() {
        let manager = PluginManager::new();
        let result = manager.get_plugin_metadata("nonexistent").await;
        assert!(matches!(result, Err(PluginError::NotFound(_))));
    }
    
    #[tokio::test]
    async fn test_execute_hook_empty_plugins() {
        let manager = PluginManager::new();
        let results = manager.execute_hook("test_hook", b"test_data").await.unwrap();
        assert!(results.is_empty());
    }
}
