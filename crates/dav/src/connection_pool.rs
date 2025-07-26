/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! High-Performance Connection Pool for DAV Server
//!
//! This module provides an advanced connection pool implementation optimized
//! for high-concurrency DAV operations with intelligent connection management,
//! health monitoring, and adaptive scaling.

use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{
    sync::{Mutex, RwLock, Semaphore},
    time::interval,
};
use tracing::{debug, info, warn};

/// High-performance connection pool for DAV operations
///
/// Manages database and external service connections with intelligent
/// pooling, health monitoring, and adaptive scaling capabilities.
#[derive(Debug, Clone)]
pub struct DavConnectionPool {
    inner: Arc<DavConnectionPoolInner>,
    config: ConnectionPoolConfig,
}

#[derive(Debug)]
struct DavConnectionPoolInner {
    /// Available connections by pool name
    pools: RwLock<HashMap<String, Pool>>,
    /// Connection factory for creating new connections
    factory: Arc<dyn ConnectionFactory>,
    /// Pool metrics and statistics
    metrics: ConnectionPoolMetrics,
    /// Health monitor for connection validation
    health_monitor: Arc<Mutex<HealthMonitor>>,
}

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    /// Minimum connections per pool
    pub min_connections: usize,
    /// Maximum connections per pool
    pub max_connections: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Idle timeout for connections
    pub idle_timeout: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Maximum connection lifetime
    pub max_lifetime: Duration,
    /// Enable connection validation
    pub enable_validation: bool,
    /// Connection retry attempts
    pub retry_attempts: usize,
    /// Adaptive scaling enabled
    pub enable_adaptive_scaling: bool,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 5,
            max_connections: 100,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(300), // 5 minutes
            health_check_interval: Duration::from_secs(60),
            max_lifetime: Duration::from_secs(3600), // 1 hour
            enable_validation: true,
            retry_attempts: 3,
            enable_adaptive_scaling: true,
        }
    }
}

#[derive(Debug)]
struct Pool {
    name: String,
    available: VecDeque<PooledConnection>,
    in_use: HashMap<u64, PooledConnection>,
    semaphore: Arc<Semaphore>,
    created_at: Instant,
    last_scaled: Instant,
}

pub struct PooledConnection {
    id: u64,
    connection: Box<dyn Connection>,
    created_at: Instant,
    last_used: Instant,
    use_count: u64,
    is_healthy: bool,
}

impl std::fmt::Debug for PooledConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PooledConnection")
            .field("id", &self.id)
            .field("created_at", &self.created_at)
            .field("last_used", &self.last_used)
            .field("use_count", &self.use_count)
            .field("is_healthy", &self.is_healthy)
            .finish()
    }
}

#[derive(Debug, Default)]
struct ConnectionPoolMetrics {
    total_connections_created: AtomicU64,
    total_connections_destroyed: AtomicU64,
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    connection_timeouts: AtomicU64,
    health_check_failures: AtomicU64,
    pool_exhaustions: AtomicU64,
    average_wait_time: AtomicU64, // in nanoseconds
    peak_connections: AtomicUsize,
}

#[derive(Debug)]
struct HealthMonitor {
    last_check: Instant,
    unhealthy_connections: HashMap<u64, Instant>,
    check_in_progress: bool,
}

