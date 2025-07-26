/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! MTA-STS (Mail Transfer Agent Strict Transport Security) Implementation
//!
//! This module provides a comprehensive implementation of RFC 8461 - SMTP MTA Strict Transport Security (MTA-STS).
//! MTA-STS is a mechanism enabling mail service providers to declare their ability to receive Transport Layer
//! Security (TLS) secure SMTP connections and to specify whether sending SMTP servers should refuse to deliver
//! to MX hosts that do not offer TLS with a trusted server certificate.
//!
//! # Architecture
//!
//! The MTA-STS implementation consists of several key components:
//!
//! ## Policy Discovery and Validation
//! - DNS TXT record lookup for MTA-STS policy discovery
//! - HTTPS policy file retrieval with security validation
//! - Policy parsing and validation according to RFC 8461
//! - Comprehensive caching with TTL management
//!
//! ## Security Features
//! - Strict certificate validation for policy retrieval
//! - Protection against downgrade attacks
//! - Comprehensive logging and monitoring
//! - Rate limiting and abuse protection
//!
//! ## Performance Characteristics
//! - Asynchronous DNS and HTTP operations
//! - Intelligent caching to minimize network requests
//! - Configurable timeouts and retry logic
//! - Memory-efficient policy storage
//!
//! # Thread Safety
//! All components are designed to be thread-safe and can handle concurrent
//! policy lookups and validations without blocking.
//!
//! # Security Considerations
//! - All policy retrievals use HTTPS with certificate validation
//! - DNS responses are validated for authenticity
//! - Policy files are size-limited to prevent DoS attacks
//! - Comprehensive audit logging for security events
//!
//! # Examples
//! ```rust
//! use crate::outbound::mta_sts::{MtaStsLookup, VerifyPolicy};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Lookup MTA-STS policy for a domain
//! let policy = server.lookup_mta_sts_policy("example.com", Duration::from_secs(30)).await?;
//!
//! // Verify if an MX host is authorized by the policy
//! let is_authorized = policy.verify("mx1.example.com");
//! let enforce_mode = policy.enforce();
//!
//! if enforce_mode && !is_authorized {
//!     // Handle policy violation according to RFC 8461
//!     return Err("MX host not authorized by MTA-STS policy".into());
//! }
//! # Ok(())
//! # }
//! ```

use std::{
    fmt::{self, Display},
    time::Duration,
};

pub mod lookup;
pub mod parse;
pub mod verify;

/// Comprehensive error types for MTA-STS operations
///
/// This enum covers all possible error conditions that can occur during
/// MTA-STS policy discovery, retrieval, parsing, and validation.
#[derive(Debug, Clone)]
pub enum Error {
    /// DNS resolution errors during policy discovery
    ///
    /// This includes failures to resolve the MTA-STS TXT record,
    /// DNSSEC validation failures, and DNS timeout errors.
    Dns {
        /// The underlying DNS error
        source: String,
        /// The domain being queried
        domain: String,
        /// The specific DNS record type that failed
        record_type: String,
    },

    /// HTTP errors during policy file retrieval
    ///
    /// This includes network connectivity issues, HTTP status errors,
    /// TLS certificate validation failures, and timeout errors.
    Http {
        /// The underlying HTTP error
        source: String,
        /// The URL that was being accessed
        url: String,
        /// HTTP status code if available
        status_code: Option<u16>,
    },

    /// Policy parsing and validation errors
    ///
    /// This includes malformed policy files, invalid syntax,
    /// unsupported policy versions, and semantic validation failures.
    InvalidPolicy {
        /// Detailed error description
        reason: String,
        /// The line number where the error occurred (if applicable)
        line_number: Option<usize>,
        /// The invalid content that caused the error
        content: Option<String>,
    },

    /// Policy file size exceeded maximum allowed limit
    ///
    /// This prevents DoS attacks through oversized policy files.
    PolicyTooLarge {
        /// The actual size of the policy file
        actual_size: usize,
        /// The maximum allowed size
        max_size: usize,
    },

    /// Operation timeout error
    ///
    /// This occurs when DNS lookups or HTTP requests exceed
    /// the configured timeout duration.
    Timeout {
        /// The operation that timed out
        operation: String,
        /// The timeout duration that was exceeded
        timeout: Duration,
        /// How long the operation actually took
        elapsed: Duration,
    },

