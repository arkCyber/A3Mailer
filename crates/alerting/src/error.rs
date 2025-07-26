/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Error types for the alerting system

use std::fmt;

/// Result type for alerting operations
pub type Result<T> = std::result::Result<T, AlertingError>;

/// Alerting system errors
#[derive(Debug, thiserror::Error)]
pub enum AlertingError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Channel error
    #[error("Channel error: {0}")]
    Channel(String),

    /// Template error
    #[error("Template error: {0}")]
    Template(String),

    /// Engine error
    #[error("Engine error: {0}")]
    Engine(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Rate limit error
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Not found error
    #[error("Not found: {0}")]
    NotFound(String),

    /// Permission denied error
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Service unavailable error
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// Timeout error
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// HTTP error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// TOML error
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    /// Handlebars error
    #[cfg(feature = "templates")]
    #[error("Handlebars error: {0}")]
    Handlebars(#[from] handlebars::RenderError),

    /// Tera error
    #[cfg(feature = "templates")]
    #[error("Tera error: {0}")]
    Tera(#[from] tera::Error),

    /// Lettre error
    #[cfg(feature = "email")]
    #[error("Email error: {0}")]
    Email(#[from] lettre::error::Error),

    /// Encryption error
    #[cfg(feature = "encryption")]
    #[error("Encryption error: {0}")]
    Encryption(#[from] aes_gcm::Error),
}

impl AlertingError {
    /// Create a new configuration error
    pub fn config<T: Into<String>>(msg: T) -> Self {
        Self::Config(msg.into())
    }

    /// Create a new channel error
    pub fn channel<T: Into<String>>(msg: T) -> Self {
        Self::Channel(msg.into())
    }

    /// Create a new template error
    pub fn template<T: Into<String>>(msg: T) -> Self {
        Self::Template(msg.into())
    }

    /// Create a new engine error
    pub fn engine<T: Into<String>>(msg: T) -> Self {
        Self::Engine(msg.into())
    }

    /// Create a new network error
    pub fn network<T: Into<String>>(msg: T) -> Self {
        Self::Network(msg.into())
    }

    /// Create a new authentication error
    pub fn authentication<T: Into<String>>(msg: T) -> Self {
        Self::Authentication(msg.into())
    }

    /// Create a new rate limit error
    pub fn rate_limit<T: Into<String>>(msg: T) -> Self {
        Self::RateLimit(msg.into())
    }

    /// Create a new validation error
    pub fn validation<T: Into<String>>(msg: T) -> Self {
        Self::Validation(msg.into())
    }

    /// Create a new not found error
    pub fn not_found<T: Into<String>>(msg: T) -> Self {
        Self::NotFound(msg.into())
    }

    /// Create a new permission denied error
    pub fn permission_denied<T: Into<String>>(msg: T) -> Self {
        Self::PermissionDenied(msg.into())
    }

    /// Create a new service unavailable error
    pub fn service_unavailable<T: Into<String>>(msg: T) -> Self {
        Self::ServiceUnavailable(msg.into())
    }

    /// Create a new timeout error
    pub fn timeout<T: Into<String>>(msg: T) -> Self {
        Self::Timeout(msg.into())
    }

    /// Create a new internal error
    pub fn internal<T: Into<String>>(msg: T) -> Self {
        Self::Internal(msg.into())
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Network(_) |
            Self::ServiceUnavailable(_) |
            Self::Timeout(_) |
            Self::RateLimit(_) => true,
            Self::Http(e) => {
                e.status().map_or(true, |status| {
                    status.is_server_error() || status == 429
                })
            }
            _ => false,
        }
    }

    /// Get error category for metrics
    pub fn category(&self) -> &'static str {
        match self {
            Self::Config(_) => "config",
            Self::Channel(_) => "channel",
            Self::Template(_) => "template",
            Self::Engine(_) => "engine",
            Self::Network(_) => "network",
            Self::Authentication(_) => "auth",
            Self::RateLimit(_) => "rate_limit",
            Self::Serialization(_) => "serialization",
            Self::Database(_) => "database",
            Self::Validation(_) => "validation",
            Self::NotFound(_) => "not_found",
            Self::PermissionDenied(_) => "permission",
            Self::ServiceUnavailable(_) => "service",
            Self::Timeout(_) => "timeout",
            Self::Internal(_) => "internal",
            Self::Io(_) => "io",
            Self::Json(_) => "json",
            Self::Http(_) => "http",
            Self::Toml(_) => "toml",
            #[cfg(feature = "templates")]
            Self::Handlebars(_) => "handlebars",
            #[cfg(feature = "templates")]
            Self::Tera(_) => "tera",
            #[cfg(feature = "email")]
            Self::Email(_) => "email",
            #[cfg(feature = "encryption")]
            Self::Encryption(_) => "encryption",
        }
    }
}

impl From<String> for AlertingError {
    fn from(msg: String) -> Self {
        Self::Internal(msg)
    }
}

impl From<&str> for AlertingError {
    fn from(msg: &str) -> Self {
        Self::Internal(msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = AlertingError::config("test config error");
        assert!(matches!(err, AlertingError::Config(_)));
        assert_eq!(err.to_string(), "Configuration error: test config error");
    }

    #[test]
    fn test_error_retryable() {
        assert!(AlertingError::network("test").is_retryable());
        assert!(AlertingError::timeout("test").is_retryable());
        assert!(!AlertingError::config("test").is_retryable());
        assert!(!AlertingError::validation("test").is_retryable());
    }

    #[test]
    fn test_error_category() {
        assert_eq!(AlertingError::config("test").category(), "config");
        assert_eq!(AlertingError::network("test").category(), "network");
        assert_eq!(AlertingError::validation("test").category(), "validation");
    }

    #[test]
    fn test_error_from_string() {
        let err: AlertingError = "test error".into();
        assert!(matches!(err, AlertingError::Internal(_)));
    }
}
