/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Storage backend abstraction for backups

use crate::{backup::BackupInfo, error::Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

/// Storage backend trait for backup storage
#[async_trait]
pub trait StorageBackend: Send + Sync + std::fmt::Debug {
    /// Store backup data
    async fn store_backup(&self, backup_id: &str, data: &[u8]) -> Result<()>;
    
    /// Load backup data
    async fn load_backup(&self, backup_id: &str) -> Result<Vec<u8>>;
    
    /// Store backup manifest
    async fn store_manifest(&self, backup_id: &str, manifest: &str) -> Result<()>;
    
    /// Load backup manifest
    async fn load_manifest(&self, backup_id: &str) -> Result<String>;
    
    /// List all available backups
    async fn list_backups(&self) -> Result<Vec<BackupInfo>>;
    
    /// Delete a backup
    async fn delete_backup(&self, backup_id: &str) -> Result<()>;
    
    /// Check if backup exists
    async fn backup_exists(&self, backup_id: &str) -> Result<bool>;
    
    /// Get storage statistics
    async fn get_stats(&self) -> Result<StorageStats>;
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub backend_type: StorageBackendType,
    pub local: Option<LocalStorageConfig>,
    pub s3: Option<S3StorageConfig>,
    pub azure: Option<AzureStorageConfig>,
    pub gcs: Option<GcsStorageConfig>,
}

/// Type of storage backend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageBackendType {
    Local,
    S3,
    Azure,
    Gcs,
}

/// Local storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStorageConfig {
    pub path: PathBuf,
    pub create_directories: bool,
}

/// S3 storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3StorageConfig {
    pub bucket: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub endpoint: Option<String>,
}

/// Azure Blob storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureStorageConfig {
    pub account_name: String,
    pub account_key: String,
    pub container_name: String,
}

/// Google Cloud Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcsStorageConfig {
    pub bucket: String,
    pub credentials_path: PathBuf,
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_backups: usize,
    pub total_size_bytes: u64,
    pub available_space_bytes: Option<u64>,
    pub oldest_backup: Option<String>,
    pub newest_backup: Option<String>,
}

/// Create a storage backend from configuration
pub async fn create_backend(config: &StorageConfig) -> Result<Arc<dyn StorageBackend>> {
    match config.backend_type {
        StorageBackendType::Local => {
            let local_config = config.local.as_ref()
                .ok_or_else(|| crate::error::BackupError::ConfigError("Missing local storage config".to_string()))?;
            Ok(Arc::new(LocalStorageBackend::new(local_config).await?))
        }
        StorageBackendType::S3 => {
            #[cfg(feature = "aws-s3")]
            {
                let s3_config = config.s3.as_ref()
                    .ok_or_else(|| crate::error::BackupError::ConfigError("Missing S3 storage config".to_string()))?;
                Ok(Arc::new(S3StorageBackend::new(s3_config).await?))
            }
            #[cfg(not(feature = "aws-s3"))]
            {
                Err(crate::error::BackupError::ConfigError("S3 support not enabled".to_string()))
            }
        }
        StorageBackendType::Azure => {
            #[cfg(feature = "azure-blob")]
            {
                let azure_config = config.azure.as_ref()
                    .ok_or_else(|| crate::error::BackupError::ConfigError("Missing Azure storage config".to_string()))?;
                Ok(Arc::new(AzureStorageBackend::new(azure_config).await?))
            }
            #[cfg(not(feature = "azure-blob"))]
            {
                Err(crate::error::BackupError::ConfigError("Azure support not enabled".to_string()))
            }
        }
        StorageBackendType::Gcs => {
            #[cfg(feature = "google-cloud")]
            {
                let gcs_config = config.gcs.as_ref()
                    .ok_or_else(|| crate::error::BackupError::ConfigError("Missing GCS storage config".to_string()))?;
                Ok(Arc::new(GcsStorageBackend::new(gcs_config).await?))
            }
            #[cfg(not(feature = "google-cloud"))]
            {
                Err(crate::error::BackupError::ConfigError("GCS support not enabled".to_string()))
            }
        }
    }
}

