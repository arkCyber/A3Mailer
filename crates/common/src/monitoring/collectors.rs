/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Metrics Collectors Module
//!
//! This module provides various metrics collectors for system resources,
//! application performance, and custom metrics collection.

use super::{SystemMetrics, ApplicationMetrics, DatabasePoolStats};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    io,
};
use tracing::debug;

/// System metrics collector
pub struct SystemMetricsCollector {
    last_cpu_times: Arc<RwLock<Option<CpuTimes>>>,
    last_network_stats: Arc<RwLock<Option<NetworkStats>>>,
    start_time: Instant,
}

/// CPU time statistics
#[derive(Debug, Clone)]
struct CpuTimes {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

/// Network statistics
#[derive(Debug, Clone)]
struct NetworkStats {
    rx_bytes: u64,
    tx_bytes: u64,
    timestamp: Instant,
}

impl SystemMetricsCollector {
    /// Create a new system metrics collector
    pub fn new() -> Self {
        debug!("Creating system metrics collector");
        Self {
            last_cpu_times: Arc::new(RwLock::new(None)),
            last_network_stats: Arc::new(RwLock::new(None)),
            start_time: Instant::now(),
        }
    }

    /// Collect current system metrics
    pub fn collect(&self) -> SystemMetrics {
        debug!("Collecting system metrics");

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let cpu_usage = self.collect_cpu_usage().unwrap_or(0.0);
        let (memory_usage, memory_total) = self.collect_memory_info().unwrap_or((0, 0));
        let (disk_usage, disk_total) = self.collect_disk_info().unwrap_or((0, 0));
        let (network_rx, network_tx) = self.collect_network_stats().unwrap_or((0, 0));
        let (load_1m, load_5m, load_15m) = self.collect_load_average().unwrap_or((0.0, 0.0, 0.0));
        let uptime = self.start_time.elapsed().as_secs();

        SystemMetrics {
            timestamp,
            cpu_usage,
            memory_usage,
            memory_total,
            disk_usage,
            disk_total,
            network_rx_bytes: network_rx,
            network_tx_bytes: network_tx,
            load_average_1m: load_1m,
            load_average_5m: load_5m,
            load_average_15m: load_15m,
            active_connections: 0, // Will be set by application
            uptime,
        }
    }

    /// Collect CPU usage percentage
    fn collect_cpu_usage(&self) -> Result<f64, io::Error> {
        #[cfg(target_os = "linux")]
        {
            let stat_content = fs::read_to_string("/proc/stat")?;
            let cpu_line = stat_content
                .lines()
                .find(|line| line.starts_with("cpu "))
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "CPU line not found"))?;

            let values: Vec<u64> = cpu_line
                .split_whitespace()
                .skip(1)
                .take(8)
                .map(|s| s.parse().unwrap_or(0))
                .collect();

            if values.len() < 4 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid CPU data"));
            }

            let current_times = CpuTimes {
                user: values[0],
                nice: values[1],
                system: values[2],
                idle: values[3],
                iowait: values.get(4).copied().unwrap_or(0),
                irq: values.get(5).copied().unwrap_or(0),
                softirq: values.get(6).copied().unwrap_or(0),
                steal: values.get(7).copied().unwrap_or(0),
            };

            let mut last_times = self.last_cpu_times.write().unwrap();

            if let Some(ref prev_times) = *last_times {
                let total_diff = (current_times.user + current_times.nice + current_times.system +
                                 current_times.idle + current_times.iowait + current_times.irq +
                                 current_times.softirq + current_times.steal) -
                                (prev_times.user + prev_times.nice + prev_times.system +
                                 prev_times.idle + prev_times.iowait + prev_times.irq +
                                 prev_times.softirq + prev_times.steal);

                let idle_diff = current_times.idle - prev_times.idle;

                if total_diff > 0 {
                    let usage = 100.0 * (1.0 - (idle_diff as f64 / total_diff as f64));
                    *last_times = Some(current_times);
                    return Ok(usage.max(0.0).min(100.0));
                }
            }

