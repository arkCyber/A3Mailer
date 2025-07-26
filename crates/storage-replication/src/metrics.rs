//! Metrics collection for replication

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

/// Replication metrics collector
#[derive(Debug)]
pub struct ReplicationMetrics {
    operations_replicated: AtomicU64,
    operations_failed: AtomicU64,
    conflicts_resolved: AtomicU64,
    bytes_replicated: AtomicU64,
    replication_lag_ms: AtomicU64,
    nodes_healthy: AtomicU64,
    nodes_total: AtomicU64,
}

impl ReplicationMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            operations_replicated: AtomicU64::new(0),
            operations_failed: AtomicU64::new(0),
            conflicts_resolved: AtomicU64::new(0),
            bytes_replicated: AtomicU64::new(0),
            replication_lag_ms: AtomicU64::new(0),
            nodes_healthy: AtomicU64::new(0),
            nodes_total: AtomicU64::new(0),
        }
    }

    /// Increment operations replicated counter
    pub fn increment_operations_replicated(&self) {
        self.operations_replicated.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment operations failed counter
    pub fn increment_operations_failed(&self) {
        self.operations_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment conflicts resolved counter
    pub fn increment_conflicts_resolved(&self) {
        self.conflicts_resolved.fetch_add(1, Ordering::Relaxed);
    }

    /// Add bytes replicated
    pub fn add_bytes_replicated(&self, bytes: u64) {
        self.bytes_replicated.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Update replication lag
    pub fn update_replication_lag(&self, lag_ms: u64) {
        self.replication_lag_ms.store(lag_ms, Ordering::Relaxed);
    }

    /// Update node counts
    pub fn update_node_counts(&self, healthy: u64, total: u64) {
        self.nodes_healthy.store(healthy, Ordering::Relaxed);
        self.nodes_total.store(total, Ordering::Relaxed);
    }

    /// Get current metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            operations_replicated: self.operations_replicated.load(Ordering::Relaxed),
            operations_failed: self.operations_failed.load(Ordering::Relaxed),
            conflicts_resolved: self.conflicts_resolved.load(Ordering::Relaxed),
            bytes_replicated: self.bytes_replicated.load(Ordering::Relaxed),
            replication_lag_ms: self.replication_lag_ms.load(Ordering::Relaxed),
            nodes_healthy: self.nodes_healthy.load(Ordering::Relaxed),
            nodes_total: self.nodes_total.load(Ordering::Relaxed),
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.operations_replicated.store(0, Ordering::Relaxed);
        self.operations_failed.store(0, Ordering::Relaxed);
        self.conflicts_resolved.store(0, Ordering::Relaxed);
        self.bytes_replicated.store(0, Ordering::Relaxed);
        self.replication_lag_ms.store(0, Ordering::Relaxed);
        self.nodes_healthy.store(0, Ordering::Relaxed);
        self.nodes_total.store(0, Ordering::Relaxed);
    }
}

impl Default for ReplicationMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of replication metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub operations_replicated: u64,
    pub operations_failed: u64,
    pub conflicts_resolved: u64,
    pub bytes_replicated: u64,
    pub replication_lag_ms: u64,
    pub nodes_healthy: u64,
    pub nodes_total: u64,
}

impl MetricsSnapshot {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        let total = self.operations_replicated + self.operations_failed;
        if total == 0 {
            1.0
        } else {
            self.operations_replicated as f64 / total as f64
        }
    }

    /// Calculate node health percentage
    pub fn node_health_percentage(&self) -> f64 {
        if self.nodes_total == 0 {
            0.0
        } else {
            self.nodes_healthy as f64 / self.nodes_total as f64 * 100.0
        }
    }

    /// Check if replication is healthy
    pub fn is_healthy(&self) -> bool {
        self.success_rate() > 0.95 && 
        self.replication_lag_ms < 1000 && 
        self.node_health_percentage() > 50.0
    }
}

/// Global metrics instance
pub type SharedMetrics = Arc<ReplicationMetrics>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_operations() {
        let metrics = ReplicationMetrics::new();
        
        metrics.increment_operations_replicated();
        metrics.increment_operations_failed();
        metrics.add_bytes_replicated(1024);
        
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.operations_replicated, 1);
        assert_eq!(snapshot.operations_failed, 1);
        assert_eq!(snapshot.bytes_replicated, 1024);
    }

    #[test]
    fn test_success_rate() {
        let snapshot = MetricsSnapshot {
            operations_replicated: 95,
            operations_failed: 5,
            conflicts_resolved: 0,
            bytes_replicated: 0,
            replication_lag_ms: 0,
            nodes_healthy: 0,
            nodes_total: 0,
        };
        
        assert_eq!(snapshot.success_rate(), 0.95);
    }

    #[test]
    fn test_health_check() {
        let healthy_snapshot = MetricsSnapshot {
            operations_replicated: 100,
            operations_failed: 1,
            conflicts_resolved: 0,
            bytes_replicated: 0,
            replication_lag_ms: 500,
            nodes_healthy: 3,
            nodes_total: 3,
        };
        
        assert!(healthy_snapshot.is_healthy());
    }
}
