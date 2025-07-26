//! High-Performance Cache Manager for A3Mailer
//!
//! This module provides a multi-tier caching system with memory, Redis,
//! and disk-based caching layers for optimal performance.

use crate::{CacheConfig, CacheStats, Result, PerformanceError};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};

/// Cache entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub value: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub access_count: u64,
    pub last_accessed: u64,
    pub size: usize,
}

/// LRU cache node
#[derive(Debug)]
struct LruNode {
    key: String,
    entry: CacheEntry,
    prev: Option<Arc<RwLock<LruNode>>>,
    next: Option<Arc<RwLock<LruNode>>>,
}

/// Memory cache with LRU eviction
#[derive(Debug)]
pub struct MemoryCache {
    data: HashMap<String, Arc<RwLock<LruNode>>>,
    head: Option<Arc<RwLock<LruNode>>>,
    tail: Option<Arc<RwLock<LruNode>>>,
    capacity: usize,
    current_size: usize,
    max_memory_bytes: usize,
    stats: CacheStats,
}

impl MemoryCache {
    /// Create a new memory cache
    pub fn new(capacity: usize, max_memory_bytes: usize) -> Self {
        Self {
            data: HashMap::new(),
            head: None,
            tail: None,
            capacity,
            current_size: 0,
            max_memory_bytes,
            stats: CacheStats {
                hits: 0,
                misses: 0,
                evictions: 0,
                memory_usage_bytes: 0,
                key_count: 0,
                hit_rate: 0.0,
            },
        }
    }

    /// Get a value from cache
    pub async fn get(&mut self, key: &str) -> Option<String> {
        if let Some(node_arc) = self.data.get(key) {
            let node = node_arc.read().await;
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            
            // Check if expired
            if node.entry.expires_at > 0 && now > node.entry.expires_at {
                drop(node);
                self.remove_key(key).await;
                self.stats.misses += 1;
                return None;
            }
            
            let value = node.entry.value.clone();
            drop(node);
            
            // Move to front (most recently used)
            self.move_to_front(key).await;
            self.stats.hits += 1;
            self.update_hit_rate();
            
            Some(value)
        } else {
            self.stats.misses += 1;
            self.update_hit_rate();
            None
        }
    }

    /// Set a value in cache
    pub async fn set(&mut self, key: &str, value: &str, ttl_seconds: u64) -> Result<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let expires_at = if ttl_seconds > 0 { now + ttl_seconds } else { 0 };
        
        let entry = CacheEntry {
            value: value.to_string(),
            created_at: now,
            expires_at,
            access_count: 1,
            last_accessed: now,
            size: key.len() + value.len(),
        };

        // Check if key already exists
        if self.data.contains_key(key) {
            self.remove_key(key).await;
        }

        // Check memory limits
        if self.current_size + entry.size > self.max_memory_bytes {
            self.evict_lru().await;
        }

        // Check capacity limits
        if self.data.len() >= self.capacity {
            self.evict_lru().await;
        }

        // Create new node
        let node = Arc::new(RwLock::new(LruNode {
            key: key.to_string(),
            entry,
            prev: None,
            next: None,
        }));

        // Add to front of list
        self.add_to_front(node.clone()).await;
        self.data.insert(key.to_string(), node);
        self.current_size += key.len() + value.len();
        self.stats.key_count += 1;
        self.stats.memory_usage_bytes = self.current_size as u64;

        Ok(())
    }

    /// Remove a key from cache
    pub async fn remove(&mut self, key: &str) -> bool {
        self.remove_key(key).await
    }

    /// Add node to front of LRU list
    async fn add_to_front(&mut self, node: Arc<RwLock<LruNode>>) {
        if let Some(head) = &self.head {
            {
                let mut node_guard = node.write().await;
                node_guard.next = Some(head.clone());
            }
            {
                let mut head_guard = head.write().await;
                head_guard.prev = Some(node.clone());
            }
        } else {
            self.tail = Some(node.clone());
        }
        self.head = Some(node);
    }

    /// Move existing node to front
    async fn move_to_front(&mut self, key: &str) {
        if let Some(node_arc) = self.data.get(key).cloned() {
            // Remove from current position
            self.remove_from_list(node_arc.clone()).await;
            // Add to front
            self.add_to_front(node_arc).await;
        }
    }

    /// Remove node from list
    async fn remove_from_list(&mut self, node_arc: Arc<RwLock<LruNode>>) {
        let node = node_arc.read().await;
        
        if let Some(prev) = &node.prev {
            let mut prev_guard = prev.write().await;
            prev_guard.next = node.next.clone();
        } else {
            self.head = node.next.clone();
        }

        if let Some(next) = &node.next {
            let mut next_guard = next.write().await;
            next_guard.prev = node.prev.clone();
        } else {
            self.tail = node.prev.clone();
        }
    }

    /// Remove key and update stats
    async fn remove_key(&mut self, key: &str) -> bool {
        if let Some(node_arc) = self.data.remove(key) {
            let node = node_arc.read().await;
            self.current_size -= node.entry.size;
            self.stats.key_count -= 1;
            self.stats.memory_usage_bytes = self.current_size as u64;
            drop(node);
            
            self.remove_from_list(node_arc).await;
            true
        } else {
            false
        }
    }

    /// Evict least recently used item
    async fn evict_lru(&mut self) {
        if let Some(tail) = &self.tail {
            let key = {
                let tail_guard = tail.read().await;
                tail_guard.key.clone()
            };
            self.remove_key(&key).await;
            self.stats.evictions += 1;
        }
    }

    /// Update hit rate
    fn update_hit_rate(&mut self) {
        let total = self.stats.hits + self.stats.misses;
        if total > 0 {
            self.stats.hit_rate = self.stats.hits as f64 / total as f64;
        }
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        self.stats.clone()
    }

    /// Clear all entries
    pub async fn clear(&mut self) {
        self.data.clear();
        self.head = None;
        self.tail = None;
        self.current_size = 0;
        self.stats.key_count = 0;
        self.stats.memory_usage_bytes = 0;
    }
}

