/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! DAV Server Security Module
//!
//! This module provides comprehensive security features for the DAV server,
//! including input validation, access control, rate limiting, and security headers.

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use hyper::{HeaderMap, StatusCode};
use tracing::{debug, info, warn, error};
use crate::DavError;

/// Security manager for DAV operations
///
/// Provides comprehensive security features including rate limiting,
/// input validation, access control, and security monitoring.
#[derive(Debug, Clone)]
pub struct DavSecurity {
    inner: Arc<RwLock<DavSecurityInner>>,
    config: SecurityConfig,
}

#[derive(Debug)]
struct DavSecurityInner {
    /// Rate limiting state by IP address
    rate_limits: HashMap<IpAddr, RateLimitState>,
    /// Failed authentication attempts by IP
    auth_failures: HashMap<IpAddr, AuthFailureState>,
    /// Blocked IP addresses
    blocked_ips: HashMap<IpAddr, BlockState>,
    /// Security event log
    security_events: Vec<SecurityEvent>,
}

/// Security configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityConfig {
    /// Maximum requests per minute per IP
    pub rate_limit_per_minute: u32,
    /// Maximum authentication failures before blocking
    pub max_auth_failures: u32,
    /// Duration to block IP after max failures
    pub block_duration: Duration,
    /// Maximum request body size in bytes
    pub max_body_size: usize,
    /// Enable security headers
    pub enable_security_headers: bool,
    /// Maximum path depth
    pub max_path_depth: usize,
    /// Allowed file extensions for uploads
    pub allowed_extensions: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            rate_limit_per_minute: 60,
            max_auth_failures: 5,
            block_duration: Duration::from_secs(300), // 5 minutes
            max_body_size: 10 * 1024 * 1024, // 10MB
            enable_security_headers: true,
            max_path_depth: 10,
            allowed_extensions: vec![
                "ics".to_string(),
                "vcf".to_string(),
                "txt".to_string(),
                "xml".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone)]
struct RateLimitState {
    requests: Vec<Instant>,
    last_cleanup: Instant,
}

#[derive(Debug, Clone)]
struct AuthFailureState {
    failures: u32,
    last_failure: Instant,
}

#[derive(Debug, Clone)]
struct BlockState {
    blocked_at: Instant,
    reason: String,
}

#[derive(Debug, Clone)]
pub struct SecurityEvent {
    pub timestamp: Instant,
    pub event_type: SecurityEventType,
    pub ip_address: Option<IpAddr>,
    pub details: String,
}

#[derive(Debug, Clone)]
pub enum SecurityEventType {
    RateLimitExceeded,
    AuthenticationFailure,
    IpBlocked,
    SuspiciousRequest,
    InvalidInput,
    AccessDenied,
}

