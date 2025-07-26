/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Configuration for backup and restore system

use crate::{
    backup::BackupConfig,
    restore::RestoreConfig,
    storage::StorageConfig,
    scheduler::ScheduleConfig,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main configuration for backup and restore system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRestoreConfig {
    pub backup: BackupConfig,
    pub restore: RestoreConfig,
    pub storage: StorageConfig,
    pub schedule: ScheduleConfig,
}

impl BackupRestoreConfig {
    /// Create a default configuration with local storage at the specified path
    pub fn default_with_path(path: &Path) -> Self {
        Self {
            backup: BackupConfig::default(),
            restore: RestoreConfig::default(),
            storage: StorageConfig {
                backend_type: crate::storage::StorageBackendType::Local,
                local: Some(crate::storage::LocalStorageConfig {
                    path: path.to_path_buf(),
                    create_directories: true,
                }),
                s3: None,
                azure: None,
                gcs: None,
            },
            schedule: ScheduleConfig::default(),
        }
    }
}

impl Default for BackupRestoreConfig {
    fn default() -> Self {
        Self {
            backup: BackupConfig::default(),
            restore: RestoreConfig::default(),
            storage: StorageConfig {
                backend_type: crate::storage::StorageBackendType::Local,
                local: Some(crate::storage::LocalStorageConfig {
                    path: "/var/lib/stalwart/backups".into(),
                    create_directories: true,
                }),
                s3: None,
                azure: None,
                gcs: None,
            },
            schedule: ScheduleConfig::default(),
        }
    }
}
