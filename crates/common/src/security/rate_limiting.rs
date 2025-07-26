/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Rate Limiting Module
//!
//! This module provides sophisticated rate limiting capabilities to protect
//! against abuse, DoS attacks, and resource exhaustion.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
    hash::Hash,
};
use tracing::{debug, info};

/// Rate limiting algorithm types
#[derive(Debug, Clone, PartialEq)]
pub enum RateLimitAlgorithm {
    /// Fixed window rate limiting
    FixedWindow,
    /// Sliding window rate limiting
    SlidingWindow,
    /// Token bucket rate limiting
    TokenBucket,
    /// Leaky bucket rate limiting
    LeakyBucket,
}

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests allowed
    pub max_requests: u32,
    /// Time window duration
    pub window_duration: Duration,
    /// Rate limiting algorithm
    pub algorithm: RateLimitAlgorithm,
    /// Burst allowance for token bucket
    pub burst_size: Option<u32>,
    /// Refill rate for token bucket (tokens per second)
    pub refill_rate: Option<f64>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window_duration: Duration::from_secs(60),
            algorithm: RateLimitAlgorithm::SlidingWindow,
            burst_size: None,
            refill_rate: None,
        }
    }
}

/// Rate limit result
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Remaining requests in current window
    pub remaining: u32,
    /// Time until window resets
    pub reset_time: Duration,
    /// Current request count
    pub current_count: u32,
}

/// Rate limiter state for different algorithms
#[derive(Debug, Clone)]
enum RateLimiterState {
    FixedWindow {
        count: u32,
        window_start: Instant,
    },
    SlidingWindow {
        requests: Vec<Instant>,
    },
    TokenBucket {
        tokens: f64,
        last_refill: Instant,
    },
    LeakyBucket {
        queue_size: u32,
        last_leak: Instant,
    },
}

/// Generic rate limiter
pub struct RateLimiter<K: Hash + Eq + Clone> {
    config: RateLimitConfig,
    states: Arc<RwLock<HashMap<K, RateLimiterState>>>,
}