            *last_times = Some(current_times);
            Ok(0.0)
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Fallback for non-Linux systems
            Ok(0.0)
        }
    }

    /// Collect memory information
    fn collect_memory_info(&self) -> Result<(u64, u64), io::Error> {
        #[cfg(target_os = "linux")]
        {
            let meminfo_content = fs::read_to_string("/proc/meminfo")?;
            let mut mem_total = 0u64;
            let mut mem_available = 0u64;

            for line in meminfo_content.lines() {
                if line.starts_with("MemTotal:") {
                    mem_total = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0) * 1024; // Convert from KB to bytes
                } else if line.starts_with("MemAvailable:") {
                    mem_available = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0) * 1024; // Convert from KB to bytes
                }
            }

            let mem_used = mem_total.saturating_sub(mem_available);
            Ok((mem_used, mem_total))
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Fallback for non-Linux systems
            Ok((0, 0))
        }
    }

    /// Collect disk information
    fn collect_disk_info(&self) -> Result<(u64, u64), io::Error> {
        #[cfg(target_os = "linux")]
        {
            // Simple fallback implementation using /proc/mounts and df-like logic
            // In a production environment, you might want to use a proper system library
            let mounts_content = fs::read_to_string("/proc/mounts").unwrap_or_default();

            // Find root filesystem
            for line in mounts_content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[1] == "/" {
                    // For simplicity, return placeholder values
                    // In production, implement proper statvfs equivalent
                    return Ok((50 * 1024 * 1024 * 1024, 100 * 1024 * 1024 * 1024)); // 50GB used, 100GB total
                }
            }

            Ok((0, 0))
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Fallback for non-Linux systems
            Ok((0, 0))
        }
    }

    /// Collect network statistics
    fn collect_network_stats(&self) -> Result<(u64, u64), io::Error> {
        #[cfg(target_os = "linux")]
        {
            let net_dev_content = fs::read_to_string("/proc/net/dev")?;
            let mut total_rx_bytes = 0u64;
            let mut total_tx_bytes = 0u64;

            for line in net_dev_content.lines().skip(2) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 {
                    let interface = parts[0].trim_end_matches(':');

                    // Skip loopback interface
                    if interface == "lo" {
                        continue;
                    }

                    if let (Ok(rx_bytes), Ok(tx_bytes)) = (
                        parts[1].parse::<u64>(),
                        parts[9].parse::<u64>(),
                    ) {
                        total_rx_bytes += rx_bytes;
                        total_tx_bytes += tx_bytes;
                    }
                }
            }

            Ok((total_rx_bytes, total_tx_bytes))
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Fallback for non-Linux systems
            Ok((0, 0))
        }
    }

    /// Collect load average
    fn collect_load_average(&self) -> Result<(f64, f64, f64), io::Error> {
        #[cfg(target_os = "linux")]
        {
            let loadavg_content = fs::read_to_string("/proc/loadavg")?;
            let parts: Vec<&str> = loadavg_content.split_whitespace().collect();

            if parts.len() >= 3 {
                let load_1m = parts[0].parse().unwrap_or(0.0);
                let load_5m = parts[1].parse().unwrap_or(0.0);
                let load_15m = parts[2].parse().unwrap_or(0.0);

                Ok((load_1m, load_5m, load_15m))
            } else {
                Ok((0.0, 0.0, 0.0))
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Fallback for non-Linux systems
            Ok((0.0, 0.0, 0.0))
        }
    }
}

impl Default for SystemMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Application metrics collector
pub struct ApplicationMetricsCollector {
    request_times: Arc<RwLock<Vec<Duration>>>,
    request_counts: Arc<RwLock<RequestCounts>>,
    last_collection: Arc<RwLock<Instant>>,
}

/// Request count statistics
#[derive(Debug, Clone, Default)]
struct RequestCounts {
    total: u64,
    successful: u64,
    failed: u64,
}

