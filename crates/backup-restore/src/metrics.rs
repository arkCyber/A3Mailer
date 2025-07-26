/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Metrics collection for backup and restore operations

use crate::{backup::BackupResult, restore::RestoreResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Backup and restore metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetrics {
    pub backup_stats: BackupStats,
    pub restore_stats: RestoreStats,
    pub storage_stats: StorageStats,
    pub performance_stats: PerformanceStats,
}

/// Backup operation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStats {
    pub total_backups: u64,
    pub successful_backups: u64,
    pub failed_backups: u64,
    pub total_bytes_backed_up: u64,
    pub total_files_backed_up: u64,
    pub average_backup_duration: f64,
    pub last_backup_time: Option<DateTime<Utc>>,
    pub backup_types: HashMap<String, u64>,
}

/// Restore operation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreStats {
    pub total_restores: u64,
    pub successful_restores: u64,
    pub failed_restores: u64,
    pub total_bytes_restored: u64,
    pub total_files_restored: u64,
    pub average_restore_duration: f64,
    pub last_restore_time: Option<DateTime<Utc>>,
}

/// Storage utilization statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_storage_used: u64,
    pub available_storage: Option<u64>,
    pub compression_ratio: f64,
    pub deduplication_ratio: f64,
    pub oldest_backup_age_days: Option<u32>,
}

/// Performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub average_throughput_mbps: f64,
    pub peak_throughput_mbps: f64,
    pub average_cpu_usage: f64,
    pub average_memory_usage: u64,
    pub network_utilization: f64,
}

impl BackupMetrics {
    /// Create new metrics instance
    pub fn new() -> Self {
        Self {
            backup_stats: BackupStats::default(),
            restore_stats: RestoreStats::default(),
            storage_stats: StorageStats::default(),
            performance_stats: PerformanceStats::default(),
        }
    }

    /// Record a backup operation
    pub fn record_backup(&mut self, result: &BackupResult) {
        self.backup_stats.total_backups += 1;
        
        if result.warnings.is_empty() {
            self.backup_stats.successful_backups += 1;
        } else {
            self.backup_stats.failed_backups += 1;
        }
        
        self.backup_stats.total_bytes_backed_up += result.bytes_processed;
        self.backup_stats.total_files_backed_up += result.files_processed as u64;
        
        // Update average duration
        let total_duration = self.backup_stats.average_backup_duration * (self.backup_stats.total_backups - 1) as f64;
        self.backup_stats.average_backup_duration = (total_duration + result.duration_seconds) / self.backup_stats.total_backups as f64;
        
        self.backup_stats.last_backup_time = Some(result.backup_info.created_at);
        
        // Update backup type counts
        let backup_type = format!("{:?}", result.backup_info.backup_type);
        *self.backup_stats.backup_types.entry(backup_type).or_insert(0) += 1;
    }

    /// Record a restore operation
    pub fn record_restore(&mut self, result: &RestoreResult) {
        self.restore_stats.total_restores += 1;
        
        if result.errors.is_empty() {
            self.restore_stats.successful_restores += 1;
        } else {
            self.restore_stats.failed_restores += 1;
        }
        
        self.restore_stats.total_bytes_restored += result.bytes_restored;
        self.restore_stats.total_files_restored += result.files_restored as u64;
        
        // Update average duration
        let total_duration = self.restore_stats.average_restore_duration * (self.restore_stats.total_restores - 1) as f64;
        self.restore_stats.average_restore_duration = (total_duration + result.duration_seconds) / self.restore_stats.total_restores as f64;
        
        self.restore_stats.last_restore_time = Some(result.restore_point.created_at);
    }

    /// Update storage statistics
    pub fn update_storage_stats(&mut self, used: u64, available: Option<u64>) {
        self.storage_stats.total_storage_used = used;
        self.storage_stats.available_storage = available;
    }

    /// Update performance statistics
    pub fn update_performance_stats(&mut self, throughput: f64, cpu: f64, memory: u64) {
        self.performance_stats.average_throughput_mbps = throughput;
        self.performance_stats.average_cpu_usage = cpu;
        self.performance_stats.average_memory_usage = memory;
        
        if throughput > self.performance_stats.peak_throughput_mbps {
            self.performance_stats.peak_throughput_mbps = throughput;
        }
    }
}

impl Default for BackupStats {
    fn default() -> Self {
        Self {
            total_backups: 0,
            successful_backups: 0,
            failed_backups: 0,
            total_bytes_backed_up: 0,
            total_files_backed_up: 0,
            average_backup_duration: 0.0,
            last_backup_time: None,
            backup_types: HashMap::new(),
        }
    }
}

impl Default for RestoreStats {
    fn default() -> Self {
        Self {
            total_restores: 0,
            successful_restores: 0,
            failed_restores: 0,
            total_bytes_restored: 0,
            total_files_restored: 0,
            average_restore_duration: 0.0,
            last_restore_time: None,
        }
    }
}

impl Default for StorageStats {
    fn default() -> Self {
        Self {
            total_storage_used: 0,
            available_storage: None,
            compression_ratio: 1.0,
            deduplication_ratio: 1.0,
            oldest_backup_age_days: None,
        }
    }
}

impl Default for PerformanceStats {
    fn default() -> Self {
        Self {
            average_throughput_mbps: 0.0,
            peak_throughput_mbps: 0.0,
            average_cpu_usage: 0.0,
            average_memory_usage: 0,
            network_utilization: 0.0,
        }
    }
}
