/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Encryption support for backups

use serde::{Deserialize, Serialize};

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub default_type: EncryptionType,
    pub key_derivation: KeyDerivationConfig,
}

/// Supported encryption types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionType {
    None,
    Aes256Gcm,
    ChaCha20Poly1305,
}

/// Key derivation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDerivationConfig {
    pub algorithm: KeyDerivationAlgorithm,
    pub iterations: u32,
    pub memory_cost: u32,
    pub parallelism: u32,
}

/// Key derivation algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyDerivationAlgorithm {
    Argon2id,
    Pbkdf2,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            default_type: EncryptionType::Aes256Gcm,
            key_derivation: KeyDerivationConfig::default(),
        }
    }
}

impl Default for KeyDerivationConfig {
    fn default() -> Self {
        Self {
            algorithm: KeyDerivationAlgorithm::Argon2id,
            iterations: 100_000,
            memory_cost: 65536, // 64 MB
            parallelism: 4,
        }
    }
}
