/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! HTTP Security Module
//! 
//! This module provides security middleware and utilities for HTTP requests,
//! including rate limiting, input validation, security headers, and audit logging.

use std::{
    collections::HashMap,
    sync::Arc,
    time::Instant,
};
use common::security::{
    SecurityManager, SecurityConfig, SecurityEvent, SecuritySeverity,
    validation::{InputValidator, ValidationConfig, ValidationError},
    rate_limiting::{IpRateLimiter, RateLimitConfig, RateLimitAlgorithm},
    headers::{SecurityHeadersManager, SecurityHeadersConfig},
    audit::{AuditLogger, AuditConfig, AuditEvent, AuditEventType, AuditSeverity, AuditOutcome},
};
use tracing::{debug, info, warn, error};

/// HTTP security middleware configuration
#[derive(Debug, Clone)]
pub struct HttpSecurityConfig {
    /// Enable security middleware
    pub enabled: bool,
    /// Security manager configuration
    pub security_config: SecurityConfig,
    /// Input validation configuration
    pub validation_config: ValidationConfig,
    /// Rate limiting configuration
    pub rate_limit_config: RateLimitConfig,
    /// Security headers configuration
    pub headers_config: SecurityHeadersConfig,
    /// Audit logging configuration
    pub audit_config: AuditConfig,
    /// Trusted proxy headers
    pub trusted_proxy_headers: Vec<String>,
    /// Enable request logging
    pub enable_request_logging: bool,
}

impl Default for HttpSecurityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            security_config: SecurityConfig::default(),
            validation_config: ValidationConfig::default(),
            rate_limit_config: RateLimitConfig {
                max_requests: 1000,
                window_duration: std::time::Duration::from_secs(60),
                algorithm: RateLimitAlgorithm::SlidingWindow,
                ..Default::default()
            },
            headers_config: SecurityHeadersConfig::default(),
            audit_config: AuditConfig::default(),
            trusted_proxy_headers: vec![
                "X-Forwarded-For".to_string(),
                "X-Real-IP".to_string(),
                "CF-Connecting-IP".to_string(),
            ],
            enable_request_logging: true,
        }
    }
}

/// HTTP security middleware
pub struct HttpSecurityMiddleware {
    config: HttpSecurityConfig,
    security_manager: Arc<SecurityManager>,
    validator: Arc<InputValidator>,
    rate_limiter: Arc<IpRateLimiter>,
    headers_manager: Arc<SecurityHeadersManager>,
    audit_logger: Arc<AuditLogger>,
}

impl HttpSecurityMiddleware {
    /// Create a new HTTP security middleware
    pub fn new(config: HttpSecurityConfig) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Initializing HTTP security middleware");
        
        let security_manager = Arc::new(SecurityManager::new(config.security_config.clone()));
        let validator = Arc::new(InputValidator::new(config.validation_config.clone())?);
        let rate_limiter = Arc::new(IpRateLimiter::new(config.rate_limit_config.clone()));
        let headers_manager = Arc::new(SecurityHeadersManager::new(config.headers_config.clone()));
        let audit_logger = Arc::new(AuditLogger::new(config.audit_config.clone()));
        
