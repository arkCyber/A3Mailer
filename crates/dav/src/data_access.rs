/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! High-Performance Data Access Layer for DAV Server
//!
//! This module provides an optimized data access layer with connection pooling,
//! query optimization, caching, and transaction management for maximum performance.

use std::{
    collections::HashMap,
    sync::{Arc, atomic::{AtomicU64, Ordering}},
    time::{Duration, Instant},
};
use tokio::sync::{RwLock, Mutex};
use tracing::{debug, info, warn, error};

/// High-performance data access layer
///
/// Provides optimized database operations with connection pooling,
/// query caching, and transaction management.
#[derive(Debug, Clone)]
pub struct DataAccessLayer {
    inner: Arc<DataAccessLayerInner>,
    config: DataAccessConfig,
}

#[derive(Debug)]
struct DataAccessLayerInner {
    /// Connection pool
    connection_pool: ConnectionPool,
    /// Query cache
    query_cache: RwLock<HashMap<String, CachedQuery>>,
    /// Transaction manager
    transaction_manager: Mutex<TransactionManager>,
    /// Performance metrics
    metrics: DataAccessMetrics,
}

/// Data access configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataAccessConfig {
    /// Maximum database connections
    pub max_connections: usize,
    /// Minimum database connections
    pub min_connections: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Query timeout
    pub query_timeout: Duration,
    /// Enable query caching
    pub enable_query_cache: bool,
    /// Maximum cached queries
    pub max_cached_queries: usize,
    /// Query cache TTL
    pub query_cache_ttl: Duration,
    /// Enable prepared statements
    pub enable_prepared_statements: bool,
    /// Transaction timeout
    pub transaction_timeout: Duration,
    /// Enable connection validation
    pub enable_connection_validation: bool,
}

impl Default for DataAccessConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            min_connections: 10,
            connection_timeout: Duration::from_secs(30),
            query_timeout: Duration::from_secs(60),
            enable_query_cache: true,
            max_cached_queries: 1000,
            query_cache_ttl: Duration::from_secs(300), // 5 minutes
            enable_prepared_statements: true,
            transaction_timeout: Duration::from_secs(300), // 5 minutes
            enable_connection_validation: true,
        }
    }
}

#[derive(Debug)]
struct ConnectionPool {
    available: Mutex<Vec<DatabaseConnection>>,
    in_use: RwLock<HashMap<u64, DatabaseConnection>>,
    connection_counter: AtomicU64,
    max_connections: usize,
    min_connections: usize,
}

#[derive(Debug, Clone)]
struct DatabaseConnection {
    id: u64,
    created_at: Instant,
    last_used: Instant,
    query_count: u64,
    is_healthy: bool,
    connection_string: String,
}

#[derive(Debug, Clone)]
struct CachedQuery {
    sql: String,
    result: QueryResult,
    cached_at: Instant,
    access_count: u64,
    last_accessed: Instant,
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub rows: Vec<HashMap<String, String>>,
    pub rows_affected: u64,
    pub execution_time: Duration,
    pub from_cache: bool,
}

#[derive(Debug)]
struct TransactionManager {
    active_transactions: HashMap<u64, Transaction>,
    transaction_counter: AtomicU64,
}

#[derive(Debug)]
struct Transaction {
    id: u64,
    connection_id: u64,
    started_at: Instant,
    operations: Vec<TransactionOperation>,
    is_committed: bool,
    is_rolled_back: bool,
}

#[derive(Debug, Clone)]
struct TransactionOperation {
    sql: String,
    parameters: Vec<String>,
    executed_at: Instant,
    result: Option<QueryResult>,
}

#[derive(Debug, Default)]
struct DataAccessMetrics {
    total_queries: AtomicU64,
    successful_queries: AtomicU64,
    failed_queries: AtomicU64,
    cached_queries: AtomicU64,
    query_cache_hits: AtomicU64,
    query_cache_misses: AtomicU64,
    active_connections: AtomicU64,
    total_connections_created: AtomicU64,
    total_connections_destroyed: AtomicU64,
    active_transactions: AtomicU64,
    committed_transactions: AtomicU64,
    rolled_back_transactions: AtomicU64,
    average_query_time: AtomicU64, // in nanoseconds
    average_connection_time: AtomicU64, // in nanoseconds
}

