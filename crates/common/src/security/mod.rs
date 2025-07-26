/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Security Module
//!
//! This module provides comprehensive security features for A3Mailer Mail Server,
//! including input validation, rate limiting, security headers, and vulnerability
//! protection mechanisms.

pub mod validation;
pub mod rate_limiting;
pub mod headers;
pub mod audit;

use std::{
    collections::HashMap,
    time::{Duration, Instant},
    sync::{Arc, RwLock},
};
use tracing::{debug, info, warn};

/// Security configuration for the mail server
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Maximum request size in bytes
    pub max_request_size: usize,
    /// Request timeout duration
    pub request_timeout: Duration,
    /// Maximum number of concurrent connections per IP
    pub max_connections_per_ip: u32,
    /// Rate limiting window duration
    pub rate_limit_window: Duration,
    /// Maximum requests per rate limiting window
    pub max_requests_per_window: u32,
    /// Enable security headers
    pub enable_security_headers: bool,
    /// Enable audit logging
    pub enable_audit_logging: bool,
    /// Trusted proxy IPs
    pub trusted_proxies: Vec<String>,
    /// Blocked IP addresses
    pub blocked_ips: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_request_size: 10 * 1024 * 1024, // 10MB
            request_timeout: Duration::from_secs(30),
            max_connections_per_ip: 100,
            rate_limit_window: Duration::from_secs(60),
            max_requests_per_window: 1000,
            enable_security_headers: true,
            enable_audit_logging: true,
            trusted_proxies: Vec::new(),
            blocked_ips: Vec::new(),
        }
    }
}

/// Security event types for audit logging
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityEvent {
    /// Authentication attempt
    AuthenticationAttempt {
        username: String,
        ip_address: String,
        success: bool,
        timestamp: Instant,
    },
    /// Rate limit exceeded
    RateLimitExceeded {
        ip_address: String,
        endpoint: String,
        timestamp: Instant,
    },
    /// Suspicious activity detected
    SuspiciousActivity {
        ip_address: String,
        description: String,
        severity: SecuritySeverity,
        timestamp: Instant,
    },
    /// Security policy violation
    PolicyViolation {
        ip_address: String,
        policy: String,
        details: String,
        timestamp: Instant,
    },
    /// Blocked request
    BlockedRequest {
        ip_address: String,
        reason: String,
        timestamp: Instant,
    },
}

/// Security event severity levels
#[derive(Debug, Clone, PartialEq)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Security metrics for monitoring
#[derive(Debug, Clone, Default)]
pub struct SecurityMetrics {
    /// Total authentication attempts
    pub auth_attempts: u64,
    /// Failed authentication attempts
    pub failed_auth_attempts: u64,
    /// Rate limit violations
    pub rate_limit_violations: u64,
    /// Blocked requests
    pub blocked_requests: u64,
    /// Suspicious activities detected
    pub suspicious_activities: u64,
    /// Security policy violations
    pub policy_violations: u64,
}

impl SecurityMetrics {
    /// Calculate authentication success rate
    pub fn auth_success_rate(&self) -> f64 {
        if self.auth_attempts == 0 {
            0.0
        } else {
            (self.auth_attempts - self.failed_auth_attempts) as f64 / self.auth_attempts as f64
        }
    }

    /// Calculate security score (0.0 to 1.0, higher is better)
    pub fn security_score(&self) -> f64 {
        let total_events = self.rate_limit_violations + self.blocked_requests +
                          self.suspicious_activities + self.policy_violations;

        if total_events == 0 {
            1.0
        } else {
            // Simple scoring algorithm - can be enhanced
            let score = 1.0 - (total_events as f64 / (self.auth_attempts.max(1) as f64));
            score.max(0.0).min(1.0)
        }
    }
}

/// Main security manager
pub struct SecurityManager {
    config: SecurityConfig,
    metrics: Arc<RwLock<SecurityMetrics>>,
    events: Arc<RwLock<Vec<SecurityEvent>>>,
    ip_connections: Arc<RwLock<HashMap<String, u32>>>,
    rate_limits: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
}

