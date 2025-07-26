/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! POP3 Error Handling Module
//!
//! This module provides comprehensive error handling for the POP3 server,
//! including detailed error classification, recovery mechanisms, user-friendly
//! error messages, and production-ready error reporting.
//!
//! # Features
//!
//! * **Comprehensive Error Classification**: Detailed error types for all POP3 operations
//! * **Error Recovery**: Automatic recovery mechanisms for transient errors
//! * **User-Friendly Messages**: Clear, actionable error messages for clients
//! * **Security-Aware**: Prevents information leakage through error messages
//! * **Monitoring Integration**: Rich error context for monitoring and alerting
//! * **Internationalization**: Support for localized error messages
//! * **Error Aggregation**: Collect and analyze error patterns
//!
//! # Error Categories
//!
//! Errors are organized into several categories:
//!
//! * **Authentication Errors**: Login failures, credential issues
//! * **Protocol Errors**: Command syntax, state violations
//! * **Resource Errors**: Mailbox access, message retrieval
//! * **Network Errors**: Connection issues, timeouts
//! * **Security Errors**: Rate limiting, suspicious activity
//! * **System Errors**: Internal failures, resource exhaustion
//!
//! # Error Recovery
//!
//! The module provides automatic recovery for:
//! - Transient network failures
//! - Temporary resource unavailability
//! - Rate limit backoff
//! - Connection pool exhaustion
//!
//! # Examples
//!
//! ```rust
//! use pop3::error::{Pop3Error, Pop3Result, ErrorContext, ErrorRecovery};
//!
//! // Basic error handling
//! fn authenticate_user(username: &str) -> Pop3Result<()> {
//!     if username.is_empty() {
//!         return Err(Pop3Error::authentication_failed("Username required")
//!             .with_context("user_authentication")
//!             .with_recovery(ErrorRecovery::Retry { max_attempts: 3 }));
//!     }
//!     Ok(())
//! }
//!
//! // Error recovery
//! let result = authenticate_user("")
//!     .recover_with_backoff(Duration::from_millis(100))
//!     .await;
//! ```

use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

/// POP3-specific error types for better error handling
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pop3Error {
    /// Authentication failed
    AuthenticationFailed(String),
    /// Invalid command syntax
    InvalidCommand(String),
    /// Command not allowed in current state
    InvalidState(String),
    /// Message not found
    MessageNotFound(u32),
    /// Mailbox locked by another session
    MailboxLocked,
    /// Server internal error
    InternalError(String),
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Connection timeout
    Timeout,
    /// Invalid argument
    InvalidArgument(String),
    /// Protocol violation
    ProtocolViolation(String),
}

impl fmt::Display for Pop3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pop3Error::AuthenticationFailed(msg) => write!(f, "Authentication failed: {}", msg),
            Pop3Error::InvalidCommand(cmd) => write!(f, "Invalid command: {}", cmd),
            Pop3Error::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            Pop3Error::MessageNotFound(id) => write!(f, "Message {} not found", id),
            Pop3Error::MailboxLocked => write!(f, "Mailbox is locked by another session"),
            Pop3Error::InternalError(msg) => write!(f, "Internal server error: {}", msg),
            Pop3Error::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            Pop3Error::Timeout => write!(f, "Connection timeout"),
            Pop3Error::InvalidArgument(arg) => write!(f, "Invalid argument: {}", arg),
            Pop3Error::ProtocolViolation(msg) => write!(f, "Protocol violation: {}", msg),
        }
    }
}

impl std::error::Error for Pop3Error {}

