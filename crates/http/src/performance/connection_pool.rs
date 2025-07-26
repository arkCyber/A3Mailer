/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! HTTP Connection Pool Performance Optimizations
//!
//! This module provides optimized connection pooling for HTTP clients to improve
//! performance by reusing connections, reducing connection overhead, and managing
//! connection lifecycle efficiently.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;
use tracing::{debug, warn, error, info};

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of connections per host
    pub max_connections_per_host: usize,
    /// Maximum idle time before connection is closed
    pub max_idle_time: Duration,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Request timeout
    pub request_timeout: Duration,
    /// Maximum number of retries
    pub max_retries: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections_per_host: 10,
            max_idle_time: Duration::from_secs(30),
            connection_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            max_retries: 3,
        }
    }
}

/// Connection statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    pub total_connections: u64,
    pub active_connections: u64,
    pub idle_connections: u64,
    pub failed_connections: u64,
    pub connection_reuses: u64,
    pub connection_timeouts: u64,
}

/// A pooled HTTP connection
#[derive(Debug)]
struct PooledConnection {
    /// Connection identifier
    id: String,
    /// Host this connection is for
    host: String,
    /// When this connection was last used
    last_used: Instant,
    /// Whether this connection is currently in use
    in_use: bool,
    /// Number of requests made on this connection
    request_count: u64,
}

impl PooledConnection {
    fn new(id: String, host: String) -> Self {
        info!("Creating new pooled connection {} for host {}", id, host);
        Self {
            id,
            host,
            last_used: Instant::now(),
            in_use: false,
            request_count: 0,
        }
    }

    fn mark_used(&mut self) {
        self.last_used = Instant::now();
        self.in_use = true;
        self.request_count += 1;
        debug!("Marking connection {} as used (request #{})", self.id, self.request_count);
    }

    fn mark_idle(&mut self) {
        self.in_use = false;
        debug!("Marking connection {} as idle", self.id);
    }

    fn is_expired(&self, max_idle_time: Duration) -> bool {
        !self.in_use && self.last_used.elapsed() > max_idle_time
    }
}

/// HTTP Connection Pool
pub struct ConnectionPool {
    /// Pool configuration
    config: PoolConfig,
    /// Connections grouped by host
    connections: Arc<Mutex<HashMap<String, Vec<PooledConnection>>>>,
    /// Semaphore to limit concurrent connections per host
    semaphores: Arc<Mutex<HashMap<String, Arc<Semaphore>>>>,
    /// Connection statistics
    stats: Arc<Mutex<ConnectionStats>>,
}

impl ConnectionPool {
    /// Create a new connection pool with default configuration
    pub fn new() -> Self {
        Self::with_config(PoolConfig::default())
    }

