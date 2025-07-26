//! Connection Pool Manager for A3Mailer
//!
//! This module provides high-performance connection pooling for database,
//! Redis, and HTTP connections with automatic lifecycle management.

use crate::{PoolConfig, Result, PerformanceError};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};

/// Connection pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    pub active_connections: u32,
    pub idle_connections: u32,
    pub total_connections: u32,
    pub max_connections: u32,
    pub utilization_percent: f64,
    pub connection_errors: u64,
    pub connection_timeouts: u64,
    pub average_wait_time_ms: f64,
}

/// Pooled connection wrapper
#[derive(Debug)]
pub struct PooledConnection {
    pub connection_id: String,
    pub created_at: Instant,
    pub last_used: Instant,
    pub use_count: u64,
    pub is_healthy: bool,
}

impl PooledConnection {
    /// Create a new pooled connection
    pub fn new(connection_id: String) -> Self {
        let now = Instant::now();
        Self {
            connection_id,
            created_at: now,
            last_used: now,
            use_count: 0,
            is_healthy: true,
        }
    }

    /// Mark connection as used
    pub fn mark_used(&mut self) {
        self.last_used = Instant::now();
        self.use_count += 1;
    }

    /// Check if connection is expired
    pub fn is_expired(&self, max_lifetime: Duration) -> bool {
        self.created_at.elapsed() > max_lifetime
    }

    /// Check if connection is idle too long
    pub fn is_idle_expired(&self, idle_timeout: Duration) -> bool {
        self.last_used.elapsed() > idle_timeout
    }
}

/// Database connection pool
#[derive(Debug)]
pub struct DatabasePool {
    config: PoolConfig,
    connections: Arc<RwLock<Vec<PooledConnection>>>,
    semaphore: Arc<Semaphore>,
    stats: Arc<RwLock<PoolStats>>,
    connection_string: String,
}

impl DatabasePool {
    /// Create a new database pool
    pub async fn new(config: &PoolConfig, connection_string: String) -> Result<Self> {
        info!("Initializing database connection pool");

        let pool = Self {
            config: config.clone(),
            connections: Arc::new(RwLock::new(Vec::new())),
            semaphore: Arc::new(Semaphore::new(config.database_pool_size as usize)),
            stats: Arc::new(RwLock::new(PoolStats {
                active_connections: 0,
                idle_connections: 0,
                total_connections: 0,
                max_connections: config.database_pool_size,
                utilization_percent: 0.0,
                connection_errors: 0,
                connection_timeouts: 0,
                average_wait_time_ms: 0.0,
            })),
            connection_string,
        };

        // Pre-populate pool with initial connections
        pool.initialize_connections().await?;

        info!("Database connection pool initialized with {} connections", config.database_pool_size);
        Ok(pool)
    }

    /// Initialize pool with connections
    async fn initialize_connections(&self) -> Result<()> {
        let initial_size = (self.config.database_pool_size / 2).max(1);
        
        for i in 0..initial_size {
            match self.create_connection().await {
                Ok(conn) => {
                    let mut connections = self.connections.write().await;
                    connections.push(conn);
                }
                Err(e) => {
                    warn!("Failed to create initial database connection {}: {}", i, e);
                }
            }
        }

        self.update_stats().await;
        Ok(())
    }

    /// Create a new database connection
    async fn create_connection(&self) -> Result<PooledConnection> {
        debug!("Creating new database connection");
        
        // Simulate database connection creation
        // In a real implementation, this would create an actual database connection
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        let connection_id = format!("db_conn_{}", uuid::Uuid::new_v4());
        Ok(PooledConnection::new(connection_id))
    }

    /// Get a connection from the pool
    pub async fn get_connection(&self) -> Result<PooledConnection> {
        let start_time = Instant::now();
        
        // Wait for available slot
        let _permit = self.semaphore.acquire().await
            .map_err(|e| PerformanceError::PoolError(format!("Failed to acquire semaphore: {}", e)))?;

        // Try to get an existing idle connection
        {
            let mut connections = self.connections.write().await;
            if let Some(mut conn) = connections.pop() {
                if conn.is_healthy && !conn.is_expired(Duration::from_secs(self.config.max_lifetime_seconds)) {
                    conn.mark_used();
                    self.update_wait_time(start_time.elapsed()).await;
                    return Ok(conn);
                }
            }
        }

        // Create new connection if none available
        match self.create_connection().await {
            Ok(mut conn) => {
                conn.mark_used();
                self.update_wait_time(start_time.elapsed()).await;
                Ok(conn)
            }
            Err(e) => {
                self.increment_error_count().await;
                Err(e)
            }
        }
    }