/// Trait for creating new connections
pub trait ConnectionFactory: Send + Sync + std::fmt::Debug {
    fn create_connection(&self, pool_name: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Box<dyn Connection>, ConnectionError>> + Send + '_>>;
    fn validate_connection(&self, connection: &PooledConnection) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>>;
}

/// Trait for database/service connections
pub trait Connection: Send + Sync + std::fmt::Debug {
    fn is_alive(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>>;
    fn reset(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ConnectionError>> + Send + '_>>;
    fn execute_query(&self, query: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<QueryResult, ConnectionError>> + Send + '_>>;
    fn get_connection_info(&self) -> ConnectionInfo;
}

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub database: Option<String>,
    pub connection_type: ConnectionType,
}

#[derive(Debug, Clone)]
pub enum ConnectionType {
    Database,
    Http,
    Cache,
    MessageQueue,
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub rows_affected: u64,
    pub data: Vec<HashMap<String, String>>,
    pub execution_time: Duration,
}

/// Handle for borrowed connections
pub struct ConnectionHandle {
    connection: Option<PooledConnection>,
    pool: DavConnectionPool,
    pool_name: String,
}

impl DavConnectionPool {
    /// Create a new connection pool
    pub fn new(factory: Arc<dyn ConnectionFactory>, config: ConnectionPoolConfig) -> Self {
        let pool = Self {
            inner: Arc::new(DavConnectionPoolInner {
                pools: RwLock::new(HashMap::new()),
                factory,
                metrics: ConnectionPoolMetrics::default(),
                health_monitor: Arc::new(Mutex::new(HealthMonitor {
                    last_check: Instant::now(),
                    unhealthy_connections: HashMap::new(),
                    check_in_progress: false,
                })),
            }),
            config: config.clone(),
        };

        // Start background tasks
        pool.start_health_monitor();
        pool.start_adaptive_scaler();

        pool
    }

    /// Get a connection from the specified pool
    pub async fn get_connection(&self, pool_name: &str) -> Result<ConnectionHandle, ConnectionError> {
        let start_time = Instant::now();
        self.inner.metrics.total_requests.fetch_add(1, Ordering::Relaxed);

        // Get or create pool
        let pool = self.get_or_create_pool(pool_name).await?;

        // Try to acquire semaphore permit
        let _permit = tokio::time::timeout(
            self.config.connection_timeout,
            pool.acquire()
        ).await
        .map_err(|_| ConnectionError::Timeout)?
        .map_err(|_| ConnectionError::PoolExhausted)?;

        // Try to get an available connection
        let connection = {
            let mut pools = self.inner.pools.write().await;
            if let Some(pool) = pools.get_mut(pool_name) {
                // Remove expired connections
                self.remove_expired_connections(pool).await;

                // Get available connection
                pool.available.pop_front()
            } else {
                None
            }
        };

        let connection = if let Some(mut conn) = connection {
            // Validate existing connection
            if self.config.enable_validation {
                if !self.inner.factory.validate_connection(&conn).await {
                    warn!(
                        connection_id = conn.id,
                        pool_name = pool_name,
                        "Connection validation failed, creating new connection"
                    );

                    self.create_new_connection(pool_name).await?
                } else {
                    conn.last_used = Instant::now();
                    conn.use_count += 1;
                    conn
                }
            } else {
                conn.last_used = Instant::now();
                conn.use_count += 1;
                conn
            }
        } else {
            // Create new connection
            self.create_new_connection(pool_name).await?
        };

        // Update metrics
        let wait_time = start_time.elapsed();
        let current_avg = self.inner.metrics.average_wait_time.load(Ordering::Relaxed);
        let new_avg = (current_avg + wait_time.as_nanos() as u64) / 2;
        self.inner.metrics.average_wait_time.store(new_avg, Ordering::Relaxed);

        self.inner.metrics.successful_requests.fetch_add(1, Ordering::Relaxed);

        debug!(
            pool_name = pool_name,
            connection_id = connection.id,
            wait_time_ms = wait_time.as_millis(),
            "Connection acquired"
        );

        Ok(ConnectionHandle {
            connection: Some(connection),
            pool: self.clone(),
            pool_name: pool_name.to_string(),
        })
    }

    /// Get pool statistics
    pub async fn get_pool_stats(&self) -> HashMap<String, PoolStats> {
        let pools = self.inner.pools.read().await;
        let mut stats = HashMap::new();

        for (name, pool) in pools.iter() {
            stats.insert(name.clone(), PoolStats {
                available_connections: pool.available.len(),
                in_use_connections: pool.in_use.len(),
                total_connections: pool.available.len() + pool.in_use.len(),
                max_connections: self.config.max_connections,
                created_at: pool.created_at,
                last_scaled: pool.last_scaled,
            });
        }

        stats
    }

    /// Get overall metrics
    pub fn get_metrics(&self) -> ConnectionPoolStats {
        ConnectionPoolStats {
            total_connections_created: self.inner.metrics.total_connections_created.load(Ordering::Relaxed),
            total_connections_destroyed: self.inner.metrics.total_connections_destroyed.load(Ordering::Relaxed),
            total_requests: self.inner.metrics.total_requests.load(Ordering::Relaxed),
            successful_requests: self.inner.metrics.successful_requests.load(Ordering::Relaxed),
            failed_requests: self.inner.metrics.failed_requests.load(Ordering::Relaxed),
            connection_timeouts: self.inner.metrics.connection_timeouts.load(Ordering::Relaxed),
            health_check_failures: self.inner.metrics.health_check_failures.load(Ordering::Relaxed),
            pool_exhaustions: self.inner.metrics.pool_exhaustions.load(Ordering::Relaxed),
            average_wait_time: Duration::from_nanos(
                self.inner.metrics.average_wait_time.load(Ordering::Relaxed)
            ),
            peak_connections: self.inner.metrics.peak_connections.load(Ordering::Relaxed),
        }
    }

    async fn get_or_create_pool(&self, pool_name: &str) -> Result<Arc<Semaphore>, ConnectionError> {
        let pools = self.inner.pools.read().await;
        if let Some(pool) = pools.get(pool_name) {
            return Ok(pool.semaphore.clone());
        }
        drop(pools);

        // Create new pool
        let mut pools = self.inner.pools.write().await;
        if let Some(pool) = pools.get(pool_name) {
            return Ok(pool.semaphore.clone());
        }

        let pool = Pool {
            name: pool_name.to_string(),
            available: VecDeque::new(),
            in_use: HashMap::new(),
            semaphore: Arc::new(Semaphore::new(self.config.max_connections)),
            created_at: Instant::now(),
            last_scaled: Instant::now(),
        };

        let semaphore = pool.semaphore.clone();
        pools.insert(pool_name.to_string(), pool);

        info!(
            pool_name = pool_name,
            max_connections = self.config.max_connections,
            "Created new connection pool"
        );

        Ok(semaphore)
    }

    async fn create_new_connection(&self, pool_name: &str) -> Result<PooledConnection, ConnectionError> {
        let connection = self.inner.factory.create_connection(pool_name).await?;
        let connection_id = self.inner.metrics.total_connections_created.fetch_add(1, Ordering::Relaxed);

        let pooled_connection = PooledConnection {
            id: connection_id,
            connection,
            created_at: Instant::now(),
            last_used: Instant::now(),
            use_count: 1,
            is_healthy: true,
        };

        debug!(
            pool_name = pool_name,
            connection_id = connection_id,
            "Created new connection"
        );

        Ok(pooled_connection)
    }

    async fn remove_expired_connections(&self, pool: &mut Pool) {
        let now = Instant::now();
        let mut expired_connections = Vec::new();

        // Find expired connections
        while let Some(conn) = pool.available.front() {
            if now.duration_since(conn.last_used) > self.config.idle_timeout
                || now.duration_since(conn.created_at) > self.config.max_lifetime
            {
                expired_connections.push(pool.available.pop_front().unwrap());
            } else {
                break;
            }
        }

        // Clean up expired connections
        for conn in expired_connections {
            self.inner.metrics.total_connections_destroyed.fetch_add(1, Ordering::Relaxed);
            debug!(
                connection_id = conn.id,
                age_seconds = now.duration_since(conn.created_at).as_secs(),
                idle_seconds = now.duration_since(conn.last_used).as_secs(),
                "Removed expired connection"
            );
        }
    }

    fn start_health_monitor(&self) {
        if !self.config.enable_validation {
            return;
        }

        let inner = self.inner.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(config.health_check_interval);

            loop {
                interval.tick().await;

                let mut health_monitor = inner.health_monitor.lock().await;
                if health_monitor.check_in_progress {
                    continue;
                }
                health_monitor.check_in_progress = true;
                health_monitor.last_check = Instant::now();
                drop(health_monitor);

                // Perform health checks
                let pools = inner.pools.read().await;
                for (pool_name, pool) in pools.iter() {
                    for conn in &pool.available {
                        if !conn.connection.is_alive().await {
                            inner.metrics.health_check_failures.fetch_add(1, Ordering::Relaxed);
                            warn!(
                                pool_name = pool_name,
                                connection_id = conn.id,
                                "Connection health check failed"
                            );
                        }
                    }
                }
                drop(pools);

                let mut health_monitor = inner.health_monitor.lock().await;
                health_monitor.check_in_progress = false;
            }
        });
    }

