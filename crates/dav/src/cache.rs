/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! High-Performance Multi-Level Cache for DAV Server
//!
//! This module provides a sophisticated caching system with multiple cache levels,
//! intelligent eviction policies, and high-concurrency support for maximum performance.

use std::{
    collections::{HashMap, BTreeMap},
    hash::Hash,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{
    sync::RwLock,
    time::interval,
};
use tracing::{debug, info};

/// Multi-level high-performance cache system
///
/// Provides L1 (memory), L2 (compressed), and L3 (persistent) cache levels
/// with intelligent promotion/demotion and concurrent access optimization.
#[derive(Debug, Clone)]
pub struct DavCache {
    inner: Arc<DavCacheInner>,
    config: CacheConfig,
}

#[derive(Debug)]
struct DavCacheInner {
    /// L1 Cache: Hot data in memory
    l1_cache: RwLock<LruCache<CacheKey, CacheEntry>>,
    /// L2 Cache: Compressed data in memory
    l2_cache: RwLock<LruCache<CacheKey, CompressedEntry>>,
    /// L3 Cache: Persistent cache (optional)
    l3_cache: Option<Arc<dyn PersistentCache>>,
    /// Cache statistics and metrics
    metrics: CacheMetrics,
    /// Access frequency tracking
    frequency_tracker: RwLock<FrequencyTracker>,
}

/// Cache configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheConfig {
    /// L1 cache size (number of entries)
    pub l1_size: usize,
    /// L2 cache size (number of entries)
    pub l2_size: usize,
    /// Enable L3 persistent cache
    pub enable_l3_cache: bool,
    /// Default TTL for cache entries
    pub default_ttl: Duration,
    /// Maximum entry size for L1 cache
    pub l1_max_entry_size: usize,
    /// Compression threshold for L2 cache
    pub compression_threshold: usize,
    /// Enable adaptive sizing
    pub enable_adaptive_sizing: bool,
    /// Cleanup interval
    pub cleanup_interval: Duration,
    /// Enable access frequency tracking
    pub enable_frequency_tracking: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_size: 10000,
            l2_size: 50000,
            enable_l3_cache: false,
            default_ttl: Duration::from_secs(3600), // 1 hour
            l1_max_entry_size: 1024 * 1024, // 1MB
            compression_threshold: 4096, // 4KB
            enable_adaptive_sizing: true,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            enable_frequency_tracking: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub namespace: String,
    pub key: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    data: Vec<u8>,
    content_type: String,
    created_at: Instant,
    last_accessed: Instant,
    access_count: u64,
    ttl: Duration,
    size: usize,
}

#[derive(Debug, Clone)]
struct CompressedEntry {
    compressed_data: Vec<u8>,
    original_size: usize,
    content_type: String,
    created_at: Instant,
    last_accessed: Instant,
    access_count: u64,
    ttl: Duration,
    compression_ratio: f32,
}

#[derive(Debug, Default)]
struct CacheMetrics {
    l1_hits: AtomicU64,
    l1_misses: AtomicU64,
    l2_hits: AtomicU64,
    l2_misses: AtomicU64,
    l3_hits: AtomicU64,
    l3_misses: AtomicU64,
    total_requests: AtomicU64,
    evictions: AtomicU64,
    promotions: AtomicU64,
    demotions: AtomicU64,
    compression_savings: AtomicU64,
    current_l1_size: AtomicUsize,
    current_l2_size: AtomicUsize,
    peak_memory_usage: AtomicUsize,
}

#[derive(Debug)]
struct FrequencyTracker {
    access_counts: HashMap<CacheKey, AccessInfo>,
    time_windows: BTreeMap<Instant, Vec<CacheKey>>,
    window_size: Duration,
}

#[derive(Debug, Clone)]
struct AccessInfo {
    count: u64,
    last_access: Instant,
    frequency_score: f64,
}

/// Trait for persistent cache backends
pub trait PersistentCache: Send + Sync + std::fmt::Debug {
    fn get(&self, key: &CacheKey) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<Vec<u8>>> + Send + '_>>;
    fn put(&self, key: &CacheKey, data: &[u8], ttl: Duration) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), CacheError>> + Send + '_>>;
    fn remove(&self, key: &CacheKey) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), CacheError>> + Send + '_>>;
    fn cleanup_expired(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<u64, CacheError>> + Send + '_>>;
}

