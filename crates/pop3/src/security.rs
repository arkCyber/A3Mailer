/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! POP3 Security Module
//!
//! This module provides comprehensive security features for the POP3 server including:
//! - Authentication rate limiting and brute force protection
//! - Command rate limiting to prevent abuse
//! - Connection limiting per IP address
//! - Session management and tracking
//! - Suspicious activity detection and alerting
//! - Security event logging and monitoring
//!
//! # Architecture
//!
//! The security system is built around the `SecurityManager` which maintains
//! thread-safe tracking of authentication attempts, command rates, and connections
//! per IP address and session.
//!
//! # Performance Characteristics
//!
//! - Authentication checks: O(1) average case with HashMap lookup
//! - Command rate limiting: O(1) per session
//! - Connection tracking: O(1) per IP address
//! - Memory usage scales with number of unique IP addresses and active sessions
//!
//! # Thread Safety
//!
//! All operations are thread-safe using Arc<Mutex<>> for shared state.
//! Lock contention is minimized through fine-grained locking strategies.
//!
//! # Examples
//!
//! ```rust
//! use pop3::security::{SecurityManager, SecurityConfig};
//! use std::net::IpAddr;
//!
//! let config = SecurityConfig::default();
//! let security = SecurityManager::new(config);
//!
//! // Check if authentication is allowed
//! let ip = "192.168.1.100".parse().unwrap();
//! if security.check_auth_allowed(ip).is_ok() {
//!     // Proceed with authentication
//! }
//!
//! // Record authentication result
//! security.record_auth_attempt(ip, false); // Failed attempt
//! ```

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, trace, warn};

use crate::error::Pop3Error;

/// Suspicious activity flag constants
///
/// These bit flags are used to track different types of suspicious
/// behavior patterns for enhanced security monitoring.
pub mod suspicious_flags {
    /// Rapid authentication attempts (faster than human typing)
    pub const RAPID_ATTEMPTS: u32 = 1 << 0;

    /// Dictionary/brute force attack pattern detected
    pub const DICTIONARY_ATTACK: u32 = 1 << 1;

    /// Connection from known malicious IP range
    pub const MALICIOUS_IP: u32 = 1 << 2;

    /// Unusual geographic location for this account
    pub const GEO_ANOMALY: u32 = 1 << 3;

    /// Automated/bot behavior detected
    pub const BOT_BEHAVIOR: u32 = 1 << 4;

    /// Multiple user agent strings from same IP
    pub const MULTIPLE_USER_AGENTS: u32 = 1 << 5;

    /// Attempting to access non-existent accounts
    pub const ACCOUNT_ENUMERATION: u32 = 1 << 6;

    /// Unusual command patterns
    pub const UNUSUAL_COMMANDS: u32 = 1 << 7;

    /// Connection during unusual hours
    pub const UNUSUAL_TIMING: u32 = 1 << 8;

    /// Repeated connection failures
    pub const CONNECTION_FAILURES: u32 = 1 << 9;
}

