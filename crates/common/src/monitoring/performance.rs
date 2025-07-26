/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! # Performance Monitoring
//!
//! This module provides advanced performance monitoring capabilities including:
//! - Real-time performance metrics collection
//! - Performance profiling and analysis
//! - Resource utilization tracking
//! - Performance bottleneck detection
//! - Automated performance alerts

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use serde::{Serialize, Deserialize};
use tracing::{debug, info, warn};

/// Performance monitoring configuration
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// Enable performance monitoring
    pub enabled: bool,
    /// Sampling interval for metrics collection
    pub sampling_interval: Duration,
    /// Number of samples to keep in memory
    pub sample_buffer_size: usize,
    /// Performance alert thresholds
    pub alert_thresholds: PerformanceThresholds,
    /// Enable detailed profiling
    pub enable_profiling: bool,
    /// Profiling sample rate (0.0 to 1.0)
    pub profiling_sample_rate: f64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sampling_interval: Duration::from_secs(5),
            sample_buffer_size: 720, // 1 hour at 5-second intervals
            alert_thresholds: PerformanceThresholds::default(),
            enable_profiling: false,
            profiling_sample_rate: 0.01, // 1% sampling
        }
    }
}

/// Performance alert thresholds
#[derive(Debug, Clone)]
pub struct PerformanceThresholds {
    /// Maximum acceptable CPU usage (percentage)
    pub max_cpu_usage: f64,
    /// Maximum acceptable memory usage (percentage)
    pub max_memory_usage: f64,
    /// Maximum acceptable response time (milliseconds)
    pub max_response_time: u64,
    /// Maximum acceptable error rate (percentage)
    pub max_error_rate: f64,
    /// Maximum acceptable queue depth
    pub max_queue_depth: u32,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            max_cpu_usage: 85.0,
            max_memory_usage: 90.0,
            max_response_time: 5000,
            max_error_rate: 5.0,
            max_queue_depth: 1000,
        }
    }
}

/// Performance sample data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSample {
    /// Timestamp when sample was taken
    pub timestamp: Instant,
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// Total memory available
    pub memory_total: u64,
    /// Active connections count
    pub active_connections: u32,
    /// Requests per second
    pub requests_per_second: f64,
    /// Average response time in milliseconds
    pub avg_response_time: f64,
    /// Error rate percentage
    pub error_rate: f64,
    /// Queue depths by queue name
    pub queue_depths: HashMap<String, u32>,
    /// Cache hit rates by cache name
    pub cache_hit_rates: HashMap<String, f64>,
}

impl Default for PerformanceSample {
    fn default() -> Self {
        Self {
            timestamp: Instant::now(),
            cpu_usage: 0.0,
            memory_usage: 0,
            memory_total: 0,
            active_connections: 0,
            requests_per_second: 0.0,
            avg_response_time: 0.0,
            error_rate: 0.0,
            queue_depths: HashMap::new(),
            cache_hit_rates: HashMap::new(),
        }
    }
}

/// Performance statistics over a time window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    /// Time window start
    pub window_start: Instant,
    /// Time window end
    pub window_end: Instant,
    /// Average CPU usage
    pub avg_cpu_usage: f64,
    /// Peak CPU usage
    pub peak_cpu_usage: f64,
    /// Average memory usage
    pub avg_memory_usage: u64,
    /// Peak memory usage
    pub peak_memory_usage: u64,
    /// Average response time
    pub avg_response_time: f64,
    /// 95th percentile response time
    pub p95_response_time: f64,
    /// 99th percentile response time
    pub p99_response_time: f64,
    /// Total requests processed
    pub total_requests: u64,
    /// Total errors
    pub total_errors: u64,
    /// Error rate
    pub error_rate: f64,
}