impl<K: Hash + Eq + Clone> RateLimiter<K> {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        info!("Creating rate limiter with config: {:?}", config);
        Self {
            config,
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a request is allowed for the given key
    pub fn check_rate_limit(&self, key: K) -> RateLimitResult {
        debug!("Checking rate limit for key");

        let now = Instant::now();
        let mut states = self.states.write().unwrap();

        let state = states.entry(key).or_insert_with(|| {
            self.create_initial_state(now)
        });

        match &self.config.algorithm {
            RateLimitAlgorithm::FixedWindow => self.check_fixed_window(state, now),
            RateLimitAlgorithm::SlidingWindow => self.check_sliding_window(state, now),
            RateLimitAlgorithm::TokenBucket => self.check_token_bucket(state, now),
            RateLimitAlgorithm::LeakyBucket => self.check_leaky_bucket(state, now),
        }
    }

    /// Create initial state based on algorithm
    fn create_initial_state(&self, now: Instant) -> RateLimiterState {
        match self.config.algorithm {
            RateLimitAlgorithm::FixedWindow => RateLimiterState::FixedWindow {
                count: 0,
                window_start: now,
            },
            RateLimitAlgorithm::SlidingWindow => RateLimiterState::SlidingWindow {
                requests: Vec::new(),
            },
            RateLimitAlgorithm::TokenBucket => RateLimiterState::TokenBucket {
                tokens: self.config.burst_size.unwrap_or(self.config.max_requests) as f64,
                last_refill: now,
            },
            RateLimitAlgorithm::LeakyBucket => RateLimiterState::LeakyBucket {
                queue_size: 0,
                last_leak: now,
            },
        }
    }

    /// Check fixed window rate limit
    fn check_fixed_window(&self, state: &mut RateLimiterState, now: Instant) -> RateLimitResult {
        if let RateLimiterState::FixedWindow { count, window_start } = state {
            // Check if window has expired
            if now.duration_since(*window_start) >= self.config.window_duration {
                *count = 0;
                *window_start = now;
            }

            let allowed = *count < self.config.max_requests;
            if allowed {
                *count += 1;
            }

            let remaining = self.config.max_requests.saturating_sub(*count);
            let reset_time = self.config.window_duration - now.duration_since(*window_start);

            RateLimitResult {
                allowed,
                remaining,
                reset_time,
                current_count: *count,
            }
        } else {
            panic!("Invalid state for fixed window algorithm");
        }
    }

    /// Check sliding window rate limit
    fn check_sliding_window(&self, state: &mut RateLimiterState, now: Instant) -> RateLimitResult {
        if let RateLimiterState::SlidingWindow { requests } = state {
            // Remove expired requests
            let window_start = now - self.config.window_duration;
            requests.retain(|&timestamp| timestamp > window_start);

            let allowed = requests.len() < self.config.max_requests as usize;
            if allowed {
                requests.push(now);
            }

            let remaining = self.config.max_requests.saturating_sub(requests.len() as u32);
            let reset_time = if let Some(&oldest) = requests.first() {
                self.config.window_duration - now.duration_since(oldest)
            } else {
                self.config.window_duration
            };

            RateLimitResult {
                allowed,
                remaining,
                reset_time,
                current_count: requests.len() as u32,
            }
        } else {
            panic!("Invalid state for sliding window algorithm");
        }
    }

    /// Check token bucket rate limit
    fn check_token_bucket(&self, state: &mut RateLimiterState, now: Instant) -> RateLimitResult {
        if let RateLimiterState::TokenBucket { tokens, last_refill } = state {
            let refill_rate = self.config.refill_rate.unwrap_or(
                self.config.max_requests as f64 / self.config.window_duration.as_secs_f64()
            );
            let max_tokens = self.config.burst_size.unwrap_or(self.config.max_requests) as f64;

            // Refill tokens
            let elapsed = now.duration_since(*last_refill).as_secs_f64();
            *tokens = (*tokens + elapsed * refill_rate).min(max_tokens);
            *last_refill = now;

            let allowed = *tokens >= 1.0;
            if allowed {
                *tokens -= 1.0;
            }

            let remaining = tokens.floor() as u32;
            let reset_time = if *tokens < 1.0 {
                Duration::from_secs_f64((1.0 - *tokens) / refill_rate)
            } else {
                Duration::from_secs(0)
            };

            RateLimitResult {
                allowed,
                remaining,
                reset_time,
                current_count: (max_tokens - *tokens) as u32,
            }
        } else {
            panic!("Invalid state for token bucket algorithm");
        }
    }

    /// Check leaky bucket rate limit
    fn check_leaky_bucket(&self, state: &mut RateLimiterState, now: Instant) -> RateLimitResult {
        if let RateLimiterState::LeakyBucket { queue_size, last_leak } = state {
            let leak_rate = self.config.max_requests as f64 / self.config.window_duration.as_secs_f64();

            // Leak requests
            let elapsed = now.duration_since(*last_leak).as_secs_f64();
            let leaked = (elapsed * leak_rate).floor() as u32;
            *queue_size = queue_size.saturating_sub(leaked);
            *last_leak = now;

            let allowed = *queue_size < self.config.max_requests;
            if allowed {
                *queue_size += 1;
            }

            let remaining = self.config.max_requests.saturating_sub(*queue_size);
            let reset_time = if *queue_size > 0 {
                Duration::from_secs_f64(*queue_size as f64 / leak_rate)
            } else {
                Duration::from_secs(0)
            };

            RateLimitResult {
                allowed,
                remaining,
                reset_time,
                current_count: *queue_size,
            }
        } else {
            panic!("Invalid state for leaky bucket algorithm");
        }
    }

    /// Clean up expired states
    pub fn cleanup(&self) {
        debug!("Cleaning up rate limiter states");

        let now = Instant::now();
        let cleanup_threshold = now - self.config.window_duration * 2;

        let mut states = self.states.write().unwrap();
        states.retain(|_, state| {
            match state {
                RateLimiterState::FixedWindow { window_start, .. } => {
                    *window_start > cleanup_threshold
                }
                RateLimiterState::SlidingWindow { requests } => {
                    !requests.is_empty() && requests.iter().any(|&t| t > cleanup_threshold)
                }
                RateLimiterState::TokenBucket { last_refill, .. } => {
                    *last_refill > cleanup_threshold
                }
                RateLimiterState::LeakyBucket { last_leak, .. } => {
                    *last_leak > cleanup_threshold
                }
            }
        });

        info!("Rate limiter cleanup completed, {} states remaining", states.len());
    }

    /// Get current statistics
    pub fn get_stats(&self) -> RateLimiterStats {
        let states = self.states.read().unwrap();
        let active_keys = states.len();

        let mut total_requests = 0;
        let _blocked_requests = 0;

        for state in states.values() {
            match state {
                RateLimiterState::FixedWindow { count, .. } => {
                    total_requests += *count as u64;
                }
                RateLimiterState::SlidingWindow { requests } => {
                    total_requests += requests.len() as u64;
                }
                RateLimiterState::TokenBucket { tokens, .. } => {
                    let max_tokens = self.config.burst_size.unwrap_or(self.config.max_requests) as f64;
                    total_requests += (max_tokens - *tokens) as u64;
                }
                RateLimiterState::LeakyBucket { queue_size, .. } => {
                    total_requests += *queue_size as u64;
                }
            }
        }

        RateLimiterStats {
            active_keys,
            total_requests,
            blocked_requests: 0, // TODO: Implement blocked request tracking
            algorithm: self.config.algorithm.clone(),
        }
    }
}

/// Rate limiter statistics
#[derive(Debug, Clone)]
pub struct RateLimiterStats {
    pub active_keys: usize,
    pub total_requests: u64,
    pub blocked_requests: u64,
    pub algorithm: RateLimitAlgorithm,
}

/// IP-based rate limiter
pub type IpRateLimiter = RateLimiter<String>;

/// User-based rate limiter
pub type UserRateLimiter = RateLimiter<u32>;

/// Endpoint-based rate limiter
pub type EndpointRateLimiter = RateLimiter<(String, String)>; // (IP, endpoint)

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_fixed_window_rate_limiter() {
        let config = RateLimitConfig {
            max_requests: 3,
            window_duration: Duration::from_millis(100),
            algorithm: RateLimitAlgorithm::FixedWindow,
            ..Default::default()
        };

        let limiter = RateLimiter::new(config);
        let key = "test_key".to_string();

        // First 3 requests should be allowed
        for _ in 0..3 {
            let result = limiter.check_rate_limit(key.clone());
            assert!(result.allowed);
        }

        // 4th request should be blocked
        let result = limiter.check_rate_limit(key.clone());
        assert!(!result.allowed);

        // After window expires, should be allowed again
        thread::sleep(Duration::from_millis(150));
        let result = limiter.check_rate_limit(key);
        assert!(result.allowed);
    }