/// Security configuration for POP3 server
///
/// Comprehensive security settings that control various aspects of
/// server protection including rate limiting, authentication controls,
/// and monitoring thresholds.
///
/// # Security Features
///
/// * **Authentication Protection**: Prevents brute force attacks through
///   configurable attempt limits and time windows
/// * **Rate Limiting**: Controls command frequency to prevent abuse
/// * **Connection Management**: Limits concurrent connections per IP
/// * **Monitoring**: Configurable logging and alerting thresholds
/// * **Session Security**: Advanced session tracking and validation
///
/// # Configuration Guidelines
///
/// For production environments, consider:
/// - `max_auth_attempts`: 3-5 attempts
/// - `auth_window`: 5-15 minutes
/// - `max_connections_per_ip`: 5-20 connections
/// - `max_commands_per_minute`: 30-120 commands
/// - `min_command_delay`: 50-200ms
///
/// # Examples
///
/// ```rust
/// use pop3::security::SecurityConfig;
/// use std::time::Duration;
///
/// // High security configuration
/// let config = SecurityConfig {
///     max_auth_attempts: 3,
///     auth_window: Duration::from_secs(900), // 15 minutes
///     max_connections_per_ip: 5,
///     max_commands_per_minute: 30,
///     min_command_delay: Duration::from_millis(200),
///     enable_security_logging: true,
///     suspicious_threshold: 5,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Maximum authentication attempts per IP address
    ///
    /// After this many failed attempts, the IP will be temporarily blocked.
    /// Recommended: 3-5 for production environments.
    pub max_auth_attempts: u32,

    /// Time window for authentication attempts tracking
    ///
    /// Failed attempts are counted within this window. After the window
    /// expires, the counter resets. Recommended: 5-15 minutes.
    pub auth_window: Duration,

    /// Maximum concurrent connections per IP address
    ///
    /// Prevents resource exhaustion attacks by limiting connections
    /// from a single IP. Recommended: 5-20 for production.
    pub max_connections_per_ip: u32,

    /// Maximum commands per minute per session
    ///
    /// Prevents command flooding attacks. Recommended: 30-120 for
    /// normal usage patterns.
    pub max_commands_per_minute: u32,

    /// Minimum delay between commands (anti-spam protection)
    ///
    /// Forces a minimum delay between commands to prevent rapid-fire
    /// attacks. Recommended: 50-200ms.
    pub min_command_delay: Duration,

    /// Enable detailed security event logging
    ///
    /// When enabled, all security events are logged with detailed
    /// context for monitoring and analysis.
    pub enable_security_logging: bool,

    /// Suspicious activity detection threshold
    ///
    /// Number of security violations before marking an IP as suspicious.
    /// Triggers enhanced monitoring and potential automated responses.
    pub suspicious_threshold: u32,

    /// Maximum session duration before forced disconnect
    ///
    /// Prevents indefinitely long sessions that could be used for
    /// resource exhaustion attacks.
    pub max_session_duration: Duration,

    /// Enable geolocation-based security checks
    ///
    /// When enabled, connections from unusual geographic locations
    /// may trigger additional security measures.
    pub enable_geo_blocking: bool,

    /// List of blocked countries (ISO 3166-1 alpha-2 codes)
    ///
    /// Connections from these countries will be automatically blocked.
    /// Example: ["CN", "RU", "KP"] for blocking China, Russia, North Korea.
    pub blocked_countries: Vec<String>,

    /// Enable TLS requirement enforcement
    ///
    /// When enabled, authentication commands are only allowed over
    /// encrypted connections.
    pub require_tls_for_auth: bool,

    /// Automatic IP blocking duration
    ///
    /// How long to block an IP after it exceeds security thresholds.
    pub auto_block_duration: Duration,

    /// Enable honeypot detection
    ///
    /// Detects and tracks potential attackers using honeypot techniques.
    pub enable_honeypot: bool,
}

impl Default for SecurityConfig {
    /// Creates a default security configuration suitable for production use
    ///
    /// The default values provide a good balance between security and usability:
    /// - Moderate authentication limits to prevent brute force attacks
    /// - Reasonable rate limits for normal POP3 usage
    /// - Security logging enabled for monitoring
    /// - Conservative connection limits to prevent resource exhaustion
    ///
    /// # Security Level
    ///
    /// This configuration provides **medium** security suitable for most
    /// production environments. For high-security environments, consider
    /// reducing limits and enabling additional features.
    fn default() -> Self {
        Self {
            max_auth_attempts: 3,
            auth_window: Duration::from_secs(300), // 5 minutes
            max_connections_per_ip: 10,
            max_commands_per_minute: 60,
            min_command_delay: Duration::from_millis(100),
            enable_security_logging: true,
            suspicious_threshold: 10,
            max_session_duration: Duration::from_secs(3600), // 1 hour
            enable_geo_blocking: false,
            blocked_countries: Vec::new(),
            require_tls_for_auth: false, // For compatibility
            auto_block_duration: Duration::from_secs(1800), // 30 minutes
            enable_honeypot: false,
        }
    }
}

impl SecurityConfig {
    /// Creates a high-security configuration for sensitive environments
    ///
    /// This configuration prioritizes security over convenience and is
    /// suitable for environments handling sensitive data or facing
    /// high threat levels.
    ///
    /// # Features
    ///
    /// - Strict authentication limits
    /// - Low command rate limits
    /// - TLS enforcement
    /// - Enhanced monitoring
    /// - Automatic blocking
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::security::SecurityConfig;
    ///
    /// let config = SecurityConfig::high_security();
    /// assert_eq!(config.max_auth_attempts, 2);
    /// assert!(config.require_tls_for_auth);
    /// ```
    pub fn high_security() -> Self {
        Self {
            max_auth_attempts: 2,
            auth_window: Duration::from_secs(900), // 15 minutes
            max_connections_per_ip: 3,
            max_commands_per_minute: 20,
            min_command_delay: Duration::from_millis(250),
            enable_security_logging: true,
            suspicious_threshold: 3,
            max_session_duration: Duration::from_secs(1800), // 30 minutes
            enable_geo_blocking: true,
            blocked_countries: vec![], // Configure as needed
            require_tls_for_auth: true,
            auto_block_duration: Duration::from_secs(3600), // 1 hour
            enable_honeypot: true,
        }
    }

