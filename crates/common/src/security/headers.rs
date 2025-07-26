/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Security Headers Module
//! 
//! This module provides security headers management to protect against
//! various web-based attacks and improve overall security posture.

use std::collections::HashMap;
use tracing::{debug, info};

/// Security headers configuration
#[derive(Debug, Clone)]
pub struct SecurityHeadersConfig {
    /// Enable Strict-Transport-Security header
    pub enable_hsts: bool,
    /// HSTS max age in seconds
    pub hsts_max_age: u32,
    /// Include subdomains in HSTS
    pub hsts_include_subdomains: bool,
    /// Enable HSTS preload
    pub hsts_preload: bool,
    
    /// Enable Content-Security-Policy header
    pub enable_csp: bool,
    /// CSP policy directives
    pub csp_policy: String,
    /// Enable CSP report-only mode
    pub csp_report_only: bool,
    
    /// Enable X-Frame-Options header
    pub enable_frame_options: bool,
    /// Frame options value
    pub frame_options: FrameOptions,
    
    /// Enable X-Content-Type-Options header
    pub enable_content_type_options: bool,
    
    /// Enable X-XSS-Protection header
    pub enable_xss_protection: bool,
    
    /// Enable Referrer-Policy header
    pub enable_referrer_policy: bool,
    /// Referrer policy value
    pub referrer_policy: ReferrerPolicy,
    
    /// Enable Permissions-Policy header
    pub enable_permissions_policy: bool,
    /// Permissions policy directives
    pub permissions_policy: Vec<String>,
    
    /// Custom security headers
    pub custom_headers: HashMap<String, String>,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            enable_hsts: true,
            hsts_max_age: 31536000, // 1 year
            hsts_include_subdomains: true,
            hsts_preload: false,
            
            enable_csp: true,
            csp_policy: "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none';".to_string(),
            csp_report_only: false,
            
            enable_frame_options: true,
            frame_options: FrameOptions::Deny,
            
            enable_content_type_options: true,
            enable_xss_protection: true,
            
            enable_referrer_policy: true,
            referrer_policy: ReferrerPolicy::StrictOriginWhenCrossOrigin,
            
            enable_permissions_policy: true,
            permissions_policy: vec![
                "camera=()".to_string(),
                "microphone=()".to_string(),
                "geolocation=()".to_string(),
                "payment=()".to_string(),
            ],
            
            custom_headers: HashMap::new(),
        }
    }
}

/// X-Frame-Options values
#[derive(Debug, Clone, PartialEq)]
pub enum FrameOptions {
    Deny,
    SameOrigin,
    AllowFrom(String),
}

impl FrameOptions {
    pub fn to_string(&self) -> String {
        match self {
            FrameOptions::Deny => "DENY".to_string(),
            FrameOptions::SameOrigin => "SAMEORIGIN".to_string(),
            FrameOptions::AllowFrom(uri) => format!("ALLOW-FROM {}", uri),
        }
    }
}

/// Referrer-Policy values
#[derive(Debug, Clone, PartialEq)]
pub enum ReferrerPolicy {
    NoReferrer,
    NoReferrerWhenDowngrade,
    Origin,
    OriginWhenCrossOrigin,
    SameOrigin,
    StrictOrigin,
    StrictOriginWhenCrossOrigin,
    UnsafeUrl,
}

impl ReferrerPolicy {
    pub fn to_string(&self) -> String {
        match self {
            ReferrerPolicy::NoReferrer => "no-referrer".to_string(),
            ReferrerPolicy::NoReferrerWhenDowngrade => "no-referrer-when-downgrade".to_string(),
            ReferrerPolicy::Origin => "origin".to_string(),
            ReferrerPolicy::OriginWhenCrossOrigin => "origin-when-cross-origin".to_string(),
            ReferrerPolicy::SameOrigin => "same-origin".to_string(),
            ReferrerPolicy::StrictOrigin => "strict-origin".to_string(),
            ReferrerPolicy::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin".to_string(),
            ReferrerPolicy::UnsafeUrl => "unsafe-url".to_string(),
        }
    }
}

/// Security headers manager
pub struct SecurityHeadersManager {
    config: SecurityHeadersConfig,
}

impl SecurityHeadersManager {
    /// Create a new security headers manager
    pub fn new(config: SecurityHeadersConfig) -> Self {
        info!("Creating security headers manager with config: {:?}", config);
        Self { config }
    }
    
