/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Compression support for backups

use serde::{Deserialize, Serialize};

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub default_type: CompressionType,
    pub level: CompressionLevel,
    pub enable_parallel: bool,
}

/// Supported compression types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Gzip,
    Zstd,
    Lz4,
}

/// Compression level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionLevel {
    Fast,
    Balanced,
    Best,
    Custom(i32),
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            default_type: CompressionType::Zstd,
            level: CompressionLevel::Balanced,
            enable_parallel: true,
        }
    }
}

impl CompressionType {
    pub fn extension(&self) -> &'static str {
        match self {
            CompressionType::None => "",
            CompressionType::Gzip => ".gz",
            CompressionType::Zstd => ".zst",
            CompressionType::Lz4 => ".lz4",
        }
    }
}

impl CompressionLevel {
    pub fn to_level(&self, compression_type: CompressionType) -> i32 {
        match (self, compression_type) {
            (CompressionLevel::Fast, CompressionType::Gzip) => 1,
            (CompressionLevel::Balanced, CompressionType::Gzip) => 6,
            (CompressionLevel::Best, CompressionType::Gzip) => 9,
            (CompressionLevel::Fast, CompressionType::Zstd) => 1,
            (CompressionLevel::Balanced, CompressionType::Zstd) => 3,
            (CompressionLevel::Best, CompressionType::Zstd) => 19,
            (CompressionLevel::Fast, CompressionType::Lz4) => 1,
            (CompressionLevel::Balanced, CompressionType::Lz4) => 4,
            (CompressionLevel::Best, CompressionType::Lz4) => 9,
            (CompressionLevel::Custom(level), _) => *level,
            (_, CompressionType::None) => 0,
        }
    }
}