impl DataAccessLayer {
    /// Create a new data access layer
    pub fn new(config: DataAccessConfig) -> Self {
        let dal = Self {
            inner: Arc::new(DataAccessLayerInner {
                connection_pool: ConnectionPool {
                    available: Mutex::new(Vec::new()),
                    in_use: RwLock::new(HashMap::new()),
                    connection_counter: AtomicU64::new(0),
                    max_connections: config.max_connections,
                    min_connections: config.min_connections,
                },
                query_cache: RwLock::new(HashMap::new()),
                transaction_manager: Mutex::new(TransactionManager {
                    active_transactions: HashMap::new(),
                    transaction_counter: AtomicU64::new(0),
                }),
                metrics: DataAccessMetrics::default(),
            }),
            config: config.clone(),
        };

        // Initialize minimum connections
        tokio::spawn({
            let dal = dal.clone();
            async move {
                dal.initialize_connections().await;
            }
        });

        info!(
            max_connections = config.max_connections,
            min_connections = config.min_connections,
            query_cache = config.enable_query_cache,
            prepared_statements = config.enable_prepared_statements,
            "Data access layer initialized"
        );

        dal
    }

    /// Execute a query with optimization
    pub async fn execute_query(
        &self,
        sql: &str,
        parameters: &[&str],
    ) -> Result<QueryResult, DataAccessError> {
        let start_time = Instant::now();
        self.inner.metrics.total_queries.fetch_add(1, Ordering::Relaxed);

        // Check query cache
        if self.config.enable_query_cache && self.is_cacheable_query(sql) {
            let cache_key = self.generate_cache_key(sql, parameters);
            if let Some(cached_result) = self.get_cached_query(&cache_key).await {
                self.inner.metrics.query_cache_hits.fetch_add(1, Ordering::Relaxed);

                debug!(
                    sql = sql,
                    cache_hit = true,
                    "Query served from cache"
                );

                return Ok(QueryResult {
                    from_cache: true,
                    ..cached_result.result
                });
            }
            self.inner.metrics.query_cache_misses.fetch_add(1, Ordering::Relaxed);
        }

        // Get database connection
        let connection = self.get_connection().await?;

        // Execute query
        let result = self.execute_query_on_connection(&connection, sql, parameters).await?;

        // Return connection to pool
        self.return_connection(connection).await;

        // Cache result if applicable
        if self.config.enable_query_cache && self.is_cacheable_query(sql) {
            let cache_key = self.generate_cache_key(sql, parameters);
            self.cache_query_result(cache_key, sql, &result).await;
        }

        // Update metrics
        let query_time = start_time.elapsed();
        let current_avg = self.inner.metrics.average_query_time.load(Ordering::Relaxed);
        let new_avg = (current_avg + query_time.as_nanos() as u64) / 2;
        self.inner.metrics.average_query_time.store(new_avg, Ordering::Relaxed);

        self.inner.metrics.successful_queries.fetch_add(1, Ordering::Relaxed);

        debug!(
            sql = sql,
            rows_affected = result.rows_affected,
            execution_time_ms = query_time.as_millis(),
            "Query executed successfully"
        );

        Ok(QueryResult {
            from_cache: false,
            ..result
        })
    }

    /// Begin a new transaction
    pub async fn begin_transaction(&self) -> Result<u64, DataAccessError> {
        let connection = self.get_connection().await?;
        let transaction_id = self.inner.connection_pool.connection_counter.fetch_add(1, Ordering::Relaxed);

        let transaction = Transaction {
            id: transaction_id,
            connection_id: connection.id,
            started_at: Instant::now(),
            operations: Vec::new(),
            is_committed: false,
            is_rolled_back: false,
        };

        {
            let mut tx_manager = self.inner.transaction_manager.lock().await;
            tx_manager.active_transactions.insert(transaction_id, transaction);
        }

        self.inner.metrics.active_transactions.fetch_add(1, Ordering::Relaxed);

        debug!(
            transaction_id = transaction_id,
            connection_id = connection.id,
            "Transaction started"
        );

        Ok(transaction_id)
    }

    /// Commit a transaction
    pub async fn commit_transaction(&self, transaction_id: u64) -> Result<(), DataAccessError> {
        let mut tx_manager = self.inner.transaction_manager.lock().await;

        if let Some(mut transaction) = tx_manager.active_transactions.remove(&transaction_id) {
            transaction.is_committed = true;

            self.inner.metrics.active_transactions.fetch_sub(1, Ordering::Relaxed);
            self.inner.metrics.committed_transactions.fetch_add(1, Ordering::Relaxed);

            debug!(
                transaction_id = transaction_id,
                operations = transaction.operations.len(),
                duration_ms = transaction.started_at.elapsed().as_millis(),
                "Transaction committed"
            );

            Ok(())
        } else {
            Err(DataAccessError::TransactionNotFound(transaction_id))
        }
    }