    /// Creates a permissive configuration for development/testing
    ///
    /// This configuration is suitable for development and testing
    /// environments where security restrictions might interfere
    /// with testing workflows.
    ///
    /// # Warning
    ///
    /// **DO NOT USE IN PRODUCTION** - This configuration provides
    /// minimal security and is vulnerable to various attacks.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::security::SecurityConfig;
    ///
    /// let config = SecurityConfig::permissive();
    /// assert_eq!(config.max_auth_attempts, 100);
    /// assert!(!config.require_tls_for_auth);
    /// ```
    pub fn permissive() -> Self {
        Self {
            max_auth_attempts: 100,
            auth_window: Duration::from_secs(60),
            max_connections_per_ip: 100,
            max_commands_per_minute: 1000,
            min_command_delay: Duration::from_millis(1),
            enable_security_logging: false,
            suspicious_threshold: 1000,
            max_session_duration: Duration::from_secs(86400), // 24 hours
            enable_geo_blocking: false,
            blocked_countries: Vec::new(),
            require_tls_for_auth: false,
            auto_block_duration: Duration::from_secs(60),
            enable_honeypot: false,
        }
    }

    /// Validates the security configuration
    ///
    /// Ensures that all configuration values are within reasonable
    /// bounds and that the configuration is internally consistent.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Configuration is valid
    /// * `Err(String)` - Configuration error with description
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::security::SecurityConfig;
    ///
    /// let config = SecurityConfig::default();
    /// assert!(config.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if self.max_auth_attempts == 0 {
            return Err("max_auth_attempts must be greater than 0".to_string());
        }

        if self.auth_window.as_secs() < 60 {
            return Err("auth_window must be at least 60 seconds".to_string());
        }

        if self.max_connections_per_ip == 0 {
            return Err("max_connections_per_ip must be greater than 0".to_string());
        }

        if self.max_commands_per_minute == 0 {
            return Err("max_commands_per_minute must be greater than 0".to_string());
        }

        if self.min_command_delay.as_millis() > 5000 {
            return Err("min_command_delay should not exceed 5 seconds".to_string());
        }

        if self.max_session_duration.as_secs() < 300 {
            return Err("max_session_duration should be at least 5 minutes".to_string());
        }

        if self.auto_block_duration.as_secs() < 60 {
            return Err("auto_block_duration should be at least 1 minute".to_string());
        }

        // Validate country codes
        for country in &self.blocked_countries {
            if country.len() != 2 || !country.chars().all(|c| c.is_ascii_uppercase()) {
                return Err(format!("Invalid country code: '{}'. Must be 2-letter ISO 3166-1 alpha-2 code", country));
            }
        }

        Ok(())
    }
}

/// Authentication attempt tracking with enhanced security monitoring
///
/// Tracks authentication attempts per IP address with detailed timing
/// information and security state management.
///
/// # Security Features
///
/// * **Attempt Counting**: Tracks total failed attempts within time window
/// * **Timing Analysis**: Records first and last attempt times for pattern detection
/// * **Blocking State**: Manages temporary IP blocking with expiration
/// * **Pattern Detection**: Identifies suspicious authentication patterns
/// * **Escalation Tracking**: Monitors repeated violations for escalated responses
///
/// # Thread Safety
///
/// This structure is designed to be used within Arc<Mutex<>> for thread-safe
/// access across multiple POP3 sessions.
#[derive(Debug, Clone)]
struct AuthAttempt {
    /// Number of failed authentication attempts in current window
    count: u32,

    /// Timestamp of the first authentication attempt in current window
    first_attempt: Instant,

    /// Timestamp of the most recent authentication attempt
    last_attempt: Instant,

    /// Optional blocking expiration time
    /// If set, the IP is blocked until this time
    blocked_until: Option<Instant>,