/// Performance monitor
pub struct PerformanceMonitor {
    config: PerformanceConfig,
    samples: Arc<RwLock<VecDeque<PerformanceSample>>>,
    last_collection: Arc<RwLock<Instant>>,
    alert_callbacks: Arc<RwLock<Vec<Box<dyn Fn(&PerformanceSample) + Send + Sync>>>>,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new(config: PerformanceConfig) -> Self {
        info!("Initializing performance monitor with config: {:?}", config);
        Self {
            config,
            samples: Arc::new(RwLock::new(VecDeque::new())),
            last_collection: Arc::new(RwLock::new(Instant::now())),
            alert_callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record a performance sample
    pub fn record_sample(&self, sample: PerformanceSample) {
        if !self.config.enabled {
            return;
        }

        debug!("Recording performance sample: CPU: {:.1}%, Memory: {} MB, RPS: {:.1}",
               sample.cpu_usage, sample.memory_usage / 1024 / 1024, sample.requests_per_second);

        // Check for performance alerts
        self.check_alerts(&sample);

        // Store the sample
        let mut samples = self.samples.write().unwrap();
        samples.push_back(sample);

        // Maintain buffer size
        while samples.len() > self.config.sample_buffer_size {
            samples.pop_front();
        }

        *self.last_collection.write().unwrap() = Instant::now();
    }

    /// Get performance statistics for a time window
    pub fn get_stats(&self, window: Duration) -> Option<PerformanceStats> {
        let samples = self.samples.read().unwrap();
        if samples.is_empty() {
            return None;
        }

        let now = Instant::now();
        let window_start = now - window;

        // Filter samples within the time window
        let window_samples: Vec<&PerformanceSample> = samples
            .iter()
            .filter(|s| s.timestamp >= window_start)
            .collect();

        if window_samples.is_empty() {
            return None;
        }

        // Calculate statistics
        let mut cpu_usage_sum = 0.0;
        let mut memory_usage_sum = 0u64;
        let mut response_times = Vec::new();
        let mut peak_cpu = 0.0;
        let mut peak_memory = 0u64;
        let mut total_requests = 0u64;
        let mut total_errors = 0u64;

        for sample in &window_samples {
            cpu_usage_sum += sample.cpu_usage;
            memory_usage_sum += sample.memory_usage;
            response_times.push(sample.avg_response_time);
            
            if sample.cpu_usage > peak_cpu {
                peak_cpu = sample.cpu_usage;
            }
            if sample.memory_usage > peak_memory {
                peak_memory = sample.memory_usage;
            }

            // Estimate requests and errors from rate
            let sample_duration = self.config.sampling_interval.as_secs_f64();
            total_requests += (sample.requests_per_second * sample_duration) as u64;
            total_errors += (sample.requests_per_second * sample.error_rate / 100.0 * sample_duration) as u64;
        }

        let sample_count = window_samples.len() as f64;
        response_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p95_index = ((response_times.len() as f64) * 0.95) as usize;
        let p99_index = ((response_times.len() as f64) * 0.99) as usize;

        Some(PerformanceStats {
            window_start,
            window_end: now,
            avg_cpu_usage: cpu_usage_sum / sample_count,
            peak_cpu_usage: peak_cpu,
            avg_memory_usage: (memory_usage_sum as f64 / sample_count) as u64,
            peak_memory_usage: peak_memory,
            avg_response_time: response_times.iter().sum::<f64>() / response_times.len() as f64,
            p95_response_time: response_times.get(p95_index).copied().unwrap_or(0.0),
            p99_response_time: response_times.get(p99_index).copied().unwrap_or(0.0),
            total_requests,
            total_errors,
            error_rate: if total_requests > 0 {
                (total_errors as f64 / total_requests as f64) * 100.0
            } else {
                0.0
            },
        })
    }

    /// Get recent samples
    pub fn get_recent_samples(&self, count: usize) -> Vec<PerformanceSample> {
        let samples = self.samples.read().unwrap();
        samples.iter().rev().take(count).cloned().collect()
    }

    /// Add alert callback
    pub fn add_alert_callback<F>(&self, callback: F)
    where
        F: Fn(&PerformanceSample) + Send + Sync + 'static,
    {
        let mut callbacks = self.alert_callbacks.write().unwrap();
        callbacks.push(Box::new(callback));
    }

    /// Check for performance alerts
    fn check_alerts(&self, sample: &PerformanceSample) {
        let thresholds = &self.config.alert_thresholds;
        let mut alerts = Vec::new();

        if sample.cpu_usage > thresholds.max_cpu_usage {
            alerts.push(format!("High CPU usage: {:.1}%", sample.cpu_usage));
        }

        let memory_usage_pct = if sample.memory_total > 0 {
            (sample.memory_usage as f64 / sample.memory_total as f64) * 100.0
        } else {
            0.0
        };

        if memory_usage_pct > thresholds.max_memory_usage {
            alerts.push(format!("High memory usage: {:.1}%", memory_usage_pct));
        }

        if sample.avg_response_time > thresholds.max_response_time as f64 {
            alerts.push(format!("High response time: {:.1}ms", sample.avg_response_time));
        }

        if sample.error_rate > thresholds.max_error_rate {
            alerts.push(format!("High error rate: {:.1}%", sample.error_rate));
        }

        for (queue_name, depth) in &sample.queue_depths {
            if *depth > thresholds.max_queue_depth {
                alerts.push(format!("High queue depth in {}: {}", queue_name, depth));
            }
        }

        if !alerts.is_empty() {
            warn!("Performance alerts triggered: {}", alerts.join(", "));
            
            // Trigger alert callbacks
            let callbacks = self.alert_callbacks.read().unwrap();
            for callback in callbacks.iter() {
                callback(sample);
            }
        }
    }

    /// Get configuration
    pub fn get_config(&self) -> &PerformanceConfig {
        &self.config
    }

    /// Get time since last collection
    pub fn time_since_last_collection(&self) -> Duration {
        self.last_collection.read().unwrap().elapsed()
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new(PerformanceConfig::default())
    }
}