    /// Rate limiting error
    ///
    /// This occurs when too many requests are made to the same
    /// domain within a short time period.
    RateLimited {
        /// The domain being rate limited
        domain: String,
        /// When the rate limit will be reset
        retry_after: Duration,
    },

    /// Cache-related errors
    ///
    /// This includes cache corruption, serialization failures,
    /// and cache storage errors.
    Cache {
        /// The cache operation that failed
        operation: String,
        /// The underlying error
        source: String,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Dns { source, domain, record_type } => {
                write!(f, "DNS lookup failed for {} record of domain '{}': {}",
                       record_type, domain, source)
            }
            Error::Http { source, url, status_code } => {
                if let Some(code) = status_code {
                    write!(f, "HTTP request failed for '{}' with status {}: {}",
                           url, code, source)
                } else {
                    write!(f, "HTTP request failed for '{}': {}", url, source)
                }
            }
            Error::InvalidPolicy { reason, line_number, content } => {
                if let Some(line) = line_number {
                    write!(f, "Invalid MTA-STS policy at line {}: {}", line, reason)?;
                    if let Some(content) = content {
                        write!(f, " (content: '{}')", content)?;
                    }
                    Ok(())
                } else {
                    write!(f, "Invalid MTA-STS policy: {}", reason)
                }
            }
            Error::PolicyTooLarge { actual_size, max_size } => {
                write!(f, "MTA-STS policy file too large: {} bytes (max: {} bytes)",
                       actual_size, max_size)
            }
            Error::Timeout { operation, timeout, elapsed } => {
                write!(f, "Operation '{}' timed out after {:?} (limit: {:?})",
                       operation, elapsed, timeout)
            }
            Error::RateLimited { domain, retry_after } => {
                write!(f, "Rate limited for domain '{}', retry after {:?}",
                       domain, retry_after)
            }
            Error::Cache { operation, source } => {
                write!(f, "Cache operation '{}' failed: {}", operation, source)
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Note: We store error sources as strings for simplicity,
        // but in a production system you might want to preserve
        // the original error types for better error chaining.
        None
    }
}

// Conversion implementations for common error types
impl From<mail_auth::Error> for Error {
    fn from(err: mail_auth::Error) -> Self {
        Error::Dns {
            source: err.to_string(),
            domain: "unknown".to_string(),
            record_type: "TXT".to_string(),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        let status_code = err.status().map(|s| s.as_u16());
        let url = err.url().map(|u| u.to_string()).unwrap_or_else(|| "unknown".to_string());

        Error::Http {
            source: err.to_string(),
            url,
            status_code,
        }
    }
}

/// MTA-STS policy enforcement modes as defined in RFC 8461
///
/// These modes determine how strictly the policy should be enforced
/// and what actions should be taken when policy violations occur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyMode {
    /// No policy enforcement - used for testing and gradual rollout
    ///
    /// In this mode, policy violations are logged but do not affect
    /// mail delivery. This is useful for monitoring and debugging.
    None,

    /// Testing mode - policy violations are reported but not enforced
    ///
    /// This mode allows organizations to test their MTA-STS configuration
    /// without risking mail delivery failures.
    Testing,

    /// Enforcement mode - policy violations result in delivery failure
    ///
    /// This is the strictest mode where policy violations will cause
    /// mail delivery to fail, providing maximum security.
    Enforce,
}

impl Display for PolicyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyMode::None => write!(f, "none"),
            PolicyMode::Testing => write!(f, "testing"),
            PolicyMode::Enforce => write!(f, "enforce"),
        }
    }
}

impl PolicyMode {
    /// Parses a policy mode from a string value
    ///
    /// # Arguments
    /// * `value` - The string value to parse
    ///
    /// # Returns
    /// The parsed policy mode or an error if the value is invalid
    pub fn from_str(value: &str) -> Result<Self, Error> {
        match value.to_lowercase().as_str() {
            "none" => Ok(PolicyMode::None),
            "testing" => Ok(PolicyMode::Testing),
            "enforce" => Ok(PolicyMode::Enforce),
            _ => Err(Error::InvalidPolicy {
                reason: format!("Invalid policy mode: '{}'", value),
                line_number: None,
                content: Some(value.to_string()),
            }),
        }
    }

    /// Returns true if this mode enforces policy violations
    pub fn is_enforcing(&self) -> bool {
        matches!(self, PolicyMode::Enforce)
    }

    /// Returns true if this mode should generate reports for violations
    pub fn should_report(&self) -> bool {
        matches!(self, PolicyMode::Testing | PolicyMode::Enforce)
    }
}