impl SecurityManager {
    /// Create a new security manager
    pub fn new(config: SecurityConfig) -> Self {
        info!("Initializing security manager with config: {:?}", config);
        Self {
            config,
            metrics: Arc::new(RwLock::new(SecurityMetrics::default())),
            events: Arc::new(RwLock::new(Vec::new())),
            ip_connections: Arc::new(RwLock::new(HashMap::new())),
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if an IP address is allowed to connect
    pub fn is_ip_allowed(&self, ip: &str) -> bool {
        debug!("Checking if IP {} is allowed", ip);

        // Check if IP is blocked
        if self.config.blocked_ips.contains(&ip.to_string()) {
            warn!("Blocked IP {} attempted to connect", ip);
            self.record_event(SecurityEvent::BlockedRequest {
                ip_address: ip.to_string(),
                reason: "IP in blocklist".to_string(),
                timestamp: Instant::now(),
            });
            return false;
        }

        // Check connection limits
        let connections = self.ip_connections.read().unwrap();
        if let Some(&count) = connections.get(ip) {
            if count >= self.config.max_connections_per_ip {
                warn!("IP {} exceeded connection limit: {}", ip, count);
                self.record_event(SecurityEvent::BlockedRequest {
                    ip_address: ip.to_string(),
                    reason: format!("Connection limit exceeded: {}", count),
                    timestamp: Instant::now(),
                });
                return false;
            }
        }

        true
    }

    /// Register a new connection from an IP
    pub fn register_connection(&self, ip: &str) {
        debug!("Registering connection from IP {}", ip);
        let mut connections = self.ip_connections.write().unwrap();
        *connections.entry(ip.to_string()).or_insert(0) += 1;
    }

    /// Unregister a connection from an IP
    pub fn unregister_connection(&self, ip: &str) {
        debug!("Unregistering connection from IP {}", ip);
        let mut connections = self.ip_connections.write().unwrap();
        if let Some(count) = connections.get_mut(ip) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                connections.remove(ip);
            }
        }
    }

    /// Check rate limiting for an IP and endpoint
    pub fn check_rate_limit(&self, ip: &str, endpoint: &str) -> bool {
        debug!("Checking rate limit for IP {} on endpoint {}", ip, endpoint);

        let key = format!("{}:{}", ip, endpoint);
        let now = Instant::now();
        let window_start = now - self.config.rate_limit_window;

        let mut rate_limits = self.rate_limits.write().unwrap();
        let requests = rate_limits.entry(key.clone()).or_insert_with(Vec::new);

        // Remove old requests outside the window
        requests.retain(|&timestamp| timestamp > window_start);

        // Check if limit is exceeded
        if requests.len() >= self.config.max_requests_per_window as usize {
            warn!("Rate limit exceeded for IP {} on endpoint {}", ip, endpoint);
            self.record_event(SecurityEvent::RateLimitExceeded {
                ip_address: ip.to_string(),
                endpoint: endpoint.to_string(),
                timestamp: now,
            });

            self.update_metrics(|metrics| metrics.rate_limit_violations += 1);
            return false;
        }

        // Add current request
        requests.push(now);
        true
    }

    /// Record an authentication attempt
    pub fn record_auth_attempt(&self, username: &str, ip: &str, success: bool) {
        debug!("Recording auth attempt for user {} from IP {}: {}", username, ip, success);

        self.record_event(SecurityEvent::AuthenticationAttempt {
            username: username.to_string(),
            ip_address: ip.to_string(),
            success,
            timestamp: Instant::now(),
        });

        self.update_metrics(|metrics| {
            metrics.auth_attempts += 1;
            if !success {
                metrics.failed_auth_attempts += 1;
            }
        });

        // Detect brute force attempts
        if !success {
            self.detect_brute_force(ip);
        }
    }

    /// Detect potential brute force attacks
    fn detect_brute_force(&self, ip: &str) {
        let events = self.events.read().unwrap();
        let recent_failures = events.iter()
            .filter(|event| {
                if let SecurityEvent::AuthenticationAttempt { ip_address, success, timestamp, .. } = event {
                    ip_address == ip && !success && timestamp.elapsed() < Duration::from_secs(300)
                } else {
                    false
                }
            })
            .count();

        if recent_failures >= 5 {
            warn!("Potential brute force attack detected from IP {}", ip);
            self.record_event(SecurityEvent::SuspiciousActivity {
                ip_address: ip.to_string(),
                description: format!("Potential brute force: {} failed attempts", recent_failures),
                severity: SecuritySeverity::High,
                timestamp: Instant::now(),
            });

            self.update_metrics(|metrics| metrics.suspicious_activities += 1);
        }
    }

    /// Record a security event
    fn record_event(&self, event: SecurityEvent) {
        if self.config.enable_audit_logging {
            debug!("Recording security event: {:?}", event);
            let mut events = self.events.write().unwrap();
            events.push(event);

            // Keep only recent events to prevent memory growth
            if events.len() > 10000 {
                events.drain(0..1000);
            }
        }
    }

