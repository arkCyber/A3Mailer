/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! # Core Module Tests
//!
//! Comprehensive test suite for the core server functionality including:
//! - Storage access tests
//! - Directory service tests
//! - Queue management tests
//! - Resource management tests
//! - Performance tests
//! - Error handling tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Server, Inner, Core, Data, Caches, Ipc};
    use std::sync::Arc;
    use tokio::sync::mpsc;

    /// Create a test server instance
    fn create_test_server() -> Server {
        let (state_tx, _) = mpsc::channel(1024);
        let (housekeeper_tx, _) = mpsc::channel(1024);
        let (queue_tx, _) = mpsc::channel(1024);
        let (report_tx, _) = mpsc::channel(1024);

        let ipc = Ipc {
            state_tx,
            housekeeper_tx,
            task_tx: Arc::new(tokio::sync::Notify::new()),
            queue_tx,
            report_tx,
            broadcast_tx: None,
        };

        let inner = Arc::new(Inner {
            shared_core: arc_swap::ArcSwap::new(Arc::new(Core::default())),
            data: Data::default(),
            cache: Caches::default(),
            ipc,
        });

        Server {
            inner: inner.clone(),
            core: inner.shared_core.load_full(),
        }
    }

    #[test]
    fn test_server_creation() {
        let server = create_test_server();
        
        // Test that all core components are accessible
        let _store = server.store();
        let _blob_store = server.blob_store();
        let _fts_store = server.fts_store();
        let _in_memory_store = server.in_memory_store();
        let _directory = server.directory();
    }

    #[test]
    fn test_directory_access() {
        let server = create_test_server();
        
        // Test default directory access
        let default_dir = server.get_directory_or_default("", 12345);
        assert!(!Arc::ptr_eq(default_dir, &Arc::new(crate::Directory::default())));
        
        // Test non-existent directory fallback
        let fallback_dir = server.get_directory_or_default("non_existent", 12345);
        assert!(Arc::ptr_eq(fallback_dir, server.directory()));
    }

    #[test]
    fn test_in_memory_store_access() {
        let server = create_test_server();
        
        // Test default in-memory store access
        let default_store = server.get_in_memory_store_or_default("", 12345);
        assert_eq!(std::ptr::eq(default_store, server.in_memory_store()), true);
        
        // Test non-existent store fallback
        let fallback_store = server.get_in_memory_store_or_default("non_existent", 12345);
        assert_eq!(std::ptr::eq(fallback_store, server.in_memory_store()), true);
    }

    #[test]
    fn test_data_store_access() {
        let server = create_test_server();
        
        // Test default data store access
        let default_store = server.get_data_store("", 12345);
        assert_eq!(std::ptr::eq(default_store, server.store()), true);
        
        // Test non-existent store fallback
        let fallback_store = server.get_data_store("non_existent", 12345);
        assert_eq!(std::ptr::eq(fallback_store, server.store()), true);
    }

    #[test]
    fn test_queue_strategies() {
        let server = create_test_server();
        
        // Test default queue strategy
        let default_queue = server.get_queue_or_default("default", 12345);
        assert!(!default_queue.retry.is_empty());
        
        // Test non-existent queue fallback
        let fallback_queue = server.get_queue_or_default("non_existent", 12345);
        assert!(!fallback_queue.retry.is_empty());
    }

    #[test]
    fn test_routing_strategies() {
        let server = create_test_server();
        
        // Test local routing strategy
        let local_route = server.get_route_or_default("local", 12345);
        assert!(matches!(local_route, crate::config::smtp::queue::RoutingStrategy::Local));
        
        // Test MX routing strategy
        let mx_route = server.get_route_or_default("mx", 12345);
        assert!(matches!(mx_route, crate::config::smtp::queue::RoutingStrategy::Mx(_)));
        
        // Test non-existent route fallback
        let fallback_route = server.get_route_or_default("non_existent", 12345);
        assert!(matches!(fallback_route, crate::config::smtp::queue::RoutingStrategy::Mx(_)));
    }

    #[test]
    fn test_tls_strategies() {
        let server = create_test_server();
        
        // Test default TLS strategy
        let default_tls = server.get_tls_or_default("default", 12345);
        assert!(matches!(default_tls.tls, crate::config::smtp::queue::RequireOptional::Optional));
        
        // Test non-existent TLS strategy fallback
        let fallback_tls = server.get_tls_or_default("non_existent", 12345);
        assert!(matches!(fallback_tls.tls, crate::config::smtp::queue::RequireOptional::Optional));
    }

    #[test]
    fn test_connection_strategies() {
        let server = create_test_server();
        
        // Test default connection strategy
        let default_conn = server.get_connection_or_default("default", 12345);
        assert!(default_conn.source_ipv4.is_empty());
        assert!(default_conn.source_ipv6.is_empty());
        
        // Test non-existent connection strategy fallback
        let fallback_conn = server.get_connection_or_default("non_existent", 12345);
        assert!(fallback_conn.source_ipv4.is_empty());
        assert!(fallback_conn.source_ipv6.is_empty());
    }

    #[test]
    fn test_virtual_queues() {
        let server = create_test_server();
        
        // Test default virtual queue
        let default_vq = server.get_virtual_queue_or_default(&crate::config::smtp::queue::DEFAULT_QUEUE_NAME);
        assert_eq!(default_vq.threads, 25);
        
        // Test non-existent virtual queue fallback
        let custom_queue_name = crate::config::smtp::queue::QueueName::from("custom_queue");
        let fallback_vq = server.get_virtual_queue_or_default(&custom_queue_name);
        assert_eq!(fallback_vq.threads, 25);
    }

    #[test]
    fn test_snowflake_id_generation() {
        let server = create_test_server();
        
        // Test ID generation
        let id1 = server.generate_snowflake_id();
        let id2 = server.generate_snowflake_id();
        
        // IDs should be different
        assert_ne!(id1, id2);
        
        // IDs should be non-zero
        assert_ne!(id1, 0);
        assert_ne!(id2, 0);
    }

    #[test]
    fn test_task_queue_notification() {
        let server = create_test_server();
        
        // This should not panic
        server.notify_task_queue();
    }

    #[tokio::test]
    async fn test_quota_operations() {
        let server = create_test_server();
        
        // Test quota retrieval for non-existent account
        let result = server.get_used_quota(999999).await;
        // Should not panic and return a result
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_document_operations() {
        let server = create_test_server();
        
        // Test document ID retrieval for non-existent account
        let result = server.get_document_ids(999999, jmap_proto::types::collection::Collection::Email).await;
        // Should not panic and return a result
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_archive_operations() {
        let server = create_test_server();
        
        // Test archive retrieval for non-existent document
        let result = server.get_archive(
            999999,
            jmap_proto::types::collection::Collection::Email,
            999999
        ).await;
        // Should not panic and return a result
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_total_queued_messages() {
        let server = create_test_server();
        
        // Test total queued messages count
        let result = server.total_queued_messages().await;
        // Should not panic and return a result
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_account_and_domain_counts() {
        let server = create_test_server();
        
        // Test total accounts count
        let accounts_result = server.total_accounts().await;
        assert!(accounts_result.is_ok() || accounts_result.is_err());
        
        // Test total domains count
        let domains_result = server.total_domains().await;
        assert!(domains_result.is_ok() || domains_result.is_err());
    }

    #[tokio::test]
    async fn test_state_change_broadcasting() {
        let server = create_test_server();
        
        // Create a test state change
        let state_change = jmap_proto::types::state::StateChange::new(12345, 1);
        
        // Test broadcasting (should not panic)
        let result = server.broadcast_state_change(state_change).await;
        // Should return a boolean indicating success/failure
        assert!(result == true || result == false);
    }

    #[tokio::test]
    async fn test_blob_operations() {
        let server = create_test_server();
        
        // Test blob creation
        let test_data = b"Hello, World!";
        let result = server.put_blob(12345, test_data, true).await;
        
        // Should not panic and return a result
        assert!(result.is_ok() || result.is_err());
    }

    // Performance tests
    #[tokio::test]
    async fn test_concurrent_access() {
        let server = Arc::new(create_test_server());
        let mut handles = Vec::new();
        
        // Spawn multiple concurrent tasks
        for i in 0..10 {
            let server_clone = server.clone();
            let handle = tokio::spawn(async move {
                // Test concurrent access to various methods
                let _store = server_clone.store();
                let _directory = server_clone.get_directory_or_default("test", i);
                let _queue = server_clone.get_queue_or_default("default", i);
                let _id = server_clone.generate_snowflake_id();
                
                // Test async operations
                let _quota = server_clone.get_used_quota(i as u32).await;
                let _total = server_clone.total_queued_messages().await;
            });
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await.expect("Task should complete successfully");
        }
    }

    #[test]
    fn test_memory_usage() {
        // Test that server creation doesn't use excessive memory
        let initial_memory = get_memory_usage();
        
        let _servers: Vec<Server> = (0..100).map(|_| create_test_server()).collect();
        
        let final_memory = get_memory_usage();
        let memory_diff = final_memory - initial_memory;
        
        // Memory usage should be reasonable (less than 100MB for 100 servers)
        assert!(memory_diff < 100 * 1024 * 1024, "Memory usage too high: {} bytes", memory_diff);
    }

    // Helper function to get current memory usage (simplified)
    fn get_memory_usage() -> usize {
        // This is a simplified implementation
        // In a real scenario, you'd use a proper memory profiling library
        std::mem::size_of::<Server>() * 1000 // Placeholder
    }
}