    #[test]
    fn test_sliding_window_rate_limiter() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_duration: Duration::from_millis(100),
            algorithm: RateLimitAlgorithm::SlidingWindow,
            ..Default::default()
        };

        let limiter = RateLimiter::new(config);
        let key = "test_key".to_string();

        // First 2 requests should be allowed
        assert!(limiter.check_rate_limit(key.clone()).allowed);
        assert!(limiter.check_rate_limit(key.clone()).allowed);

        // 3rd request should be blocked
        assert!(!limiter.check_rate_limit(key.clone()).allowed);

        // After partial window, should still be blocked
        thread::sleep(Duration::from_millis(50));
        assert!(!limiter.check_rate_limit(key.clone()).allowed);

        // After full window, should be allowed
        thread::sleep(Duration::from_millis(60));
        assert!(limiter.check_rate_limit(key).allowed);
    }

    #[test]
    fn test_token_bucket_rate_limiter() {
        let config = RateLimitConfig {
            max_requests: 10,
            window_duration: Duration::from_secs(1),
            algorithm: RateLimitAlgorithm::TokenBucket,
            burst_size: Some(5),
            refill_rate: Some(2.0), // 2 tokens per second
        };

        let limiter = RateLimiter::new(config);
        let key = "test_key".to_string();

        // Should allow burst of 5 requests
        for _ in 0..5 {
            let result = limiter.check_rate_limit(key.clone());
            assert!(result.allowed);
        }

        // 6th request should be blocked
        let result = limiter.check_rate_limit(key.clone());
        assert!(!result.allowed);

        // After some time, should refill tokens
        thread::sleep(Duration::from_millis(600)); // Should refill ~1 token
        let result = limiter.check_rate_limit(key);
        assert!(result.allowed);
    }

    #[test]
    fn test_rate_limiter_cleanup() {
        let config = RateLimitConfig {
            max_requests: 10,
            window_duration: Duration::from_millis(50),
            algorithm: RateLimitAlgorithm::SlidingWindow,
            ..Default::default()
        };

        let limiter = RateLimiter::new(config);

        // Create some states
        for i in 0..5 {
            limiter.check_rate_limit(format!("key_{}", i));
        }

        let stats_before = limiter.get_stats();
        assert_eq!(stats_before.active_keys, 5);

        // Wait for states to expire
        thread::sleep(Duration::from_millis(150));

        // Cleanup should remove expired states
        limiter.cleanup();
        let stats_after = limiter.get_stats();
        assert_eq!(stats_after.active_keys, 0);
    }
}