    /// Return a connection to the pool
    pub async fn return_connection(&self, mut connection: PooledConnection) {
        if connection.is_healthy && 
           !connection.is_expired(Duration::from_secs(self.config.max_lifetime_seconds)) &&
           !connection.is_idle_expired(Duration::from_secs(self.config.idle_timeout_seconds)) {
            
            let mut connections = self.connections.write().await;
            if connections.len() < self.config.database_pool_size as usize {
                connections.push(connection);
            }
        }
        
        self.update_stats().await;
    }

    /// Update pool statistics
    async fn update_stats(&self) {
        let connections = self.connections.read().await;
        let mut stats = self.stats.write().await;
        
        stats.idle_connections = connections.len() as u32;
        stats.total_connections = connections.len() as u32;
        stats.active_connections = stats.max_connections - stats.idle_connections;
        stats.utilization_percent = (stats.active_connections as f64 / stats.max_connections as f64) * 100.0;
    }

    /// Update average wait time
    async fn update_wait_time(&self, wait_time: Duration) {
        let mut stats = self.stats.write().await;
        let wait_time_ms = wait_time.as_millis() as f64;
        
        // Simple moving average
        if stats.average_wait_time_ms == 0.0 {
            stats.average_wait_time_ms = wait_time_ms;
        } else {
            stats.average_wait_time_ms = (stats.average_wait_time_ms * 0.9) + (wait_time_ms * 0.1);
        }
    }

    /// Increment error count
    async fn increment_error_count(&self) {
        let mut stats = self.stats.write().await;
        stats.connection_errors += 1;
    }

    /// Get pool statistics
    pub async fn get_stats(&self) -> PoolStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Cleanup expired connections
    pub async fn cleanup_expired(&self) -> Result<()> {
        debug!("Cleaning up expired database connections");
        
        let mut connections = self.connections.write().await;
        let max_lifetime = Duration::from_secs(self.config.max_lifetime_seconds);
        let idle_timeout = Duration::from_secs(self.config.idle_timeout_seconds);
        
        connections.retain(|conn| {
            conn.is_healthy && 
            !conn.is_expired(max_lifetime) && 
            !conn.is_idle_expired(idle_timeout)
        });
        
        self.update_stats().await;
        Ok(())
    }
}

/// HTTP client pool
#[derive(Debug)]
pub struct HttpClientPool {
    config: PoolConfig,
    clients: Arc<RwLock<Vec<reqwest::Client>>>,
    stats: Arc<RwLock<PoolStats>>,
}

impl HttpClientPool {
    /// Create a new HTTP client pool
    pub async fn new(config: &PoolConfig) -> Result<Self> {
        info!("Initializing HTTP client pool");

        let pool = Self {
            config: config.clone(),
            clients: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(PoolStats {
                active_connections: 0,
                idle_connections: 0,
                total_connections: 0,
                max_connections: config.http_pool_size,
                utilization_percent: 0.0,
                connection_errors: 0,
                connection_timeouts: 0,
                average_wait_time_ms: 0.0,
            })),
        };

        // Pre-create HTTP clients
        pool.initialize_clients().await?;

        info!("HTTP client pool initialized with {} clients", config.http_pool_size);
        Ok(pool)
    }

    /// Initialize pool with HTTP clients
    async fn initialize_clients(&self) -> Result<()> {
        let mut clients = self.clients.write().await;
        
        for _ in 0..self.config.http_pool_size {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(self.config.connection_timeout_seconds))
                .pool_max_idle_per_host(10)
                .pool_idle_timeout(Duration::from_secs(self.config.idle_timeout_seconds))
                .build()
                .map_err(|e| PerformanceError::PoolError(format!("Failed to create HTTP client: {}", e)))?;
            
            clients.push(client);
        }

        Ok(())
    }

    /// Get an HTTP client from the pool
    pub async fn get_client(&self) -> Result<reqwest::Client> {
        let clients = self.clients.read().await;
        
        if let Some(client) = clients.first() {
            Ok(client.clone())
        } else {
            // Create a new client if pool is empty
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(self.config.connection_timeout_seconds))
                .build()
                .map_err(|e| PerformanceError::PoolError(format!("Failed to create HTTP client: {}", e)))?;
            
            Ok(client)
        }
    }

    /// Get pool statistics
    pub async fn get_stats(&self) -> PoolStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
}