    /// Generate security headers for HTTP responses
    pub fn generate_headers(&self, is_https: bool) -> HashMap<String, String> {
        debug!("Generating security headers (HTTPS: {})", is_https);
        
        let mut headers = HashMap::new();
        
        // Strict-Transport-Security (only for HTTPS)
        if self.config.enable_hsts && is_https {
            let mut hsts_value = format!("max-age={}", self.config.hsts_max_age);
            
            if self.config.hsts_include_subdomains {
                hsts_value.push_str("; includeSubDomains");
            }
            
            if self.config.hsts_preload {
                hsts_value.push_str("; preload");
            }
            
            headers.insert("Strict-Transport-Security".to_string(), hsts_value);
        }
        
        // Content-Security-Policy
        if self.config.enable_csp {
            let header_name = if self.config.csp_report_only {
                "Content-Security-Policy-Report-Only"
            } else {
                "Content-Security-Policy"
            };
            
            headers.insert(header_name.to_string(), self.config.csp_policy.clone());
        }
        
        // X-Frame-Options
        if self.config.enable_frame_options {
            headers.insert(
                "X-Frame-Options".to_string(),
                self.config.frame_options.to_string(),
            );
        }
        
        // X-Content-Type-Options
        if self.config.enable_content_type_options {
            headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
        }
        
        // X-XSS-Protection
        if self.config.enable_xss_protection {
            headers.insert("X-XSS-Protection".to_string(), "1; mode=block".to_string());
        }
        
        // Referrer-Policy
        if self.config.enable_referrer_policy {
            headers.insert(
                "Referrer-Policy".to_string(),
                self.config.referrer_policy.to_string(),
            );
        }
        
        // Permissions-Policy
        if self.config.enable_permissions_policy && !self.config.permissions_policy.is_empty() {
            let policy = self.config.permissions_policy.join(", ");
            headers.insert("Permissions-Policy".to_string(), policy);
        }
        
        // Custom headers
        for (name, value) in &self.config.custom_headers {
            headers.insert(name.clone(), value.clone());
        }
        
        debug!("Generated {} security headers", headers.len());
        headers
    }
    