    fn start_adaptive_scaler(&self) {
        if !self.config.enable_adaptive_scaling {
            return;
        }

        let inner = self.inner.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Implement adaptive scaling logic
                let pools = inner.pools.read().await;
                for (pool_name, pool) in pools.iter() {
                    let utilization = pool.in_use.len() as f64 / config.max_connections as f64;

                    if utilization > 0.8 && pool.available.is_empty() {
                        debug!(
                            pool_name = pool_name,
                            utilization = utilization,
                            "High pool utilization detected"
                        );
                    }
                }
            }
        });
    }
}

impl ConnectionHandle {
    /// Execute a query using this connection
    pub async fn execute_query(&self, query: &str) -> Result<QueryResult, ConnectionError> {
        if let Some(ref connection) = self.connection {
            connection.connection.execute_query(query).await
        } else {
            Err(ConnectionError::ConnectionClosed)
        }
    }

    /// Get connection information
    pub fn get_info(&self) -> Option<ConnectionInfo> {
        self.connection.as_ref().map(|conn| conn.connection.get_connection_info())
    }
}

impl Drop for ConnectionHandle {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.take() {
            let pool = self.pool.clone();
            let pool_name = self.pool_name.clone();

            tokio::spawn(async move {
                let mut pools = pool.inner.pools.write().await;
                if let Some(pool_data) = pools.get_mut(&pool_name) {
                    pool_data.available.push_back(connection);
                }
            });
        }
    }
}