        Ok(Self {
            config,
            security_manager,
            validator,
            rate_limiter,
            headers_manager,
            audit_logger,
        })
    }
    
    /// Process incoming HTTP request
    pub fn process_request(
        &self,
        method: &str,
        path: &str,
        headers: &HashMap<String, String>,
        body: Option<&[u8]>,
        remote_addr: &str,
    ) -> SecurityCheckResult {
        if !self.config.enabled {
            return SecurityCheckResult::Allow;
        }
        
        let start_time = Instant::now();
        let client_ip = self.extract_client_ip(headers, remote_addr);
        
        debug!("Processing security check for {} {} from {}", method, path, client_ip);
        
        // Check if IP is allowed
        if !self.security_manager.is_ip_allowed(&client_ip) {
            warn!("Blocked request from IP {}", client_ip);
            self.log_security_event(
                AuditEventType::SecurityViolation,
                AuditSeverity::Warning,
                AuditOutcome::Failure,
                "ip_blocked",
                &format!("Request blocked from IP {}", client_ip),
                Some(&client_ip),
                None,
                Some(path),
            );
            return SecurityCheckResult::Block("IP address blocked".to_string());
        }
        
        // Check rate limiting
        if !self.rate_limiter.check_rate_limit(client_ip.clone()).allowed {
            warn!("Rate limit exceeded for IP {} on {}", client_ip, path);
            self.log_security_event(
                AuditEventType::SecurityViolation,
                AuditSeverity::Warning,
                AuditOutcome::Failure,
                "rate_limit_exceeded",
                &format!("Rate limit exceeded for IP {} on {}", client_ip, path),
                Some(&client_ip),
                None,
                Some(path),
            );
            return SecurityCheckResult::RateLimit;
        }
        
        // Validate request headers
        if let Err(e) = self.validate_headers(headers) {
            warn!("Invalid headers from IP {}: {}", client_ip, e);
            self.log_security_event(
                AuditEventType::SecurityViolation,
                AuditSeverity::Warning,
                AuditOutcome::Failure,
                "invalid_headers",
                &format!("Invalid headers: {}", e),
                Some(&client_ip),
                None,
                Some(path),
            );
            return SecurityCheckResult::Block(format!("Invalid headers: {}", e));
        }
        
        // Validate request body if present
        if let Some(body_data) = body {
            if let Err(e) = self.validate_body(body_data) {
                warn!("Invalid request body from IP {}: {}", client_ip, e);
                self.log_security_event(
                    AuditEventType::SecurityViolation,
                    AuditSeverity::Warning,
                    AuditOutcome::Failure,
                    "invalid_body",
                    &format!("Invalid request body: {}", e),
                    Some(&client_ip),
                    None,
                    Some(path),
                );
                return SecurityCheckResult::Block(format!("Invalid request body: {}", e));
            }
        }
        
        // Register connection
        self.security_manager.register_connection(&client_ip);
        
        // Log successful request if enabled
        if self.config.enable_request_logging {
            self.log_security_event(
                AuditEventType::Network,
                AuditSeverity::Info,
                AuditOutcome::Success,
                "http_request",
                &format!("{} {} from {}", method, path, client_ip),
                Some(&client_ip),
                None,
                Some(path),
            );
        }
        
        let processing_time = start_time.elapsed();
        debug!("Security check completed in {:?}", processing_time);
        
        SecurityCheckResult::Allow
    }
    
    /// Generate security headers for HTTP response
    pub fn generate_response_headers(&self, is_https: bool) -> HashMap<String, String> {
        self.headers_manager.generate_headers(is_https)
    }
    
    /// Extract client IP from headers or remote address
    fn extract_client_ip(&self, headers: &HashMap<String, String>, remote_addr: &str) -> String {
        // Check trusted proxy headers
        for header_name in &self.config.trusted_proxy_headers {
            if let Some(header_value) = headers.get(header_name) {
                // Take the first IP from comma-separated list
                if let Some(ip) = header_value.split(',').next() {
                    let ip = ip.trim();
                    if !ip.is_empty() && self.validator.validate_ip_address(ip).is_ok() {
                        return ip.to_string();
                    }
                }
            }
        }
        
        // Fall back to remote address
        remote_addr.to_string()
    }
    
    /// Validate HTTP headers
    fn validate_headers(&self, headers: &HashMap<String, String>) -> Result<(), ValidationError> {
        for (name, value) in headers {
            // Validate header name
            self.validator.validate_string(name, "header_name")?;
            
            // Validate header value
            self.validator.validate_string(value, "header_value")?;
            
            // Check for suspicious headers
            if name.to_lowercase().contains("script") || value.contains("<script") {
                return Err(ValidationError::MaliciousContent);
            }
        }
        
        Ok(())
    }
    
    /// Validate request body
    fn validate_body(&self, body: &[u8]) -> Result<(), ValidationError> {
        // Check body size
        if body.len() > self.config.security_config.max_request_size {
            return Err(ValidationError::TooLong {
                actual: body.len(),
                max: self.config.security_config.max_request_size,
            });
        }
        
        // Convert to string for validation (if valid UTF-8)
        if let Ok(body_str) = std::str::from_utf8(body) {
            self.validator.validate_string(body_str, "request_body")?;
        }
        
        Ok(())
    }
    
    /// Log a security event
    fn log_security_event(
        &self,
        event_type: AuditEventType,
        severity: AuditSeverity,
        outcome: AuditOutcome,
        action: &str,
        description: &str,
        source_ip: Option<&str>,
        user_id: Option<&str>,
        resource: Option<&str>,
    ) {
        let mut event = AuditEvent::new(
            event_type,
            severity,
            outcome,
            action.to_string(),
            description.to_string(),
        );
        
        if let Some(ip) = source_ip {
            event = event.with_source_ip(ip.to_string());
        }
        
        if let Some(user) = user_id {
            event = event.with_user_id(user.to_string());
        }
        
        if let Some(res) = resource {
            event = event.with_resource(res.to_string());
        }
        
        self.audit_logger.log_event(event);
    }
    
    /// Unregister connection when request is complete
    pub fn unregister_connection(&self, remote_addr: &str, headers: &HashMap<String, String>) {
        let client_ip = self.extract_client_ip(headers, remote_addr);
        self.security_manager.unregister_connection(&client_ip);
    }
    
    /// Get security statistics
    pub fn get_security_stats(&self) -> SecurityStats {
        let security_metrics = self.security_manager.get_metrics();
        let rate_limiter_stats = self.rate_limiter.get_stats();
        let audit_stats = self.audit_logger.get_stats();
        
        SecurityStats {
            auth_attempts: security_metrics.auth_attempts,
            failed_auth_attempts: security_metrics.failed_auth_attempts,
            rate_limit_violations: security_metrics.rate_limit_violations,
            blocked_requests: security_metrics.blocked_requests,
            suspicious_activities: security_metrics.suspicious_activities,
            active_rate_limit_keys: rate_limiter_stats.active_keys,
            total_audit_events: audit_stats.total_events,
            security_score: security_metrics.security_score(),
        }
    }
    
    /// Perform periodic cleanup
    pub fn cleanup(&self) {
        debug!("Performing security middleware cleanup");
        self.rate_limiter.cleanup();
        self.audit_logger.cleanup_old_events();
    }
}

