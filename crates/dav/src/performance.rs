/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! DAV Server Performance Optimization Module
//!
//! This module provides performance optimization features for the DAV server,
//! including caching, connection pooling, and request optimization.

use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tracing::{debug, info};

/// Performance optimizer for DAV operations
///
/// Provides caching, connection pooling, and other performance optimizations
/// to improve DAV server response times and resource utilization.
#[derive(Debug, Clone)]
pub struct DavPerformance {
    inner: Arc<RwLock<DavPerformanceInner>>,
    config: PerformanceConfig,
}

#[derive(Debug)]
struct DavPerformanceInner {
    /// Response cache for frequently accessed resources
    response_cache: LRU<CacheKey, CacheEntry>,
    /// Property cache for WebDAV properties
    property_cache: LRU<String, PropertyCacheEntry>,
    /// Connection pool statistics
    connection_stats: ConnectionStats,
    /// Performance metrics
    metrics: PerformanceMetrics,
}

/// Performance configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceConfig {
    /// Maximum number of cached responses
    pub max_cache_entries: usize,
    /// Cache entry TTL
    pub cache_ttl: Duration,
    /// Maximum property cache entries
    pub max_property_cache: usize,
    /// Property cache TTL
    pub property_cache_ttl: Duration,
    /// Enable response compression
    pub enable_compression: bool,
    /// Compression threshold in bytes
    pub compression_threshold: usize,
    /// Maximum concurrent connections per IP
    pub max_connections_per_ip: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_cache_entries: 1000,
            cache_ttl: Duration::from_secs(300), // 5 minutes
            max_property_cache: 5000,
            property_cache_ttl: Duration::from_secs(600), // 10 minutes
            enable_compression: true,
            compression_threshold: 1024, // 1KB
            max_connections_per_ip: 10,
            connection_timeout: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    path: String,
    method: String,
    etag: Option<String>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    data: Vec<u8>,
    content_type: String,
    etag: String,
    created_at: Instant,
    access_count: u64,
    last_accessed: Instant,
}

#[derive(Debug, Clone)]
struct PropertyCacheEntry {
    properties: Vec<u8>, // Serialized properties
    created_at: Instant,
    last_accessed: Instant,
}

#[derive(Debug, Clone, Default)]
struct ConnectionStats {
    active_connections: u64,
    total_connections: u64,
    connection_errors: u64,
    average_connection_time: Duration,
}

#[derive(Debug, Clone, Default)]
struct PerformanceMetrics {
    cache_hits: u64,
    cache_misses: u64,
    cache_evictions: u64,
    compression_savings: u64,
    total_requests: u64,
    average_response_time: Duration,
}

