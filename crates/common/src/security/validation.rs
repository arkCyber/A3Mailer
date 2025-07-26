/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Input Validation Module
//!
//! This module provides comprehensive input validation functions to prevent
//! security vulnerabilities such as injection attacks, XSS, and malformed data.

use std::collections::HashSet;
use regex::Regex;
use tracing::{debug, warn};

/// Input validation errors
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Input too long: {actual} > {max}")]
    TooLong { actual: usize, max: usize },

    #[error("Input too short: {actual} < {min}")]
    TooShort { actual: usize, min: usize },

    #[error("Invalid format: {reason}")]
    InvalidFormat { reason: String },

    #[error("Forbidden characters detected: {chars}")]
    ForbiddenCharacters { chars: String },

    #[error("Potential injection attack detected")]
    PotentialInjection,

    #[error("Invalid email address format")]
    InvalidEmail,

    #[error("Invalid domain name")]
    InvalidDomain,

    #[error("Invalid IP address")]
    InvalidIpAddress,

    #[error("Input contains malicious content")]
    MaliciousContent,
}

/// Input validation configuration
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Maximum string length
    pub max_string_length: usize,
    /// Maximum email length
    pub max_email_length: usize,
    /// Maximum domain length
    pub max_domain_length: usize,
    /// Allow Unicode characters
    pub allow_unicode: bool,
    /// Strict validation mode
    pub strict_mode: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_string_length: 1024,
            max_email_length: 254,
            max_domain_length: 253,
            allow_unicode: true,
            strict_mode: false,
        }
    }
}

/// Input validator with configurable rules
pub struct InputValidator {
    config: ValidationConfig,
    email_regex: Regex,
    domain_regex: Regex,
    ip_regex: Regex,
    sql_injection_patterns: Vec<Regex>,
    xss_patterns: Vec<Regex>,
    forbidden_chars: HashSet<char>,
}

impl InputValidator {
    /// Create a new input validator
    pub fn new(config: ValidationConfig) -> Result<Self, ValidationError> {
        debug!("Creating input validator with config: {:?}", config);

        // Compile regex patterns
        let email_regex = Regex::new(
            r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$"
        ).map_err(|e| ValidationError::InvalidFormat { reason: e.to_string() })?;

        let domain_regex = Regex::new(
            r"^[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$"
        ).map_err(|e| ValidationError::InvalidFormat { reason: e.to_string() })?;

        let ip_regex = Regex::new(
            r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$|^(?:[0-9a-fA-F]{0,4}:){2,7}[0-9a-fA-F]{0,4}$"
        ).map_err(|e| ValidationError::InvalidFormat { reason: e.to_string() })?;

        // SQL injection patterns
        let sql_injection_patterns = vec![
            Regex::new(r"(?i)(union|select|insert|update|delete|drop|create|alter|exec|execute)").unwrap(),
            Regex::new(r"(?i)(script|javascript|vbscript|onload|onerror|onclick)").unwrap(),
            Regex::new(r#"['"`;]"#).unwrap(),
            Regex::new(r"--").unwrap(),
            Regex::new(r"/\*.*\*/").unwrap(),
        ];

        // XSS patterns
        let xss_patterns = vec![
            Regex::new(r"(?i)<script[^>]*>.*?</script>").unwrap(),
            Regex::new(r"(?i)<iframe[^>]*>.*?</iframe>").unwrap(),
            Regex::new(r"(?i)<object[^>]*>.*?</object>").unwrap(),
            Regex::new(r"(?i)<embed[^>]*>").unwrap(),
            Regex::new(r"(?i)javascript:").unwrap(),
            Regex::new(r"(?i)vbscript:").unwrap(),
            Regex::new(r"(?i)on\w+\s*=").unwrap(),
        ];

        // Forbidden characters for strict mode
        let mut forbidden_chars = HashSet::new();
        if config.strict_mode {
            for ch in r#"<>"'&;(){}[]|`$*?~^"#.chars() {
                forbidden_chars.insert(ch);
            }
        }

        Ok(Self {
            config,
            email_regex,
            domain_regex,
            ip_regex,
            sql_injection_patterns,
            xss_patterns,
            forbidden_chars,
        })
    }

