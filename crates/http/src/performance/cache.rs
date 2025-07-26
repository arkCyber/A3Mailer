/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! HTTP Response Caching Performance Optimizations
//!
//! This module provides intelligent caching mechanisms for HTTP responses to reduce
//! latency, minimize redundant requests, and improve overall system performance.

use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tracing::{debug, info};

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in cache
    pub max_entries: usize,
    /// Default TTL for cache entries
    pub default_ttl: Duration,
    /// Maximum size of cached response body
    pub max_response_size: usize,
    /// Whether to cache responses with errors
    pub cache_errors: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            default_ttl: Duration::from_secs(300), // 5 minutes
            max_response_size: 1024 * 1024, // 1MB
            cache_errors: false,
        }
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub entries: usize,
    pub total_size: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
}

/// Cache key for HTTP requests
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheKey {
    pub method: String,
    pub url: String,
    pub headers_hash: u64,
}

impl CacheKey {
    pub fn new(method: &str, url: &str, headers: &[(String, String)]) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for (name, value) in headers {
            name.hash(&mut hasher);
            value.hash(&mut hasher);
        }

        Self {
            method: method.to_string(),
            url: url.to_string(),
            headers_hash: hasher.finish(),
        }
    }
}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.method.hash(state);
        self.url.hash(state);
        self.headers_hash.hash(state);
    }
}

/// Cached HTTP response
#[derive(Debug, Clone)]
pub struct CachedResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub created_at: Instant,
    pub ttl: Duration,
    pub size: usize,
}

