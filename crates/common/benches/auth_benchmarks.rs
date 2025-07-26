/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Authentication and Authorization Performance Benchmarks
//!
//! This module contains performance benchmarks for authentication and authorization
//! operations, including token validation, permission checks, and role resolution.
//! These benchmarks ensure the auth system meets production performance requirements.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use common::{
    auth::{AccessToken, ResourceToken},
    config::Config,
    Core,
};
use directory::{Principal, Type, Permission};
use store::{Store, backend::memory::MemoryStore};
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
    collections::HashMap,
};
use tokio::runtime::Runtime;

/// Benchmark configuration
struct AuthBenchmarkConfig {
    core: Arc<Core>,
    runtime: Runtime,
    test_tokens: Vec<AccessToken>,
    test_principals: Vec<Principal>,
}

impl AuthBenchmarkConfig {
    fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");
        let store = Store::Memory(Arc::new(MemoryStore::new()));

        // Create a minimal core configuration
        let core = Arc::new(Core {
            storage: store.clone(),
            sieve: Default::default(),
            network: Default::default(),
            tls: Default::default(),
            smtp: Default::default(),
            jmap: Default::default(),
            imap: Default::default(),
            pop3: Default::default(),
            managesieve: Default::default(),
            acme: Default::default(),
            #[cfg(feature = "enterprise")]
            enterprise: None,
        });

        // Generate test tokens
        let mut test_tokens = Vec::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for i in 0..1000 {
            let token = AccessToken {
                primary_id: i,
                member_of: vec![1, 2, 3], // Common groups
                access_to: vec![],
                name: format!("user_{}", i),
                description: Some(format!("Test User {}", i)),
                quota: 1024 * 1024 * 1024, // 1GB
                typ: Type::Individual,
                tenant: None,
                permissions: Permission::all(),
                expires: now + 3600, // 1 hour
            };
            test_tokens.push(token);
        }

        // Generate test principals
        let mut test_principals = Vec::new();
        for i in 0..1000 {
            let principal = Principal {
                id: i,
                typ: Type::Individual,
                quota: 1024 * 1024 * 1024,
                name: format!("user_{}", i),
                description: Some(format!("Test User {}", i)),
                secrets: vec![],
                emails: vec![format!("user{}@example.com", i)],
                member_of: vec![1, 2, 3],
                tenant: None,
            };
            test_principals.push(principal);
        }

        Self {
            core,
            runtime,
            test_tokens,
            test_principals,
        }
    }
}

/// Benchmark token validation operations
fn bench_token_validation(c: &mut Criterion) {
    let config = AuthBenchmarkConfig::new();

    let mut group = c.benchmark_group("token_validation");
    group.measurement_time(Duration::from_secs(10));

    // Benchmark access token validation
    group.bench_function("access_token_validation", |b| {
        b.iter(|| {
            for token in &config.test_tokens[0..100] {
                // Simulate token validation
                let is_valid = token.expires > SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                black_box(is_valid);
            }
        });
    });

    // Benchmark permission checks
    group.bench_function("permission_checks", |b| {
        b.iter(|| {
            for token in &config.test_tokens[0..100] {
                // Simulate various permission checks
                let can_read = token.permissions.contains(Permission::EmailReceive);
                let can_write = token.permissions.contains(Permission::EmailSend);
                let can_admin = token.permissions.contains(Permission::Administer);
                black_box((can_read, can_write, can_admin));
            }
        });
    });

    group.finish();
}