impl ApplicationMetricsCollector {
    /// Create a new application metrics collector
    pub fn new() -> Self {
        debug!("Creating application metrics collector");
        Self {
            request_times: Arc::new(RwLock::new(Vec::new())),
            request_counts: Arc::new(RwLock::new(RequestCounts::default())),
            last_collection: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Record a request
    pub fn record_request(&self, duration: Duration, success: bool) {
        let mut request_times = self.request_times.write().unwrap();
        let mut request_counts = self.request_counts.write().unwrap();

        request_times.push(duration);
        request_counts.total += 1;

        if success {
            request_counts.successful += 1;
        } else {
            request_counts.failed += 1;
        }

        // Keep only recent request times (last 1000 requests)
        if request_times.len() > 1000 {
            request_times.drain(0..100);
        }
    }

    /// Collect current application metrics
    pub fn collect(&self) -> ApplicationMetrics {
        debug!("Collecting application metrics");

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let request_times = self.request_times.read().unwrap();
        let request_counts = self.request_counts.read().unwrap();
        let mut last_collection = self.last_collection.write().unwrap();

        let time_since_last = last_collection.elapsed();
        let requests_per_second = if time_since_last.as_secs() > 0 {
            request_counts.total as f64 / time_since_last.as_secs_f64()
        } else {
            0.0
        };

        let (avg_response_time, p95_response_time, p99_response_time) =
            self.calculate_response_time_percentiles(&request_times);

        *last_collection = Instant::now();

        ApplicationMetrics {
            timestamp,
            total_requests: request_counts.total,
            successful_requests: request_counts.successful,
            failed_requests: request_counts.failed,
            avg_response_time,
            p95_response_time,
            p99_response_time,
            requests_per_second,
            active_sessions: 0, // Will be set by application
            queue_sizes: HashMap::new(), // Will be set by application
            cache_hit_rates: HashMap::new(), // Will be set by application
            db_pool_stats: DatabasePoolStats::default(), // Will be set by application
        }
    }

    /// Calculate response time percentiles
    fn calculate_response_time_percentiles(&self, request_times: &[Duration]) -> (f64, f64, f64) {
        if request_times.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let mut times: Vec<f64> = request_times
            .iter()
            .map(|d| d.as_millis() as f64)
            .collect();

        times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let avg = times.iter().sum::<f64>() / times.len() as f64;

        let p95_index = ((times.len() as f64 * 0.95) as usize).min(times.len() - 1);
        let p99_index = ((times.len() as f64 * 0.99) as usize).min(times.len() - 1);

        let p95 = times[p95_index];
        let p99 = times[p99_index];

        (avg, p95, p99)
    }

    /// Reset counters
    pub fn reset(&self) {
        let mut request_times = self.request_times.write().unwrap();
        let mut request_counts = self.request_counts.write().unwrap();

        request_times.clear();
        *request_counts = RequestCounts::default();
    }
}

impl Default for ApplicationMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Custom metrics collector
pub struct CustomMetricsCollector {
    metrics: Arc<RwLock<HashMap<String, f64>>>,
    counters: Arc<RwLock<HashMap<String, u64>>>,
    histograms: Arc<RwLock<HashMap<String, Vec<f64>>>>,
}

impl CustomMetricsCollector {
    /// Create a new custom metrics collector
    pub fn new() -> Self {
        debug!("Creating custom metrics collector");
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            counters: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set a gauge metric
    pub fn set_gauge(&self, name: String, value: f64) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.insert(name, value);
    }

    /// Increment a counter metric
    pub fn increment_counter(&self, name: String, value: u64) {
        let mut counters = self.counters.write().unwrap();
        *counters.entry(name).or_insert(0) += value;
    }

    /// Record a histogram value
    pub fn record_histogram(&self, name: String, value: f64) {
        let mut histograms = self.histograms.write().unwrap();
        histograms.entry(name).or_insert_with(Vec::new).push(value);
    }

    /// Get all metrics
    pub fn get_all_metrics(&self) -> HashMap<String, f64> {
        let metrics = self.metrics.read().unwrap();
        let counters = self.counters.read().unwrap();
        let histograms = self.histograms.read().unwrap();

        let mut all_metrics = metrics.clone();

        // Add counters
        for (name, value) in counters.iter() {
            all_metrics.insert(name.clone(), *value as f64);
        }

        // Add histogram summaries
        for (name, values) in histograms.iter() {
            if !values.is_empty() {
                let sum: f64 = values.iter().sum();
                let count = values.len() as f64;
                let avg = sum / count;

                all_metrics.insert(format!("{}_sum", name), sum);
                all_metrics.insert(format!("{}_count", name), count);
                all_metrics.insert(format!("{}_avg", name), avg);
            }
        }

        all_metrics
    }

    /// Reset all metrics
    pub fn reset(&self) {
        let mut metrics = self.metrics.write().unwrap();
        let mut counters = self.counters.write().unwrap();
        let mut histograms = self.histograms.write().unwrap();

        metrics.clear();
        counters.clear();
        histograms.clear();
    }
}

impl Default for CustomMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_metrics_collector() {
        let collector = SystemMetricsCollector::new();
        let metrics = collector.collect();

        // Basic validation
        assert!(metrics.timestamp > 0);
        assert!(metrics.uptime >= 0);
    }

    #[test]
    fn test_application_metrics_collector() {
        let collector = ApplicationMetricsCollector::new();

        // Record some requests
        collector.record_request(Duration::from_millis(100), true);
        collector.record_request(Duration::from_millis(200), true);
        collector.record_request(Duration::from_millis(300), false);

        let metrics = collector.collect();

        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.successful_requests, 2);
        assert_eq!(metrics.failed_requests, 1);
        assert!(metrics.avg_response_time > 0.0);
    }

    #[test]
    fn test_custom_metrics_collector() {
        let collector = CustomMetricsCollector::new();

        collector.set_gauge("cpu_usage".to_string(), 75.5);
        collector.increment_counter("requests_total".to_string(), 10);
        collector.record_histogram("response_time".to_string(), 123.45);

        let metrics = collector.get_all_metrics();

        assert_eq!(metrics.get("cpu_usage"), Some(&75.5));
        assert_eq!(metrics.get("requests_total"), Some(&10.0));
        assert_eq!(metrics.get("response_time_count"), Some(&1.0));
        assert_eq!(metrics.get("response_time_avg"), Some(&123.45));
    }

    #[test]
    fn test_response_time_percentiles() {
        let collector = ApplicationMetricsCollector::new();

        // Record requests with known response times
        for i in 1..=100 {
            collector.record_request(Duration::from_millis(i * 10), true);
        }

        let request_times = collector.request_times.read().unwrap();
        let (avg, p95, p99) = collector.calculate_response_time_percentiles(&request_times);

        assert!(avg > 0.0);
        assert!(p95 > avg);
        assert!(p99 > p95);
    }
}