impl CachedResponse {
    pub fn new(
        status: u16,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
        ttl: Duration,
    ) -> Self {
        let size = body.len() + headers.iter()
            .map(|(k, v)| k.len() + v.len())
            .sum::<usize>();

        Self {
            status,
            headers,
            body,
            created_at: Instant::now(),
            ttl,
            size,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }

    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

/// LRU Cache for HTTP responses
pub struct HttpCache {
    config: CacheConfig,
    entries: Arc<RwLock<HashMap<CacheKey, CachedResponse>>>,
    access_order: Arc<RwLock<Vec<CacheKey>>>,
    stats: Arc<RwLock<CacheStats>>,
}

impl HttpCache {
    /// Create a new HTTP cache with default configuration
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Create a new HTTP cache with custom configuration
    pub fn with_config(config: CacheConfig) -> Self {
        info!("Creating HTTP cache with config: {:?}", config);
        Self {
            config,
            entries: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Get a cached response
    pub fn get(&self, key: &CacheKey) -> Option<CachedResponse> {
        debug!("Cache lookup for key: {:?}", key);

        let entries = self.entries.read().unwrap();
        if let Some(response) = entries.get(key) {
            if response.is_expired() {
                debug!("Cache entry expired for key: {:?}", key);
                drop(entries);
                self.remove(key);
                self.update_stats(|stats| stats.misses += 1);
                return None;
            }

            debug!("Cache hit for key: {:?}", key);
            self.update_access_order(key);
            self.update_stats(|stats| stats.hits += 1);
            return Some(response.clone());
        }

        debug!("Cache miss for key: {:?}", key);
        self.update_stats(|stats| stats.misses += 1);
        None
    }

    /// Store a response in the cache
    pub fn put(&self, key: CacheKey, response: CachedResponse) {
        debug!("Storing response in cache for key: {:?}", key);

        // Check if response is too large
        if response.size > self.config.max_response_size {
            debug!("Response too large to cache: {} bytes", response.size);
            return;
        }

        // Don't cache errors unless configured to do so
        if !self.config.cache_errors && response.status >= 400 {
            debug!("Not caching error response: {}", response.status);
            return;
        }

        let mut entries = self.entries.write().unwrap();
        let mut access_order = self.access_order.write().unwrap();

        // If cache is full, evict LRU entry
        if entries.len() >= self.config.max_entries && !entries.contains_key(&key) {
            if let Some(lru_key) = access_order.first().cloned() {
                debug!("Evicting LRU entry: {:?}", lru_key);
                entries.remove(&lru_key);
                access_order.retain(|k| k != &lru_key);
                self.update_stats(|stats| {
                    stats.evictions += 1;
                    stats.entries = entries.len();
                });
            }
        }

        // Add or update entry
        let is_new = !entries.contains_key(&key);
        let response_size = response.size;
        entries.insert(key.clone(), response);

        // Update access order
        access_order.retain(|k| k != &key);
        access_order.push(key.clone());

        if is_new {
            self.update_stats(|stats| {
                stats.entries = entries.len();
                stats.total_size += response_size;
            });
        }

        info!("Cached response for key: {:?}", key);
    }

    /// Remove a specific entry from the cache
    pub fn remove(&self, key: &CacheKey) {
        debug!("Removing cache entry for key: {:?}", key);

        let mut entries = self.entries.write().unwrap();
        let mut access_order = self.access_order.write().unwrap();

        if let Some(response) = entries.remove(key) {
            access_order.retain(|k| k != key);
            self.update_stats(|stats| {
                stats.entries = entries.len();
                stats.total_size -= response.size;
            });
        }
    }

    /// Clear all expired entries
    pub fn cleanup_expired(&self) {
        debug!("Cleaning up expired cache entries");

        let mut entries = self.entries.write().unwrap();
        let mut access_order = self.access_order.write().unwrap();

        let expired_keys: Vec<CacheKey> = entries
            .iter()
            .filter(|(_, response)| response.is_expired())
            .map(|(key, _)| key.clone())
            .collect();

        let mut total_size_removed = 0;
        for key in &expired_keys {
            if let Some(response) = entries.remove(key) {
                total_size_removed += response.size;
            }
        }

        access_order.retain(|key| !expired_keys.contains(key));

        if !expired_keys.is_empty() {
            info!("Cleaned up {} expired cache entries", expired_keys.len());
            self.update_stats(|stats| {
                stats.entries = entries.len();
                stats.total_size -= total_size_removed;
            });
        }
    }

    /// Clear all cache entries
    pub fn clear(&self) {
        info!("Clearing all cache entries");

        let mut entries = self.entries.write().unwrap();
        let mut access_order = self.access_order.write().unwrap();

        entries.clear();
        access_order.clear();

        self.update_stats(|stats| {
            stats.entries = 0;
            stats.total_size = 0;
        });
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        self.stats.read().unwrap().clone()
    }

    /// Get cache configuration
    pub fn get_config(&self) -> &CacheConfig {
        &self.config
    }

    // Private helper methods

    fn update_access_order(&self, key: &CacheKey) {
        let mut access_order = self.access_order.write().unwrap();
        access_order.retain(|k| k != key);
        access_order.push(key.clone());
    }

    fn update_stats<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut CacheStats),
    {
        if let Ok(mut stats) = self.stats.write() {
            update_fn(&mut stats);
        }
    }
}

impl Default for HttpCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic_operations() {
        let cache = HttpCache::new();
        let key = CacheKey::new("GET", "http://example.com", &[]);
        let response = CachedResponse::new(
            200,
            vec![("content-type".to_string(), "text/html".to_string())],
            b"Hello, World!".to_vec(),
            Duration::from_secs(60),
        );

        // Cache miss
        assert!(cache.get(&key).is_none());

        // Store response
        cache.put(key.clone(), response.clone());

        // Cache hit
        let cached = cache.get(&key).unwrap();
        assert_eq!(cached.status, response.status);
        assert_eq!(cached.body, response.body);

        let stats = cache.get_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.entries, 1);
    }

    #[test]
    fn test_cache_expiration() {
        let cache = HttpCache::new();
        let key = CacheKey::new("GET", "http://example.com", &[]);
        let response = CachedResponse::new(
            200,
            vec![],
            b"Hello".to_vec(),
            Duration::from_millis(10),
        );

        cache.put(key.clone(), response);

        // Should be cached
        assert!(cache.get(&key).is_some());

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(20));

        // Should be expired
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn test_cache_eviction() {
        let config = CacheConfig {
            max_entries: 2,
            ..Default::default()
        };
        let cache = HttpCache::with_config(config);

        // Fill cache
        for i in 0..3 {
            let key = CacheKey::new("GET", &format!("http://example{}.com", i), &[]);
            let response = CachedResponse::new(200, vec![], vec![i], Duration::from_secs(60));
            cache.put(key, response);
        }

        let stats = cache.get_stats();
        assert_eq!(stats.entries, 2);
        assert_eq!(stats.evictions, 1);
    }
}
