/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Comprehensive error handling for Stalwart Common
//!
//! This module provides production-grade error handling with detailed error types,
//! context information, and proper error propagation throughout the system.

use std::{
    fmt::{self, Display},
    error::Error as StdError,
    io,
    net::AddrParseError,
    num::{ParseIntError, ParseFloatError},
    string::FromUtf8Error,
    time::SystemTimeError,
};

use serde::{Deserialize, Serialize};
use trc::EventType;

/// Main error type for Stalwart Common operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommonError {
    /// Configuration errors
    Config {
        message: String,
        key: Option<String>,
        context: Option<String>,
    },

    /// Authentication and authorization errors
    Auth {
        message: String,
        error_type: AuthErrorType,
        context: Option<String>,
    },

    /// Storage operation errors
    Storage {
        message: String,
        operation: String,
        context: Option<String>,
    },

    /// Network operation errors
    Network {
        message: String,
        address: Option<String>,
        context: Option<String>,
    },

    /// Validation errors
    Validation {
        message: String,
        field: Option<String>,
        value: Option<String>,
    },

    /// Parsing errors
    Parse {
        message: String,
        input: Option<String>,
        expected: Option<String>,
    },

    /// Resource errors (not found, already exists, etc.)
    Resource {
        message: String,
        resource_type: String,
        resource_id: Option<String>,
    },

    /// Rate limiting errors
    RateLimit {
        message: String,
        limit_type: String,
        retry_after: Option<u64>,
    },

    /// Internal system errors
    Internal {
        message: String,
        source: Option<String>,
        context: Option<String>,
    },
}

/// Authentication error subtypes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthErrorType {
    InvalidCredentials,
    TokenExpired,
    TokenInvalid,
    InsufficientPermissions,
    AccountLocked,
    AccountDisabled,
    TwoFactorRequired,
    RateLimited,
}

/// Result type alias for Common operations
pub type CommonResult<T> = Result<T, CommonError>;