    /// Update security metrics
    fn update_metrics<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut SecurityMetrics),
    {
        if let Ok(mut metrics) = self.metrics.write() {
            update_fn(&mut metrics);
        }
    }

    /// Get current security metrics
    pub fn get_metrics(&self) -> SecurityMetrics {
        self.metrics.read().unwrap().clone()
    }

    /// Get recent security events
    pub fn get_recent_events(&self, limit: usize) -> Vec<SecurityEvent> {
        let events = self.events.read().unwrap();
        events.iter().rev().take(limit).cloned().collect()
    }

    /// Get security configuration
    pub fn get_config(&self) -> &SecurityConfig {
        &self.config
    }

    /// Clean up old data
    pub fn cleanup(&self) {
        debug!("Cleaning up old security data");

        let now = Instant::now();
        let cleanup_threshold = now - Duration::from_secs(3600); // 1 hour

        // Clean up rate limit data
        let mut rate_limits = self.rate_limits.write().unwrap();
        for requests in rate_limits.values_mut() {
            requests.retain(|&timestamp| timestamp > cleanup_threshold);
        }
        rate_limits.retain(|_, requests| !requests.is_empty());

        // Clean up old events
        let mut events = self.events.write().unwrap();
        events.retain(|event| {
            match event {
                SecurityEvent::AuthenticationAttempt { timestamp, .. } |
                SecurityEvent::RateLimitExceeded { timestamp, .. } |
                SecurityEvent::SuspiciousActivity { timestamp, .. } |
                SecurityEvent::PolicyViolation { timestamp, .. } |
                SecurityEvent::BlockedRequest { timestamp, .. } => {
                    timestamp > &cleanup_threshold
                }
            }
        });

        info!("Security cleanup completed");
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new(SecurityConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_ip_blocking() {
        let mut config = SecurityConfig::default();
        config.blocked_ips.push("192.168.1.100".to_string());

        let manager = SecurityManager::new(config);

        assert!(!manager.is_ip_allowed("192.168.1.100"));
        assert!(manager.is_ip_allowed("192.168.1.101"));
    }

    #[test]
    fn test_connection_limits() {
        let mut config = SecurityConfig::default();
        config.max_connections_per_ip = 2;

        let manager = SecurityManager::new(config);
        let ip = "192.168.1.1";

        // First two connections should be allowed
        assert!(manager.is_ip_allowed(ip));
        manager.register_connection(ip);
        assert!(manager.is_ip_allowed(ip));
        manager.register_connection(ip);

        // Third connection should be blocked
        assert!(!manager.is_ip_allowed(ip));

        // After unregistering, should be allowed again
        manager.unregister_connection(ip);
        assert!(manager.is_ip_allowed(ip));
    }

    #[test]
    fn test_rate_limiting() {
        let mut config = SecurityConfig::default();
        config.max_requests_per_window = 2;
        config.rate_limit_window = Duration::from_secs(1);

        let manager = SecurityManager::new(config);
        let ip = "192.168.1.1";
        let endpoint = "/api/test";

        // First two requests should be allowed
        assert!(manager.check_rate_limit(ip, endpoint));
        assert!(manager.check_rate_limit(ip, endpoint));

        // Third request should be blocked
        assert!(!manager.check_rate_limit(ip, endpoint));

        // After waiting, should be allowed again
        thread::sleep(Duration::from_millis(1100));
        assert!(manager.check_rate_limit(ip, endpoint));
    }

    #[test]
    fn test_auth_attempts() {
        let manager = SecurityManager::new(SecurityConfig::default());

        manager.record_auth_attempt("user1", "192.168.1.1", true);
        manager.record_auth_attempt("user2", "192.168.1.1", false);

        let metrics = manager.get_metrics();
        assert_eq!(metrics.auth_attempts, 2);
        assert_eq!(metrics.failed_auth_attempts, 1);
        assert_eq!(metrics.auth_success_rate(), 0.5);
    }

    #[test]
    fn test_security_metrics() {
        let mut metrics = SecurityMetrics::default();
        metrics.auth_attempts = 100;
        metrics.failed_auth_attempts = 10;
        metrics.rate_limit_violations = 5;

        assert_eq!(metrics.auth_success_rate(), 0.9);
        assert!(metrics.security_score() > 0.9);
    }
}