impl DavSecurity {
    /// Create a new security manager
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(DavSecurityInner {
                rate_limits: HashMap::new(),
                auth_failures: HashMap::new(),
                blocked_ips: HashMap::new(),
                security_events: Vec::new(),
            })),
            config,
        }
    }

    /// Check if a request is allowed from the given IP address
    pub fn check_rate_limit(&self, ip: IpAddr) -> Result<(), DavError> {
        if let Ok(mut inner) = self.inner.write() {
            // Check if IP is blocked
            if let Some(block_state) = inner.blocked_ips.get(&ip) {
                if block_state.blocked_at.elapsed() < self.config.block_duration {
                    warn!(ip = %ip, "Request from blocked IP address");
                    return Err(DavError::auth("IP address is blocked", StatusCode::FORBIDDEN));
                } else {
                    // Block has expired, remove it
                    inner.blocked_ips.remove(&ip);
                    info!(ip = %ip, "IP block expired, removing from blocklist");
                }
            }

            // Check rate limit
            let now = Instant::now();
            let rate_state = inner.rate_limits.entry(ip).or_insert_with(|| RateLimitState {
                requests: Vec::new(),
                last_cleanup: now,
            });

            // Clean up old requests (older than 1 minute)
            if rate_state.last_cleanup.elapsed() > Duration::from_secs(60) {
                let cutoff = now - Duration::from_secs(60);
                rate_state.requests.retain(|&req_time| req_time > cutoff);
                rate_state.last_cleanup = now;
            }

            // Check if rate limit is exceeded
            if rate_state.requests.len() >= self.config.rate_limit_per_minute as usize {
                let request_count = rate_state.requests.len();

                self.record_security_event(
                    &mut inner,
                    SecurityEventType::RateLimitExceeded,
                    Some(ip),
                    format!("Rate limit exceeded: {} requests in last minute", request_count),
                );

                warn!(
                    ip = %ip,
                    requests = request_count,
                    limit = self.config.rate_limit_per_minute,
                    "Rate limit exceeded"
                );

                return Err(DavError::auth("Rate limit exceeded", StatusCode::TOO_MANY_REQUESTS));
            }

            // Record this request
            rate_state.requests.push(now);

            debug!(
                ip = %ip,
                requests = rate_state.requests.len(),
                "Rate limit check passed"
            );
        }

        Ok(())
    }

    /// Record an authentication failure
    pub fn record_auth_failure(&self, ip: IpAddr, reason: &str) {
        if let Ok(mut inner) = self.inner.write() {
            let now = Instant::now();
            let failure_state = inner.auth_failures.entry(ip).or_insert_with(|| AuthFailureState {
                failures: 0,
                last_failure: now,
            });

            failure_state.failures += 1;
            failure_state.last_failure = now;

            let failure_count = failure_state.failures;

            self.record_security_event(
                &mut inner,
                SecurityEventType::AuthenticationFailure,
                Some(ip),
                format!("Authentication failure: {} (attempt {})", reason, failure_count),
            );

            warn!(
                ip = %ip,
                failures = failure_count,
                reason = reason,
                "Authentication failure recorded"
            );

            // Block IP if too many failures
            if failure_count >= self.config.max_auth_failures {
                inner.blocked_ips.insert(ip, BlockState {
                    blocked_at: now,
                    reason: format!("Too many authentication failures ({})", failure_count),
                });

                self.record_security_event(
                    &mut inner,
                    SecurityEventType::IpBlocked,
                    Some(ip),
                    format!("IP blocked due to {} authentication failures", failure_count),
                );

                error!(
                    ip = %ip,
                    failures = failure_count,
                    "IP address blocked due to repeated authentication failures"
                );
            }
        }
    }

    /// Validate request path for security issues
    pub fn validate_path(&self, path: &str) -> Result<(), DavError> {
        // Check path depth
        let depth = path.split('/').filter(|s| !s.is_empty()).count();
        if depth > self.config.max_path_depth {
            return Err(DavError::validation_with_field(
                format!("Path depth {} exceeds maximum {}", depth, self.config.max_path_depth),
                "path",
            ));
        }

        // Check for path traversal attempts
        if path.contains("..") || path.contains("./") || path.contains("\\") {
            return Err(DavError::validation_with_field(
                "Path contains invalid characters or traversal attempts",
                "path",
            ));
        }

        // Check for null bytes
        if path.contains('\0') {
            return Err(DavError::validation_with_field(
                "Path contains null bytes",
                "path",
            ));
        }

        // Check for control characters
        if path.chars().any(|c| c.is_control() && c != '\t') {
            return Err(DavError::validation_with_field(
                "Path contains control characters",
                "path",
            ));
        }

        debug!(path = path, "Path validation passed");
        Ok(())
    }

    /// Validate request body size
    pub fn validate_body_size(&self, size: usize) -> Result<(), DavError> {
        if size > self.config.max_body_size {
            return Err(DavError::validation_with_field(
                format!("Request body size {} exceeds maximum {}", size, self.config.max_body_size),
                "content-length",
            ));
        }
        Ok(())
    }

    /// Validate file extension for uploads
    pub fn validate_file_extension(&self, filename: &str) -> Result<(), DavError> {
        if let Some(extension) = filename.split('.').last() {
            let extension = extension.to_lowercase();
            if !self.config.allowed_extensions.contains(&extension) {
                return Err(DavError::validation_with_field(
                    format!("File extension '{}' is not allowed", extension),
                    "filename",
                ));
            }
        }
        Ok(())
    }

    /// Generate security headers for responses
    pub fn generate_security_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        if self.config.enable_security_headers {
            // Content Security Policy
            headers.insert(
                "Content-Security-Policy",
                "default-src 'self'; script-src 'none'; object-src 'none'".parse().unwrap(),
            );

            // X-Frame-Options
            headers.insert("X-Frame-Options", "DENY".parse().unwrap());

            // X-Content-Type-Options
            headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());

            // Referrer Policy
            headers.insert("Referrer-Policy", "strict-origin-when-cross-origin".parse().unwrap());

            // Permissions Policy
            headers.insert(
                "Permissions-Policy",
                "geolocation=(), microphone=(), camera=()".parse().unwrap(),
            );
        }

        headers
    }

    /// Get security statistics
    pub fn get_security_stats(&self) -> SecurityStats {
        if let Ok(inner) = self.inner.read() {
            SecurityStats {
                blocked_ips: inner.blocked_ips.len(),
                rate_limited_ips: inner.rate_limits.len(),
                auth_failures: inner.auth_failures.values().map(|s| s.failures as u64).sum(),
                security_events: inner.security_events.len(),
                recent_events: inner.security_events.iter()
                    .rev()
                    .take(10)
                    .cloned()
                    .collect(),
            }
        } else {
            SecurityStats::default()
        }
    }

    /// Clear expired entries and clean up state
    pub fn cleanup(&self) {
        if let Ok(mut inner) = self.inner.write() {
            let _now = Instant::now();

            // Remove expired blocks
            inner.blocked_ips.retain(|_, block_state| {
                block_state.blocked_at.elapsed() < self.config.block_duration
            });

            // Remove old auth failures (older than block duration)
            inner.auth_failures.retain(|_, failure_state| {
                failure_state.last_failure.elapsed() < self.config.block_duration
            });

            // Remove old rate limit entries
            inner.rate_limits.retain(|_, rate_state| {
                rate_state.last_cleanup.elapsed() < Duration::from_secs(300) // 5 minutes
            });

            // Keep only recent security events (last 1000)
            if inner.security_events.len() > 1000 {
                let events_len = inner.security_events.len();
                inner.security_events.drain(0..events_len - 1000);
            }

            debug!("Security state cleanup completed");
        }
    }

    fn record_security_event(
        &self,
        inner: &mut DavSecurityInner,
        event_type: SecurityEventType,
        ip_address: Option<IpAddr>,
        details: String,
    ) {
        inner.security_events.push(SecurityEvent {
            timestamp: Instant::now(),
            event_type,
            ip_address,
            details,
        });
    }
}