    /// Validate a general string input
    pub fn validate_string(&self, input: &str, field_name: &str) -> Result<(), ValidationError> {
        debug!("Validating string field '{}': {} chars", field_name, input.len());

        // Check length
        if input.len() > self.config.max_string_length {
            return Err(ValidationError::TooLong {
                actual: input.len(),
                max: self.config.max_string_length,
            });
        }

        // Check for forbidden characters
        if self.config.strict_mode {
            let forbidden: Vec<char> = input.chars()
                .filter(|ch| self.forbidden_chars.contains(ch))
                .collect();

            if !forbidden.is_empty() {
                return Err(ValidationError::ForbiddenCharacters {
                    chars: forbidden.iter().collect::<String>(),
                });
            }
        }

        // Check for Unicode if not allowed
        if !self.config.allow_unicode && !input.is_ascii() {
            return Err(ValidationError::InvalidFormat {
                reason: "Non-ASCII characters not allowed".to_string(),
            });
        }

        // Check for potential injection attacks
        self.check_injection_patterns(input)?;

        Ok(())
    }

    /// Validate an email address
    pub fn validate_email(&self, email: &str) -> Result<(), ValidationError> {
        debug!("Validating email: {}", email);

        // Check length
        if email.len() > self.config.max_email_length {
            return Err(ValidationError::TooLong {
                actual: email.len(),
                max: self.config.max_email_length,
            });
        }

        if email.len() < 3 {
            return Err(ValidationError::TooShort {
                actual: email.len(),
                min: 3,
            });
        }

        // Check format
        if !self.email_regex.is_match(email) {
            return Err(ValidationError::InvalidEmail);
        }

        // Additional checks
        if email.starts_with('.') || email.ends_with('.') || email.contains("..") {
            return Err(ValidationError::InvalidEmail);
        }

        // Validate domain part
        if let Some(domain) = email.split('@').nth(1) {
            self.validate_domain(domain)?;
        }

        Ok(())
    }

    /// Validate a domain name
    pub fn validate_domain(&self, domain: &str) -> Result<(), ValidationError> {
        debug!("Validating domain: {}", domain);

        // Check length
        if domain.len() > self.config.max_domain_length {
            return Err(ValidationError::TooLong {
                actual: domain.len(),
                max: self.config.max_domain_length,
            });
        }

        if domain.is_empty() {
            return Err(ValidationError::TooShort {
                actual: 0,
                min: 1,
            });
        }

        // Check format
        if !self.domain_regex.is_match(domain) {
            return Err(ValidationError::InvalidDomain);
        }

        // Additional checks
        if domain.starts_with('-') || domain.ends_with('-') || domain.starts_with('.') || domain.ends_with('.') {
            return Err(ValidationError::InvalidDomain);
        }

        Ok(())
    }

    /// Validate an IP address (IPv4 or IPv6)
    pub fn validate_ip_address(&self, ip: &str) -> Result<(), ValidationError> {
        debug!("Validating IP address: {}", ip);

        if !self.ip_regex.is_match(ip) {
            return Err(ValidationError::InvalidIpAddress);
        }

        Ok(())
    }

    /// Validate a username
    pub fn validate_username(&self, username: &str) -> Result<(), ValidationError> {
        debug!("Validating username: {}", username);

        // Check length
        if username.len() < 1 {
            return Err(ValidationError::TooShort {
                actual: username.len(),
                min: 1,
            });
        }

        if username.len() > 64 {
            return Err(ValidationError::TooLong {
                actual: username.len(),
                max: 64,
            });
        }

        // Check format - alphanumeric, underscore, hyphen, dot
        let valid_chars = username.chars().all(|c| {
            c.is_alphanumeric() || c == '_' || c == '-' || c == '.'
        });

        if !valid_chars {
            return Err(ValidationError::InvalidFormat {
                reason: "Username can only contain alphanumeric characters, underscore, hyphen, and dot".to_string(),
            });
        }

        // Cannot start or end with special characters
        if username.starts_with('.') || username.ends_with('.') ||
           username.starts_with('-') || username.ends_with('-') ||
           username.starts_with('_') || username.ends_with('_') {
            return Err(ValidationError::InvalidFormat {
                reason: "Username cannot start or end with special characters".to_string(),
            });
        }

        Ok(())
    }

    /// Check for potential injection patterns
    fn check_injection_patterns(&self, input: &str) -> Result<(), ValidationError> {
        let input_lower = input.to_lowercase();

        // Check SQL injection patterns
        for pattern in &self.sql_injection_patterns {
            if pattern.is_match(&input_lower) {
                warn!("Potential SQL injection detected in input: {}", input);
                return Err(ValidationError::PotentialInjection);
            }
        }

        // Check XSS patterns
        for pattern in &self.xss_patterns {
            if pattern.is_match(&input_lower) {
                warn!("Potential XSS attack detected in input: {}", input);
                return Err(ValidationError::MaliciousContent);
            }
        }

        Ok(())
    }