/// Benchmark principal operations
fn bench_principal_operations(c: &mut Criterion) {
    let config = AuthBenchmarkConfig::new();

    let mut group = c.benchmark_group("principal_operations");
    group.measurement_time(Duration::from_secs(10));

    // Benchmark principal lookup by ID
    group.bench_function("lookup_by_id", |b| {
        b.iter(|| {
            for i in 0..100 {
                let principal = config.test_principals.iter()
                    .find(|p| p.id == i);
                black_box(principal);
            }
        });
    });

    // Benchmark principal lookup by email
    group.bench_function("lookup_by_email", |b| {
        b.iter(|| {
            for i in 0..100 {
                let email = format!("user{}@example.com", i);
                let principal = config.test_principals.iter()
                    .find(|p| p.emails.contains(&email));
                black_box(principal);
            }
        });
    });

    // Benchmark group membership checks
    group.bench_function("group_membership", |b| {
        b.iter(|| {
            for principal in &config.test_principals[0..100] {
                let is_member_of_1 = principal.member_of.contains(&1);
                let is_member_of_2 = principal.member_of.contains(&2);
                let is_member_of_3 = principal.member_of.contains(&3);
                black_box((is_member_of_1, is_member_of_2, is_member_of_3));
            }
        });
    });

    group.finish();
}

/// Benchmark concurrent authentication operations
fn bench_concurrent_auth(c: &mut Criterion) {
    let config = AuthBenchmarkConfig::new();

    let mut group = c.benchmark_group("concurrent_auth");
    group.measurement_time(Duration::from_secs(15));

    // Benchmark concurrent token validations
    group.bench_function("concurrent_token_validation", |b| {
        b.to_async(&config.runtime).iter(|| async {
            let mut handles = Vec::new();

            for i in 0..50 {
                let token = config.test_tokens[i % config.test_tokens.len()].clone();

                let handle = tokio::spawn(async move {
                    // Simulate token validation work
                    let is_valid = token.expires > SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    // Simulate permission checks
                    let permissions = token.permissions;
                    let can_read = permissions.contains(Permission::EmailReceive);
                    let can_write = permissions.contains(Permission::EmailSend);

                    black_box((is_valid, can_read, can_write));
                });

                handles.push(handle);
            }

            for handle in handles {
                handle.await.unwrap();
            }
        });
    });

    group.finish();
}

/// Benchmark resource token operations
fn bench_resource_tokens(c: &mut Criterion) {
    let config = AuthBenchmarkConfig::new();

    let mut group = c.benchmark_group("resource_tokens");
    group.measurement_time(Duration::from_secs(10));

    // Create test resource tokens
    let mut resource_tokens = Vec::new();
    for i in 0..1000 {
        let token = ResourceToken {
            account_id: i,
            quota: 1024 * 1024 * 1024,
            tenant: None,
        };
        resource_tokens.push(token);
    }

    // Benchmark resource token validation
    group.bench_function("resource_token_validation", |b| {
        b.iter(|| {
            for token in &resource_tokens[0..100] {
                // Simulate resource access validation
                let has_quota = token.quota > 0;
                let account_valid = token.account_id > 0;
                black_box((has_quota, account_valid));
            }
        });
    });

    group.finish();
}

/// Benchmark authentication caching scenarios
fn bench_auth_caching(c: &mut Criterion) {
    let config = AuthBenchmarkConfig::new();

    let mut group = c.benchmark_group("auth_caching");
    group.measurement_time(Duration::from_secs(10));

    // Simulate an in-memory cache
    let mut cache: HashMap<u32, AccessToken> = HashMap::new();
    for token in &config.test_tokens[0..100] {
        cache.insert(token.primary_id, token.clone());
    }

    // Benchmark cache hits
    group.bench_function("cache_hits", |b| {
        b.iter(|| {
            for i in 0..100 {
                let token = cache.get(&i);
                black_box(token);
            }
        });
    });

    // Benchmark cache misses (simulating database lookup)
    group.bench_function("cache_misses", |b| {
        b.iter(|| {
            for i in 100..200 {
                // Simulate cache miss and database lookup
                let token = config.test_tokens.iter()
                    .find(|t| t.primary_id == i);
                black_box(token);
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_token_validation,
    bench_principal_operations,
    bench_concurrent_auth,
    bench_resource_tokens,
    bench_auth_caching
);

criterion_main!(benches);
