/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! # Backup and Restore System
//!
//! This crate provides comprehensive backup and disaster recovery capabilities
//! for Stalwart Mail Server, including:
//!
//! - Incremental and full backups
//! - Multiple storage backends (local, S3, Azure, GCS)
//! - Compression and encryption
//! - Automated scheduling
//! - Point-in-time recovery
//! - Disaster recovery orchestration

pub mod backup;
pub mod restore;
pub mod storage;
pub mod compression;
pub mod encryption;
pub mod scheduler;
pub mod config;
pub mod error;
pub mod metrics;

pub use backup::{BackupManager, BackupOptions, BackupType};
pub use restore::{RestoreManager, RestoreOptions, RestorePoint};
pub use storage::{StorageBackend, StorageConfig};
pub use compression::{CompressionType, CompressionConfig};
pub use encryption::{EncryptionConfig, EncryptionType};
pub use scheduler::{BackupScheduler, ScheduleConfig};
pub use config::BackupRestoreConfig;
pub use error::{BackupError, RestoreError, Result};

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// Main backup and restore service
#[derive(Debug, Clone)]
pub struct BackupRestoreService {
    inner: Arc<BackupRestoreServiceInner>,
}

#[derive(Debug)]
struct BackupRestoreServiceInner {
    config: BackupRestoreConfig,
    backup_manager: BackupManager,
    restore_manager: RestoreManager,
    scheduler: BackupScheduler,
    metrics: Arc<RwLock<metrics::BackupMetrics>>,
}

impl BackupRestoreService {
    /// Create a new backup and restore service
    pub async fn new(config: BackupRestoreConfig) -> Result<Self> {
        info!("Initializing backup and restore service");

        let storage_backend = storage::create_backend(&config.storage).await?;
        let backup_manager = BackupManager::new(storage_backend.clone(), &config.backup).await?;
        let restore_manager = RestoreManager::new(storage_backend, &config.restore).await?;
        let scheduler = BackupScheduler::new(&config.schedule).await?;
        let metrics = Arc::new(RwLock::new(metrics::BackupMetrics::new()));

        Ok(Self {
            inner: Arc::new(BackupRestoreServiceInner {
                config,
                backup_manager,
                restore_manager,
                scheduler,
                metrics,
            }),
        })
    }

    /// Start the backup and restore service
    pub async fn start(&self) -> Result<()> {
        info!("Starting backup and restore service");

        // Start the scheduler
        self.inner.scheduler.start().await?;

        // Register metrics
        self.register_metrics().await;

        info!("Backup and restore service started successfully");
        Ok(())
    }

    /// Stop the backup and restore service
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping backup and restore service");

        // Stop the scheduler
        self.inner.scheduler.stop().await?;

        info!("Backup and restore service stopped");
        Ok(())
    }

    /// Perform a manual backup
    pub async fn backup(&self, options: BackupOptions) -> Result<backup::BackupResult> {
        info!("Starting manual backup with options: {:?}", options);

        let result = self.inner.backup_manager.backup(options).await?;

        // Update metrics
        {
            let mut metrics = self.inner.metrics.write().await;
            metrics.record_backup(&result);
        }

        info!("Manual backup completed successfully");
        Ok(result)
    }

    /// Perform a restore operation
    pub async fn restore(&self, options: RestoreOptions) -> Result<restore::RestoreResult> {
        warn!("Starting restore operation with options: {:?}", options);

        let result = self.inner.restore_manager.restore(options).await?;

        // Update metrics
        {
            let mut metrics = self.inner.metrics.write().await;
            metrics.record_restore(&result);
        }

        warn!("Restore operation completed");
        Ok(result)
    }

    /// List available backups
    pub async fn list_backups(&self) -> Result<Vec<backup::BackupInfo>> {
        self.inner.backup_manager.list_backups().await
    }

    /// Get backup metrics
    pub async fn get_metrics(&self) -> metrics::BackupMetrics {
        self.inner.metrics.read().await.clone()
    }

    /// Validate backup integrity
    pub async fn validate_backup(&self, backup_id: &str) -> Result<bool> {
        self.inner.backup_manager.validate_backup(backup_id).await
    }

    /// Clean up old backups according to retention policy
    pub async fn cleanup_old_backups(&self) -> Result<usize> {
        self.inner.backup_manager.cleanup_old_backups().await
    }

    /// Register metrics with the metrics system
    async fn register_metrics(&self) {
        // TODO: Register Prometheus metrics
        info!("Backup and restore metrics registered");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_service_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = BackupRestoreConfig::default_with_path(temp_dir.path());
        
        let service = BackupRestoreService::new(config).await;
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_service_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let config = BackupRestoreConfig::default_with_path(temp_dir.path());
        
        let service = BackupRestoreService::new(config).await.unwrap();
        
        // Start service
        assert!(service.start().await.is_ok());
        
        // Stop service
        assert!(service.stop().await.is_ok());
    }
}
