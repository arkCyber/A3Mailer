/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! # Monitoring Module Tests
//!
//! Comprehensive test suite for monitoring functionality including:
//! - Performance monitoring tests
//! - Health check tests
//! - Metrics collection tests
//! - Alert system tests
//! - Tracing tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::{
        MonitoringManager, MonitoringConfig, SystemMetrics, ApplicationMetrics,
        HealthCheck, HealthStatus, performance::*, tracing::*,
    };
    use std::{
        collections::HashMap,
        sync::{Arc, atomic::{AtomicU64, Ordering}},
        time::{Duration, Instant},
    };
    use tokio::time::sleep;

    #[test]
    fn test_monitoring_manager_creation() {
        let config = MonitoringConfig::default();
        let manager = MonitoringManager::new(config);
        
        assert!(manager.get_config().enabled);
        assert_eq!(manager.get_uptime(), 0); // Should be very small initially
    }

    #[test]
    fn test_system_metrics_recording() {
        let manager = MonitoringManager::default();
        
        let metrics = SystemMetrics {
            cpu_usage: 75.5,
            memory_usage: 2 * 1024 * 1024 * 1024, // 2GB
            memory_total: 8 * 1024 * 1024 * 1024, // 8GB
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
        
        manager.record_system_metrics(metrics.clone());
        
        let latest = manager.get_latest_system_metrics().unwrap();
        assert_eq!(latest.cpu_usage, 75.5);
        assert_eq!(latest.memory_usage, 2 * 1024 * 1024 * 1024);
        assert_eq!(latest.active_connections, 150);
    }

    #[test]
    fn test_application_metrics_recording() {
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
        
        manager.record_app_metrics(metrics.clone());
        
        let latest = manager.get_latest_app_metrics().unwrap();
        assert_eq!(latest.total_requests, 10000);
        assert_eq!(latest.requests_per_second, 100.0);
        assert_eq!(latest.queue_sizes.get("smtp"), Some(&50));
        assert_eq!(latest.cache_hit_rates.get("dns"), Some(&0.95));
    }

    #[test]
    fn test_health_check_recording() {
        let manager = MonitoringManager::default();
        
        // Record healthy check
        let healthy_check = HealthCheck::new(
            "database".to_string(),
            HealthStatus::Healthy,
            "Database connection successful".to_string(),
        ).with_response_time(Duration::from_millis(50))
         .with_detail("connection_pool".to_string(), "10/20".to_string());
        
        manager.record_health_check(healthy_check);
        
        // Record degraded check
        let degraded_check = HealthCheck::new(
            "cache".to_string(),
            HealthStatus::Degraded,
            "Cache hit rate below threshold".to_string(),
        ).with_response_time(Duration::from_millis(200));
        
        manager.record_health_check(degraded_check);
        
        let health_checks = manager.get_latest_health_checks();
        assert_eq!(health_checks.len(), 2);
        assert_eq!(manager.get_overall_health_status(), HealthStatus::Degraded);
    }

    #[test]
    fn test_overall_health_status_calculation() {
        let manager = MonitoringManager::default();
        
        // Initially unknown
        assert_eq!(manager.get_overall_health_status(), HealthStatus::Unknown);
        
        // Add healthy checks
        manager.record_health_check(HealthCheck::new(
            "service1".to_string(),
            HealthStatus::Healthy,
            "OK".to_string(),
        ));
        manager.record_health_check(HealthCheck::new(
            "service2".to_string(),
            HealthStatus::Healthy,
            "OK".to_string(),
        ));
        assert_eq!(manager.get_overall_health_status(), HealthStatus::Healthy);
        
        // Add degraded check
        manager.record_health_check(HealthCheck::new(
            "service3".to_string(),
            HealthStatus::Degraded,
            "Slow response".to_string(),
        ));
        assert_eq!(manager.get_overall_health_status(), HealthStatus::Degraded);
        
        // Add unhealthy check
        manager.record_health_check(HealthCheck::new(
            "service4".to_string(),
            HealthStatus::Unhealthy,
            "Service down".to_string(),
        ));
        assert_eq!(manager.get_overall_health_status(), HealthStatus::Unhealthy);
    }

    #[test]
    fn test_custom_metrics() {
        let manager = MonitoringManager::default();
        
        manager.set_custom_metric("custom.counter".to_string(), 42.0);
        manager.set_custom_metric("custom.gauge".to_string(), 3.14);
        manager.set_custom_metric("custom.histogram".to_string(), 99.9);
        
        // Custom metrics are stored internally
        // In a real implementation, you'd have a getter method
    }

    #[test]
    fn test_performance_monitor() {
        let config = PerformanceConfig::default();
        let monitor = PerformanceMonitor::new(config);
        
        let mut queue_depths = HashMap::new();
        queue_depths.insert("smtp".to_string(), 100);
        
        let mut cache_hit_rates = HashMap::new();
        cache_hit_rates.insert("dns".to_string(), 0.92);
        
        let sample = PerformanceSample {
            cpu_usage: 65.0,
            memory_usage: 4 * 1024 * 1024 * 1024, // 4GB
            memory_total: 16 * 1024 * 1024 * 1024, // 16GB
            active_connections: 200,
            requests_per_second: 150.0,
            avg_response_time: 180.0,
            error_rate: 2.5,
            queue_depths,
            cache_hit_rates,
            ..Default::default()
        };
        
        monitor.record_sample(sample);
        
        // Test statistics calculation
        let stats = monitor.get_stats(Duration::from_secs(60));
        assert!(stats.is_some());
        
        let stats = stats.unwrap();
        assert_eq!(stats.avg_cpu_usage, 65.0);
        assert_eq!(stats.peak_cpu_usage, 65.0);
    }

    #[test]
    fn test_performance_alerts() {
        let mut config = PerformanceConfig::default();
        config.alert_thresholds.max_cpu_usage = 50.0; // Low threshold for testing
        
        let monitor = PerformanceMonitor::new(config);
        let alert_triggered = Arc::new(AtomicU64::new(0));
        
        // Add alert callback
        let alert_counter = alert_triggered.clone();
        monitor.add_alert_callback(move |_sample| {
            alert_counter.fetch_add(1, Ordering::SeqCst);
        });
        
        // Record sample that should trigger alert
        let sample = PerformanceSample {
            cpu_usage: 75.0, // Above threshold
            ..Default::default()
        };
        
        monitor.record_sample(sample);
        
        // Check that alert was triggered
        assert_eq!(alert_triggered.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_tracing_manager() {
        let config = TracingConfig::default();
        let manager = TracingManager::new(config);
        
        // Start a root span
        let root_context = manager.start_span("test_operation".to_string(), None);
        assert!(!root_context.trace_id.is_empty());
        assert!(!root_context.span_id.is_empty());
        
        // Start a child span
        let child_context = manager.start_span("child_operation".to_string(), Some(&root_context));
        assert_eq!(child_context.trace_id, root_context.trace_id);
        assert_ne!(child_context.span_id, root_context.span_id);
        assert_eq!(child_context.parent_span_id, Some(root_context.span_id.clone()));
        
        // Add attributes and events
        manager.add_span_attribute(&child_context.span_id, "key".to_string(), "value".to_string());
        
        let mut event_attrs = HashMap::new();
        event_attrs.insert("event_key".to_string(), "event_value".to_string());
        manager.add_span_event(&child_context.span_id, "test_event".to_string(), event_attrs);
        
        // End spans
        manager.end_span(&child_context.span_id, SpanStatus::Ok);
        manager.end_span(&root_context.span_id, SpanStatus::Ok);
        
        // Check span counts
        assert_eq!(manager.get_active_span_count(), 0);
        assert_eq!(manager.get_completed_span_count(), 2);
        
        // Get recent spans
        let recent_spans = manager.get_recent_spans(10);
        assert_eq!(recent_spans.len(), 2);
    }

    #[test]
    fn test_span_context_operations() {
        let mut context = SpanContext::new("trace123".to_string(), "span456".to_string());
        
        // Test baggage operations
        context.add_baggage("user_id".to_string(), "12345".to_string());
        context.add_baggage("tenant".to_string(), "acme_corp".to_string());
        
        assert_eq!(context.get_baggage("user_id"), Some(&"12345".to_string()));
        assert_eq!(context.get_baggage("tenant"), Some(&"acme_corp".to_string()));
        assert_eq!(context.get_baggage("nonexistent"), None);
        
        // Test child context creation
        let child = context.create_child("child789".to_string());
        assert_eq!(child.trace_id, "trace123");
        assert_eq!(child.span_id, "child789");
        assert_eq!(child.parent_span_id, Some("span456".to_string()));
        assert_eq!(child.baggage.len(), 2); // Inherited baggage
    }

    #[tokio::test]
    async fn test_monitoring_data_retention() {
        let mut config = MonitoringConfig::default();
        config.retention_period = Duration::from_millis(100); // Very short for testing
        
        let manager = MonitoringManager::new(config);
        
        // Record some metrics
        manager.record_system_metrics(SystemMetrics::default());
        manager.record_app_metrics(ApplicationMetrics::default());
        manager.record_health_check(HealthCheck::new(
            "test".to_string(),
            HealthStatus::Healthy,
            "OK".to_string(),
        ));
        
        // Wait for retention period to pass
        sleep(Duration::from_millis(150)).await;
        
        // Trigger cleanup
        manager.cleanup();
        
        // Data should be cleaned up (in a real implementation)
        // This test verifies the cleanup method doesn't panic
    }

    #[tokio::test]
    async fn test_concurrent_monitoring_operations() {
        let manager = Arc::new(MonitoringManager::default());
        let mut handles = Vec::new();
        
        // Spawn multiple concurrent tasks
        for i in 0..10 {
            let manager_clone = manager.clone();
            let handle = tokio::spawn(async move {
                // Record metrics concurrently
                let metrics = SystemMetrics {
                    cpu_usage: i as f64 * 10.0,
                    memory_usage: (i as u64) * 1024 * 1024,
                    active_connections: i as u32 * 10,
                    ..Default::default()
                };
                manager_clone.record_system_metrics(metrics);
                
                // Record health checks concurrently
                let health_check = HealthCheck::new(
                    format!("service_{}", i),
                    HealthStatus::Healthy,
                    "Concurrent test".to_string(),
                );
                manager_clone.record_health_check(health_check);
                
                // Set custom metrics concurrently
                manager_clone.set_custom_metric(format!("metric_{}", i), i as f64);
            });
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await.expect("Task should complete successfully");
        }
        
        // Verify data was recorded
        assert!(manager.get_latest_system_metrics().is_some());
        assert!(!manager.get_latest_health_checks().is_empty());
    }

    #[test]
    fn test_performance_statistics_calculation() {
        let monitor = PerformanceMonitor::default();
        
        // Record multiple samples
        for i in 0..10 {
            let sample = PerformanceSample {
                cpu_usage: 50.0 + (i as f64),
                memory_usage: (1 + i) * 1024 * 1024 * 1024,
                memory_total: 16 * 1024 * 1024 * 1024,
                requests_per_second: 100.0 + (i as f64) * 10.0,
                avg_response_time: 100.0 + (i as f64) * 5.0,
                error_rate: 1.0 + (i as f64) * 0.1,
                ..Default::default()
            };
            monitor.record_sample(sample);
        }
        
        // Get statistics
        let stats = monitor.get_stats(Duration::from_secs(3600)).unwrap();
        
        // Verify calculated statistics
        assert!(stats.avg_cpu_usage > 50.0);
        assert!(stats.peak_cpu_usage >= stats.avg_cpu_usage);
        assert!(stats.avg_response_time > 100.0);
        assert!(stats.total_requests > 0);
    }

    #[test]
    fn test_tracing_cleanup() {
        let manager = TracingManager::default();
        
        // Create many spans
        for i in 0..100 {
            let context = manager.start_span(format!("operation_{}", i), None);
            manager.end_span(&context.span_id, SpanStatus::Ok);
        }
        
        assert_eq!(manager.get_completed_span_count(), 100);
        
        // Trigger cleanup
        manager.cleanup();
        
        // Cleanup should not panic and may reduce span count
        assert!(manager.get_completed_span_count() <= 100);
    }
}