impl DavPerformance {
    /// Create a new performance optimizer
    pub fn new(config: PerformanceConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(DavPerformanceInner {
                response_cache: LRU::new(config.max_cache_entries),
                property_cache: LRU::new(config.max_property_cache),
                connection_stats: ConnectionStats::default(),
                metrics: PerformanceMetrics::default(),
            })),
            config,
        }
    }

    /// Check if a response is cached
    pub fn get_cached_response(&self, path: &str, method: &str, etag: Option<&str>) -> Option<CachedResponse> {
        if let Ok(mut inner) = self.inner.write() {
            let key = CacheKey {
                path: path.to_string(),
                method: method.to_string(),
                etag: etag.map(|s| s.to_string()),
            };

            if let Some(entry) = inner.response_cache.get_mut(&key) {
                // Check if entry is still valid
                if entry.created_at.elapsed() < self.config.cache_ttl {
                    entry.access_count += 1;
                    entry.last_accessed = Instant::now();
                    let access_count = entry.access_count;

                    let response = CachedResponse {
                        data: entry.data.clone(),
                        content_type: entry.content_type.clone(),
                        etag: entry.etag.clone(),
                    };

                    inner.metrics.cache_hits += 1;

                    debug!(
                        path = path,
                        method = method,
                        access_count = access_count,
                        "Cache hit for response"
                    );

                    return Some(response);
                } else {
                    // Entry expired, remove it
                    inner.response_cache.remove(&key);
                    inner.metrics.cache_evictions += 1;
                }
            }

            inner.metrics.cache_misses += 1;
        }

        None
    }

    /// Cache a response
    pub fn cache_response(
        &self,
        path: &str,
        method: &str,
        data: Vec<u8>,
        content_type: String,
        etag: String,
    ) {
        if let Ok(mut inner) = self.inner.write() {
            let key = CacheKey {
                path: path.to_string(),
                method: method.to_string(),
                etag: Some(etag.clone()),
            };

            let entry = CacheEntry {
                data,
                content_type,
                etag,
                created_at: Instant::now(),
                access_count: 0,
                last_accessed: Instant::now(),
            };

            if let Some(_old_entry) = inner.response_cache.put(key.clone(), entry) {
                inner.metrics.cache_evictions += 1;
            }

            debug!(
                path = path,
                method = method,
                cache_size = inner.response_cache.len(),
                "Response cached"
            );
        }
    }

    /// Get cached properties
    pub fn get_cached_properties(&self, resource_path: &str) -> Option<Vec<u8>> {
        if let Ok(mut inner) = self.inner.write() {
            let key = resource_path.to_string();
            if let Some(entry) = inner.property_cache.get_mut(&key) {
                if entry.created_at.elapsed() < self.config.property_cache_ttl {
                    entry.last_accessed = Instant::now();

                    debug!(
                        path = resource_path,
                        "Property cache hit"
                    );

                    return Some(entry.properties.clone());
                } else {
                    inner.property_cache.remove(&key);
                }
            }
        }

        None
    }

    /// Cache properties
    pub fn cache_properties(&self, resource_path: &str, properties: Vec<u8>) {
        if let Ok(mut inner) = self.inner.write() {
            let entry = PropertyCacheEntry {
                properties,
                created_at: Instant::now(),
                last_accessed: Instant::now(),
            };

            inner.property_cache.put(resource_path.to_string(), entry);

            debug!(
                path = resource_path,
                cache_size = inner.property_cache.len(),
                "Properties cached"
            );
        }
    }

    /// Check if response should be compressed
    pub fn should_compress(&self, content_length: usize, content_type: &str) -> bool {
        if !self.config.enable_compression {
            return false;
        }

        if content_length < self.config.compression_threshold {
            return false;
        }

        // Compress text-based content types
        matches!(
            content_type,
            "text/xml" | "application/xml" | "text/calendar" | "text/vcard" | "application/json"
        )
    }

    /// Record compression savings
    pub fn record_compression(&self, original_size: usize, compressed_size: usize) {
        if let Ok(mut inner) = self.inner.write() {
            inner.metrics.compression_savings += (original_size - compressed_size) as u64;

            debug!(
                original_size = original_size,
                compressed_size = compressed_size,
                savings = original_size - compressed_size,
                "Compression applied"
            );
        }
    }

    /// Record request metrics
    pub fn record_request(&self, response_time: Duration) {
        if let Ok(mut inner) = self.inner.write() {
            inner.metrics.total_requests += 1;

            // Update average response time using exponential moving average
            let alpha = 0.1; // Smoothing factor
            let new_avg = Duration::from_nanos(
                (inner.metrics.average_response_time.as_nanos() as f64 * (1.0 - alpha) +
                 response_time.as_nanos() as f64 * alpha) as u64
            );
            inner.metrics.average_response_time = new_avg;
        }
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> PerformanceStats {
        if let Ok(inner) = self.inner.read() {
            let cache_hit_rate = if inner.metrics.cache_hits + inner.metrics.cache_misses > 0 {
                inner.metrics.cache_hits as f64 /
                (inner.metrics.cache_hits + inner.metrics.cache_misses) as f64
            } else {
                0.0
            };

            PerformanceStats {
                cache_hit_rate,
                cache_entries: inner.response_cache.len(),
                property_cache_entries: inner.property_cache.len(),
                total_requests: inner.metrics.total_requests,
                average_response_time: inner.metrics.average_response_time,
                compression_savings: inner.metrics.compression_savings,
                cache_hits: inner.metrics.cache_hits,
                cache_misses: inner.metrics.cache_misses,
                cache_evictions: inner.metrics.cache_evictions,
                active_connections: inner.connection_stats.active_connections,
            }
        } else {
            PerformanceStats::default()
        }
    }

    /// Clear all caches
    pub fn clear_caches(&self) {
        if let Ok(mut inner) = self.inner.write() {
            inner.response_cache.clear();
            inner.property_cache.clear();

            info!("All caches cleared");
        }
    }

    /// Optimize cache by removing least recently used entries
    pub fn optimize_cache(&self) {
        if let Ok(mut inner) = self.inner.write() {
            let _now = Instant::now();

            // Remove expired entries from response cache
            let mut expired_keys = Vec::new();
            for (key, entry) in inner.response_cache.iter() {
                if entry.created_at.elapsed() > self.config.cache_ttl {
                    expired_keys.push(key.clone());
                }
            }

            for key in expired_keys {
                inner.response_cache.remove(&key);
                inner.metrics.cache_evictions += 1;
            }

            // Remove expired entries from property cache
            let mut expired_paths = Vec::new();
            for (path, entry) in inner.property_cache.iter() {
                if entry.created_at.elapsed() > self.config.property_cache_ttl {
                    expired_paths.push(path.clone());
                }
            }

            for path in expired_paths {
                inner.property_cache.remove(&path);
            }

            debug!(
                response_cache_size = inner.response_cache.len(),
                property_cache_size = inner.property_cache.len(),
                "Cache optimization completed"
            );
        }
    }
}

