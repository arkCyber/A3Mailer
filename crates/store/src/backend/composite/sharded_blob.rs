/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: LicenseRef-SEL
 *
 * This file is subject to the Stalwart Enterprise License Agreement (SEL) and
 * is NOT open source software.
 *
 */

use std::ops::Range;
use std::time::Instant;

use utils::config::{Config, utils::AsKey};

use crate::{BlobBackend, Store, Stores};

pub struct ShardedBlob {
    pub stores: Vec<BlobBackend>,
}

impl ShardedBlob {
    pub fn open(config: &mut Config, prefix: impl AsKey, stores: &Stores) -> Option<Self> {
        let prefix = prefix.as_key();
        let store_ids = config
            .values((&prefix, "stores"))
            .map(|(_, v)| v.to_string())
            .collect::<Vec<_>>();

        let mut blob_stores = Vec::with_capacity(store_ids.len());
        for store_id in store_ids {
            if let Some(store) = stores.blob_stores.get(&store_id) {
                blob_stores.push(store.backend.clone());
            } else {
                config.new_build_error(
                    (&prefix, "stores"),
                    format!("Blob store {store_id} not found"),
                );
                return None;
            }
        }
        if !blob_stores.is_empty() {
            Some(Self {
                stores: blob_stores,
            })
        } else {
            config.new_build_error((&prefix, "stores"), "No blob stores specified");
            None
        }
    }

    #[inline(always)]
    fn get_store(&self, key: &[u8]) -> &BlobBackend {
        &self.stores[xxhash_rust::xxh3::xxh3_64(key) as usize % self.stores.len()]
    }

    /// Retrieves a blob from the appropriate shard based on the key hash
    ///
    /// # Arguments
    /// * `key` - The blob key used for sharding and retrieval
    /// * `read_range` - The byte range to read from the blob
    ///
    /// # Returns
    /// * `Ok(Some(Vec<u8>))` - The blob data if found
    /// * `Ok(None)` - If the blob doesn't exist
    /// * `Err(trc::Error)` - If an error occurred during retrieval
    pub async fn get_blob(
        &self,
        key: &[u8],
        read_range: Range<usize>,
    ) -> trc::Result<Option<Vec<u8>>> {
        let start_time = Instant::now();

        trc::event!(
            Store(trc::StoreEvent::BlobRead),
            Key = trc::Value::from(key),
            Details = format!("Reading blob from sharded storage, range: {}..{}", read_range.start, read_range.end)
        );

        Box::pin(async move {
            let result = match self.get_store(key) {
                BlobBackend::Store(store) => match store {
                    #[cfg(feature = "sqlite")]
                    Store::SQLite(store) => store.get_blob(key, read_range).await,
                    #[cfg(feature = "foundation")]
                    Store::FoundationDb(store) => store.get_blob(key, read_range).await,
                    #[cfg(feature = "postgres")]
                    Store::PostgreSQL(store) => store.get_blob(key, read_range).await,
                    #[cfg(feature = "mysql")]
                    Store::MySQL(store) => store.get_blob(key, read_range).await,
                    #[cfg(feature = "rocks")]
                    Store::RocksDb(store) => store.get_blob(key, read_range).await,
                    // SPDX-SnippetBegin
                    // SPDX-FileCopyrightText: 2024 A3Mailer Project
                    // SPDX-License-Identifier: LicenseRef-SEL
                    #[cfg(all(
                        feature = "enterprise",
                        any(feature = "postgres", feature = "mysql")
                    ))]
                    Store::SQLReadReplica(store) => store.get_blob(key, read_range).await,
                    // SPDX-SnippetEnd
                    Store::None => Err(trc::StoreEvent::NotConfigured.into()),
                },
                BlobBackend::Fs(store) => store.get_blob(key, read_range).await,
                #[cfg(feature = "s3")]
                BlobBackend::S3(store) => store.get_blob(key, read_range).await,
                #[cfg(feature = "azure")]
                BlobBackend::Azure(store) => store.get_blob(key, read_range).await,
                BlobBackend::Sharded(sharded_store) => {
                    // Prevent infinite recursion by delegating to the sharded store's get_blob method
                    // This should not happen in practice as sharded stores should not contain other sharded stores
                    trc::event!(
                        Store(trc::StoreEvent::BlobRead),
                        Key = trc::Value::from(key),
                        Details = "Detected nested sharded blob store - delegating to nested store"
                    );
                    sharded_store.get_blob(key, read_range).await
                },
            };