/// Result of security check
#[derive(Debug, Clone)]
pub enum SecurityCheckResult {
    /// Request is allowed to proceed
    Allow,
    /// Request should be blocked with reason
    Block(String),
    /// Request is rate limited
    RateLimit,
}

/// Security statistics
#[derive(Debug, Clone)]
pub struct SecurityStats {
    pub auth_attempts: u64,
    pub failed_auth_attempts: u64,
    pub rate_limit_violations: u64,
    pub blocked_requests: u64,
    pub suspicious_activities: u64,
    pub active_rate_limit_keys: usize,
    pub total_audit_events: u64,
    pub security_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_security_middleware_creation() {
        let config = HttpSecurityConfig::default();
        let middleware = HttpSecurityMiddleware::new(config);
        assert!(middleware.is_ok());
    }
    
    #[test]
    fn test_request_processing() {
        let config = HttpSecurityConfig::default();
        let middleware = HttpSecurityMiddleware::new(config).unwrap();
        
        let headers = HashMap::new();
        let result = middleware.process_request(
            "GET",
            "/api/test",
            &headers,
            None,
            "192.168.1.1",
        );
        
        matches!(result, SecurityCheckResult::Allow);
    }
    
    #[test]
    fn test_ip_extraction() {
        let config = HttpSecurityConfig::default();
        let middleware = HttpSecurityMiddleware::new(config).unwrap();
        
        let mut headers = HashMap::new();
        headers.insert("X-Forwarded-For".to_string(), "203.0.113.1, 192.168.1.1".to_string());
        
        let ip = middleware.extract_client_ip(&headers, "192.168.1.1");
        assert_eq!(ip, "203.0.113.1");
    }
    
    #[test]
    fn test_header_validation() {
        let config = HttpSecurityConfig::default();
        let middleware = HttpSecurityMiddleware::new(config).unwrap();
        
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        
        assert!(middleware.validate_headers(&headers).is_ok());
        
        // Test malicious header
        headers.insert("X-Script".to_string(), "<script>alert('xss')</script>".to_string());
        assert!(middleware.validate_headers(&headers).is_err());
    }
}
