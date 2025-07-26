//! # A3Mailer Performance Optimization
//!
//! High-performance caching, connection pooling, and optimization utilities
//! for A3Mailer's AI-powered Web3-native email server.
//!
//! ## Features
//!
//! - **Multi-tier Caching**: Memory, Redis, and disk-based caching
//! - **Connection Pooling**: Database and network connection management
//! - **Load Balancing**: Request distribution and resource optimization
//! - **Memory Management**: Efficient memory usage and garbage collection
//! - **Async Optimization**: High-performance async operations
//! - **Resource Monitoring**: Real-time performance tracking
//!
//! ## Architecture
//!
//! The performance system consists of:
//! - Cache Manager: Multi-tier caching with TTL and eviction policies
//! - Pool Manager: Connection and resource pooling
//! - Load Balancer: Request distribution and failover
//! - Memory Manager: Memory optimization and monitoring
//!
//! ## Example
//!
//! ```rust,no_run
//! use a3mailer_performance::{PerformanceManager, CacheConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = CacheConfig::default();
//!     let perf_manager = PerformanceManager::new(config).await?;
//!
//!     // Cache data
//!     perf_manager.cache_set("key", "value", 3600).await?;
//!
//!     // Get cached data
//!     let value = perf_manager.cache_get("key").await?;
//!
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

pub mod cache;
pub mod pool;
pub mod load_balancer;
pub mod memory;
pub mod error;

pub use error::{PerformanceError, Result};

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub cache: CacheConfig,
    pub pool: PoolConfig,
    pub load_balancer: LoadBalancerConfig,
    pub memory: MemoryConfig,
    pub monitoring: PerformanceMonitoringConfig,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub memory_cache_size_mb: u64,
    pub redis_url: Option<String>,
    pub disk_cache_path: Option<String>,
    pub default_ttl_seconds: u64,
    pub max_key_size: usize,
    pub max_value_size: usize,
    pub eviction_policy: String, // "lru", "lfu", "ttl"
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub database_pool_size: u32,
    pub redis_pool_size: u32,
    pub http_pool_size: u32,
    pub connection_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: u64,
}

/// Load balancer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    pub enabled: bool,
    pub strategy: String, // "round_robin", "least_connections", "weighted"
    pub health_check_interval_seconds: u64,
    pub failover_threshold: u32,
    pub circuit_breaker_enabled: bool,
}

/// Memory management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub max_heap_size_mb: u64,
    pub gc_threshold_mb: u64,
    pub memory_monitoring_enabled: bool,
    pub oom_protection_enabled: bool,
}

/// Performance monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMonitoringConfig {
    pub enabled: bool,
    pub metrics_collection_interval_seconds: u64,
    pub performance_alerts_enabled: bool,
    pub slow_query_threshold_ms: u64,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub cache_hit_rate: f64,
    pub cache_miss_rate: f64,
    pub average_response_time_ms: f64,
    pub requests_per_second: f64,
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f64,
    pub active_connections: u32,
    pub pool_utilization: f64,
    pub timestamp: DateTime<Utc>,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub memory_usage_bytes: u64,
    pub key_count: u64,
    pub hit_rate: f64,
}

/// Main performance manager
pub struct PerformanceManager {
    config: PerformanceConfig,
    cache_manager: Arc<RwLock<cache::CacheManager>>,
    pool_manager: Arc<RwLock<pool::PoolManager>>,
    load_balancer: Arc<RwLock<load_balancer::LoadBalancer>>,
    memory_manager: Arc<RwLock<memory::MemoryManager>>,
    metrics: Arc<RwLock<PerformanceMetrics>>,
    start_time: Instant,
}

impl PerformanceManager {
    /// Create a new performance manager
    pub async fn new(config: PerformanceConfig) -> Result<Self> {
        info!("Initializing performance management system");

        // Initialize components
        let cache_manager = Arc::new(RwLock::new(
            cache::CacheManager::new(&config.cache).await?
        ));

        let pool_manager = Arc::new(RwLock::new(
            pool::PoolManager::new(&config.pool).await?
        ));

        let load_balancer = Arc::new(RwLock::new(
            load_balancer::LoadBalancer::new(&config.load_balancer).await?
        ));

        let memory_manager = Arc::new(RwLock::new(
            memory::MemoryManager::new(&config.memory).await?
        ));

        let metrics = Arc::new(RwLock::new(PerformanceMetrics {
            cache_hit_rate: 0.0,
            cache_miss_rate: 0.0,
            average_response_time_ms: 0.0,
            requests_per_second: 0.0,
            memory_usage_mb: 0,
            cpu_usage_percent: 0.0,
            active_connections: 0,
            pool_utilization: 0.0,
            timestamp: Utc::now(),
        }));

        let manager = Self {
            config,
            cache_manager,
            pool_manager,
            load_balancer,
            memory_manager,
            metrics,
            start_time: Instant::now(),
        };

        // Start background optimization tasks
        manager.start_background_tasks().await?;

        info!("Performance management system initialized successfully");
        Ok(manager)
    }