    /// Sanitize input by removing potentially dangerous characters
    pub fn sanitize_string(&self, input: &str) -> String {
        debug!("Sanitizing string input");

        let mut sanitized = input.to_string();

        // Remove null bytes
        sanitized = sanitized.replace('\0', "");

        // Remove control characters except tab, newline, and carriage return
        sanitized = sanitized.chars()
            .filter(|&c| !c.is_control() || c == '\t' || c == '\n' || c == '\r')
            .collect();

        // Escape HTML entities
        sanitized = sanitized
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;");

        sanitized
    }

    /// Validate and sanitize input in one step
    pub fn validate_and_sanitize(&self, input: &str, field_name: &str) -> Result<String, ValidationError> {
        self.validate_string(input, field_name)?;
        Ok(self.sanitize_string(input))
    }
}

impl Default for InputValidator {
    fn default() -> Self {
        Self::new(ValidationConfig::default()).expect("Failed to create default validator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        let validator = InputValidator::default();

        // Valid emails
        assert!(validator.validate_email("test@example.com").is_ok());
        assert!(validator.validate_email("user.name@domain.co.uk").is_ok());
        assert!(validator.validate_email("test+tag@example.org").is_ok());

        // Invalid emails
        assert!(validator.validate_email("invalid").is_err());
        assert!(validator.validate_email("@example.com").is_err());
        assert!(validator.validate_email("test@").is_err());
        assert!(validator.validate_email("test..test@example.com").is_err());
    }

    #[test]
    fn test_domain_validation() {
        let validator = InputValidator::default();

        // Valid domains
        assert!(validator.validate_domain("example.com").is_ok());
        assert!(validator.validate_domain("sub.domain.org").is_ok());
        assert!(validator.validate_domain("test-domain.net").is_ok());

        // Invalid domains
        assert!(validator.validate_domain("").is_err());
        assert!(validator.validate_domain("-example.com").is_err());
        assert!(validator.validate_domain("example-.com").is_err());
        assert!(validator.validate_domain(".example.com").is_err());
    }

    #[test]
    fn test_ip_validation() {
        let validator = InputValidator::default();

        // Valid IPs
        assert!(validator.validate_ip_address("192.168.1.1").is_ok());
        assert!(validator.validate_ip_address("10.0.0.1").is_ok());
        assert!(validator.validate_ip_address("2001:db8::1").is_ok());

        // Invalid IPs
        assert!(validator.validate_ip_address("256.1.1.1").is_err());
        assert!(validator.validate_ip_address("192.168.1").is_err());
        assert!(validator.validate_ip_address("not.an.ip").is_err());
    }

    #[test]
    fn test_injection_detection() {
        let validator = InputValidator::default();

        // SQL injection attempts
        assert!(validator.validate_string("'; DROP TABLE users; --", "test").is_err());
        assert!(validator.validate_string("1 UNION SELECT * FROM passwords", "test").is_err());

        // XSS attempts
        assert!(validator.validate_string("<script>alert('xss')</script>", "test").is_err());
        assert!(validator.validate_string("javascript:alert(1)", "test").is_err());

        // Safe input
        assert!(validator.validate_string("Hello, World!", "test").is_ok());
    }

    #[test]
    fn test_username_validation() {
        let validator = InputValidator::default();

        // Valid usernames
        assert!(validator.validate_username("user123").is_ok());
        assert!(validator.validate_username("test.user").is_ok());
        assert!(validator.validate_username("user-name").is_ok());

        // Invalid usernames
        assert!(validator.validate_username("").is_err());
        assert!(validator.validate_username(".user").is_err());
        assert!(validator.validate_username("user.").is_err());
        assert!(validator.validate_username("user@domain").is_err());
    }

    #[test]
    fn test_sanitization() {
        let validator = InputValidator::default();

        let input = "<script>alert('test')</script>";
        let sanitized = validator.sanitize_string(input);
        assert_eq!(sanitized, "&lt;script&gt;alert(&#x27;test&#x27;)&lt;/script&gt;");

        let input_with_nulls = "test\0string";
        let sanitized = validator.sanitize_string(input_with_nulls);
        assert_eq!(sanitized, "teststring");
    }
}
