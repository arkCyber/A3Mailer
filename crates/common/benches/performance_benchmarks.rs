/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! # Performance Benchmarks
//!
//! This module contains comprehensive performance benchmarks for the common crate,
//! focusing on production-critical paths and performance-sensitive operations.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use common::{
    monitoring::{
        MonitoringManager, SystemMetrics, ApplicationMetrics,
    },
};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::runtime::Runtime;

/// Benchmark monitoring system performance
fn benchmark_monitoring(c: &mut Criterion) {
    let mut group = c.benchmark_group("monitoring");

    // Benchmark system metrics recording
    group.bench_function("system_metrics_recording", |b| {
        let manager = MonitoringManager::default();
        let metrics = SystemMetrics {
            cpu_usage: 75.0,
            memory_usage: 4 * 1024 * 1024 * 1024, // 4GB
            memory_total: 16 * 1024 * 1024 * 1024, // 16GB
            disk_usage: 100 * 1024 * 1024 * 1024, // 100GB
            disk_total: 500 * 1024 * 1024 * 1024, // 500GB
            network_rx_bytes: 1024 * 1024, // 1MB
            network_tx_bytes: 2 * 1024 * 1024, // 2MB
            load_average_1m: 1.5,
            load_average_5m: 1.2,
            load_average_15m: 1.0,
            active_connections: 150,
            uptime: 3600, // 1 hour
            ..Default::default()
        };

        b.iter(|| {
            manager.record_system_metrics(black_box(metrics.clone()));
        });
    });

    // Benchmark application metrics recording
    group.bench_function("app_metrics_recording", |b| {
        let manager = MonitoringManager::default();
        let mut queue_sizes = HashMap::new();
        queue_sizes.insert("smtp".to_string(), 50);
        queue_sizes.insert("delivery".to_string(), 25);

        let mut cache_hit_rates = HashMap::new();
        cache_hit_rates.insert("dns".to_string(), 0.95);
        cache_hit_rates.insert("auth".to_string(), 0.88);

        let metrics = ApplicationMetrics {
            total_requests: 10000,
            successful_requests: 9800,
            failed_requests: 200,
            avg_response_time: 125.5,
            p95_response_time: 250.0,
            p99_response_time: 500.0,
            requests_per_second: 100.0,
            active_sessions: 75,
            queue_sizes,
            cache_hit_rates,
            ..Default::default()
        };

        b.iter(|| {
            manager.record_app_metrics(black_box(metrics.clone()));
        });
    });

    group.finish();
}

/// Benchmark data structure operations
fn benchmark_data_structures(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_structures");

    // Benchmark HashMap operations in monitoring
    group.bench_function("hashmap_operations", |b| {
        b.iter(|| {
            let mut queue_sizes = HashMap::new();
            let mut cache_hit_rates = HashMap::new();

            for i in 0..100 {
                queue_sizes.insert(format!("queue_{}", i), i as u32);
                cache_hit_rates.insert(format!("cache_{}", i), i as f64 / 100.0);
            }

            black_box(queue_sizes);
            black_box(cache_hit_rates);
        });
    });

    // Benchmark Vec operations
    group.bench_function("vec_operations", |b| {
        b.iter(|| {
            let mut events = Vec::new();

            for i in 0..100 {
                let mut attributes = HashMap::new();
                attributes.insert("key".to_string(), format!("value_{}", i));
                events.push(attributes);
            }

            black_box(events);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_monitoring,
    benchmark_data_structures
);

criterion_main!(benches);