    /// Cache a value with TTL
    pub async fn cache_set(&self, key: &str, value: &str, ttl_seconds: u64) -> Result<()> {
        let cache_manager = self.cache_manager.read().await;
        cache_manager.set(key, value, ttl_seconds).await
    }

    /// Get a cached value
    pub async fn cache_get(&self, key: &str) -> Result<Option<String>> {
        let cache_manager = self.cache_manager.read().await;
        cache_manager.get(key).await
    }

    /// Delete a cached value
    pub async fn cache_delete(&self, key: &str) -> Result<bool> {
        let cache_manager = self.cache_manager.read().await;
        cache_manager.delete(key).await
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> Result<CacheStats> {
        let cache_manager = self.cache_manager.read().await;
        cache_manager.get_stats().await
    }

    /// Get a database connection from the pool
    pub async fn get_db_connection(&self) -> Result<pool::PooledConnection> {
        let pool_manager = self.pool_manager.read().await;
        pool_manager.get_database_connection().await
    }

    /// Get an HTTP client from the pool
    pub async fn get_http_client(&self) -> Result<reqwest::Client> {
        let pool_manager = self.pool_manager.read().await;
        pool_manager.get_http_client().await
    }

    /// Execute a load-balanced request
    pub async fn execute_balanced_request<T>(&self, request: T) -> Result<T::Output>
    where
        T: load_balancer::BalancedRequest,
    {
        let load_balancer = self.load_balancer.read().await;
        load_balancer.execute_request(request).await
    }

    /// Get current performance metrics
    pub async fn get_performance_metrics(&self) -> Result<PerformanceMetrics> {
        let metrics = self.metrics.read().await;
        Ok(metrics.clone())
    }

    /// Force garbage collection
    pub async fn force_gc(&self) -> Result<()> {
        let memory_manager = self.memory_manager.read().await;
        memory_manager.force_gc().await
    }

    /// Get memory usage statistics
    pub async fn get_memory_stats(&self) -> Result<memory::MemoryStats> {
        let memory_manager = self.memory_manager.read().await;
        memory_manager.get_stats().await
    }

    /// Optimize performance based on current metrics
    pub async fn optimize_performance(&self) -> Result<()> {
        debug!("Running performance optimization");

        // Get current metrics
        let metrics = self.get_performance_metrics().await?;
        let cache_stats = self.get_cache_stats().await?;
        let memory_stats = self.get_memory_stats().await?;

        // Cache optimization
        if cache_stats.hit_rate < 0.8 {
            warn!("Low cache hit rate: {:.2}%", cache_stats.hit_rate * 100.0);
            self.optimize_cache().await?;
        }

        // Memory optimization
        if memory_stats.usage_percent > 80.0 {
            warn!("High memory usage: {:.1}%", memory_stats.usage_percent);
            self.optimize_memory().await?;
        }

        // Connection pool optimization
        if metrics.pool_utilization > 90.0 {
            warn!("High pool utilization: {:.1}%", metrics.pool_utilization);
            self.optimize_pools().await?;
        }

        info!("Performance optimization completed");
        Ok(())
    }

    /// Start background optimization tasks
    async fn start_background_tasks(&self) -> Result<()> {
        info!("Starting background performance tasks");

        // Metrics collection task
        let metrics = Arc::clone(&self.metrics);
        let cache_manager = Arc::clone(&self.cache_manager);
        let memory_manager = Arc::clone(&self.memory_manager);
        let pool_manager = Arc::clone(&self.pool_manager);
        let interval = self.config.monitoring.metrics_collection_interval_seconds;

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(Duration::from_secs(interval));
            loop {
                interval_timer.tick().await;
                
                if let Err(e) = Self::collect_metrics(
                    &metrics,
                    &cache_manager,
                    &memory_manager,
                    &pool_manager,
                ).await {
                    error!("Failed to collect performance metrics: {}", e);
                }
            }
        });