/// Redis cache client
#[derive(Debug)]
pub struct RedisCache {
    client: Option<redis::Client>,
    connection_pool: Option<r2d2::Pool<redis::Client>>,
    stats: CacheStats,
}

impl RedisCache {
    /// Create a new Redis cache
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| PerformanceError::CacheError(format!("Redis connection failed: {}", e)))?;

        // Test connection
        let mut conn = client.get_connection()
            .map_err(|e| PerformanceError::CacheError(format!("Redis connection test failed: {}", e)))?;

        // Ping to verify connection
        let _: String = redis::cmd("PING").query(&mut conn)
            .map_err(|e| PerformanceError::CacheError(format!("Redis ping failed: {}", e)))?;

        Ok(Self {
            client: Some(client),
            connection_pool: None,
            stats: CacheStats {
                hits: 0,
                misses: 0,
                evictions: 0,
                memory_usage_bytes: 0,
                key_count: 0,
                hit_rate: 0.0,
            },
        })
    }

    /// Get a value from Redis
    pub async fn get(&mut self, key: &str) -> Result<Option<String>> {
        if let Some(client) = &self.client {
            let mut conn = client.get_connection()
                .map_err(|e| PerformanceError::CacheError(e.to_string()))?;

            match redis::cmd("GET").arg(key).query::<Option<String>>(&mut conn) {
                Ok(Some(value)) => {
                    self.stats.hits += 1;
                    self.update_hit_rate();
                    Ok(Some(value))
                }
                Ok(None) => {
                    self.stats.misses += 1;
                    self.update_hit_rate();
                    Ok(None)
                }
                Err(e) => Err(PerformanceError::CacheError(e.to_string())),
            }
        } else {
            Err(PerformanceError::CacheError("Redis client not initialized".to_string()))
        }
    }

    /// Set a value in Redis
    pub async fn set(&mut self, key: &str, value: &str, ttl_seconds: u64) -> Result<()> {
        if let Some(client) = &self.client {
            let mut conn = client.get_connection()
                .map_err(|e| PerformanceError::CacheError(e.to_string()))?;

            if ttl_seconds > 0 {
                redis::cmd("SETEX").arg(key).arg(ttl_seconds).arg(value).query::<()>(&mut conn)
                    .map_err(|e| PerformanceError::CacheError(e.to_string()))?;
            } else {
                redis::cmd("SET").arg(key).arg(value).query::<()>(&mut conn)
                    .map_err(|e| PerformanceError::CacheError(e.to_string()))?;
            }

            Ok(())
        } else {
            Err(PerformanceError::CacheError("Redis client not initialized".to_string()))
        }
    }

    /// Delete a key from Redis
    pub async fn delete(&mut self, key: &str) -> Result<bool> {
        if let Some(client) = &self.client {
            let mut conn = client.get_connection()
                .map_err(|e| PerformanceError::CacheError(e.to_string()))?;

            let deleted: i32 = redis::cmd("DEL").arg(key).query(&mut conn)
                .map_err(|e| PerformanceError::CacheError(e.to_string()))?;

            Ok(deleted > 0)
        } else {
            Err(PerformanceError::CacheError("Redis client not initialized".to_string()))
        }
    }

    /// Update hit rate
    fn update_hit_rate(&mut self) {
        let total = self.stats.hits + self.stats.misses;
        if total > 0 {
            self.stats.hit_rate = self.stats.hits as f64 / total as f64;
        }
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        self.stats.clone()
    }
}

/// Multi-tier cache manager
pub struct CacheManager {
    config: CacheConfig,
    memory_cache: Arc<RwLock<MemoryCache>>,
    redis_cache: Option<Arc<RwLock<RedisCache>>>,
    stats: Arc<RwLock<CacheStats>>,
}