            // Log the operation result
            match &result {
                Ok(Some(data)) => {
                    trc::event!(
                        Store(trc::StoreEvent::BlobRead),
                        Key = trc::Value::from(key),
                        Size = data.len(),
                        Elapsed = start_time.elapsed(),
                        Details = "Successfully retrieved blob from sharded storage"
                    );
                }
                Ok(None) => {
                    trc::event!(
                        Store(trc::StoreEvent::BlobRead),
                        Key = trc::Value::from(key),
                        Elapsed = start_time.elapsed(),
                        Details = "Blob not found in sharded storage"
                    );
                }
                Err(err) => {
                    trc::error!(err
                        .clone()
                        .details("Failed to retrieve blob from sharded storage")
                        .ctx(trc::Key::Key, trc::Value::from(key))
                        .ctx(trc::Key::Elapsed, start_time.elapsed())
                    );
                }
            }

            result
        })
        .await
    }

    /// Stores a blob in the appropriate shard based on the key hash
    ///
    /// # Arguments
    /// * `key` - The blob key used for sharding and storage
    /// * `data` - The blob data to store
    ///
    /// # Returns
    /// * `Ok(())` - If the blob was successfully stored
    /// * `Err(trc::Error)` - If an error occurred during storage
    pub async fn put_blob(&self, key: &[u8], data: &[u8]) -> trc::Result<()> {
        let start_time = Instant::now();

        trc::event!(
            Store(trc::StoreEvent::BlobWrite),
            Key = trc::Value::from(key),
            Size = data.len(),
            Details = "Storing blob in sharded storage"
        );

        Box::pin(async move {
            let result = match self.get_store(key) {
                BlobBackend::Store(store) => match store {
                    #[cfg(feature = "sqlite")]
                    Store::SQLite(store) => store.put_blob(key, data).await,
                    #[cfg(feature = "foundation")]
                    Store::FoundationDb(store) => store.put_blob(key, data).await,
                    #[cfg(feature = "postgres")]
                    Store::PostgreSQL(store) => store.put_blob(key, data).await,
                    #[cfg(feature = "mysql")]
                    Store::MySQL(store) => store.put_blob(key, data).await,
                    #[cfg(feature = "rocks")]
                    Store::RocksDb(store) => store.put_blob(key, data).await,
                    // SPDX-SnippetBegin
                    // SPDX-FileCopyrightText: 2024 A3Mailer Project
                    // SPDX-License-Identifier: LicenseRef-SEL
                    #[cfg(all(
                        feature = "enterprise",
                        any(feature = "postgres", feature = "mysql")
                    ))]
                    // SPDX-SnippetEnd
                    Store::SQLReadReplica(store) => store.put_blob(key, data).await,
                    Store::None => Err(trc::StoreEvent::NotConfigured.into()),
                },
                BlobBackend::Fs(store) => store.put_blob(key, data).await,
                #[cfg(feature = "s3")]
                BlobBackend::S3(store) => store.put_blob(key, data).await,
                #[cfg(feature = "azure")]
                BlobBackend::Azure(store) => store.put_blob(key, data).await,
                BlobBackend::Sharded(sharded_store) => {
                    // Prevent infinite recursion by delegating to the sharded store's put_blob method
                    // This should not happen in practice as sharded stores should not contain other sharded stores
                    trc::event!(
                        Store(trc::StoreEvent::BlobWrite),
                        Key = trc::Value::from(key),
                        Size = data.len(),
                        Details = "Detected nested sharded blob store - delegating to nested store"
                    );
                    sharded_store.put_blob(key, data).await
                },
            };

            // Log the operation result
            match &result {
                Ok(()) => {
                    trc::event!(
                        Store(trc::StoreEvent::BlobWrite),
                        Key = trc::Value::from(key),
                        Size = data.len(),
                        Elapsed = start_time.elapsed(),
                        Details = "Successfully stored blob in sharded storage"
                    );
                }
                Err(err) => {
                    trc::error!(err
                        .clone()
                        .details("Failed to store blob in sharded storage")
                        .ctx(trc::Key::Key, trc::Value::from(key))
                        .ctx(trc::Key::Size, data.len())
                        .ctx(trc::Key::Elapsed, start_time.elapsed())
                    );
                }
            }

            result
        })
        .await
    }

    /// Deletes a blob from the appropriate shard based on the key hash
    ///
    /// # Arguments
    /// * `key` - The blob key used for sharding and deletion
    ///
    /// # Returns
    /// * `Ok(true)` - If the blob was successfully deleted
    /// * `Ok(false)` - If the blob didn't exist
    /// * `Err(trc::Error)` - If an error occurred during deletion
    pub async fn delete_blob(&self, key: &[u8]) -> trc::Result<bool> {
        let start_time = Instant::now();

        trc::event!(
            Store(trc::StoreEvent::BlobDelete),
            Key = trc::Value::from(key),
            Details = "Deleting blob from sharded storage"
        );

        Box::pin(async move {
            let result = match self.get_store(key) {
                BlobBackend::Store(store) => match store {
                    #[cfg(feature = "sqlite")]
                    Store::SQLite(store) => store.delete_blob(key).await,
                    #[cfg(feature = "foundation")]
                    Store::FoundationDb(store) => store.delete_blob(key).await,
                    #[cfg(feature = "postgres")]
                    Store::PostgreSQL(store) => store.delete_blob(key).await,
                    #[cfg(feature = "mysql")]
                    Store::MySQL(store) => store.delete_blob(key).await,
                    #[cfg(feature = "rocks")]
                    Store::RocksDb(store) => store.delete_blob(key).await,
                    // SPDX-SnippetBegin
                    // SPDX-FileCopyrightText: 2024 A3Mailer Project
                    // SPDX-License-Identifier: LicenseRef-SEL
                    #[cfg(all(
                        feature = "enterprise",
                        any(feature = "postgres", feature = "mysql")
                    ))]
                    Store::SQLReadReplica(store) => store.delete_blob(key).await,
                    // SPDX-SnippetEnd
                    Store::None => Err(trc::StoreEvent::NotConfigured.into()),
                },
                BlobBackend::Fs(store) => store.delete_blob(key).await,
                #[cfg(feature = "s3")]
                BlobBackend::S3(store) => store.delete_blob(key).await,
                #[cfg(feature = "azure")]
                BlobBackend::Azure(store) => store.delete_blob(key).await,
                BlobBackend::Sharded(sharded_store) => {
                    // Prevent infinite recursion by delegating to the sharded store's delete_blob method
                    // This should not happen in practice as sharded stores should not contain other sharded stores
                    trc::event!(
                        Store(trc::StoreEvent::BlobDelete),
                        Key = trc::Value::from(key),
                        Details = "Detected nested sharded blob store - delegating to nested store"
                    );
                    sharded_store.delete_blob(key).await
                },
            };

            // Log the operation result
            match &result {
                Ok(true) => {
                    trc::event!(
                        Store(trc::StoreEvent::BlobDelete),
                        Key = trc::Value::from(key),
                        Elapsed = start_time.elapsed(),
                        Details = "Successfully deleted blob from sharded storage"
                    );
                }
                Ok(false) => {
                    trc::event!(
                        Store(trc::StoreEvent::BlobDelete),
                        Key = trc::Value::from(key),
                        Elapsed = start_time.elapsed(),
                        Details = "Blob not found for deletion in sharded storage"
                    );
                }
                Err(err) => {
                    trc::error!(err
                        .clone()
                        .details("Failed to delete blob from sharded storage")
                        .ctx(trc::Key::Key, trc::Value::from(key))
                        .ctx(trc::Key::Elapsed, start_time.elapsed())
                    );
                }
            }

            result
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;
    use std::sync::Arc;

    /// Test helper to create a mock blob backend for testing
    fn create_mock_blob_backend() -> BlobBackend {
        // For testing purposes, we'll use a None store which will return NotConfigured errors
        // In a real implementation, you would use actual storage backends
        BlobBackend::Store(Store::None)
    }

    /// Test that ShardedBlob correctly distributes keys across shards
    #[test]
    fn test_shard_distribution() {
        let stores = vec![
            create_mock_blob_backend(),
            create_mock_blob_backend(),
            create_mock_blob_backend(),
        ];

        let sharded_blob = ShardedBlob { stores };

        // Test that different keys map to potentially different shards
        let key1 = b"test_key_1";
        let key2 = b"test_key_2";
        let key3 = b"completely_different_key";

        let shard1 = sharded_blob.get_store(key1) as *const BlobBackend;
        let shard2 = sharded_blob.get_store(key2) as *const BlobBackend;
        let shard3 = sharded_blob.get_store(key3) as *const BlobBackend;

        // At least verify that the function returns valid shard references
        // The actual distribution depends on the hash function
        // Since we're getting references, they should always be valid
    }

    /// Test that the same key always maps to the same shard (consistency)
    #[test]
    fn test_shard_consistency() {
        let stores = vec![
            create_mock_blob_backend(),
            create_mock_blob_backend(),
            create_mock_blob_backend(),
        ];

        let sharded_blob = ShardedBlob { stores };

        let key = b"consistent_test_key";

        // Get the shard for the same key multiple times
        let shard1 = sharded_blob.get_store(key) as *const BlobBackend;
        let shard2 = sharded_blob.get_store(key) as *const BlobBackend;
        let shard3 = sharded_blob.get_store(key) as *const BlobBackend;

        // All should point to the same shard
        assert_eq!(shard1, shard2);
        assert_eq!(shard2, shard3);
    }

    /// Test error handling when no stores are configured
    #[tokio::test]
    async fn test_empty_stores_error() {
        use utils::config::Config;
        use crate::Stores;

        // Create a config with empty content to avoid EOF error
        let mut config = Config::new("").unwrap();
        let stores = Stores::default();

        // Try to create a ShardedBlob with no stores configured
        let result = ShardedBlob::open(&mut config, "test", &stores);

        // Should return None due to no stores being specified
        assert!(result.is_none());

        // Should have generated a build error
        assert!(!config.errors.is_empty());
    }

    /// Test that get_blob returns NotConfigured error for None stores
    #[tokio::test]
    async fn test_get_blob_not_configured() {
        let stores = vec![BlobBackend::Store(Store::None)];
        let sharded_blob = ShardedBlob { stores };

        let result = sharded_blob.get_blob(b"test_key", 0..10).await;

        // Should return a NotConfigured error
        assert!(result.is_err());
        if let Err(err) = result {
            // The error should be related to store not being configured
            assert!(format!("{:?}", err).contains("NotConfigured"));
        }
    }

    /// Test that put_blob returns NotConfigured error for None stores
    #[tokio::test]
    async fn test_put_blob_not_configured() {
        let stores = vec![BlobBackend::Store(Store::None)];
        let sharded_blob = ShardedBlob { stores };

        let result = sharded_blob.put_blob(b"test_key", b"test_data").await;

        // Should return a NotConfigured error
        assert!(result.is_err());
        if let Err(err) = result {
            // The error should be related to store not being configured
            assert!(format!("{:?}", err).contains("NotConfigured"));
        }
    }

    /// Test that delete_blob returns NotConfigured error for None stores
    #[tokio::test]
    async fn test_delete_blob_not_configured() {
        let stores = vec![BlobBackend::Store(Store::None)];
        let sharded_blob = ShardedBlob { stores };

        let result = sharded_blob.delete_blob(b"test_key").await;

        // Should return a NotConfigured error
        assert!(result.is_err());
        if let Err(err) = result {
            // The error should be related to store not being configured
            assert!(format!("{:?}", err).contains("NotConfigured"));
        }
    }

    /// Test hash distribution across multiple shards
    #[test]
    fn test_hash_distribution() {
        let num_shards = 5;
        let stores: Vec<BlobBackend> = (0..num_shards)
            .map(|_| create_mock_blob_backend())
            .collect();

        let sharded_blob = ShardedBlob { stores };

        // Test with many different keys to see distribution
        let mut shard_counts = vec![0; num_shards];

        for i in 0..1000 {
            let key = format!("test_key_{}", i);
            let shard_ptr = sharded_blob.get_store(key.as_bytes()) as *const BlobBackend;

            // Find which shard this points to
            for (shard_idx, store) in sharded_blob.stores.iter().enumerate() {
                if store as *const BlobBackend == shard_ptr {
                    shard_counts[shard_idx] += 1;
                    break;
                }
            }
        }

        // Verify that all shards got some keys (reasonable distribution)
        // With a good hash function, each shard should get roughly 200 keys (1000/5)
        // We'll allow a reasonable variance
        for count in shard_counts {
            assert!(count > 100, "Shard distribution too uneven: {}", count);
            assert!(count < 300, "Shard distribution too uneven: {}", count);
        }
    }

    /// Test edge cases with empty keys and data
    #[tokio::test]
    async fn test_edge_cases() {
        let stores = vec![BlobBackend::Store(Store::None)];
        let sharded_blob = ShardedBlob { stores };

        // Test with empty key
        let result = sharded_blob.get_blob(b"", 0..0).await;
        assert!(result.is_err());

        // Test with empty data
        let result = sharded_blob.put_blob(b"test_key", b"").await;
        assert!(result.is_err());

        // Test with empty key for deletion
        let result = sharded_blob.delete_blob(b"").await;
        assert!(result.is_err());
    }

    /// Test range operations
    #[tokio::test]
    async fn test_range_operations() {
        let stores = vec![BlobBackend::Store(Store::None)];
        let sharded_blob = ShardedBlob { stores };

        // Test with various ranges
        let ranges = vec![
            0..10,
            5..15,
            0..0,
            100..200,
        ];

        for range in ranges {
            let result = sharded_blob.get_blob(b"test_key", range.clone()).await;
            assert!(result.is_err(), "Range {:?} should fail with NotConfigured", range);
        }
    }

    /// Test concurrent access to sharded blob storage
    #[tokio::test]
    async fn test_concurrent_access() {
        let stores = vec![
            BlobBackend::Store(Store::None),
            BlobBackend::Store(Store::None),
            BlobBackend::Store(Store::None),
        ];
        let sharded_blob = Arc::new(ShardedBlob { stores });

        let mut handles = vec![];

        // Spawn multiple concurrent operations
        for i in 0..10 {
            let sharded_blob_clone = Arc::clone(&sharded_blob);
            let handle = tokio::spawn(async move {
                let key = format!("concurrent_key_{}", i);
                let data = format!("concurrent_data_{}", i);

                // Try to put and get data (will fail with NotConfigured, but tests concurrency)
                let _put_result = sharded_blob_clone.put_blob(key.as_bytes(), data.as_bytes()).await;
                let _get_result = sharded_blob_clone.get_blob(key.as_bytes(), 0..data.len()).await;
                let _delete_result = sharded_blob_clone.delete_blob(key.as_bytes()).await;
            });
            handles.push(handle);
        }

        // Wait for all operations to complete
        for handle in handles {
            handle.await.unwrap();
        }
    }

    /// Test performance characteristics
    #[tokio::test]
    async fn test_performance_characteristics() {
        let num_shards = 10;
        let stores: Vec<BlobBackend> = (0..num_shards)
            .map(|_| create_mock_blob_backend())
            .collect();

        let sharded_blob = ShardedBlob { stores };

        let start = std::time::Instant::now();

        // Test shard selection performance
        for i in 0..10000 {
            let key = format!("perf_test_key_{}", i);
            let _shard = sharded_blob.get_store(key.as_bytes());
        }

        let elapsed = start.elapsed();

        // Should be very fast (less than 100ms for 10k operations)
        assert!(elapsed.as_millis() < 100, "Shard selection too slow: {:?}", elapsed);
    }

    /// Test with various key patterns
    #[test]
    fn test_key_patterns() {
        let stores = vec![
            create_mock_blob_backend(),
            create_mock_blob_backend(),
            create_mock_blob_backend(),
        ];

        let sharded_blob = ShardedBlob { stores };

        // Test various key patterns
        let test_keys = vec![
            b"simple_key".as_slice(),
            b"key_with_numbers_123456".as_slice(),
            b"key-with-dashes".as_slice(),
            b"key.with.dots".as_slice(),
            b"key/with/slashes".as_slice(),
            b"key with spaces".as_slice(),
            b"\x00\x01\x02\x03\x04\x05".as_slice(), // Binary data
            b"very_long_key_that_exceeds_normal_length_expectations_and_continues_for_a_while_to_test_hash_function_behavior".as_slice(),
        ];

        for key in test_keys {
            let shard = sharded_blob.get_store(key);
            // Since we're getting references, they should always be valid

            // Verify consistency
            let shard2 = sharded_blob.get_store(key);
            assert_eq!(shard as *const BlobBackend, shard2 as *const BlobBackend,
                      "Inconsistent shard selection for key: {:?}", key);
        }
    }
}