    /// Validate CSP policy syntax
    pub fn validate_csp_policy(policy: &str) -> Result<(), String> {
        debug!("Validating CSP policy: {}", policy);
        
        // Basic CSP validation
        let directives: Vec<&str> = policy.split(';').map(|d| d.trim()).collect();
        
        for directive in directives {
            if directive.is_empty() {
                continue;
            }
            
            let parts: Vec<&str> = directive.split_whitespace().collect();
            if parts.is_empty() {
                return Err("Empty directive found".to_string());
            }
            
            let directive_name = parts[0];
            
            // Check if directive name is valid
            if !is_valid_csp_directive(directive_name) {
                return Err(format!("Invalid CSP directive: {}", directive_name));
            }
            
            // Validate directive values
            for value in &parts[1..] {
                if !is_valid_csp_value(value) {
                    return Err(format!("Invalid CSP value: {}", value));
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if headers are properly configured for security
    pub fn audit_configuration(&self) -> Vec<String> {
        let mut issues = Vec::new();
        
        // Check HSTS configuration
        if !self.config.enable_hsts {
            issues.push("HSTS is disabled - consider enabling for HTTPS connections".to_string());
        } else if self.config.hsts_max_age < 86400 {
            issues.push("HSTS max-age is less than 1 day - consider increasing".to_string());
        }
        
        // Check CSP configuration
        if !self.config.enable_csp {
            issues.push("Content Security Policy is disabled - consider enabling".to_string());
        } else if self.config.csp_policy.contains("'unsafe-eval'") {
            issues.push("CSP allows 'unsafe-eval' - consider removing for better security".to_string());
        }
        
        // Check frame options
        if !self.config.enable_frame_options {
            issues.push("X-Frame-Options is disabled - consider enabling to prevent clickjacking".to_string());
        }
        
        // Check content type options
        if !self.config.enable_content_type_options {
            issues.push("X-Content-Type-Options is disabled - consider enabling to prevent MIME sniffing".to_string());
        }
        
        // Check XSS protection
        if !self.config.enable_xss_protection {
            issues.push("X-XSS-Protection is disabled - consider enabling for legacy browser support".to_string());
        }
        
        if issues.is_empty() {
            vec!["Security headers configuration looks good".to_string()]
        } else {
            issues
        }
    }
    
    /// Get current configuration
    pub fn get_config(&self) -> &SecurityHeadersConfig {
        &self.config
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: SecurityHeadersConfig) {
        info!("Updating security headers configuration");
        self.config = config;
    }
}

impl Default for SecurityHeadersManager {
    fn default() -> Self {
        Self::new(SecurityHeadersConfig::default())
    }
}

/// Check if a CSP directive name is valid
fn is_valid_csp_directive(directive: &str) -> bool {
    matches!(directive, 
        "default-src" | "script-src" | "style-src" | "img-src" | "font-src" |
        "connect-src" | "media-src" | "object-src" | "child-src" | "frame-src" |
        "worker-src" | "frame-ancestors" | "form-action" | "base-uri" |
        "plugin-types" | "sandbox" | "report-uri" | "report-to" |
        "require-sri-for" | "upgrade-insecure-requests" | "block-all-mixed-content"
    )
}

/// Check if a CSP value is valid
fn is_valid_csp_value(value: &str) -> bool {
    // Allow common CSP values
    if matches!(value, "'self'" | "'none'" | "'unsafe-inline'" | "'unsafe-eval'" | 
                     "'strict-dynamic'" | "'unsafe-hashes'" | "data:" | "blob:" | 
                     "filesystem:" | "https:" | "http:" | "ws:" | "wss:") {
        return true;
    }
    
    // Allow nonce and hash values
    if value.starts_with("'nonce-") || value.starts_with("'sha") {
        return true;
    }
    
    // Allow URLs (basic validation)
    if value.contains("://") || value.starts_with("*.") || value.contains('.') {
        return true;
    }
    
    // Allow scheme-only values
    if value.ends_with(':') {
        return true;
    }
    
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_security_headers_generation() {
        let manager = SecurityHeadersManager::default();
        
        // Test HTTPS headers
        let https_headers = manager.generate_headers(true);
        assert!(https_headers.contains_key("Strict-Transport-Security"));
        assert!(https_headers.contains_key("Content-Security-Policy"));
        assert!(https_headers.contains_key("X-Frame-Options"));
        assert!(https_headers.contains_key("X-Content-Type-Options"));
        
        // Test HTTP headers (no HSTS)
        let http_headers = manager.generate_headers(false);
        assert!(!http_headers.contains_key("Strict-Transport-Security"));
        assert!(http_headers.contains_key("Content-Security-Policy"));
    }
    
    #[test]
    fn test_csp_validation() {
        // Valid CSP
        assert!(SecurityHeadersManager::validate_csp_policy(
            "default-src 'self'; script-src 'self' 'unsafe-inline'"
        ).is_ok());
        
        // Invalid directive
        assert!(SecurityHeadersManager::validate_csp_policy(
            "invalid-directive 'self'"
        ).is_err());
        
        // Empty policy
        assert!(SecurityHeadersManager::validate_csp_policy("").is_ok());
    }
    
    #[test]
    fn test_frame_options() {
        assert_eq!(FrameOptions::Deny.to_string(), "DENY");
        assert_eq!(FrameOptions::SameOrigin.to_string(), "SAMEORIGIN");
        assert_eq!(
            FrameOptions::AllowFrom("https://example.com".to_string()).to_string(),
            "ALLOW-FROM https://example.com"
        );
    }
    
    #[test]
    fn test_referrer_policy() {
        assert_eq!(ReferrerPolicy::NoReferrer.to_string(), "no-referrer");
        assert_eq!(ReferrerPolicy::StrictOriginWhenCrossOrigin.to_string(), "strict-origin-when-cross-origin");
    }
    
    #[test]
    fn test_configuration_audit() {
        let mut config = SecurityHeadersConfig::default();
        config.enable_hsts = false;
        config.enable_csp = false;
        
        let manager = SecurityHeadersManager::new(config);
        let issues = manager.audit_configuration();
        
        assert!(issues.len() > 1);
        assert!(issues.iter().any(|issue| issue.contains("HSTS")));
        assert!(issues.iter().any(|issue| issue.contains("Content Security Policy")));
    }
}