    /// Rollback a transaction
    pub async fn rollback_transaction(&self, transaction_id: u64) -> Result<(), DataAccessError> {
        let mut tx_manager = self.inner.transaction_manager.lock().await;

        if let Some(mut transaction) = tx_manager.active_transactions.remove(&transaction_id) {
            transaction.is_rolled_back = true;

            self.inner.metrics.active_transactions.fetch_sub(1, Ordering::Relaxed);
            self.inner.metrics.rolled_back_transactions.fetch_add(1, Ordering::Relaxed);

            warn!(
                transaction_id = transaction_id,
                operations = transaction.operations.len(),
                duration_ms = transaction.started_at.elapsed().as_millis(),
                "Transaction rolled back"
            );

            Ok(())
        } else {
            Err(DataAccessError::TransactionNotFound(transaction_id))
        }
    }

    /// Get data access performance statistics
    pub async fn get_performance_stats(&self) -> DataAccessStats {
        let query_cache = self.inner.query_cache.read().await;
        let tx_manager = self.inner.transaction_manager.lock().await;

        DataAccessStats {
            total_queries: self.inner.metrics.total_queries.load(Ordering::Relaxed),
            successful_queries: self.inner.metrics.successful_queries.load(Ordering::Relaxed),
            failed_queries: self.inner.metrics.failed_queries.load(Ordering::Relaxed),
            cached_queries: query_cache.len(),
            query_cache_hits: self.inner.metrics.query_cache_hits.load(Ordering::Relaxed),
            query_cache_misses: self.inner.metrics.query_cache_misses.load(Ordering::Relaxed),
            cache_hit_rate: {
                let hits = self.inner.metrics.query_cache_hits.load(Ordering::Relaxed);
                let misses = self.inner.metrics.query_cache_misses.load(Ordering::Relaxed);
                if hits + misses > 0 {
                    hits as f64 / (hits + misses) as f64
                } else {
                    0.0
                }
            },
            active_connections: self.inner.metrics.active_connections.load(Ordering::Relaxed),
            total_connections_created: self.inner.metrics.total_connections_created.load(Ordering::Relaxed),
            total_connections_destroyed: self.inner.metrics.total_connections_destroyed.load(Ordering::Relaxed),
            active_transactions: tx_manager.active_transactions.len(),
            committed_transactions: self.inner.metrics.committed_transactions.load(Ordering::Relaxed),
            rolled_back_transactions: self.inner.metrics.rolled_back_transactions.load(Ordering::Relaxed),
            average_query_time: Duration::from_nanos(
                self.inner.metrics.average_query_time.load(Ordering::Relaxed)
            ),
            average_connection_time: Duration::from_nanos(
                self.inner.metrics.average_connection_time.load(Ordering::Relaxed)
            ),
        }
    }

    async fn initialize_connections(&self) {
        for _ in 0..self.config.min_connections {
            if let Ok(connection) = self.create_connection().await {
                let mut available = self.inner.connection_pool.available.lock().await;
                available.push(connection);
            }
        }

        info!(
            initialized_connections = self.config.min_connections,
            "Initial connections created"
        );
    }

    async fn get_connection(&self) -> Result<DatabaseConnection, DataAccessError> {
        let start_time = Instant::now();

        // Try to get from available pool
        {
            let mut available = self.inner.connection_pool.available.lock().await;
            if let Some(mut connection) = available.pop() {
                connection.last_used = Instant::now();
                connection.query_count += 1;

                let mut in_use = self.inner.connection_pool.in_use.write().await;
                in_use.insert(connection.id, connection.clone());

                let connection_time = start_time.elapsed();
                let current_avg = self.inner.metrics.average_connection_time.load(Ordering::Relaxed);
                let new_avg = (current_avg + connection_time.as_nanos() as u64) / 2;
                self.inner.metrics.average_connection_time.store(new_avg, Ordering::Relaxed);

                return Ok(connection);
            }
        }

        // Create new connection if under limit
        let in_use = self.inner.connection_pool.in_use.read().await;
        if in_use.len() < self.inner.connection_pool.max_connections {
            drop(in_use);
            let connection = self.create_connection().await?;

            let mut in_use = self.inner.connection_pool.in_use.write().await;
            in_use.insert(connection.id, connection.clone());

            Ok(connection)
        } else {
            Err(DataAccessError::ConnectionPoolExhausted)
        }
    }