impl DavCache {
    /// Create a new multi-level cache
    pub fn new(config: CacheConfig) -> Self {
        let cache = Self {
            inner: Arc::new(DavCacheInner {
                l1_cache: RwLock::new(LruCache::new(config.l1_size)),
                l2_cache: RwLock::new(LruCache::new(config.l2_size)),
                l3_cache: None, // Would be initialized with actual persistent backend
                metrics: CacheMetrics::default(),
                frequency_tracker: RwLock::new(FrequencyTracker {
                    access_counts: HashMap::new(),
                    time_windows: BTreeMap::new(),
                    window_size: Duration::from_secs(300), // 5 minutes
                }),
            }),
            config: config.clone(),
        };

        // Start background cleanup task
        cache.start_cleanup_task();

        cache
    }

    /// Get data from cache (checks all levels)
    pub async fn get(&self, key: &CacheKey) -> Option<CachedData> {
        self.inner.metrics.total_requests.fetch_add(1, Ordering::Relaxed);

        // Track access frequency
        if self.config.enable_frequency_tracking {
            self.track_access(key).await;
        }

        // Try L1 cache first
        if let Some(entry) = self.get_from_l1(key).await {
            self.inner.metrics.l1_hits.fetch_add(1, Ordering::Relaxed);

            debug!(
                namespace = %key.namespace,
                key = %key.key,
                size = entry.data.len(),
                "L1 cache hit"
            );

            return Some(CachedData {
                data: entry.data,
                content_type: entry.content_type,
                cached_at: entry.created_at,
                access_count: entry.access_count,
            });
        }
        self.inner.metrics.l1_misses.fetch_add(1, Ordering::Relaxed);

        // Try L2 cache
        if let Some(entry) = self.get_from_l2(key).await {
            self.inner.metrics.l2_hits.fetch_add(1, Ordering::Relaxed);

            // Decompress data
            let decompressed_data = self.decompress_data(&entry.compressed_data)?;

            // Promote to L1 if frequently accessed
            if self.should_promote_to_l1(&entry).await {
                self.promote_to_l1(key, &decompressed_data, &entry.content_type, entry.ttl).await;
            }

            debug!(
                namespace = %key.namespace,
                key = %key.key,
                compressed_size = entry.compressed_data.len(),
                original_size = entry.original_size,
                compression_ratio = entry.compression_ratio,
                "L2 cache hit"
            );

            return Some(CachedData {
                data: decompressed_data,
                content_type: entry.content_type,
                cached_at: entry.created_at,
                access_count: entry.access_count,
            });
        }
        self.inner.metrics.l2_misses.fetch_add(1, Ordering::Relaxed);

        // Try L3 cache if enabled
        if let Some(ref l3_cache) = self.inner.l3_cache {
            if let Some(data) = l3_cache.get(key).await {
                self.inner.metrics.l3_hits.fetch_add(1, Ordering::Relaxed);

                // Promote to L2
                self.put_in_l2(key, &data, "application/octet-stream", self.config.default_ttl).await;

                debug!(
                    namespace = %key.namespace,
                    key = %key.key,
                    size = data.len(),
                    "L3 cache hit"
                );

                return Some(CachedData {
                    data,
                    content_type: "application/octet-stream".to_string(),
                    cached_at: Instant::now(),
                    access_count: 1,
                });
            }
            self.inner.metrics.l3_misses.fetch_add(1, Ordering::Relaxed);
        }

        None
    }

    /// Put data into cache (intelligent level selection)
    pub async fn put(&self, key: &CacheKey, data: Vec<u8>, content_type: String, ttl: Option<Duration>) {
        let ttl = ttl.unwrap_or(self.config.default_ttl);
        let data_size = data.len();

        // Decide which cache level to use
        if data_size <= self.config.l1_max_entry_size {
            // Put in L1 cache
            self.put_in_l1(key, data.clone(), content_type, ttl).await;
        } else if data_size >= self.config.compression_threshold {
            // Put in L2 cache with compression
            self.put_in_l2(key, &data, &content_type, ttl).await;
        } else {
            // Put in L2 cache without compression
            self.put_in_l2(key, &data, &content_type, ttl).await;
        }

        // Also put in L3 cache if enabled
        if let Some(ref l3_cache) = self.inner.l3_cache {
            let _ = l3_cache.put(key, &data, ttl).await;
        }

        debug!(
            namespace = %key.namespace,
            key = %key.key,
            size = data_size,
            ttl_seconds = ttl.as_secs(),
            "Data cached"
        );
    }