    /// Total number of authentication attempts (including successful ones)
    /// Used for long-term pattern analysis
    total_attempts: u32,

    /// Number of times this IP has been blocked
    /// Used for escalating security responses
    block_count: u32,

    /// Last successful authentication time
    /// Used for detecting account compromise patterns
    last_success: Option<Instant>,

    /// Suspicious activity flags
    /// Bit flags indicating various types of suspicious behavior
    suspicious_flags: u32,

    /// User agents seen from this IP
    /// Used for detecting bot/automated attack patterns
    user_agents: Vec<String>,
}

/// Command rate limiting
#[derive(Debug, Clone)]
struct CommandRate {
    count: u32,
    window_start: Instant,
    last_command: Instant,
}

/// Connection tracking
#[derive(Debug, Clone)]
struct ConnectionInfo {
    count: u32,
    first_connection: Instant,
}

/// Security statistics for monitoring and alerting
///
/// Provides comprehensive security metrics for operational monitoring,
/// alerting, and security analysis.
#[derive(Debug, Default)]
pub struct SecurityStats {
    /// Total authentication attempts processed
    pub total_auth_attempts: AtomicU64,

    /// Total failed authentication attempts
    pub failed_auth_attempts: AtomicU64,

    /// Total successful authentications
    pub successful_auth_attempts: AtomicU64,

    /// Total IPs currently blocked
    pub blocked_ips: AtomicU64,

    /// Total commands rate limited
    pub rate_limited_commands: AtomicU64,

    /// Total connections rejected
    pub rejected_connections: AtomicU64,

    /// Total suspicious activities detected
    pub suspicious_activities: AtomicU64,

    /// Total security violations
    pub security_violations: AtomicU64,

    /// Peak concurrent connections
    pub peak_connections: AtomicU64,

    /// Total honeypot hits
    pub honeypot_hits: AtomicU64,
}