    async fn return_connection(&self, connection: DatabaseConnection) {
        let mut in_use = self.inner.connection_pool.in_use.write().await;
        in_use.remove(&connection.id);
        drop(in_use);

        let mut available = self.inner.connection_pool.available.lock().await;
        available.push(connection);
    }

    async fn create_connection(&self) -> Result<DatabaseConnection, DataAccessError> {
        let connection_id = self.inner.connection_pool.connection_counter.fetch_add(1, Ordering::Relaxed);

        // Simulate connection creation
        tokio::time::sleep(Duration::from_millis(10)).await;

        let connection = DatabaseConnection {
            id: connection_id,
            created_at: Instant::now(),
            last_used: Instant::now(),
            query_count: 0,
            is_healthy: true,
            connection_string: "postgresql://localhost:5432/stalwart".to_string(),
        };

        self.inner.metrics.total_connections_created.fetch_add(1, Ordering::Relaxed);
        self.inner.metrics.active_connections.fetch_add(1, Ordering::Relaxed);

        debug!(
            connection_id = connection_id,
            "Database connection created"
        );

        Ok(connection)
    }

    async fn execute_query_on_connection(
        &self,
        connection: &DatabaseConnection,
        sql: &str,
        parameters: &[&str],
    ) -> Result<QueryResult, DataAccessError> {
        let start_time = Instant::now();

        // Simulate query execution
        let execution_time = match sql.to_lowercase().as_str() {
            s if s.contains("select") => Duration::from_millis(10),
            s if s.contains("insert") => Duration::from_millis(20),
            s if s.contains("update") => Duration::from_millis(30),
            s if s.contains("delete") => Duration::from_millis(25),
            _ => Duration::from_millis(15),
        };

        tokio::time::sleep(execution_time).await;

        // Simulate result
        let result = QueryResult {
            rows: vec![
                {
                    let mut row = HashMap::new();
                    row.insert("id".to_string(), "1".to_string());
                    row.insert("name".to_string(), "test".to_string());
                    row
                }
            ],
            rows_affected: 1,
            execution_time: start_time.elapsed(),
            from_cache: false,
        };

        debug!(
            connection_id = connection.id,
            sql = sql,
            parameters = ?parameters,
            execution_time_ms = execution_time.as_millis(),
            "Query executed on connection"
        );

        Ok(result)
    }

    async fn get_cached_query(&self, cache_key: &str) -> Option<CachedQuery> {
        let mut cache = self.inner.query_cache.write().await;

        if let Some(cached) = cache.get_mut(cache_key) {
            if cached.cached_at.elapsed() < self.config.query_cache_ttl {
                cached.access_count += 1;
                cached.last_accessed = Instant::now();
                Some(cached.clone())
            } else {
                cache.remove(cache_key);
                None
            }
        } else {
            None
        }
    }

    async fn cache_query_result(&self, cache_key: String, sql: &str, result: &QueryResult) {
        let mut cache = self.inner.query_cache.write().await;

        if cache.len() >= self.config.max_cached_queries {
            // Remove oldest entry
            if let Some((oldest_key, _)) = cache.iter()
                .min_by_key(|(_, query)| query.last_accessed)
                .map(|(k, v)| (k.clone(), v.clone()))
            {
                cache.remove(&oldest_key);
            }
        }

        let cached_query = CachedQuery {
            sql: sql.to_string(),
            result: result.clone(),
            cached_at: Instant::now(),
            access_count: 0,
            last_accessed: Instant::now(),
        };

        cache.insert(cache_key, cached_query);
        self.inner.metrics.cached_queries.fetch_add(1, Ordering::Relaxed);
    }

    fn is_cacheable_query(&self, sql: &str) -> bool {
        let sql_lower = sql.to_lowercase();
        sql_lower.starts_with("select") && !sql_lower.contains("now()") && !sql_lower.contains("random()")
    }

    fn generate_cache_key(&self, sql: &str, parameters: &[&str]) -> String {
        format!("{}:{}", sql, parameters.join(","))
    }
}

/// Data access error types
#[derive(Debug, Clone)]
pub enum DataAccessError {
    ConnectionPoolExhausted,
    ConnectionTimeout,
    QueryTimeout,
    TransactionNotFound(u64),
    TransactionTimeout(u64),
    DatabaseError(String),
    CacheError(String),
}