/// Security statistics
#[derive(Debug, Clone, Default)]
pub struct SecurityStats {
    /// Number of currently blocked IP addresses
    pub blocked_ips: usize,
    /// Number of IP addresses with rate limiting state
    pub rate_limited_ips: usize,
    /// Total authentication failures
    pub auth_failures: u64,
    /// Total security events recorded
    pub security_events: usize,
    /// Recent security events
    pub recent_events: Vec<SecurityEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_rate_limiting() {
        let config = SecurityConfig {
            rate_limit_per_minute: 5,
            ..Default::default()
        };
        let security = DavSecurity::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // Should allow first 5 requests
        for _ in 0..5 {
            assert!(security.check_rate_limit(ip).is_ok());
        }

        // 6th request should be rate limited
        assert!(security.check_rate_limit(ip).is_err());
    }

    #[test]
    fn test_auth_failure_blocking() {
        let config = SecurityConfig {
            max_auth_failures: 3,
            ..Default::default()
        };
        let security = DavSecurity::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));

        // Record auth failures
        for i in 1..=3 {
            security.record_auth_failure(ip, &format!("failure {}", i));
        }

        // IP should now be blocked
        assert!(security.check_rate_limit(ip).is_err());
    }

    #[test]
    fn test_path_validation() {
        let security = DavSecurity::new(SecurityConfig::default());

        // Valid paths
        assert!(security.validate_path("/calendar/user/personal").is_ok());
        assert!(security.validate_path("/contacts/user/work").is_ok());

        // Invalid paths
        assert!(security.validate_path("/calendar/../../../etc/passwd").is_err());
        assert!(security.validate_path("/calendar/./secret").is_err());
        assert!(security.validate_path("/calendar\\user").is_err());
        assert!(security.validate_path("/calendar/user\0").is_err());
    }

    #[test]
    fn test_body_size_validation() {
        let config = SecurityConfig {
            max_body_size: 1024,
            ..Default::default()
        };
        let security = DavSecurity::new(config);

        assert!(security.validate_body_size(512).is_ok());
        assert!(security.validate_body_size(1024).is_ok());
        assert!(security.validate_body_size(1025).is_err());
    }

    #[test]
    fn test_file_extension_validation() {
        let config = SecurityConfig {
            allowed_extensions: vec!["ics".to_string(), "vcf".to_string()],
            ..Default::default()
        };
        let security = DavSecurity::new(config);

        assert!(security.validate_file_extension("calendar.ics").is_ok());
        assert!(security.validate_file_extension("contacts.vcf").is_ok());
        assert!(security.validate_file_extension("script.js").is_err());
        assert!(security.validate_file_extension("malware.exe").is_err());
    }

    #[test]
    fn test_security_headers() {
        let security = DavSecurity::new(SecurityConfig::default());
        let headers = security.generate_security_headers();

        assert!(headers.contains_key("Content-Security-Policy"));
        assert!(headers.contains_key("X-Frame-Options"));
        assert!(headers.contains_key("X-Content-Type-Options"));
        assert!(headers.contains_key("Referrer-Policy"));
        assert!(headers.contains_key("Permissions-Policy"));
    }

    #[test]
    fn test_security_stats() {
        let security = DavSecurity::new(SecurityConfig::default());
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3));

        // Generate some security events
        security.record_auth_failure(ip, "test failure");
        let _ = security.check_rate_limit(ip);

        let stats = security.get_security_stats();
        assert!(stats.auth_failures > 0);
        assert!(stats.security_events > 0);
        assert!(!stats.recent_events.is_empty());
    }

    #[test]
    fn test_cleanup() {
        let security = DavSecurity::new(SecurityConfig::default());
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 4));

        // Add some data
        let _ = security.check_rate_limit(ip);
        security.record_auth_failure(ip, "test");

        let stats_before = security.get_security_stats();
        assert!(stats_before.rate_limited_ips > 0);

        // Cleanup should not remove recent data
        security.cleanup();

        let stats_after = security.get_security_stats();
        assert_eq!(stats_after.rate_limited_ips, stats_before.rate_limited_ips);
    }
}