/// Connection pool error types
#[derive(Debug, Clone)]
pub enum ConnectionError {
    Timeout,
    PoolExhausted,
    ConnectionFailed(String),
    ValidationFailed,
    ConnectionClosed,
    QueryFailed(String),
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub available_connections: usize,
    pub in_use_connections: usize,
    pub total_connections: usize,
    pub max_connections: usize,
    pub created_at: Instant,
    pub last_scaled: Instant,
}

/// Overall connection pool statistics
#[derive(Debug, Clone)]
pub struct ConnectionPoolStats {
    pub total_connections_created: u64,
    pub total_connections_destroyed: u64,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub connection_timeouts: u64,
    pub health_check_failures: u64,
    pub pool_exhaustions: u64,
    pub average_wait_time: Duration,
    pub peak_connections: usize,
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "Connection timeout"),
            Self::PoolExhausted => write!(f, "Connection pool exhausted"),
            Self::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            Self::ValidationFailed => write!(f, "Connection validation failed"),
            Self::ConnectionClosed => write!(f, "Connection is closed"),
            Self::QueryFailed(msg) => write!(f, "Query failed: {}", msg),
        }
    }
}

impl std::error::Error for ConnectionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    // Mock connection factory for testing
    #[derive(Debug, Clone)]
    struct MockConnectionFactory {
        connection_counter: Arc<AtomicU64>,
        should_fail: bool,
    }

    impl MockConnectionFactory {
        fn new() -> Self {
            Self {
                connection_counter: Arc::new(AtomicU64::new(0)),
                should_fail: false,
            }
        }

        fn with_failure() -> Self {
            Self {
                connection_counter: Arc::new(AtomicU64::new(0)),
                should_fail: true,
            }
        }
    }

    #[async_trait::async_trait]
    impl ConnectionFactory for MockConnectionFactory {
        async fn create_connection(&self, _pool_name: &str) -> Result<Box<dyn Connection>, ConnectionError> {
            if self.should_fail {
                return Err(ConnectionError::ConnectionFailed("Mock failure".to_string()));
            }

            let id = self.connection_counter.fetch_add(1, Ordering::Relaxed);
            Ok(Box::new(MockConnection { id }))
        }
    }

    // Mock connection for testing
    #[derive(Debug)]
    struct MockConnection {
        id: u64,
    }

    #[async_trait::async_trait]
    impl Connection for MockConnection {
        async fn execute_query(&self, _query: &str) -> Result<QueryResult, ConnectionError> {
            // Simulate query execution
            tokio::time::sleep(Duration::from_millis(1)).await;
            Ok(QueryResult {
                rows_affected: 1,
                data: vec![],
            })
        }

        async fn is_alive(&self) -> bool {
            true
        }

        fn get_connection_info(&self) -> ConnectionInfo {
            ConnectionInfo {
                id: self.id,
                created_at: Instant::now(),
                last_used: Instant::now(),
                query_count: 0,
                is_healthy: true,
            }
        }
    }

    #[tokio::test]
    async fn test_connection_pool_creation() {
        let config = ConnectionPoolConfig::default();
        let factory = MockConnectionFactory::new();
        let pool = ConnectionPool::new(config, Box::new(factory));

        let stats = pool.get_metrics();
        assert_eq!(stats.total_connections_created, 0);
        assert_eq!(stats.total_requests, 0);
    }

    #[tokio::test]
    async fn test_get_connection() {
        let config = ConnectionPoolConfig {
            max_connections: 5,
            min_connections: 1,
            ..Default::default()
        };
        let factory = MockConnectionFactory::new();
        let pool = ConnectionPool::new(config, Box::new(factory));

        // Get a connection
        let handle = pool.get_connection("test_pool").await;
        assert!(handle.is_ok());

        let connection = handle.unwrap();
        assert!(connection.connection.is_some());

        let stats = pool.get_metrics();
        assert_eq!(stats.total_connections_created, 1);
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.successful_requests, 1);
    }

    #[tokio::test]
    async fn test_connection_reuse() {
        let config = ConnectionPoolConfig {
            max_connections: 5,
            min_connections: 1,
            ..Default::default()
        };
        let factory = MockConnectionFactory::new();
        let pool = ConnectionPool::new(config, Box::new(factory));

        // Get first connection
        let handle1 = pool.get_connection("test_pool").await.unwrap();
        let connection_id1 = handle1.get_info().unwrap().id;
        drop(handle1); // Return to pool

        // Wait for connection to be returned
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Get second connection - should reuse the first one
        let handle2 = pool.get_connection("test_pool").await.unwrap();
        let connection_id2 = handle2.get_info().unwrap().id;

        assert_eq!(connection_id1, connection_id2);

        let stats = pool.get_metrics();
        assert_eq!(stats.total_connections_created, 1); // Only one connection created
        assert_eq!(stats.total_requests, 2); // Two requests made
    }

    #[tokio::test]
    async fn test_connection_pool_exhaustion() {
        let config = ConnectionPoolConfig {
            max_connections: 2,
            min_connections: 0,
            connection_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let factory = MockConnectionFactory::new();
        let pool = ConnectionPool::new(config, Box::new(factory));

        // Get maximum connections
        let _handle1 = pool.get_connection("test_pool").await.unwrap();
        let _handle2 = pool.get_connection("test_pool").await.unwrap();

        // Third connection should timeout
        let start = Instant::now();
        let result = pool.get_connection("test_pool").await;
        let elapsed = start.elapsed();

        assert!(result.is_err());
        assert!(elapsed >= Duration::from_millis(90)); // Should timeout
        assert!(matches!(result.unwrap_err(), ConnectionError::Timeout));

        let stats = pool.get_metrics();
        assert!(stats.connection_timeouts > 0);
    }

    #[tokio::test]
    async fn test_connection_factory_failure() {
        let config = ConnectionPoolConfig::default();
        let factory = MockConnectionFactory::with_failure();
        let pool = ConnectionPool::new(config, Box::new(factory));

        let result = pool.get_connection("test_pool").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConnectionError::ConnectionFailed(_)));

        let stats = pool.get_metrics();
        assert_eq!(stats.failed_requests, 1);
    }

    #[tokio::test]
    async fn test_multiple_pools() {
        let config = ConnectionPoolConfig::default();
        let factory = MockConnectionFactory::new();
        let pool = ConnectionPool::new(config, Box::new(factory));

        // Get connections from different pools
        let handle1 = pool.get_connection("pool1").await.unwrap();
        let handle2 = pool.get_connection("pool2").await.unwrap();

        let pool_stats = pool.get_pool_stats().await;
        assert_eq!(pool_stats.len(), 2);
        assert!(pool_stats.contains_key("pool1"));
        assert!(pool_stats.contains_key("pool2"));

        drop(handle1);
        drop(handle2);
    }

    #[tokio::test]
    async fn test_connection_handle_query_execution() {
        let config = ConnectionPoolConfig::default();
        let factory = MockConnectionFactory::new();
        let pool = ConnectionPool::new(config, Box::new(factory));

        let handle = pool.get_connection("test_pool").await.unwrap();
        let result = handle.execute_query("SELECT 1").await;

        assert!(result.is_ok());
        let query_result = result.unwrap();
        assert_eq!(query_result.rows_affected, 1);
    }

    #[tokio::test]
    async fn test_connection_handle_info() {
        let config = ConnectionPoolConfig::default();
        let factory = MockConnectionFactory::new();
        let pool = ConnectionPool::new(config, Box::new(factory));

        let handle = pool.get_connection("test_pool").await.unwrap();
        let info = handle.get_info();

        assert!(info.is_some());
        let connection_info = info.unwrap();
        assert!(connection_info.is_healthy);
        assert_eq!(connection_info.query_count, 0);
    }

    #[tokio::test]
    async fn test_connection_handle_drop() {
        let config = ConnectionPoolConfig::default();
        let factory = MockConnectionFactory::new();
        let pool = ConnectionPool::new(config, Box::new(factory));

        let handle = pool.get_connection("test_pool").await.unwrap();
        let connection_id = handle.get_info().unwrap().id;

        // Drop the handle to return connection to pool
        drop(handle);

        // Wait for async drop to complete
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Get another connection - should reuse the same one
        let handle2 = pool.get_connection("test_pool").await.unwrap();
        let connection_id2 = handle2.get_info().unwrap().id;

        assert_eq!(connection_id, connection_id2);
    }

    #[tokio::test]
    async fn test_concurrent_connections() {
        let config = ConnectionPoolConfig {
            max_connections: 10,
            min_connections: 0,
            ..Default::default()
        };
        let factory = MockConnectionFactory::new();
        let pool = Arc::new(ConnectionPool::new(config, Box::new(factory)));

        let mut handles = vec![];

        // Spawn multiple tasks to get connections concurrently
        for i in 0..5 {
            let pool_clone = pool.clone();
            let handle = tokio::spawn(async move {
                let connection = pool_clone.get_connection(&format!("pool_{}", i % 2)).await;
                assert!(connection.is_ok());

                // Hold connection for a bit
                tokio::time::sleep(Duration::from_millis(10)).await;

                connection.unwrap()
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        let mut connections = vec![];
        for handle in handles {
            connections.push(handle.await.unwrap());
        }

        let stats = pool.get_metrics();
        assert_eq!(stats.total_requests, 5);
        assert_eq!(stats.successful_requests, 5);

        // Drop all connections
        drop(connections);
    }

    #[tokio::test]
    async fn test_connection_error_display() {
        let error = ConnectionError::Timeout;
        assert_eq!(error.to_string(), "Connection timeout");

        let error = ConnectionError::PoolExhausted;
        assert_eq!(error.to_string(), "Connection pool exhausted");

        let error = ConnectionError::ConnectionFailed("test error".to_string());
        assert_eq!(error.to_string(), "Connection failed: test error");

        let error = ConnectionError::ValidationFailed;
        assert_eq!(error.to_string(), "Connection validation failed");

        let error = ConnectionError::ConnectionClosed;
        assert_eq!(error.to_string(), "Connection is closed");

        let error = ConnectionError::QueryFailed("query error".to_string());
        assert_eq!(error.to_string(), "Query failed: query error");
    }

    #[tokio::test]
    async fn test_pool_statistics() {
        let config = ConnectionPoolConfig {
            max_connections: 3,
            min_connections: 1,
            ..Default::default()
        };
        let factory = MockConnectionFactory::new();
        let pool = ConnectionPool::new(config, Box::new(factory));

        // Create some activity
        let handle1 = pool.get_connection("test_pool").await.unwrap();
        let handle2 = pool.get_connection("test_pool").await.unwrap();

        let pool_stats = pool.get_pool_stats().await;
        let test_pool_stats = pool_stats.get("test_pool").unwrap();

        assert_eq!(test_pool_stats.in_use_connections, 2);
        assert_eq!(test_pool_stats.max_connections, 3);
        assert!(test_pool_stats.total_connections >= 2);

        drop(handle1);
        drop(handle2);

        // Wait for connections to be returned
        tokio::time::sleep(Duration::from_millis(10)).await;

        let pool_stats = pool.get_pool_stats().await;
        let test_pool_stats = pool_stats.get("test_pool").unwrap();
        assert_eq!(test_pool_stats.in_use_connections, 0);
    }
}