/// Local filesystem storage backend
#[derive(Debug)]
pub struct LocalStorageBackend {
    config: LocalStorageConfig,
}

impl LocalStorageBackend {
    pub async fn new(config: &LocalStorageConfig) -> Result<Self> {
        if config.create_directories {
            tokio::fs::create_dir_all(&config.path).await
                .map_err(|e| crate::error::BackupError::IoError(e))?;
        }
        
        Ok(Self {
            config: config.clone(),
        })
    }
}

#[async_trait]
impl StorageBackend for LocalStorageBackend {
    async fn store_backup(&self, backup_id: &str, data: &[u8]) -> Result<()> {
        let path = self.config.path.join(format!("{}.backup", backup_id));
        tokio::fs::write(path, data).await
            .map_err(|e| crate::error::BackupError::IoError(e))?;
        Ok(())
    }
    
    async fn load_backup(&self, backup_id: &str) -> Result<Vec<u8>> {
        let path = self.config.path.join(format!("{}.backup", backup_id));
        tokio::fs::read(path).await
            .map_err(|e| crate::error::BackupError::IoError(e))
    }
    
    async fn store_manifest(&self, backup_id: &str, manifest: &str) -> Result<()> {
        let path = self.config.path.join(format!("{}.manifest", backup_id));
        tokio::fs::write(path, manifest).await
            .map_err(|e| crate::error::BackupError::IoError(e))?;
        Ok(())
    }
    
    async fn load_manifest(&self, backup_id: &str) -> Result<String> {
        let path = self.config.path.join(format!("{}.manifest", backup_id));
        tokio::fs::read_to_string(path).await
            .map_err(|e| crate::error::BackupError::IoError(e))
    }
    
    async fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        // TODO: Implement backup listing from filesystem
        Ok(Vec::new())
    }
    
    async fn delete_backup(&self, backup_id: &str) -> Result<()> {
        let backup_path = self.config.path.join(format!("{}.backup", backup_id));
        let manifest_path = self.config.path.join(format!("{}.manifest", backup_id));
        
        if backup_path.exists() {
            tokio::fs::remove_file(backup_path).await
                .map_err(|e| crate::error::BackupError::IoError(e))?;
        }
        
        if manifest_path.exists() {
            tokio::fs::remove_file(manifest_path).await
                .map_err(|e| crate::error::BackupError::IoError(e))?;
        }
        
        Ok(())
    }
    
    async fn backup_exists(&self, backup_id: &str) -> Result<bool> {
        let path = self.config.path.join(format!("{}.backup", backup_id));
        Ok(path.exists())
    }
    
    async fn get_stats(&self) -> Result<StorageStats> {
        // TODO: Implement storage statistics
        Ok(StorageStats {
            total_backups: 0,
            total_size_bytes: 0,
            available_space_bytes: None,
            oldest_backup: None,
            newest_backup: None,
        })
    }
}

// Placeholder implementations for cloud storage backends
#[cfg(feature = "aws-s3")]
#[derive(Debug)]
pub struct S3StorageBackend;

#[cfg(feature = "aws-s3")]
impl S3StorageBackend {
    pub async fn new(_config: &S3StorageConfig) -> Result<Self> {
        // TODO: Implement S3 backend
        Ok(Self)
    }
}

#[cfg(feature = "azure-blob")]
#[derive(Debug)]
pub struct AzureStorageBackend;

#[cfg(feature = "azure-blob")]
impl AzureStorageBackend {
    pub async fn new(_config: &AzureStorageConfig) -> Result<Self> {
        // TODO: Implement Azure backend
        Ok(Self)
    }
}

#[cfg(feature = "google-cloud")]
#[derive(Debug)]
pub struct GcsStorageBackend;

#[cfg(feature = "google-cloud")]
impl GcsStorageBackend {
    pub async fn new(_config: &GcsStorageConfig) -> Result<Self> {
        // TODO: Implement GCS backend
        Ok(Self)
    }
}