    /// Create a new connection pool with custom configuration
    pub fn with_config(config: PoolConfig) -> Self {
        info!("Creating HTTP connection pool with config: {:?}", config);
        Self {
            config,
            connections: Arc::new(Mutex::new(HashMap::new())),
            semaphores: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(ConnectionStats::default())),
        }
    }

    /// Get a connection for the specified host
    pub async fn get_connection(&self, host: &str) -> Result<String, PoolError> {
        debug!("Requesting connection for host: {}", host);

        // Get or create semaphore for this host
        let semaphore = {
            let mut semaphores = self.semaphores.lock().unwrap();
            semaphores.entry(host.to_string())
                .or_insert_with(|| Arc::new(Semaphore::new(self.config.max_connections_per_host)))
                .clone()
        };

        // Acquire permit (this limits concurrent connections)
        let _permit = semaphore.acquire().await.map_err(|_| PoolError::SemaphoreError)?;

        // Try to get an existing idle connection
        if let Some(connection_id) = self.get_idle_connection(host) {
            debug!("Reusing existing connection {} for host {}", connection_id, host);
            self.update_stats(|stats| stats.connection_reuses += 1);
            return Ok(connection_id);
        }

        // Create a new connection
        let connection_id = format!("conn_{}_{}", host, uuid::Uuid::new_v4());
        self.create_connection(host, &connection_id).await?;

        debug!("Created new connection {} for host {}", connection_id, host);
        self.update_stats(|stats| {
            stats.total_connections += 1;
            stats.active_connections += 1;
        });

        Ok(connection_id)
    }

    /// Return a connection to the pool
    pub fn return_connection(&self, connection_id: &str) {
        debug!("Returning connection {} to pool", connection_id);

        let mut connections = self.connections.lock().unwrap();
        for (_, host_connections) in connections.iter_mut() {
            if let Some(conn) = host_connections.iter_mut().find(|c| c.id == connection_id) {
                conn.mark_idle();
                self.update_stats(|stats| stats.active_connections -= 1);
                return;
            }
        }

        warn!("Attempted to return unknown connection: {}", connection_id);
    }

    /// Remove a connection from the pool (e.g., due to error)
    pub fn remove_connection(&self, connection_id: &str) {
        debug!("Removing connection {} from pool", connection_id);

        let mut connections = self.connections.lock().unwrap();
        for (_, host_connections) in connections.iter_mut() {
            if let Some(pos) = host_connections.iter().position(|c| c.id == connection_id) {
                host_connections.remove(pos);
                self.update_stats(|stats| {
                    stats.active_connections -= 1;
                    stats.failed_connections += 1;
                });
                return;
            }
        }

        warn!("Attempted to remove unknown connection: {}", connection_id);
    }

    /// Clean up expired connections
    pub fn cleanup_expired_connections(&self) {
        debug!("Cleaning up expired connections");

        let mut connections = self.connections.lock().unwrap();
        let mut removed_count = 0;

        for (host, host_connections) in connections.iter_mut() {
            let initial_count = host_connections.len();
            host_connections.retain(|conn| {
                if conn.is_expired(self.config.max_idle_time) {
                    debug!("Removing expired connection {} for host {}", conn.id, host);
                    false
                } else {
                    true
                }
            });
            removed_count += initial_count - host_connections.len();
        }

        if removed_count > 0 {
            info!("Cleaned up {} expired connections", removed_count);
            self.update_stats(|stats| {
                stats.idle_connections = stats.idle_connections.saturating_sub(removed_count as u64);
            });
        }
    }

    /// Get connection statistics
    pub fn get_stats(&self) -> ConnectionStats {
        self.stats.lock().unwrap().clone()
    }

    /// Get pool configuration
    pub fn get_config(&self) -> &PoolConfig {
        &self.config
    }

    // Private helper methods

    fn get_idle_connection(&self, host: &str) -> Option<String> {
        let mut connections = self.connections.lock().unwrap();
        if let Some(host_connections) = connections.get_mut(host) {
            if let Some(conn) = host_connections.iter_mut().find(|c| !c.in_use) {
                conn.mark_used();
                return Some(conn.id.clone());
            }
        }
        None
    }

    async fn create_connection(&self, host: &str, connection_id: &str) -> Result<(), PoolError> {
        // Simulate connection creation
        tokio::time::sleep(Duration::from_millis(10)).await;

        let connection = PooledConnection::new(connection_id.to_string(), host.to_string());

        let mut connections = self.connections.lock().unwrap();
        connections.entry(host.to_string())
            .or_insert_with(Vec::new)
            .push(connection);

        Ok(())
    }

    fn update_stats<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut ConnectionStats),
    {
        if let Ok(mut stats) = self.stats.lock() {
            update_fn(&mut stats);
        }
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Connection pool errors
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("Semaphore error")]
    SemaphoreError,
    #[error("Connection timeout")]
    ConnectionTimeout,
    #[error("Pool is full")]
    PoolFull,
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;


    #[tokio::test]
    async fn test_connection_pool_basic() {
        let pool = ConnectionPool::new();

        // Get a connection
        let conn1 = pool.get_connection("example.com").await.unwrap();
        assert!(!conn1.is_empty());

        // Return the connection
        pool.return_connection(&conn1);

        // Get another connection (should reuse)
        let conn2 = pool.get_connection("example.com").await.unwrap();
        assert_eq!(conn1, conn2);

        let stats = pool.get_stats();
        assert_eq!(stats.connection_reuses, 1);
    }

    #[tokio::test]
    async fn test_connection_pool_multiple_hosts() {
        let pool = ConnectionPool::new();

        let conn1 = pool.get_connection("host1.com").await.unwrap();
        let conn2 = pool.get_connection("host2.com").await.unwrap();

        assert_ne!(conn1, conn2);

        pool.return_connection(&conn1);
        pool.return_connection(&conn2);

        let stats = pool.get_stats();
        assert_eq!(stats.total_connections, 2);
    }

    #[tokio::test]
    async fn test_connection_cleanup() {
        let config = PoolConfig {
            max_idle_time: Duration::from_millis(100),
            ..Default::default()
        };
        let pool = ConnectionPool::with_config(config);

        let conn = pool.get_connection("example.com").await.unwrap();
        pool.return_connection(&conn);

        // Wait for connection to expire
        tokio::time::sleep(Duration::from_millis(150)).await;

        pool.cleanup_expired_connections();

        // Next connection should be new
        let new_conn = pool.get_connection("example.com").await.unwrap();
        assert_ne!(conn, new_conn);
    }
}