/// Main pool manager
pub struct PoolManager {
    config: PoolConfig,
    database_pool: Arc<DatabasePool>,
    http_pool: Arc<HttpClientPool>,
    redis_pool: Option<Arc<DatabasePool>>,
}

impl PoolManager {
    /// Create a new pool manager
    pub async fn new(config: &PoolConfig) -> Result<Self> {
        info!("Initializing pool manager");

        // Initialize database pool
        let database_pool = Arc::new(
            DatabasePool::new(config, "postgresql://localhost/a3mailer".to_string()).await?
        );

        // Initialize HTTP client pool
        let http_pool = Arc::new(HttpClientPool::new(config).await?);

        // Initialize Redis pool if needed
        let redis_pool = Some(Arc::new(
            DatabasePool::new(config, "redis://localhost:6379".to_string()).await?
        ));

        info!("Pool manager initialized successfully");
        Ok(Self {
            config: config.clone(),
            database_pool,
            http_pool,
            redis_pool,
        })
    }

    /// Get a database connection
    pub async fn get_database_connection(&self) -> Result<PooledConnection> {
        self.database_pool.get_connection().await
    }

    /// Return a database connection
    pub async fn return_database_connection(&self, connection: PooledConnection) {
        self.database_pool.return_connection(connection).await;
    }

    /// Get an HTTP client
    pub async fn get_http_client(&self) -> Result<reqwest::Client> {
        self.http_pool.get_client().await
    }

    /// Get a Redis connection
    pub async fn get_redis_connection(&self) -> Result<PooledConnection> {
        if let Some(redis_pool) = &self.redis_pool {
            redis_pool.get_connection().await
        } else {
            Err(PerformanceError::PoolError("Redis pool not configured".to_string()))
        }
    }

    /// Get combined pool statistics
    pub async fn get_stats(&self) -> Result<PoolStats> {
        let db_stats = self.database_pool.get_stats().await;
        let http_stats = self.http_pool.get_stats().await;
        
        // Combine statistics
        Ok(PoolStats {
            active_connections: db_stats.active_connections + http_stats.active_connections,
            idle_connections: db_stats.idle_connections + http_stats.idle_connections,
            total_connections: db_stats.total_connections + http_stats.total_connections,
            max_connections: db_stats.max_connections + http_stats.max_connections,
            utilization_percent: (db_stats.utilization_percent + http_stats.utilization_percent) / 2.0,
            connection_errors: db_stats.connection_errors + http_stats.connection_errors,
            connection_timeouts: db_stats.connection_timeouts + http_stats.connection_timeouts,
            average_wait_time_ms: (db_stats.average_wait_time_ms + http_stats.average_wait_time_ms) / 2.0,
        })
    }

    /// Optimize pool performance
    pub async fn optimize(&self) -> Result<()> {
        debug!("Optimizing connection pools");
        
        // Cleanup expired connections
        self.database_pool.cleanup_expired().await?;
        
        if let Some(redis_pool) = &self.redis_pool {
            redis_pool.cleanup_expired().await?;
        }
        
        let stats = self.get_stats().await?;
        
        // Log optimization recommendations
        if stats.utilization_percent > 90.0 {
            warn!("High pool utilization: {:.1}%. Consider increasing pool size.", stats.utilization_percent);
        }
        
        if stats.average_wait_time_ms > 100.0 {
            warn!("High average wait time: {:.1}ms. Consider optimizing connection creation.", stats.average_wait_time_ms);
        }
        
        Ok(())
    }

    /// Shutdown pool manager
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down pool manager");
        
        // Cleanup would go here in a real implementation
        
        info!("Pool manager shutdown complete");
        Ok(())
    }
}