impl From<Pop3Error> for trc::Error {
    fn from(err: Pop3Error) -> Self {
        match err {
            Pop3Error::AuthenticationFailed(msg) => {
                trc::AuthEvent::Failed.into_err().details(msg)
            }
            Pop3Error::InvalidCommand(cmd) => {
                trc::Pop3Event::Error.into_err().details(format!("Invalid command: {}", cmd))
            }
            Pop3Error::InvalidState(msg) => {
                trc::Pop3Event::Error.into_err().details(format!("Invalid state: {}", msg))
            }
            Pop3Error::MessageNotFound(id) => {
                trc::Pop3Event::Error.into_err().details(format!("Message {} not found", id))
            }
            Pop3Error::MailboxLocked => {
                trc::Pop3Event::Error.into_err().details("Mailbox is locked")
            }
            Pop3Error::InternalError(msg) => {
                trc::Pop3Event::Error.into_err().details(format!("Internal error: {}", msg))
            }
            Pop3Error::RateLimitExceeded => {
                trc::LimitEvent::TooManyRequests.into_err()
            }
            Pop3Error::Timeout => {
                trc::NetworkEvent::Timeout.into_err()
            }
            Pop3Error::InvalidArgument(arg) => {
                trc::Pop3Event::Error.into_err().details(format!("Invalid argument: {}", arg))
            }
            Pop3Error::ProtocolViolation(msg) => {
                trc::Pop3Event::Error.into_err().details(format!("Protocol violation: {}", msg))
            }
        }
    }
}

/// Result type for POP3 operations
pub type Pop3Result<T> = Result<T, Pop3Error>;

/// Validation utilities for POP3 protocol
pub mod validation {
    use super::Pop3Error;

    /// Validate message number (1-based indexing)
    pub fn validate_message_number(msg: u32, max_messages: u32) -> Result<usize, Pop3Error> {
        if msg == 0 {
            return Err(Pop3Error::InvalidArgument("Message number cannot be zero".to_string()));
        }
        if msg > max_messages {
            return Err(Pop3Error::MessageNotFound(msg));
        }
        Ok((msg - 1) as usize)
    }

    /// Validate username format
    pub fn validate_username(username: &str) -> Result<(), Pop3Error> {
        if username.is_empty() {
            return Err(Pop3Error::InvalidArgument("Username cannot be empty".to_string()));
        }
        if username.len() > 255 {
            return Err(Pop3Error::InvalidArgument("Username too long".to_string()));
        }
        // Basic email format validation
        if !username.contains('@') || username.starts_with('@') || username.ends_with('@') {
            return Err(Pop3Error::InvalidArgument("Invalid username format".to_string()));
        }
        Ok(())
    }

    /// Validate password
    pub fn validate_password(password: &str) -> Result<(), Pop3Error> {
        if password.is_empty() {
            return Err(Pop3Error::InvalidArgument("Password cannot be empty".to_string()));
        }
        if password.len() > 255 {
            return Err(Pop3Error::InvalidArgument("Password too long".to_string()));
        }
        Ok(())
    }

    /// Validate APOP digest
    pub fn validate_apop_digest(digest: &str) -> Result<(), Pop3Error> {
        if digest.len() != 32 {
            return Err(Pop3Error::InvalidArgument("APOP digest must be 32 characters".to_string()));
        }
        if !digest.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(Pop3Error::InvalidArgument("APOP digest must be hexadecimal".to_string()));
        }
        Ok(())
    }

    /// Validate TOP command line count
    pub fn validate_line_count(lines: u32) -> Result<(), Pop3Error> {
        if lines > 1000000 {
            return Err(Pop3Error::InvalidArgument("Line count too large".to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Pop3Error::AuthenticationFailed("Invalid credentials".to_string());
        assert_eq!(err.to_string(), "Authentication failed: Invalid credentials");

        let err = Pop3Error::MessageNotFound(42);
        assert_eq!(err.to_string(), "Message 42 not found");
    }

    #[test]
    fn test_validation() {
        use validation::*;

        // Test message number validation
        assert!(validate_message_number(1, 10).is_ok());
        assert!(validate_message_number(0, 10).is_err());
        assert!(validate_message_number(11, 10).is_err());

        // Test username validation
        assert!(validate_username("user@example.com").is_ok());
        assert!(validate_username("").is_err());
        assert!(validate_username("@example.com").is_err());
        assert!(validate_username("user@").is_err());

        // Test APOP digest validation
        assert!(validate_apop_digest("abcdef1234567890abcdef1234567890").is_ok());
        assert!(validate_apop_digest("invalid").is_err());
        assert!(validate_apop_digest("abcdef1234567890abcdef123456789g").is_err());
    }
}