/// Cached response data
#[derive(Debug, Clone)]
pub struct CachedResponse {
    pub data: Vec<u8>,
    pub content_type: String,
    pub etag: String,
}

/// Performance statistics
#[derive(Debug, Clone, Default)]
pub struct PerformanceStats {
    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f64,
    /// Number of cached responses
    pub cache_entries: usize,
    /// Number of cached properties
    pub property_cache_entries: usize,
    /// Total requests processed
    pub total_requests: u64,
    /// Average response time
    pub average_response_time: Duration,
    /// Total bytes saved through compression
    pub compression_savings: u64,
    /// Total cache hits
    pub cache_hits: u64,
    /// Total cache misses
    pub cache_misses: u64,
    /// Total cache evictions
    pub cache_evictions: u64,
    /// Active connections
    pub active_connections: u64,
}

// Simple LRU cache implementation
#[derive(Debug)]
struct LRU<K, V> {
    map: HashMap<K, V>,
    capacity: usize,
}

impl<K: Clone + Hash + Eq, V> LRU<K, V> {
    fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
            capacity,
        }
    }

    fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.map.get_mut(key)
    }

    fn put(&mut self, key: K, value: V) -> Option<V> {
        if self.map.len() >= self.capacity && !self.map.contains_key(&key) {
            // Remove a random entry (simplified LRU)
            if let Some(old_key) = self.map.keys().next().cloned() {
                self.map.remove(&old_key);
            }
        }
        self.map.insert(key, value)
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        self.map.remove(key)
    }

    fn clear(&mut self) {
        self.map.clear();
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    fn iter(&self) -> std::collections::hash_map::Iter<K, V> {
        self.map.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_caching() {
        let performance = DavPerformance::new(PerformanceConfig::default());

        // Cache a response
        performance.cache_response(
            "/calendar/user/personal",
            "GET",
            b"test data".to_vec(),
            "text/calendar".to_string(),
            "etag123".to_string(),
        );

        // Retrieve cached response
        let cached = performance.get_cached_response("/calendar/user/personal", "GET", Some("etag123"));
        assert!(cached.is_some());

        let cached = cached.unwrap();
        assert_eq!(cached.data, b"test data");
        assert_eq!(cached.content_type, "text/calendar");
        assert_eq!(cached.etag, "etag123");
    }

    #[test]
    fn test_property_caching() {
        let performance = DavPerformance::new(PerformanceConfig::default());

        // Cache properties
        let properties = b"serialized properties".to_vec();
        performance.cache_properties("/calendar/user/personal", properties.clone());

        // Retrieve cached properties
        let cached = performance.get_cached_properties("/calendar/user/personal");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), properties);
    }

    #[test]
    fn test_compression_decision() {
        let performance = DavPerformance::new(PerformanceConfig::default());

        // Should compress large XML content
        assert!(performance.should_compress(2048, "text/xml"));
        assert!(performance.should_compress(2048, "application/xml"));
        assert!(performance.should_compress(2048, "text/calendar"));

        // Should not compress small content
        assert!(!performance.should_compress(512, "text/xml"));

        // Should not compress binary content
        assert!(!performance.should_compress(2048, "image/jpeg"));
        assert!(!performance.should_compress(2048, "application/octet-stream"));
    }

    #[test]
    fn test_performance_stats() {
        let performance = DavPerformance::new(PerformanceConfig::default());

        // Record some metrics
        performance.record_request(Duration::from_millis(100));
        performance.record_request(Duration::from_millis(200));
        performance.record_compression(2048, 1024);

        let stats = performance.get_performance_stats();
        assert_eq!(stats.total_requests, 2);
        assert!(stats.average_response_time > Duration::ZERO);
        assert_eq!(stats.compression_savings, 1024);
    }

    #[test]
    fn test_cache_optimization() {
        let config = PerformanceConfig {
            cache_ttl: Duration::from_millis(100),
            ..Default::default()
        };
        let performance = DavPerformance::new(config);

        // Add some cache entries
        performance.cache_response(
            "/test1",
            "GET",
            b"data1".to_vec(),
            "text/plain".to_string(),
            "etag1".to_string(),
        );

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));

        // Optimize cache (should remove expired entries)
        performance.optimize_cache();

        // Entry should be gone
        let cached = performance.get_cached_response("/test1", "GET", Some("etag1"));
        assert!(cached.is_none());
    }

    #[test]
    fn test_cache_hit_rate() {
        let performance = DavPerformance::new(PerformanceConfig::default());

        // Cache a response
        performance.cache_response(
            "/test",
            "GET",
            b"data".to_vec(),
            "text/plain".to_string(),
            "etag".to_string(),
        );

        // Hit the cache
        let _ = performance.get_cached_response("/test", "GET", Some("etag"));

        // Miss the cache
        let _ = performance.get_cached_response("/other", "GET", Some("etag"));

        let stats = performance.get_performance_stats();
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.cache_hit_rate, 0.5);
    }
}