/// Data access performance statistics
#[derive(Debug, Clone)]
pub struct DataAccessStats {
    pub total_queries: u64,
    pub successful_queries: u64,
    pub failed_queries: u64,
    pub cached_queries: usize,
    pub query_cache_hits: u64,
    pub query_cache_misses: u64,
    pub cache_hit_rate: f64,
    pub active_connections: u64,
    pub total_connections_created: u64,
    pub total_connections_destroyed: u64,
    pub active_transactions: usize,
    pub committed_transactions: u64,
    pub rolled_back_transactions: u64,
    pub average_query_time: Duration,
    pub average_connection_time: Duration,
}

impl std::fmt::Display for DataAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionPoolExhausted => write!(f, "Connection pool exhausted"),
            Self::ConnectionTimeout => write!(f, "Connection timeout"),
            Self::QueryTimeout => write!(f, "Query timeout"),
            Self::TransactionNotFound(id) => write!(f, "Transaction {} not found", id),
            Self::TransactionTimeout(id) => write!(f, "Transaction {} timed out", id),
            Self::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            Self::CacheError(msg) => write!(f, "Cache error: {}", msg),
        }
    }
}

impl std::error::Error for DataAccessError {}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_data_access_layer_creation() {
        let config = DataAccessConfig::default();
        let dal = DataAccessLayer::new(config);

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(100)).await;

        let stats = dal.get_performance_stats().await;
        assert_eq!(stats.total_queries, 0);
        assert_eq!(stats.active_transactions, 0);
    }

    #[tokio::test]
    async fn test_query_execution() {
        let config = DataAccessConfig {
            max_connections: 5,
            min_connections: 2,
            enable_query_cache: false, // Disable cache for this test
            ..Default::default()
        };
        let dal = DataAccessLayer::new(config);

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Execute a simple query
        let result = dal.execute_query(
            "SELECT * FROM users WHERE id = ?",
            &["1"],
        ).await;

        assert!(result.is_ok());
        let query_result = result.unwrap();
        assert!(!query_result.from_cache);
        assert_eq!(query_result.rows_affected, 1);
        assert!(!query_result.rows.is_empty());

        let stats = dal.get_performance_stats().await;
        assert_eq!(stats.total_queries, 1);
        assert_eq!(stats.successful_queries, 1);
    }

    #[tokio::test]
    async fn test_query_caching() {
        let config = DataAccessConfig {
            enable_query_cache: true,
            max_cached_queries: 10,
            query_cache_ttl: Duration::from_secs(60),
            ..Default::default()
        };
        let dal = DataAccessLayer::new(config);

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(100)).await;

        let sql = "SELECT * FROM users WHERE active = ?";
        let params = &["true"];

        // First query should miss cache
        let result1 = dal.execute_query(sql, params).await.unwrap();
        assert!(!result1.from_cache);

        // Second query should hit cache
        let result2 = dal.execute_query(sql, params).await.unwrap();
        assert!(result2.from_cache);

        let stats = dal.get_performance_stats().await;
        assert!(stats.query_cache_hits > 0);
        assert!(stats.cache_hit_rate > 0.0);
    }

    #[tokio::test]
    async fn test_transaction_management() {
        let config = DataAccessConfig::default();
        let dal = DataAccessLayer::new(config);

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Begin transaction
        let tx_id = dal.begin_transaction().await.unwrap();
        assert!(tx_id > 0);

        let stats = dal.get_performance_stats().await;
        assert_eq!(stats.active_transactions, 1);

        // Commit transaction
        let result = dal.commit_transaction(tx_id).await;
        assert!(result.is_ok());

        let stats = dal.get_performance_stats().await;
        assert_eq!(stats.active_transactions, 0);
        assert_eq!(stats.committed_transactions, 1);
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let config = DataAccessConfig::default();
        let dal = DataAccessLayer::new(config);

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Begin transaction
        let tx_id = dal.begin_transaction().await.unwrap();

        // Rollback transaction
        let result = dal.rollback_transaction(tx_id).await;
        assert!(result.is_ok());

        let stats = dal.get_performance_stats().await;
        assert_eq!(stats.active_transactions, 0);
        assert_eq!(stats.rolled_back_transactions, 1);
    }

    #[tokio::test]
    async fn test_connection_pool_management() {
        let config = DataAccessConfig {
            max_connections: 3,
            min_connections: 1,
            ..Default::default()
        };
        let dal = DataAccessLayer::new(config);

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Execute multiple queries to test connection pooling
        let mut handles = vec![];
        for i in 0..5 {
            let dal_clone = dal.clone();
            let handle = tokio::spawn(async move {
                dal_clone.execute_query(
                    &format!("SELECT * FROM test WHERE id = {}", i),
                    &[],
                ).await
            });
            handles.push(handle);
        }

        // Wait for all queries to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        let stats = dal.get_performance_stats().await;
        assert_eq!(stats.total_queries, 5);
        assert!(stats.total_connections_created >= 1);
    }

    #[tokio::test]
    async fn test_query_cache_expiration() {
        let config = DataAccessConfig {
            enable_query_cache: true,
            query_cache_ttl: Duration::from_millis(100), // Very short TTL
            ..Default::default()
        };
        let dal = DataAccessLayer::new(config);

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(50)).await;

        let sql = "SELECT * FROM cache_test";
        let params = &[];

        // First query
        let result1 = dal.execute_query(sql, params).await.unwrap();
        assert!(!result1.from_cache);

        // Wait for cache to expire
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Second query should miss cache due to expiration
        let result2 = dal.execute_query(sql, params).await.unwrap();
        assert!(!result2.from_cache);
    }

    #[tokio::test]
    async fn test_connection_pool_exhaustion() {
        let config = DataAccessConfig {
            max_connections: 1,
            min_connections: 0,
            connection_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let dal = DataAccessLayer::new(config);

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Start a long-running query to exhaust the pool
        let dal_clone = dal.clone();
        let _handle = tokio::spawn(async move {
            // This will hold the only connection
            tokio::time::sleep(Duration::from_millis(200)).await;
            dal_clone.execute_query("SELECT SLEEP(1)", &[]).await
        });

        // Wait a bit for the connection to be taken
        tokio::time::sleep(Duration::from_millis(50)).await;

        // This should fail due to pool exhaustion
        let result = timeout(
            Duration::from_millis(50),
            dal.execute_query("SELECT 1", &[])
        ).await;

        // Either timeout or connection pool exhausted error
        assert!(result.is_err() || result.unwrap().is_err());
    }

    #[tokio::test]
    async fn test_performance_statistics() {
        let config = DataAccessConfig {
            enable_query_cache: true,
            ..Default::default()
        };
        let dal = DataAccessLayer::new(config);

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Execute some queries
        for i in 0..3 {
            let _ = dal.execute_query(
                &format!("SELECT {} as value", i),
                &[],
            ).await;
        }

        // Begin and commit a transaction
        let tx_id = dal.begin_transaction().await.unwrap();
        let _ = dal.commit_transaction(tx_id).await;

        let stats = dal.get_performance_stats().await;
        assert_eq!(stats.total_queries, 3);
        assert_eq!(stats.successful_queries, 3);
        assert_eq!(stats.committed_transactions, 1);
        assert!(stats.average_query_time > Duration::ZERO);
        assert!(stats.total_connections_created > 0);
    }

    #[tokio::test]
    async fn test_cacheable_query_detection() {
        let config = DataAccessConfig::default();
        let dal = DataAccessLayer::new(config);

        // Test cacheable queries
        assert!(dal.is_cacheable_query("SELECT * FROM users"));
        assert!(dal.is_cacheable_query("select name from products"));

        // Test non-cacheable queries
        assert!(!dal.is_cacheable_query("INSERT INTO users VALUES (1, 'test')"));
        assert!(!dal.is_cacheable_query("UPDATE users SET name = 'test'"));
        assert!(!dal.is_cacheable_query("DELETE FROM users"));
        assert!(!dal.is_cacheable_query("SELECT NOW()"));
        assert!(!dal.is_cacheable_query("SELECT RANDOM()"));
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let config = DataAccessConfig::default();
        let dal = DataAccessLayer::new(config);

        let sql = "SELECT * FROM users WHERE id = ? AND name = ?";
        let params = &["1", "test"];

        let key1 = dal.generate_cache_key(sql, params);
        let key2 = dal.generate_cache_key(sql, params);

        // Same parameters should generate same key
        assert_eq!(key1, key2);

        // Different parameters should generate different key
        let key3 = dal.generate_cache_key(sql, &["2", "test"]);
        assert_ne!(key1, key3);
    }
}