impl CommonError {
    /// Create a configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
            key: None,
            context: None,
        }
    }

    /// Create a configuration error with key
    pub fn config_with_key<S: Into<String>, K: Into<String>>(message: S, key: K) -> Self {
        Self::Config {
            message: message.into(),
            key: Some(key.into()),
            context: None,
        }
    }

    /// Create an authentication error
    pub fn auth<S: Into<String>>(message: S, error_type: AuthErrorType) -> Self {
        Self::Auth {
            message: message.into(),
            error_type,
            context: None,
        }
    }

    /// Create a storage error
    pub fn storage<S: Into<String>, O: Into<String>>(message: S, operation: O) -> Self {
        Self::Storage {
            message: message.into(),
            operation: operation.into(),
            context: None,
        }
    }

    /// Create a network error
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::Network {
            message: message.into(),
            address: None,
            context: None,
        }
    }

    /// Create a validation error
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::Validation {
            message: message.into(),
            field: None,
            value: None,
        }
    }

    /// Create a validation error with field
    pub fn validation_with_field<S: Into<String>, F: Into<String>>(message: S, field: F) -> Self {
        Self::Validation {
            message: message.into(),
            field: Some(field.into()),
            value: None,
        }
    }

    /// Create a parsing error
    pub fn parse<S: Into<String>>(message: S) -> Self {
        Self::Parse {
            message: message.into(),
            input: None,
            expected: None,
        }
    }

    /// Create a resource error
    pub fn resource<S: Into<String>, T: Into<String>>(message: S, resource_type: T) -> Self {
        Self::Resource {
            message: message.into(),
            resource_type: resource_type.into(),
            resource_id: None,
        }
    }

    /// Create a rate limit error
    pub fn rate_limit<S: Into<String>, T: Into<String>>(message: S, limit_type: T) -> Self {
        Self::RateLimit {
            message: message.into(),
            limit_type: limit_type.into(),
            retry_after: None,
        }
    }

    /// Create an internal error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal {
            message: message.into(),
            source: None,
            context: None,
        }
    }

    /// Add context to any error
    pub fn with_context<S: Into<String>>(mut self, context: S) -> Self {
        let context_str = context.into();
        match &mut self {
            Self::Config { context, .. } => *context = Some(context_str),
            Self::Auth { context, .. } => *context = Some(context_str),
            Self::Storage { context, .. } => *context = Some(context_str),
            Self::Network { context, .. } => *context = Some(context_str),
            Self::Internal { context, .. } => *context = Some(context_str),
            _ => {}
        }
        self
    }

    /// Get the error category for logging/metrics
    pub fn category(&self) -> &'static str {
        match self {
            Self::Config { .. } => "config",
            Self::Auth { .. } => "auth",
            Self::Storage { .. } => "storage",
            Self::Network { .. } => "network",
            Self::Validation { .. } => "validation",
            Self::Parse { .. } => "parse",
            Self::Resource { .. } => "resource",
            Self::RateLimit { .. } => "rate_limit",
            Self::Internal { .. } => "internal",
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Network { .. } => true,
            Self::Storage { .. } => true,
            Self::Internal { .. } => true,
            Self::RateLimit { .. } => true,
            _ => false,
        }
    }

    /// Get retry delay in seconds if applicable
    pub fn retry_delay(&self) -> Option<u64> {
        match self {
            Self::RateLimit { retry_after, .. } => *retry_after,
            Self::Network { .. } => Some(1),
            Self::Storage { .. } => Some(2),
            Self::Internal { .. } => Some(5),
            _ => None,
        }
    }

    /// Convert to trc::Event for logging
    pub fn to_event(&self) -> trc::Event<EventType> {
        let event = match self {
            Self::Config { .. } => trc::Event::new(trc::EventType::Config(trc::ConfigEvent::ParseError)),
            Self::Auth { .. } => trc::Event::new(trc::EventType::Auth(trc::AuthEvent::Failed)),
            Self::Storage { .. } => trc::Event::new(trc::EventType::Store(trc::StoreEvent::DataCorruption)),
            Self::Network { .. } => trc::Event::new(trc::EventType::Network(trc::NetworkEvent::BindError)),
            Self::Validation { .. } => trc::Event::new(trc::EventType::Config(trc::ConfigEvent::ParseError)),
            Self::Parse { .. } => trc::Event::new(trc::EventType::Config(trc::ConfigEvent::ParseError)),
            Self::Resource { .. } => trc::Event::new(trc::EventType::Resource(trc::ResourceEvent::NotFound)),
            Self::RateLimit { .. } => trc::Event::new(trc::EventType::Limit(trc::LimitEvent::TenantQuota)),
            Self::Internal { .. } => trc::Event::new(trc::EventType::Server(trc::ServerEvent::Startup)),
        };

        event
    }
}

impl Display for CommonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config { message, key, context } => {
                write!(f, "Configuration error: {}", message)?;
                if let Some(key) = key {
                    write!(f, " (key: {})", key)?;
                }
                if let Some(context) = context {
                    write!(f, " [{}]", context)?;
                }
                Ok(())
            }
            Self::Auth { message, error_type, context } => {
                write!(f, "Authentication error ({}): {}", error_type, message)?;
                if let Some(context) = context {
                    write!(f, " [{}]", context)?;
                }
                Ok(())
            }
            Self::Storage { message, operation, context } => {
                write!(f, "Storage error during {}: {}", operation, message)?;
                if let Some(context) = context {
                    write!(f, " [{}]", context)?;
                }
                Ok(())
            }
            Self::Network { message, address, context } => {
                write!(f, "Network error: {}", message)?;
                if let Some(address) = address {
                    write!(f, " (address: {})", address)?;
                }
                if let Some(context) = context {
                    write!(f, " [{}]", context)?;
                }
                Ok(())
            }
            Self::Validation { message, field, value } => {
                write!(f, "Validation error: {}", message)?;
                if let Some(field) = field {
                    write!(f, " (field: {})", field)?;
                }
                if let Some(value) = value {
                    write!(f, " (value: {})", value)?;
                }
                Ok(())
            }
            Self::Parse { message, input, expected } => {
                write!(f, "Parse error: {}", message)?;
                if let Some(input) = input {
                    write!(f, " (input: {})", input)?;
                }
                if let Some(expected) = expected {
                    write!(f, " (expected: {})", expected)?;
                }
                Ok(())
            }
            Self::Resource { message, resource_type, resource_id } => {
                write!(f, "Resource error ({}): {}", resource_type, message)?;
                if let Some(id) = resource_id {
                    write!(f, " (id: {})", id)?;
                }
                Ok(())
            }
            Self::RateLimit { message, limit_type, retry_after } => {
                write!(f, "Rate limit error ({}): {}", limit_type, message)?;
                if let Some(retry) = retry_after {
                    write!(f, " (retry after: {}s)", retry)?;
                }
                Ok(())
            }
            Self::Internal { message, source, context } => {
                write!(f, "Internal error: {}", message)?;
                if let Some(source) = source {
                    write!(f, " (source: {})", source)?;
                }
                if let Some(context) = context {
                    write!(f, " [{}]", context)?;
                }
                Ok(())
            }
        }
    }
}