impl CacheManager {
    /// Create a new cache manager
    pub async fn new(config: &CacheConfig) -> Result<Self> {
        info!("Initializing cache manager");

        // Initialize memory cache
        let memory_cache = Arc::new(RwLock::new(MemoryCache::new(
            10000, // Default capacity
            (config.memory_cache_size_mb * 1024 * 1024) as usize,
        )));

        // Initialize Redis cache if configured
        let redis_cache = if let Some(redis_url) = &config.redis_url {
            match RedisCache::new(redis_url).await {
                Ok(cache) => Some(Arc::new(RwLock::new(cache))),
                Err(e) => {
                    warn!("Failed to initialize Redis cache: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let stats = Arc::new(RwLock::new(CacheStats {
            hits: 0,
            misses: 0,
            evictions: 0,
            memory_usage_bytes: 0,
            key_count: 0,
            hit_rate: 0.0,
        }));

        info!("Cache manager initialized successfully");
        Ok(Self {
            config: config.clone(),
            memory_cache,
            redis_cache,
            stats,
        })
    }

    /// Get a value from cache (tries memory first, then Redis)
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        // Try memory cache first
        {
            let mut memory_cache = self.memory_cache.write().await;
            if let Some(value) = memory_cache.get(key).await {
                debug!("Cache hit in memory for key: {}", key);
                return Ok(Some(value));
            }
        }

        // Try Redis cache
        if let Some(redis_cache) = &self.redis_cache {
            let mut redis = redis_cache.write().await;
            if let Ok(Some(value)) = redis.get(key).await {
                debug!("Cache hit in Redis for key: {}", key);
                
                // Promote to memory cache
                let mut memory_cache = self.memory_cache.write().await;
                let _ = memory_cache.set(key, &value, self.config.default_ttl_seconds).await;
                
                return Ok(Some(value));
            }
        }

        debug!("Cache miss for key: {}", key);
        Ok(None)
    }

    /// Set a value in cache
    pub async fn set(&self, key: &str, value: &str, ttl_seconds: u64) -> Result<()> {
        // Set in memory cache
        {
            let mut memory_cache = self.memory_cache.write().await;
            memory_cache.set(key, value, ttl_seconds).await?;
        }

        // Set in Redis cache
        if let Some(redis_cache) = &self.redis_cache {
            let mut redis = redis_cache.write().await;
            let _ = redis.set(key, value, ttl_seconds).await;
        }

        debug!("Set cache value for key: {}", key);
        Ok(())
    }

    /// Delete a value from cache
    pub async fn delete(&self, key: &str) -> Result<bool> {
        let mut deleted = false;

        // Delete from memory cache
        {
            let mut memory_cache = self.memory_cache.write().await;
            if memory_cache.remove(key).await {
                deleted = true;
            }
        }

        // Delete from Redis cache
        if let Some(redis_cache) = &self.redis_cache {
            let mut redis = redis_cache.write().await;
            if let Ok(redis_deleted) = redis.delete(key).await {
                deleted = deleted || redis_deleted;
            }
        }

        debug!("Deleted cache value for key: {}", key);
        Ok(deleted)
    }

    /// Get combined cache statistics
    pub async fn get_stats(&self) -> Result<CacheStats> {
        let memory_stats = {
            let memory_cache = self.memory_cache.read().await;
            memory_cache.get_stats()
        };

        let redis_stats = if let Some(redis_cache) = &self.redis_cache {
            let redis = redis_cache.read().await;
            redis.get_stats()
        } else {
            CacheStats {
                hits: 0,
                misses: 0,
                evictions: 0,
                memory_usage_bytes: 0,
                key_count: 0,
                hit_rate: 0.0,
            }
        };

        Ok(CacheStats {
            hits: memory_stats.hits + redis_stats.hits,
            misses: memory_stats.misses + redis_stats.misses,
            evictions: memory_stats.evictions + redis_stats.evictions,
            memory_usage_bytes: memory_stats.memory_usage_bytes,
            key_count: memory_stats.key_count,
            hit_rate: {
                let total_hits = memory_stats.hits + redis_stats.hits;
                let total_requests = total_hits + memory_stats.misses + redis_stats.misses;
                if total_requests > 0 {
                    total_hits as f64 / total_requests as f64
                } else {
                    0.0
                }
            },
        })
    }

    /// Clean up expired entries
    pub async fn cleanup_expired(&self) -> Result<()> {
        debug!("Cleaning up expired cache entries");
        
        // Memory cache cleanup is handled automatically during get operations
        // Redis cache cleanup is handled by Redis TTL
        
        Ok(())
    }

    /// Optimize cache performance
    pub async fn optimize(&self) -> Result<()> {
        debug!("Optimizing cache performance");
        
        let stats = self.get_stats().await?;
        
        // If hit rate is low, consider adjusting cache size or TTL
        if stats.hit_rate < 0.5 {
            warn!("Low cache hit rate: {:.2}%", stats.hit_rate * 100.0);
        }
        
        // If memory usage is high, trigger cleanup
        if stats.memory_usage_bytes > (self.config.memory_cache_size_mb * 1024 * 1024 * 80 / 100) {
            warn!("High memory usage in cache: {} bytes", stats.memory_usage_bytes);
        }
        
        Ok(())
    }

    /// Shutdown cache manager
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down cache manager");
        
        // Clear memory cache
        {
            let mut memory_cache = self.memory_cache.write().await;
            memory_cache.clear().await;
        }
        
        info!("Cache manager shutdown complete");
        Ok(())
    }
}