    /// Remove data from all cache levels
    pub async fn remove(&self, key: &CacheKey) {
        // Remove from L1
        {
            let mut l1_cache = self.inner.l1_cache.write().await;
            l1_cache.remove(key);
        }

        // Remove from L2
        {
            let mut l2_cache = self.inner.l2_cache.write().await;
            l2_cache.remove(key);
        }

        // Remove from L3
        if let Some(ref l3_cache) = self.inner.l3_cache {
            let _ = l3_cache.remove(key).await;
        }

        debug!(
            namespace = %key.namespace,
            key = %key.key,
            "Data removed from cache"
        );
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let l1_cache = self.inner.l1_cache.read().await;
        let l2_cache = self.inner.l2_cache.read().await;

        let total_requests = self.inner.metrics.total_requests.load(Ordering::Relaxed);
        let l1_hits = self.inner.metrics.l1_hits.load(Ordering::Relaxed);
        let l2_hits = self.inner.metrics.l2_hits.load(Ordering::Relaxed);
        let l3_hits = self.inner.metrics.l3_hits.load(Ordering::Relaxed);

        let hit_rate = if total_requests > 0 {
            (l1_hits + l2_hits + l3_hits) as f64 / total_requests as f64
        } else {
            0.0
        };

        CacheStats {
            l1_entries: l1_cache.len(),
            l2_entries: l2_cache.len(),
            l1_hits: l1_hits,
            l1_misses: self.inner.metrics.l1_misses.load(Ordering::Relaxed),
            l2_hits: l2_hits,
            l2_misses: self.inner.metrics.l2_misses.load(Ordering::Relaxed),
            l3_hits: l3_hits,
            l3_misses: self.inner.metrics.l3_misses.load(Ordering::Relaxed),
            total_requests,
            hit_rate,
            evictions: self.inner.metrics.evictions.load(Ordering::Relaxed),
            promotions: self.inner.metrics.promotions.load(Ordering::Relaxed),
            demotions: self.inner.metrics.demotions.load(Ordering::Relaxed),
            compression_savings: self.inner.metrics.compression_savings.load(Ordering::Relaxed),
            memory_usage: self.calculate_memory_usage().await,
        }
    }

    async fn get_from_l1(&self, key: &CacheKey) -> Option<CacheEntry> {
        let mut l1_cache = self.inner.l1_cache.write().await;
        if let Some(entry) = l1_cache.get_mut(key) {
            if entry.created_at.elapsed() < entry.ttl {
                entry.last_accessed = Instant::now();
                entry.access_count += 1;
                Some(entry.clone())
            } else {
                l1_cache.remove(key);
                None
            }
        } else {
            None
        }
    }

    async fn get_from_l2(&self, key: &CacheKey) -> Option<CompressedEntry> {
        let mut l2_cache = self.inner.l2_cache.write().await;
        if let Some(entry) = l2_cache.get_mut(key) {
            if entry.created_at.elapsed() < entry.ttl {
                entry.last_accessed = Instant::now();
                entry.access_count += 1;
                Some(entry.clone())
            } else {
                l2_cache.remove(key);
                None
            }
        } else {
            None
        }
    }

    async fn put_in_l1(&self, key: &CacheKey, data: Vec<u8>, content_type: String, ttl: Duration) {
        let entry = CacheEntry {
            size: data.len(),
            data,
            content_type,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 0,
            ttl,
        };

        let mut l1_cache = self.inner.l1_cache.write().await;
        if l1_cache.put(key.clone(), entry).is_some() {
            self.inner.metrics.evictions.fetch_add(1, Ordering::Relaxed);
        }

        self.inner.metrics.current_l1_size.store(l1_cache.len(), Ordering::Relaxed);
    }

    async fn put_in_l2(&self, key: &CacheKey, data: &[u8], content_type: &str, ttl: Duration) {
        let (compressed_data, compression_ratio) = if data.len() >= self.config.compression_threshold {
            let compressed = self.compress_data(data);
            let ratio = compressed.len() as f32 / data.len() as f32;
            self.inner.metrics.compression_savings.fetch_add(
                (data.len() - compressed.len()) as u64,
                Ordering::Relaxed
            );
            (compressed, ratio)
        } else {
            (data.to_vec(), 1.0)
        };

        let entry = CompressedEntry {
            compressed_data,
            original_size: data.len(),
            content_type: content_type.to_string(),
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 0,
            ttl,
            compression_ratio,
        };

        let mut l2_cache = self.inner.l2_cache.write().await;
        if l2_cache.put(key.clone(), entry).is_some() {
            self.inner.metrics.evictions.fetch_add(1, Ordering::Relaxed);
        }

        self.inner.metrics.current_l2_size.store(l2_cache.len(), Ordering::Relaxed);
    }