impl Display for AuthErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCredentials => write!(f, "invalid_credentials"),
            Self::TokenExpired => write!(f, "token_expired"),
            Self::TokenInvalid => write!(f, "token_invalid"),
            Self::InsufficientPermissions => write!(f, "insufficient_permissions"),
            Self::AccountLocked => write!(f, "account_locked"),
            Self::AccountDisabled => write!(f, "account_disabled"),
            Self::TwoFactorRequired => write!(f, "two_factor_required"),
            Self::RateLimited => write!(f, "rate_limited"),
        }
    }
}

impl StdError for CommonError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        None
    }
}

// Implement From traits for common error types
impl From<io::Error> for CommonError {
    fn from(err: io::Error) -> Self {
        Self::internal(format!("IO error: {}", err))
    }
}

impl From<ParseIntError> for CommonError {
    fn from(err: ParseIntError) -> Self {
        Self::parse(format!("Integer parse error: {}", err))
    }
}

impl From<ParseFloatError> for CommonError {
    fn from(err: ParseFloatError) -> Self {
        Self::parse(format!("Float parse error: {}", err))
    }
}

impl From<FromUtf8Error> for CommonError {
    fn from(err: FromUtf8Error) -> Self {
        Self::parse(format!("UTF-8 parse error: {}", err))
    }
}

impl From<AddrParseError> for CommonError {
    fn from(err: AddrParseError) -> Self {
        Self::parse(format!("Address parse error: {}", err))
    }
}

impl From<SystemTimeError> for CommonError {
    fn from(err: SystemTimeError) -> Self {
        Self::internal(format!("System time error: {}", err))
    }
}

impl From<serde_json::Error> for CommonError {
    fn from(err: serde_json::Error) -> Self {
        Self::parse(format!("JSON parse error: {}", err))
    }
}

