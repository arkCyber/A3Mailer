/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Integration tests module
//! 
//! This module contains comprehensive integration tests that verify
//! the interaction between different components of the Stalwart system.
//! These tests ensure that the system works correctly as a whole.

pub mod blob_storage_test;

/// Run all integration tests
#[tokio::test]
async fn run_all_integration_tests() {
    println!("Starting comprehensive integration test suite...");
    
    // Run blob storage integration tests
    blob_storage_test::test_blob_storage_integration().await;
    blob_storage_test::test_blob_key_patterns().await;
    blob_storage_test::test_blob_storage_stress().await;
    
    println!("All integration tests completed successfully!");
}