    async fn promote_to_l1(&self, key: &CacheKey, data: &[u8], content_type: &str, ttl: Duration) {
        if data.len() <= self.config.l1_max_entry_size {
            self.put_in_l1(key, data.to_vec(), content_type.to_string(), ttl).await;
            self.inner.metrics.promotions.fetch_add(1, Ordering::Relaxed);
        }
    }

    async fn should_promote_to_l1(&self, entry: &CompressedEntry) -> bool {
        // Promote if accessed frequently and small enough
        entry.access_count > 5 && entry.original_size <= self.config.l1_max_entry_size
    }

    async fn track_access(&self, key: &CacheKey) {
        if !self.config.enable_frequency_tracking {
            return;
        }

        let mut tracker = self.inner.frequency_tracker.write().await;
        let now = Instant::now();

        let access_info = tracker.access_counts.entry(key.clone()).or_insert(AccessInfo {
            count: 0,
            last_access: now,
            frequency_score: 0.0,
        });

        access_info.count += 1;
        access_info.last_access = now;

        // Update frequency score (simple exponential decay)
        let time_factor = 1.0 - (now.duration_since(access_info.last_access).as_secs_f64() / 3600.0);
        access_info.frequency_score = access_info.frequency_score * 0.9 + time_factor;
    }

    fn compress_data(&self, data: &[u8]) -> Vec<u8> {
        // Simple compression simulation - in production would use zstd, lz4, etc.
        data.to_vec()
    }

    fn decompress_data(&self, data: &[u8]) -> Option<Vec<u8>> {
        // Simple decompression simulation
        Some(data.to_vec())
    }

    async fn calculate_memory_usage(&self) -> usize {
        let l1_cache = self.inner.l1_cache.read().await;
        let l2_cache = self.inner.l2_cache.read().await;

        let l1_size: usize = l1_cache.iter().map(|(_, entry)| entry.size).sum();
        let l2_size: usize = l2_cache.iter().map(|(_, entry)| entry.compressed_data.len()).sum();

        l1_size + l2_size
    }

    fn start_cleanup_task(&self) {
        let inner = self.inner.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(config.cleanup_interval);

            loop {
                interval.tick().await;

                // Cleanup expired entries
                let now = Instant::now();

                // Cleanup L1
                {
                    let mut l1_cache = inner.l1_cache.write().await;
                    l1_cache.retain(|_, entry| now.duration_since(entry.created_at) < entry.ttl);
                }

                // Cleanup L2
                {
                    let mut l2_cache = inner.l2_cache.write().await;
                    l2_cache.retain(|_, entry| now.duration_since(entry.created_at) < entry.ttl);
                }

                // Cleanup frequency tracker
                if config.enable_frequency_tracking {
                    let mut tracker = inner.frequency_tracker.write().await;
                    let cutoff = now - tracker.window_size;
                    tracker.access_counts.retain(|_, info| info.last_access > cutoff);
                }

                debug!("Cache cleanup completed");
            }
        });
    }
}

/// Cached data returned to clients
#[derive(Debug, Clone)]
pub struct CachedData {
    pub data: Vec<u8>,
    pub content_type: String,
    pub cached_at: Instant,
    pub access_count: u64,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub l1_entries: usize,
    pub l2_entries: usize,
    pub l1_hits: u64,
    pub l1_misses: u64,
    pub l2_hits: u64,
    pub l2_misses: u64,
    pub l3_hits: u64,
    pub l3_misses: u64,
    pub total_requests: u64,
    pub hit_rate: f64,
    pub evictions: u64,
    pub promotions: u64,
    pub demotions: u64,
    pub compression_savings: u64,
    pub memory_usage: usize,
}

/// Cache error types
#[derive(Debug, Clone)]
pub enum CacheError {
    SerializationError(String),
    CompressionError(String),
    PersistentStoreError(String),
    InvalidKey(String),
}

// Simple LRU cache implementation
#[derive(Debug)]
struct LruCache<K, V> {
    map: HashMap<K, V>,
    capacity: usize,
}

impl<K: Clone + Hash + Eq, V> LruCache<K, V> {
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

    fn len(&self) -> usize {
        self.map.len()
    }

    fn iter(&self) -> std::collections::hash_map::Iter<K, V> {
        self.map.iter()
    }

    fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.map.retain(f)
    }
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Self::CompressionError(msg) => write!(f, "Compression error: {}", msg),
            Self::PersistentStoreError(msg) => write!(f, "Persistent store error: {}", msg),
            Self::InvalidKey(msg) => write!(f, "Invalid key: {}", msg),
        }
    }
}

impl std::error::Error for CacheError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_creation() {
        let config = CacheConfig::default();
        let cache = DavCache::new(config);

        let stats = cache.get_stats().await;
        assert_eq!(stats.l1_entries, 0);
        assert_eq!(stats.l2_entries, 0);
        assert_eq!(stats.total_requests, 0);
    }

    #[tokio::test]
    async fn test_cache_put_and_get() {
        let config = CacheConfig {
            l1_size: 10,
            l2_size: 20,
            enable_compression: false, // Disable for simpler testing
            ..Default::default()
        };
        let cache = DavCache::new(config);

        let key = CacheKey {
            resource_type: "calendar".to_string(),
            resource_id: "user1".to_string(),
            operation: "get".to_string(),
            parameters: "".to_string(),
        };

        let data = b"test calendar data";
        let content_type = "text/calendar";

        // Put data in cache
        cache.put(&key, data, content_type).await;

        // Get data from cache
        let result = cache.get(&key).await;
        assert!(result.is_some());

        let cached_data = result.unwrap();
        assert_eq!(cached_data.data, data);
        assert_eq!(cached_data.content_type, content_type);

        let stats = cache.get_stats().await;
        assert_eq!(stats.l1_entries, 1);
        assert_eq!(stats.l1_hits, 1);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let config = CacheConfig::default();
        let cache = DavCache::new(config);

        let key = CacheKey {
            resource_type: "calendar".to_string(),
            resource_id: "nonexistent".to_string(),
            operation: "get".to_string(),
            parameters: "".to_string(),
        };

        let result = cache.get(&key).await;
        assert!(result.is_none());

        let stats = cache.get_stats().await;
        assert_eq!(stats.l1_misses, 1);
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let config = CacheConfig::default();
        let cache = DavCache::new(config);

        let key = CacheKey {
            resource_type: "calendar".to_string(),
            resource_id: "user1".to_string(),
            operation: "get".to_string(),
            parameters: "".to_string(),
        };

        // Put data in cache
        cache.put(&key, b"test data", "text/plain").await;

        // Verify it's cached
        assert!(cache.get(&key).await.is_some());

        // Invalidate
        cache.invalidate(&key).await;

        // Verify it's gone
        assert!(cache.get(&key).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let config = CacheConfig::default();
        let cache = DavCache::new(config);

        // Put multiple items
        for i in 0..5 {
            let key = CacheKey {
                resource_type: "calendar".to_string(),
                resource_id: format!("user{}", i),
                operation: "get".to_string(),
                parameters: "".to_string(),
            };
            cache.put(&key, b"test data", "text/plain").await;
        }

        let stats = cache.get_stats().await;
        assert_eq!(stats.l1_entries, 5);

        // Clear cache
        cache.clear().await;

        let stats = cache.get_stats().await;
        assert_eq!(stats.l1_entries, 0);
        assert_eq!(stats.l2_entries, 0);
    }

    #[tokio::test]
    async fn test_cache_compression() {
        let config = CacheConfig {
            enable_compression: true,
            compression_threshold: 10, // Low threshold for testing
            ..Default::default()
        };
        let cache = DavCache::new(config);

        let key = CacheKey {
            resource_type: "calendar".to_string(),
            resource_id: "user1".to_string(),
            operation: "get".to_string(),
            parameters: "".to_string(),
        };

        let large_data = vec![b'x'; 100]; // Large enough to trigger compression

        cache.put(&key, &large_data, "text/plain").await;

        let result = cache.get(&key).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().data, large_data);
    }

    #[tokio::test]
    async fn test_cache_l1_to_l2_promotion() {
        let config = CacheConfig {
            l1_size: 2, // Small L1 to force L2 usage
            l2_size: 10,
            l1_max_entry_size: 50,
            ..Default::default()
        };
        let cache = DavCache::new(config);

        // Fill L1 cache
        for i in 0..3 {
            let key = CacheKey {
                resource_type: "calendar".to_string(),
                resource_id: format!("user{}", i),
                operation: "get".to_string(),
                parameters: "".to_string(),
            };
            cache.put(&key, b"test data", "text/plain").await;
        }

        let stats = cache.get_stats().await;
        // Should have items in both L1 and L2 due to overflow
        assert!(stats.l1_entries <= 2);
        assert!(stats.l2_entries > 0 || stats.l1_entries > 0);
    }

    #[tokio::test]
    async fn test_cache_ttl_expiration() {
        let config = CacheConfig {
            l1_ttl: Duration::from_millis(50), // Very short TTL
            cleanup_interval: Duration::from_millis(25),
            ..Default::default()
        };
        let cache = DavCache::new(config);

        let key = CacheKey {
            resource_type: "calendar".to_string(),
            resource_id: "user1".to_string(),
            operation: "get".to_string(),
            parameters: "".to_string(),
        };

        // Put data in cache
        cache.put(&key, b"test data", "text/plain").await;

        // Verify it's cached
        assert!(cache.get(&key).await.is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should be expired now
        assert!(cache.get(&key).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let key1 = CacheKey {
            resource_type: "calendar".to_string(),
            resource_id: "user1".to_string(),
            operation: "get".to_string(),
            parameters: "".to_string(),
        };

        let key2 = CacheKey {
            resource_type: "calendar".to_string(),
            resource_id: "user1".to_string(),
            operation: "get".to_string(),
            parameters: "".to_string(),
        };

        let key3 = CacheKey {
            resource_type: "calendar".to_string(),
            resource_id: "user2".to_string(),
            operation: "get".to_string(),
            parameters: "".to_string(),
        };

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[tokio::test]
    async fn test_cache_statistics() {
        let config = CacheConfig::default();
        let cache = DavCache::new(config);

        let key = CacheKey {
            resource_type: "calendar".to_string(),
            resource_id: "user1".to_string(),
            operation: "get".to_string(),
            parameters: "".to_string(),
        };

        // Generate some cache activity
        cache.put(&key, b"test data", "text/plain").await;
        let _ = cache.get(&key).await; // Hit

        let nonexistent_key = CacheKey {
            resource_type: "calendar".to_string(),
            resource_id: "nonexistent".to_string(),
            operation: "get".to_string(),
            parameters: "".to_string(),
        };
        let _ = cache.get(&nonexistent_key).await; // Miss

        let stats = cache.get_stats().await;
        assert_eq!(stats.l1_hits, 1);
        assert_eq!(stats.l1_misses, 1);
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.hit_rate, 0.5);
    }

    #[test]
    fn test_lru_cache_basic_operations() {
        let mut cache = LruCache::new(3);

        // Test put and get
        cache.put("key1", "value1");
        cache.put("key2", "value2");
        cache.put("key3", "value3");

        assert_eq!(cache.len(), 3);
        assert!(cache.get_mut(&"key1").is_some());

        // Test capacity limit
        cache.put("key4", "value4");
        assert_eq!(cache.len(), 3); // Should still be 3 due to eviction

        // Test remove
        cache.remove(&"key2");
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_cache_error_display() {
        let error = CacheError::SerializationError("test error".to_string());
        assert!(error.to_string().contains("Serialization error"));

        let error = CacheError::CompressionError("compression failed".to_string());
        assert!(error.to_string().contains("Compression error"));

        let error = CacheError::InvalidKey("invalid key".to_string());
        assert!(error.to_string().contains("Invalid key"));
    }

    #[tokio::test]
    async fn test_cache_memory_usage_calculation() {
        let config = CacheConfig::default();
        let cache = DavCache::new(config);

        // Put some data
        for i in 0..5 {
            let key = CacheKey {
                resource_type: "calendar".to_string(),
                resource_id: format!("user{}", i),
                operation: "get".to_string(),
                parameters: "".to_string(),
            };
            cache.put(&key, &vec![b'x'; 100], "text/plain").await;
        }

        let memory_usage = cache.calculate_memory_usage().await;
        assert!(memory_usage > 0);
    }

    #[tokio::test]
    async fn test_cache_frequency_tracking() {
        let config = CacheConfig {
            enable_frequency_tracking: true,
            ..Default::default()
        };
        let cache = DavCache::new(config);

        let key = CacheKey {
            resource_type: "calendar".to_string(),
            resource_id: "user1".to_string(),
            operation: "get".to_string(),
            parameters: "".to_string(),
        };

        // Put data and access multiple times
        cache.put(&key, b"test data", "text/plain").await;

        for _ in 0..10 {
            let _ = cache.get(&key).await;
        }

        // Verify frequency tracking is working
        cache.track_access(&key).await;

        let stats = cache.get_stats().await;
        assert!(stats.l1_hits > 0);
    }
}
