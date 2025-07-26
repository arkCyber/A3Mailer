/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Store Performance Benchmarks
//!
//! This module contains comprehensive performance benchmarks for the store module.
//! These benchmarks help ensure the store layer meets production performance requirements.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use store::{backend::memory::StaticMemoryStore, Value};
use std::{
    time::Duration,
    collections::HashMap,
};

/// Test data sizes for benchmarks
const SMALL_DATA_SIZE: usize = 100;
const MEDIUM_DATA_SIZE: usize = 1_000;
const LARGE_DATA_SIZE: usize = 10_000;

/// Create test values for benchmarking
fn create_test_values(size: usize) -> Vec<(String, Value<'static>)> {
    (0..size)
        .map(|i| {
            let key = format!("key_{}", i);
            let value = match i % 4 {
                0 => Value::Text(format!("text_value_{}", i).into()),
                1 => Value::Integer((i as i64).into()),
                2 => Value::Float((i as f64 * 3.14159).into()),
                3 => Value::Text(format!("long_text_value_with_more_content_{}_that_simulates_real_world_usage", i).into()),
                _ => unreachable!(),
            };
            (key, value)
        })
        .collect()
}

/// Benchmark data processing operations
fn bench_data_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_processing");
    group.measurement_time(Duration::from_secs(5));

    // Benchmark data processing
    group.bench_function("process_small_data", |b| {
        let data = vec![0u8; SMALL_DATA_SIZE];
        b.iter(|| {
            let sum: u64 = data.iter().map(|&x| x as u64).sum();
            black_box(sum);
        });
    });

    group.bench_function("process_medium_data", |b| {
        let data = vec![0u8; MEDIUM_DATA_SIZE];
        b.iter(|| {
            let sum: u64 = data.iter().map(|&x| x as u64).sum();
            black_box(sum);
        });
    });

    group.bench_function("process_large_data", |b| {
        let data = vec![0u8; LARGE_DATA_SIZE];
        b.iter(|| {
            let sum: u64 = data.iter().map(|&x| x as u64).sum();
            black_box(sum);
        });
    });

    group.finish();
}

/// Benchmark hash map operations
fn bench_hashmap_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashmap_operations");
    group.measurement_time(Duration::from_secs(5));

    // Benchmark hash map insertions
    group.bench_function("insert_1000_items", |b| {
        b.iter(|| {
            let mut map = HashMap::new();
            for i in 0..1000 {
                map.insert(format!("key_{}", i), i);
            }
            black_box(map);
        });
    });

    // Benchmark hash map lookups
    group.bench_function("lookup_1000_items", |b| {
        let mut map = HashMap::new();
        for i in 0..1000 {
            map.insert(format!("key_{}", i), i);
        }

        b.iter(|| {
            for i in 0..1000 {
                let key = format!("key_{}", i);
                let value = map.get(&key);
                black_box(value);
            }
        });
    });

    group.finish();
}

/// Benchmark string operations
fn bench_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");
    group.measurement_time(Duration::from_secs(5));

    // Benchmark string concatenation
    group.bench_function("string_concat_1000", |b| {
        b.iter(|| {
            let mut result = String::new();
            for i in 0..1000 {
                result.push_str(&format!("item_{}", i));
            }
            black_box(result);
        });
    });

    // Benchmark string parsing
    group.bench_function("string_parse_numbers", |b| {
        let numbers: Vec<String> = (0..1000).map(|i| i.to_string()).collect();
        b.iter(|| {
            let mut sum = 0;
            for num_str in &numbers {
                if let Ok(num) = num_str.parse::<i32>() {
                    sum += num;
                }
            }
            black_box(sum);
        });
    });

    group.finish();
}

/// Benchmark StaticMemoryStore insert operations
fn bench_static_memory_store_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("StaticMemoryStore Insert");

    for size in [SMALL_DATA_SIZE, MEDIUM_DATA_SIZE].iter() {
        let test_data = create_test_values(*size);

        group.bench_with_input(BenchmarkId::new("insert", size), size, |b, _| {
            b.iter(|| {
                let mut store = StaticMemoryStore::default();
                for (key, value) in &test_data {
                    store.insert(black_box(key), black_box(value.clone()));
                }
                store
            });
        });
    }

    group.finish();
}

/// Benchmark StaticMemoryStore get operations
fn bench_static_memory_store_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("StaticMemoryStore Get");

    for size in [SMALL_DATA_SIZE, MEDIUM_DATA_SIZE].iter() {
        let test_data = create_test_values(*size);
        let mut store = StaticMemoryStore::default();

        // Pre-populate the store
        for (key, value) in &test_data {
            store.insert(key, value.clone());
        }

        group.bench_with_input(BenchmarkId::new("get", size), size, |b, _| {
            b.iter(|| {
                for (key, _) in &test_data {
                    let _result = store.get(black_box(key));
                }
            });
        });
    }

    group.finish();
}

/// Benchmark glob pattern matching
fn bench_glob_pattern_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("Glob Pattern Matching");

    let mut store = StaticMemoryStore::default();

    // Add exact matches
    for i in 0..1000 {
        let key = format!("exact_key_{}", i);
        store.insert(&key, Value::Integer((i as i64).into()));
    }

    // Add glob patterns
    store.insert("user_*", Value::Text("user_pattern".into()));
    store.insert("admin_*", Value::Text("admin_pattern".into()));
    store.insert("*_config", Value::Text("config_pattern".into()));

    group.bench_function("exact_match", |b| {
        b.iter(|| {
            for i in 0..100 {
                let key = format!("exact_key_{}", i);
                let _result = store.get(black_box(&key));
            }
        });
    });

    group.bench_function("glob_match", |b| {
        b.iter(|| {
            let test_keys = ["user_123", "admin_456", "system_config"];
            for key in &test_keys {
                let _result = store.get(black_box(key));
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_data_processing,
    bench_hashmap_operations,
    bench_string_operations,
    bench_static_memory_store_insert,
    bench_static_memory_store_get,
    bench_glob_pattern_matching
);

criterion_main!(benches);