        // Cache cleanup task
        let cache_manager = Arc::clone(&self.cache_manager);
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            loop {
                interval_timer.tick().await;
                
                if let Err(e) = cache_manager.read().await.cleanup_expired().await {
                    error!("Failed to cleanup expired cache entries: {}", e);
                }
            }
        });

        // Memory monitoring task
        let memory_manager = Arc::clone(&self.memory_manager);
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(Duration::from_secs(60)); // 1 minute
            loop {
                interval_timer.tick().await;
                
                if let Err(e) = memory_manager.read().await.monitor_memory().await {
                    error!("Failed to monitor memory usage: {}", e);
                }
            }
        });

        info!("Background performance tasks started");
        Ok(())
    }

    /// Collect performance metrics
    async fn collect_metrics(
        metrics: &Arc<RwLock<PerformanceMetrics>>,
        cache_manager: &Arc<RwLock<cache::CacheManager>>,
        memory_manager: &Arc<RwLock<memory::MemoryManager>>,
        pool_manager: &Arc<RwLock<pool::PoolManager>>,
    ) -> Result<()> {
        debug!("Collecting performance metrics");

        // Get cache stats
        let cache_stats = cache_manager.read().await.get_stats().await?;
        
        // Get memory stats
        let memory_stats = memory_manager.read().await.get_stats().await?;
        
        // Get pool stats
        let pool_stats = pool_manager.read().await.get_stats().await?;

        // Update metrics
        let mut metrics_guard = metrics.write().await;
        metrics_guard.cache_hit_rate = cache_stats.hit_rate;
        metrics_guard.cache_miss_rate = 1.0 - cache_stats.hit_rate;
        metrics_guard.memory_usage_mb = memory_stats.used_bytes / 1024 / 1024;
        metrics_guard.active_connections = pool_stats.active_connections;
        metrics_guard.pool_utilization = pool_stats.utilization_percent / 100.0;
        metrics_guard.timestamp = Utc::now();

        debug!("Performance metrics collected successfully");
        Ok(())
    }

    /// Optimize cache performance
    async fn optimize_cache(&self) -> Result<()> {
        debug!("Optimizing cache performance");
        
        let cache_manager = self.cache_manager.read().await;
        cache_manager.optimize().await?;
        
        info!("Cache optimization completed");
        Ok(())
    }

    /// Optimize memory usage
    async fn optimize_memory(&self) -> Result<()> {
        debug!("Optimizing memory usage");
        
        let memory_manager = self.memory_manager.read().await;
        memory_manager.optimize().await?;
        
        info!("Memory optimization completed");
        Ok(())
    }

    /// Optimize connection pools
    async fn optimize_pools(&self) -> Result<()> {
        debug!("Optimizing connection pools");
        
        let pool_manager = self.pool_manager.read().await;
        pool_manager.optimize().await?;
        
        info!("Pool optimization completed");
        Ok(())
    }

    /// Get system uptime
    pub fn get_uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get performance statistics
    pub async fn get_performance_stats(&self) -> Result<HashMap<String, String>> {
        let mut stats = HashMap::new();
        
        let metrics = self.get_performance_metrics().await?;
        let cache_stats = self.get_cache_stats().await?;
        let memory_stats = self.get_memory_stats().await?;
        
        stats.insert("uptime_seconds".to_string(), self.get_uptime().as_secs().to_string());
        stats.insert("cache_hit_rate".to_string(), format!("{:.2}%", cache_stats.hit_rate * 100.0));
        stats.insert("memory_usage_mb".to_string(), metrics.memory_usage_mb.to_string());
        stats.insert("active_connections".to_string(), metrics.active_connections.to_string());
        stats.insert("pool_utilization".to_string(), format!("{:.1}%", metrics.pool_utilization * 100.0));
        stats.insert("requests_per_second".to_string(), format!("{:.1}", metrics.requests_per_second));
        
        Ok(stats)
    }

    /// Shutdown performance manager
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down performance management system");
        
        // Shutdown components
        self.cache_manager.write().await.shutdown().await?;
        self.pool_manager.write().await.shutdown().await?;
        self.load_balancer.write().await.shutdown().await?;
        self.memory_manager.write().await.shutdown().await?;
        
        info!("Performance management system shutdown complete");
        Ok(())
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            cache: CacheConfig::default(),
            pool: PoolConfig::default(),
            load_balancer: LoadBalancerConfig::default(),
            memory: MemoryConfig::default(),
            monitoring: PerformanceMonitoringConfig::default(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            memory_cache_size_mb: 256,
            redis_url: None,
            disk_cache_path: None,
            default_ttl_seconds: 3600,
            max_key_size: 1024,
            max_value_size: 1024 * 1024, // 1MB
            eviction_policy: "lru".to_string(),
        }
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            database_pool_size: 100,
            redis_pool_size: 50,
            http_pool_size: 200,
            connection_timeout_seconds: 30,
            idle_timeout_seconds: 600,
            max_lifetime_seconds: 3600,
        }
    }
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: "round_robin".to_string(),
            health_check_interval_seconds: 30,
            failover_threshold: 3,
            circuit_breaker_enabled: true,
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_heap_size_mb: 2048,
            gc_threshold_mb: 1536,
            memory_monitoring_enabled: true,
            oom_protection_enabled: true,
        }
    }
}

impl Default for PerformanceMonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_collection_interval_seconds: 60,
            performance_alerts_enabled: true,
            slow_query_threshold_ms: 1000,
        }
    }
}

/// Initialize performance management system
pub async fn init_performance(config: PerformanceConfig) -> Result<PerformanceManager> {
    info!("Initializing A3Mailer performance management system");
    
    let manager = PerformanceManager::new(config).await?;
    
    info!("A3Mailer performance management system initialized successfully");
    Ok(manager)
}