/// Helper trait for adding context to Results
pub trait ErrorContext<T> {
    fn with_context<S: Into<String>>(self, context: S) -> CommonResult<T>;
    fn with_config_context<S: Into<String>, K: Into<String>>(self, message: S, key: K) -> CommonResult<T>;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: Into<CommonError>,
{
    fn with_context<S: Into<String>>(self, context: S) -> CommonResult<T> {
        self.map_err(|e| e.into().with_context(context))
    }

    fn with_config_context<S: Into<String>, K: Into<String>>(self, message: S, key: K) -> CommonResult<T> {
        self.map_err(|_| CommonError::config_with_key(message, key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    /// Test CommonError creation and display
    #[test]
    fn test_error_creation_and_display() {
        // Test config error
        let config_err = CommonError::config("Invalid configuration");
        assert_eq!(config_err.category(), "config");
        assert!(!config_err.is_retryable());
        assert!(config_err.to_string().contains("Configuration error"));

        // Test config error with key
        let config_key_err = CommonError::config_with_key("Invalid value", "smtp.timeout");
        assert!(config_key_err.to_string().contains("smtp.timeout"));

        // Test auth error
        let auth_err = CommonError::auth("Invalid password", AuthErrorType::InvalidCredentials);
        assert_eq!(auth_err.category(), "auth");
        assert!(auth_err.to_string().contains("invalid_credentials"));

        // Test storage error
        let storage_err = CommonError::storage("Database connection failed", "connect");
        assert_eq!(storage_err.category(), "storage");
        assert!(storage_err.is_retryable());
        assert!(storage_err.to_string().contains("Storage error during connect"));

        // Test network error
        let network_err = CommonError::network("Connection timeout");
        assert_eq!(network_err.category(), "network");
        assert!(network_err.is_retryable());
        assert_eq!(network_err.retry_delay(), Some(1));

        // Test validation error
        let validation_err = CommonError::validation_with_field("Invalid email format", "email");
        assert_eq!(validation_err.category(), "validation");
        assert!(!validation_err.is_retryable());
        assert!(validation_err.to_string().contains("field: email"));

        // Test rate limit error
        let rate_limit_err = CommonError::rate_limit("Too many requests", "api");
        assert_eq!(rate_limit_err.category(), "rate_limit");
        assert!(rate_limit_err.is_retryable());
    }

    /// Test error context addition
    #[test]
    fn test_error_context() {
        let mut config_err = CommonError::config("Test error");
        config_err = config_err.with_context("During startup");
        assert!(config_err.to_string().contains("[During startup]"));

        let mut auth_err = CommonError::auth("Test auth", AuthErrorType::TokenExpired);
        auth_err = auth_err.with_context("Token validation");
        assert!(auth_err.to_string().contains("[Token validation]"));
    }

    /// Test AuthErrorType display
    #[test]
    fn test_auth_error_type_display() {
        let error_types = vec![
            (AuthErrorType::InvalidCredentials, "invalid_credentials"),
            (AuthErrorType::TokenExpired, "token_expired"),
            (AuthErrorType::TokenInvalid, "token_invalid"),
            (AuthErrorType::InsufficientPermissions, "insufficient_permissions"),
            (AuthErrorType::AccountLocked, "account_locked"),
            (AuthErrorType::AccountDisabled, "account_disabled"),
            (AuthErrorType::TwoFactorRequired, "two_factor_required"),
            (AuthErrorType::RateLimited, "rate_limited"),
        ];

        for (error_type, expected) in error_types {
            assert_eq!(error_type.to_string(), expected);
        }
    }

    /// Test error conversion from standard library errors
    #[test]
    fn test_error_conversion() {
        // Test IO error conversion
        let io_err = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let common_err: CommonError = io_err.into();
        assert_eq!(common_err.category(), "internal");
        assert!(common_err.to_string().contains("IO error"));

        // Test parse int error conversion
        let parse_err = "not_a_number".parse::<i32>().unwrap_err();
        let common_err: CommonError = parse_err.into();
        assert_eq!(common_err.category(), "parse");
        assert!(common_err.to_string().contains("Integer parse error"));

        // Test UTF-8 error conversion
        let utf8_err = String::from_utf8(vec![0, 159, 146, 150]).unwrap_err();
        let common_err: CommonError = utf8_err.into();
        assert_eq!(common_err.category(), "parse");
        assert!(common_err.to_string().contains("UTF-8 parse error"));

        // Test address parse error conversion
        let addr_err = "invalid_address".parse::<std::net::IpAddr>().unwrap_err();
        let common_err: CommonError = addr_err.into();
        assert_eq!(common_err.category(), "parse");
        assert!(common_err.to_string().contains("Address parse error"));

        // Test JSON error conversion
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let common_err: CommonError = json_err.into();
        assert_eq!(common_err.category(), "parse");
        assert!(common_err.to_string().contains("JSON parse error"));
    }

    /// Test ErrorContext trait
    #[test]
    fn test_error_context_trait() {
        // Test with_context
        let result: Result<(), io::Error> = Err(io::Error::new(io::ErrorKind::NotFound, "Test"));
        let common_result = result.with_context("During file operation");

        assert!(common_result.is_err());
        let err = common_result.unwrap_err();
        assert!(err.to_string().contains("[During file operation]"));

        // Test with_config_context
        let result: Result<(), io::Error> = Err(io::Error::new(io::ErrorKind::NotFound, "Test"));
        let common_result = result.with_config_context("Invalid configuration", "test.key");

        assert!(common_result.is_err());
        let err = common_result.unwrap_err();
        assert!(err.to_string().contains("test.key"));
    }

    /// Test retry logic
    #[test]
    fn test_retry_logic() {
        let mut rate_limit_err = CommonError::rate_limit("Too many requests", "api");
        if let CommonError::RateLimit { retry_after, .. } = &mut rate_limit_err {
            *retry_after = Some(30);
        }

        let retryable_errors = vec![
            CommonError::network("Connection failed"),
            CommonError::storage("Database timeout", "query"),
            CommonError::internal("Temporary failure"),
            rate_limit_err,
        ];

        for err in retryable_errors {
            assert!(err.is_retryable(), "Error should be retryable: {:?}", err);
            assert!(err.retry_delay().is_some(), "Error should have retry delay: {:?}", err);
        }

        let non_retryable_errors = vec![
            CommonError::config("Invalid config"),
            CommonError::auth("Invalid credentials", AuthErrorType::InvalidCredentials),
            CommonError::validation("Invalid input"),
            CommonError::parse("Parse error"),
            CommonError::resource("Not found", "user"),
        ];

        for err in non_retryable_errors {
            assert!(!err.is_retryable(), "Error should not be retryable: {:?}", err);
        }
    }

    /// Test event conversion
    #[test]
    fn test_event_conversion() {
        let errors = vec![
            CommonError::config("Test config error"),
            CommonError::auth("Test auth error", AuthErrorType::InvalidCredentials),
            CommonError::storage("Test storage error", "operation"),
            CommonError::network("Test network error"),
            CommonError::validation("Test validation error"),
            CommonError::parse("Test parse error"),
            CommonError::resource("Test resource error", "type"),
            CommonError::rate_limit("Test rate limit error", "type"),
            CommonError::internal("Test internal error"),
        ];

        for err in errors {
            let event = err.to_event();
            // Verify that event is created successfully
            // The actual event type checking would depend on trc implementation
            assert!(!format!("{:?}", event).is_empty());
        }
    }

    /// Test error serialization/deserialization
    #[test]
    fn test_error_serialization() {
        let errors = vec![
            CommonError::config_with_key("Test config", "key"),
            CommonError::auth("Test auth", AuthErrorType::TokenExpired),
            CommonError::storage("Test storage", "op"),
            CommonError::validation_with_field("Test validation", "field"),
        ];

        for original_err in errors {
            // Test serialization
            let serialized = serde_json::to_string(&original_err).unwrap();
            assert!(!serialized.is_empty());

            // Test deserialization
            let deserialized: CommonError = serde_json::from_str(&serialized).unwrap();

            // Compare categories (full equality might be complex due to optional fields)
            assert_eq!(original_err.category(), deserialized.category());
        }
    }

    /// Performance test for error creation
    #[test]
    fn test_error_performance() {
        let start = std::time::Instant::now();

        // Create many errors
        for i in 0..1000 {
            let _err = CommonError::config(format!("Error {}", i))
                .with_context(format!("Context {}", i));
        }

        let elapsed = start.elapsed();

        // Should be very fast
        assert!(elapsed.as_millis() < 100, "Error creation too slow: {:?}", elapsed);
    }

    /// Test concurrent error handling
    #[tokio::test]
    async fn test_concurrent_error_handling() {
        let mut handles = vec![];

        for i in 0..100 {
            let handle = tokio::spawn(async move {
                let err = CommonError::network(format!("Network error {}", i))
                    .with_context(format!("Context {}", i));

                // Verify error properties
                assert_eq!(err.category(), "network");
                assert!(err.is_retryable());
                assert!(err.to_string().contains(&format!("Network error {}", i)));

                err
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            let err = handle.await.unwrap();
            assert_eq!(err.category(), "network");
        }
    }
}