impl SecurityStats {
    /// Creates a new statistics instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets a snapshot of current statistics
    pub fn snapshot(&self) -> SecurityStatsSnapshot {
        SecurityStatsSnapshot {
            total_auth_attempts: self.total_auth_attempts.load(Ordering::Relaxed),
            failed_auth_attempts: self.failed_auth_attempts.load(Ordering::Relaxed),
            successful_auth_attempts: self.successful_auth_attempts.load(Ordering::Relaxed),
            blocked_ips: self.blocked_ips.load(Ordering::Relaxed),
            rate_limited_commands: self.rate_limited_commands.load(Ordering::Relaxed),
            rejected_connections: self.rejected_connections.load(Ordering::Relaxed),
            suspicious_activities: self.suspicious_activities.load(Ordering::Relaxed),
            security_violations: self.security_violations.load(Ordering::Relaxed),
            peak_connections: self.peak_connections.load(Ordering::Relaxed),
            honeypot_hits: self.honeypot_hits.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of security statistics at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityStatsSnapshot {
    pub total_auth_attempts: u64,
    pub failed_auth_attempts: u64,
    pub successful_auth_attempts: u64,
    pub blocked_ips: u64,
    pub rate_limited_commands: u64,
    pub rejected_connections: u64,
    pub suspicious_activities: u64,
    pub security_violations: u64,
    pub peak_connections: u64,
    pub honeypot_hits: u64,
}

/// Advanced security manager for POP3 sessions
///
/// Provides comprehensive security features including authentication protection,
/// rate limiting, connection management, and advanced threat detection.
///
/// # Architecture
///
/// The SecurityManager uses thread-safe data structures to track security
/// state across multiple concurrent POP3 sessions. It maintains separate
/// tracking for:
///
/// * **Authentication attempts** per IP address
/// * **Command rates** per session
/// * **Connection counts** per IP address
/// * **Security statistics** for monitoring
///
/// # Performance
///
/// - All operations are O(1) average case with HashMap lookups
/// - Memory usage scales with number of unique IPs and active sessions
/// - Lock contention is minimized through fine-grained locking
/// - Automatic cleanup prevents unbounded memory growth
///
/// # Thread Safety
///
/// All methods are thread-safe and can be called concurrently from
/// multiple POP3 session handlers.
///
/// # Examples
///
/// ```rust
/// use pop3::security::{SecurityManager, SecurityConfig};
/// use std::net::IpAddr;
///
/// let config = SecurityConfig::default();
/// let security = SecurityManager::new(config);
///
/// // Check authentication permission
/// let ip = "192.168.1.100".parse().unwrap();
/// match security.check_auth_allowed(ip) {
///     Ok(()) => println!("Authentication allowed"),
///     Err(_) => println!("Authentication blocked"),
/// }
///
/// // Get security statistics
/// let stats = security.get_stats();
/// println!("Total auth attempts: {}", stats.total_auth_attempts);
/// ```
pub struct SecurityManager {
    /// Security configuration
    config: SecurityConfig,

    /// Authentication attempt tracking per IP
    auth_attempts: Arc<Mutex<HashMap<IpAddr, AuthAttempt>>>,

    /// Command rate limiting per session
    command_rates: Arc<Mutex<HashMap<u64, CommandRate>>>,

    /// Connection tracking per IP
    connections: Arc<Mutex<HashMap<IpAddr, ConnectionInfo>>>,

    /// Security statistics for monitoring
    stats: Arc<SecurityStats>,

    /// Blocked IP addresses with expiration times
    blocked_ips: Arc<Mutex<HashMap<IpAddr, Instant>>>,

    /// Suspicious IP addresses under enhanced monitoring
    suspicious_ips: Arc<Mutex<HashMap<IpAddr, u32>>>, // IP -> violation count
}

impl SecurityManager {
    /// Creates a new SecurityManager with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Security configuration settings
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::security::{SecurityManager, SecurityConfig};
    ///
    /// let config = SecurityConfig::default();
    /// let security = SecurityManager::new(config);
    /// ```
    pub fn new(config: SecurityConfig) -> Self {
        info!(
            max_auth_attempts = config.max_auth_attempts,
            auth_window_secs = config.auth_window.as_secs(),
            max_connections_per_ip = config.max_connections_per_ip,
            max_commands_per_minute = config.max_commands_per_minute,
            "Initializing SecurityManager with configuration"
        );

        Self {
            config,
            auth_attempts: Arc::new(Mutex::new(HashMap::new())),
            command_rates: Arc::new(Mutex::new(HashMap::new())),
            connections: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(SecurityStats::new()),
            blocked_ips: Arc::new(Mutex::new(HashMap::new())),
            suspicious_ips: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Gets current security statistics
    ///
    /// Returns a snapshot of security metrics for monitoring and alerting.
    ///
    /// # Returns
    ///
    /// SecurityStatsSnapshot containing current statistics
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::security::{SecurityManager, SecurityConfig};
    ///
    /// let security = SecurityManager::new(SecurityConfig::default());
    /// let stats = security.get_stats();
    /// println!("Failed attempts: {}", stats.failed_auth_attempts);
    /// ```
    pub fn get_stats(&self) -> SecurityStatsSnapshot {
        self.stats.snapshot()
    }

    /// Gets the current security configuration
    ///
    /// # Returns
    ///
    /// Reference to the current SecurityConfig
    pub fn get_config(&self) -> &SecurityConfig {
        &self.config
    }

    /// Updates the security configuration
    ///
    /// # Arguments
    ///
    /// * `config` - New security configuration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::security::{SecurityManager, SecurityConfig};
    ///
    /// let mut security = SecurityManager::new(SecurityConfig::default());
    /// let new_config = SecurityConfig::high_security();
    /// security.update_config(new_config);
    /// ```
    pub fn update_config(&mut self, config: SecurityConfig) {
        info!("Updating security configuration");
        self.config = config;
    }

    /// Performs periodic cleanup of expired entries
    ///
    /// This method should be called periodically to clean up expired
    /// authentication attempts, blocked IPs, and other time-based entries
    /// to prevent unbounded memory growth.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::security::{SecurityManager, SecurityConfig};
    ///
    /// let security = SecurityManager::new(SecurityConfig::default());
    /// security.cleanup_expired();
    /// ```
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        let mut cleaned_count = 0;

        // Clean up expired authentication attempts
        {
            let mut attempts = self.auth_attempts.lock().unwrap();
            let initial_count = attempts.len();
            attempts.retain(|_ip, attempt| {
                if let Some(blocked_until) = attempt.blocked_until {
                    blocked_until > now
                } else {
                    now.duration_since(attempt.first_attempt) <= self.config.auth_window
                }
            });
            cleaned_count += initial_count - attempts.len();
        }

        // Clean up expired blocked IPs
        {
            let mut blocked = self.blocked_ips.lock().unwrap();
            let initial_count = blocked.len();
            blocked.retain(|_ip, &mut expiry| expiry > now);
            cleaned_count += initial_count - blocked.len();
        }

        // Clean up old command rates (sessions that haven't been active)
        {
            let mut rates = self.command_rates.lock().unwrap();
            let initial_count = rates.len();
            rates.retain(|_session_id, rate| {
                now.duration_since(rate.last_command) <= Duration::from_secs(3600) // 1 hour
            });
            cleaned_count += initial_count - rates.len();
        }

        if cleaned_count > 0 {
            debug!(cleaned_entries = cleaned_count, "Cleaned up expired security entries");
        }
    }

    /// Check if authentication is allowed for this IP address
    ///
    /// Performs comprehensive security checks including:
    /// - Rate limiting based on failed attempts
    /// - IP blocking status
    /// - Geographic restrictions (if enabled)
    /// - Suspicious activity detection
    ///
    /// # Arguments
    ///
    /// * `ip` - IP address to check
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Authentication is allowed
    /// * `Err(Pop3Error)` - Authentication is blocked with reason
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::security::{SecurityManager, SecurityConfig};
    /// use std::net::IpAddr;
    ///
    /// let security = SecurityManager::new(SecurityConfig::default());
    /// let ip = "192.168.1.100".parse().unwrap();
    ///
    /// match security.check_auth_allowed(ip) {
    ///     Ok(()) => println!("Authentication allowed"),
    ///     Err(e) => println!("Authentication blocked: {:?}", e),
    /// }
    /// ```
    pub fn check_auth_allowed(&self, ip: IpAddr) -> Result<(), Pop3Error> {
        let now = Instant::now();

        // Check if IP is explicitly blocked
        {
            let blocked = self.blocked_ips.lock().unwrap();
            if let Some(&expiry) = blocked.get(&ip) {
                if now < expiry {
                    debug!(ip = %ip, "IP is explicitly blocked");
                    self.stats.rejected_connections.fetch_add(1, Ordering::Relaxed);
                    return Err(Pop3Error::RateLimitExceeded);
                }
            }
        }

        // Check geographic restrictions
        if self.config.enable_geo_blocking && !self.config.blocked_countries.is_empty() {
            if let Some(country) = self.get_country_for_ip(ip) {
                if self.config.blocked_countries.contains(&country) {
                    warn!(ip = %ip, country = %country, "Connection blocked due to geographic restrictions");
                    self.stats.rejected_connections.fetch_add(1, Ordering::Relaxed);
                    return Err(Pop3Error::RateLimitExceeded);
                }
            }
        }

        // Check authentication attempts
        let mut attempts = self.auth_attempts.lock().unwrap();

        let should_check_suspicious = if let Some(attempt) = attempts.get(&ip) {
            // Check if still blocked from previous violations
            if let Some(blocked_until) = attempt.blocked_until {
                if now < blocked_until {
                    debug!(
                        ip = %ip,
                        blocked_until = ?blocked_until,
                        "IP blocked due to authentication failures"
                    );
                    self.stats.rejected_connections.fetch_add(1, Ordering::Relaxed);
                    return Err(Pop3Error::RateLimitExceeded);
                }
            }

            // Check if within time window
            if now.duration_since(attempt.first_attempt) > self.config.auth_window {
                // Reset counter if outside window
                debug!(ip = %ip, "Resetting authentication attempt counter (window expired)");
                attempts.remove(&ip);
                false
            } else if attempt.count >= self.config.max_auth_attempts {
                // Block for configured duration
                let mut attempt = attempt.clone();
                attempt.blocked_until = Some(now + self.config.auto_block_duration);
                attempt.block_count += 1;

                warn!(
                    ip = %ip,
                    attempt_count = attempt.count,
                    block_count = attempt.block_count,
                    "IP blocked due to excessive authentication failures"
                );

                attempts.insert(ip, attempt);
                self.stats.blocked_ips.fetch_add(1, Ordering::Relaxed);
                self.stats.security_violations.fetch_add(1, Ordering::Relaxed);

                // Add to suspicious IPs if not already tracked
                {
                    let mut suspicious = self.suspicious_ips.lock().unwrap();
                    let violation_count = suspicious.entry(ip).or_insert(0);
                    *violation_count += 1;

                    if *violation_count >= self.config.suspicious_threshold {
                        error!(
                            ip = %ip,
                            violation_count = *violation_count,
                            "IP marked as highly suspicious due to repeated violations"
                        );
                        self.stats.suspicious_activities.fetch_add(1, Ordering::Relaxed);
                    }
                }

                return Err(Pop3Error::RateLimitExceeded);
            } else {
                // Check for suspicious patterns
                self.is_suspicious_pattern(attempt)
            }
        } else {
            false
        };

        // Handle suspicious pattern detection outside the lock
        if should_check_suspicious {
            warn!(ip = %ip, "Suspicious authentication pattern detected");
            self.stats.suspicious_activities.fetch_add(1, Ordering::Relaxed);

            // Don't block immediately, but flag for monitoring
            let mut suspicious = self.suspicious_ips.lock().unwrap();
            let violation_count = suspicious.entry(ip).or_insert(0);
            *violation_count += 1;
        }

        trace!(ip = %ip, "Authentication check passed");
        Ok(())
    }

    /// Checks for suspicious authentication patterns
    ///
    /// Analyzes authentication attempt patterns to detect potential
    /// automated attacks or suspicious behavior.
    ///
    /// # Arguments
    ///
    /// * `attempt` - Authentication attempt record to analyze
    ///
    /// # Returns
    ///
    /// `true` if suspicious patterns are detected, `false` otherwise
    fn is_suspicious_pattern(&self, attempt: &AuthAttempt) -> bool {
        let now = Instant::now();

        // Check for rapid-fire attempts (faster than human typing)
        if attempt.count > 1 {
            let time_between_attempts = now.duration_since(attempt.last_attempt);
            if time_between_attempts < Duration::from_millis(500) {
                return true;
            }
        }

        // Check for dictionary attack patterns (many attempts in short time)
        if attempt.count >= 5 {
            let total_time = now.duration_since(attempt.first_attempt);
            if total_time < Duration::from_secs(60) {
                return true;
            }
        }

        // Check for repeated blocking (persistent attacker)
        if attempt.block_count >= 3 {
            return true;
        }

        false
    }

    /// Gets the country code for an IP address
    ///
    /// This is a placeholder implementation. In production, this would
    /// integrate with a GeoIP database or service.
    ///
    /// # Arguments
    ///
    /// * `ip` - IP address to look up
    ///
    /// # Returns
    ///
    /// Optional country code (ISO 3166-1 alpha-2)
    fn get_country_for_ip(&self, _ip: IpAddr) -> Option<String> {
        // TODO: Implement actual GeoIP lookup
        // This would typically use MaxMind GeoIP2 or similar service
        None
    }

    /// Record authentication attempt
    pub fn record_auth_attempt(&self, ip: IpAddr, success: bool) {
        let mut attempts = self.auth_attempts.lock().unwrap();
        let now = Instant::now();

        if success {
            // Clear attempts on successful auth
            attempts.remove(&ip);
            return;
        }

        let attempt = attempts.entry(ip).or_insert(AuthAttempt {
            count: 0,
            first_attempt: now,
            last_attempt: now,
            blocked_until: None,
            total_attempts: 0,
            block_count: 0,
            last_success: None,
            suspicious_flags: 0,
            user_agents: Vec::new(),
        });

        attempt.count += 1;
        attempt.last_attempt = now;

        if self.config.enable_security_logging {
            trc::event!(
                Auth(trc::AuthEvent::Failed),
                RemoteIp = ip,
                Total = attempt.count as u64,
            );
        }
    }

    /// Check if new connection is allowed
    pub fn check_connection_allowed(&self, ip: IpAddr) -> Result<(), Pop3Error> {
        let mut connections = self.connections.lock().unwrap();
        let now = Instant::now();

        let conn_info = connections.entry(ip).or_insert(ConnectionInfo {
            count: 0,
            first_connection: now,
        });

        // Reset counter every hour
        if now.duration_since(conn_info.first_connection) > Duration::from_secs(3600) {
            conn_info.count = 0;
            conn_info.first_connection = now;
        }

        if conn_info.count >= self.config.max_connections_per_ip {
            return Err(Pop3Error::RateLimitExceeded);
        }

        conn_info.count += 1;
        Ok(())
    }

    /// Record connection close
    pub fn record_connection_close(&self, ip: IpAddr) {
        let mut connections = self.connections.lock().unwrap();
        if let Some(conn_info) = connections.get_mut(&ip) {
            if conn_info.count > 0 {
                conn_info.count -= 1;
            }
        }
    }

    /// Check command rate limit
    pub fn check_command_rate(&self, session_id: u64) -> Result<(), Pop3Error> {
        let mut rates = self.command_rates.lock().unwrap();
        let now = Instant::now();

        let is_new_session = !rates.contains_key(&session_id);
        let rate = rates.entry(session_id).or_insert(CommandRate {
            count: 0,
            window_start: now,
            last_command: now - self.config.min_command_delay, // Allow first command immediately
        });

        // Check minimum delay between commands (skip for first command)
        if !is_new_session && now.duration_since(rate.last_command) < self.config.min_command_delay {
            return Err(Pop3Error::RateLimitExceeded);
        }

        // Reset counter every minute
        if now.duration_since(rate.window_start) > Duration::from_secs(60) {
            rate.count = 0;
            rate.window_start = now;
        }

        if rate.count >= self.config.max_commands_per_minute {
            return Err(Pop3Error::RateLimitExceeded);
        }

        rate.count += 1;
        rate.last_command = now;
        Ok(())
    }

    /// Detects suspicious activity patterns for enhanced monitoring
    ///
    /// Analyzes various behavioral patterns to identify potential
    /// security threats or automated attacks.
    ///
    /// # Arguments
    ///
    /// * `ip` - IP address to analyze
    /// * `session_id` - Session identifier for rate analysis
    ///
    /// # Returns
    ///
    /// `true` if suspicious activity is detected, `false` otherwise
    pub fn detect_suspicious_activity(&self, ip: IpAddr, session_id: u64) -> bool {
        let attempts = self.auth_attempts.lock().unwrap();
        let rates = self.command_rates.lock().unwrap();

        // Check for excessive authentication attempts
        if let Some(attempt) = attempts.get(&ip) {
            if attempt.count >= self.config.suspicious_threshold {
                if self.config.enable_security_logging {
                    warn!(
                        ip = %ip,
                        attempt_count = attempt.count,
                        "Excessive authentication attempts detected"
                    );
                }
                return true;
            }
        }

        // Check for command flooding
        if let Some(rate) = rates.get(&session_id) {
            if rate.count >= self.config.suspicious_threshold * 6 { // 6x normal rate
                if self.config.enable_security_logging {
                    warn!(
                        ip = %ip,
                        session_id = session_id,
                        command_count = rate.count,
                        "Command flooding detected"
                    );
                }
                return true;
            }
        }

        false
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_auth_rate_limiting() {
        let config = SecurityConfig {
            max_auth_attempts: 3,
            auth_window: Duration::from_secs(60),
            ..Default::default()
        };
        let manager = SecurityManager::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // First 3 attempts should be allowed
        for _ in 0..3 {
            assert!(manager.check_auth_allowed(ip).is_ok());
            manager.record_auth_attempt(ip, false);
        }

        // 4th attempt should be blocked
        assert!(manager.check_auth_allowed(ip).is_err());
    }

    #[test]
    fn test_successful_auth_clears_attempts() {
        let manager = SecurityManager::new(SecurityConfig::default());
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));

        // Record failed attempts
        manager.record_auth_attempt(ip, false);
        manager.record_auth_attempt(ip, false);

        // Successful auth should clear attempts
        manager.record_auth_attempt(ip, true);

        // Should be able to authenticate again
        assert!(manager.check_auth_allowed(ip).is_ok());
    }

    #[test]
    fn test_command_rate_limiting() {
        let config = SecurityConfig {
            max_commands_per_minute: 3, // Lower limit for easier testing
            min_command_delay: Duration::from_millis(1), // Very short delay for testing
            ..Default::default()
        };
        let manager = SecurityManager::new(config);

        // First command should always be allowed
        assert!(manager.check_command_rate(1).is_ok(), "First command should be allowed");

        // Add small delay
        std::thread::sleep(Duration::from_millis(2));

        // Second command should be allowed
        assert!(manager.check_command_rate(1).is_ok(), "Second command should be allowed");

        // Add small delay
        std::thread::sleep(Duration::from_millis(2));

        // Third command should be allowed
        assert!(manager.check_command_rate(1).is_ok(), "Third command should be allowed");

        // Add small delay
        std::thread::sleep(Duration::from_millis(2));

        // Fourth command should be blocked due to rate limit
        assert!(manager.check_command_rate(1).is_err(), "Fourth command should be blocked");
    }
}
