//! Metrics Collection for A3Mailer
//!
//! This module provides comprehensive metrics collection compatible with
//! Prometheus and other monitoring systems.

use crate::{MonitoringConfig, Result, MonitoringError};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};

/// Metric types supported by the system
#[derive(Debug, Clone, PartialEq)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// Metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub name: String,
    pub metric_type: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: u64,
    pub help: String,
}

/// Histogram bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    pub upper_bound: f64,
    pub count: u64,
}

/// Histogram metric data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramMetric {
    pub name: String,
    pub buckets: Vec<HistogramBucket>,
    pub count: u64,
    pub sum: f64,
    pub labels: HashMap<String, String>,
}

/// Counter metric data
#[derive(Debug, Clone)]
pub struct CounterMetric {
    pub name: String,
    pub value: Arc<RwLock<f64>>,
    pub labels: HashMap<String, String>,
    pub help: String,
}

/// Gauge metric data
#[derive(Debug, Clone)]
pub struct GaugeMetric {
    pub name: String,
    pub value: Arc<RwLock<f64>>,
    pub labels: HashMap<String, String>,
    pub help: String,
}

/// Metrics collector for Prometheus-compatible metrics
pub struct MetricsCollector {
    config: MonitoringConfig,
    counters: Arc<RwLock<HashMap<String, CounterMetric>>>,
    gauges: Arc<RwLock<HashMap<String, GaugeMetric>>>,
    histograms: Arc<RwLock<HashMap<String, HistogramMetric>>>,
    start_time: Instant,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub async fn new(config: &MonitoringConfig) -> Result<Self> {
        info!("Initializing metrics collector");
        
        let collector = Self {
            config: config.clone(),
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        };
        
        // Initialize default metrics
        collector.initialize_default_metrics().await?;
        
        info!("Metrics collector initialized successfully");
        Ok(collector)
    }

    /// Increment a counter metric
    pub async fn increment_counter(&self, name: &str, labels: &[(&str, &str)]) -> Result<()> {
        let metric_key = self.create_metric_key(name, labels);
        
        let mut counters = self.counters.write().await;
        
        if let Some(counter) = counters.get(&metric_key) {
            let mut value = counter.value.write().await;
            *value += 1.0;
        } else {
            // Create new counter
            let counter = CounterMetric {
                name: name.to_string(),
                value: Arc::new(RwLock::new(1.0)),
                labels: labels.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
                help: format!("Counter metric: {}", name),
            };
            counters.insert(metric_key, counter);
        }
        
        debug!("Incremented counter: {} with labels: {:?}", name, labels);
        Ok(())
    }

    /// Add value to a counter metric
    pub async fn add_to_counter(&self, name: &str, value: f64, labels: &[(&str, &str)]) -> Result<()> {
        let metric_key = self.create_metric_key(name, labels);
        
        let mut counters = self.counters.write().await;
        
        if let Some(counter) = counters.get(&metric_key) {
            let mut counter_value = counter.value.write().await;
            *counter_value += value;
        } else {
            // Create new counter
            let counter = CounterMetric {
                name: name.to_string(),
                value: Arc::new(RwLock::new(value)),
                labels: labels.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
                help: format!("Counter metric: {}", name),
            };
            counters.insert(metric_key, counter);
        }
        
        debug!("Added {} to counter: {} with labels: {:?}", value, name, labels);
        Ok(())
    }

    /// Set a gauge metric value
    pub async fn set_gauge(&self, name: &str, value: f64, labels: &[(&str, &str)]) -> Result<()> {
        let metric_key = self.create_metric_key(name, labels);
        
        let mut gauges = self.gauges.write().await;
        
        if let Some(gauge) = gauges.get(&metric_key) {
            let mut gauge_value = gauge.value.write().await;
            *gauge_value = value;
        } else {
            // Create new gauge
            let gauge = GaugeMetric {
                name: name.to_string(),
                value: Arc::new(RwLock::new(value)),
                labels: labels.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
                help: format!("Gauge metric: {}", name),
            };
            gauges.insert(metric_key, gauge);
        }
        
        debug!("Set gauge: {} = {} with labels: {:?}", name, value, labels);
        Ok(())
    }

    /// Record a histogram observation
    pub async fn record_histogram(&self, name: &str, value: f64, labels: &[(&str, &str)]) -> Result<()> {
        let metric_key = self.create_metric_key(name, labels);
        
        let mut histograms = self.histograms.write().await;
        
        if let Some(histogram) = histograms.get_mut(&metric_key) {
            // Update existing histogram
            histogram.count += 1;
            histogram.sum += value;
            
            // Update buckets
            for bucket in &mut histogram.buckets {
                if value <= bucket.upper_bound {
                    bucket.count += 1;
                }
            }
        } else {
            // Create new histogram with default buckets
            let buckets = self.create_default_buckets(value);
            let histogram = HistogramMetric {
                name: name.to_string(),
                buckets,
                count: 1,
                sum: value,
                labels: labels.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            };
            histograms.insert(metric_key, histogram);
        }
        
        debug!("Recorded histogram: {} = {} with labels: {:?}", name, value, labels);
        Ok(())
    }

    /// Get all metrics in Prometheus format
    pub async fn get_prometheus_metrics(&self) -> Result<String> {
        let mut output = String::new();
        
        // Add counters
        let counters = self.counters.read().await;
        for counter in counters.values() {
            output.push_str(&format!("# HELP {} {}\n", counter.name, counter.help));
            output.push_str(&format!("# TYPE {} counter\n", counter.name));
            
            let value = *counter.value.read().await;
            let labels_str = self.format_labels(&counter.labels);
            output.push_str(&format!("{}{} {}\n", counter.name, labels_str, value));
        }
        
        // Add gauges
        let gauges = self.gauges.read().await;
        for gauge in gauges.values() {
            output.push_str(&format!("# HELP {} {}\n", gauge.name, gauge.help));
            output.push_str(&format!("# TYPE {} gauge\n", gauge.name));
            
            let value = *gauge.value.read().await;
            let labels_str = self.format_labels(&gauge.labels);
            output.push_str(&format!("{}{} {}\n", gauge.name, labels_str, value));
        }
        
        // Add histograms
        let histograms = self.histograms.read().await;
        for histogram in histograms.values() {
            output.push_str(&format!("# HELP {} Histogram metric\n", histogram.name));
            output.push_str(&format!("# TYPE {} histogram\n", histogram.name));
            
            let labels_str = self.format_labels(&histogram.labels);
            
            // Histogram buckets
            for bucket in &histogram.buckets {
                let mut bucket_labels = histogram.labels.clone();
                bucket_labels.insert("le".to_string(), bucket.upper_bound.to_string());
                let bucket_labels_str = self.format_labels(&bucket_labels);
                output.push_str(&format!("{}_bucket{} {}\n", 
                                        histogram.name, bucket_labels_str, bucket.count));
            }
            
            // Histogram count and sum
            output.push_str(&format!("{}_count{} {}\n", histogram.name, labels_str, histogram.count));
            output.push_str(&format!("{}_sum{} {}\n", histogram.name, labels_str, histogram.sum));
        }
        
        debug!("Generated Prometheus metrics ({} bytes)", output.len());
        Ok(output)
    }

    /// Get metrics as JSON
    pub async fn get_json_metrics(&self) -> Result<String> {
        let mut metrics = Vec::new();
        
        // Collect counters
        let counters = self.counters.read().await;
        for counter in counters.values() {
            let value = *counter.value.read().await;
            metrics.push(MetricPoint {
                name: counter.name.clone(),
                metric_type: "counter".to_string(),
                value,
                labels: counter.labels.clone(),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH)
                    .unwrap_or_default().as_secs(),
                help: counter.help.clone(),
            });
        }
        
        // Collect gauges
        let gauges = self.gauges.read().await;
        for gauge in gauges.values() {
            let value = *gauge.value.read().await;
            metrics.push(MetricPoint {
                name: gauge.name.clone(),
                metric_type: "gauge".to_string(),
                value,
                labels: gauge.labels.clone(),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH)
                    .unwrap_or_default().as_secs(),
                help: gauge.help.clone(),
            });
        }
        
        // Collect histograms (simplified)
        let histograms = self.histograms.read().await;
        for histogram in histograms.values() {
            metrics.push(MetricPoint {
                name: format!("{}_count", histogram.name),
                metric_type: "histogram".to_string(),
                value: histogram.count as f64,
                labels: histogram.labels.clone(),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH)
                    .unwrap_or_default().as_secs(),
                help: "Histogram count".to_string(),
            });
            
            metrics.push(MetricPoint {
                name: format!("{}_sum", histogram.name),
                metric_type: "histogram".to_string(),
                value: histogram.sum,
                labels: histogram.labels.clone(),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH)
                    .unwrap_or_default().as_secs(),
                help: "Histogram sum".to_string(),
            });
        }
        
        serde_json::to_string_pretty(&metrics)
            .map_err(|e| MonitoringError::SerializationError(e.to_string()))
    }

    /// Initialize default system metrics
    async fn initialize_default_metrics(&self) -> Result<()> {
        debug!("Initializing default metrics");
        
        // System uptime
        self.set_gauge("a3mailer_uptime_seconds", 0.0, &[]).await?;
        
        // Email processing metrics
        self.increment_counter("a3mailer_emails_processed_total", &[("protocol", "smtp")]).await?;
        self.increment_counter("a3mailer_emails_processed_total", &[("protocol", "imap")]).await?;
        self.increment_counter("a3mailer_emails_processed_total", &[("protocol", "pop3")]).await?;
        
        // AI metrics
        self.record_histogram("a3mailer_ai_inference_duration_ms", 0.0, &[("model", "threat_detection")]).await?;
        self.record_histogram("a3mailer_ai_inference_duration_ms", 0.0, &[("model", "content_analysis")]).await?;
        
        // Web3 metrics
        self.record_histogram("a3mailer_web3_operation_duration_ms", 0.0, &[("operation", "did_resolution")]).await?;
        self.record_histogram("a3mailer_web3_operation_duration_ms", 0.0, &[("operation", "ipfs_storage")]).await?;
        
        // Connection metrics
        self.set_gauge("a3mailer_active_connections", 0.0, &[]).await?;
        
        info!("Default metrics initialized");
        Ok(())
    }

    /// Create metric key from name and labels
    fn create_metric_key(&self, name: &str, labels: &[(&str, &str)]) -> String {
        let mut key = name.to_string();
        if !labels.is_empty() {
            key.push('{');
            for (i, (k, v)) in labels.iter().enumerate() {
                if i > 0 {
                    key.push(',');
                }
                key.push_str(&format!("{}=\"{}\"", k, v));
            }
            key.push('}');
        }
        key
    }

    /// Format labels for Prometheus output
    fn format_labels(&self, labels: &HashMap<String, String>) -> String {
        if labels.is_empty() {
            return String::new();
        }
        
        let mut formatted = String::from("{");
        let mut first = true;
        for (key, value) in labels {
            if !first {
                formatted.push(',');
            }
            formatted.push_str(&format!("{}=\"{}\"", key, value));
            first = false;
        }
        formatted.push('}');
        formatted
    }

    /// Create default histogram buckets
    fn create_default_buckets(&self, initial_value: f64) -> Vec<HistogramBucket> {
        let bounds = vec![0.001, 0.01, 0.1, 1.0, 10.0, 100.0, 1000.0, 10000.0, f64::INFINITY];
        
        bounds.into_iter().map(|bound| {
            HistogramBucket {
                upper_bound: bound,
                count: if initial_value <= bound { 1 } else { 0 },
            }
        }).collect()
    }

    /// Update system metrics
    pub async fn update_system_metrics(&self) -> Result<()> {
        // Update uptime
        let uptime = self.start_time.elapsed().as_secs() as f64;
        self.set_gauge("a3mailer_uptime_seconds", uptime, &[]).await?;
        
        // Update memory usage (simplified)
        let memory_usage = self.get_memory_usage().await?;
        self.set_gauge("a3mailer_memory_usage_bytes", memory_usage, &[]).await?;
        
        // Update CPU usage (simplified)
        let cpu_usage = self.get_cpu_usage().await?;
        self.set_gauge("a3mailer_cpu_usage_percent", cpu_usage, &[]).await?;
        
        debug!("System metrics updated");
        Ok(())
    }

    /// Get memory usage (simplified implementation)
    async fn get_memory_usage(&self) -> Result<f64> {
        // In a real implementation, this would use system APIs
        // For now, return a mock value
        Ok(1024.0 * 1024.0 * 100.0) // 100MB
    }

    /// Get CPU usage (simplified implementation)
    async fn get_cpu_usage(&self) -> Result<f64> {
        // In a real implementation, this would calculate actual CPU usage
        // For now, return a mock value
        Ok(15.5) // 15.5%
    }

    /// Get metrics collector statistics
    pub async fn get_stats(&self) -> Result<HashMap<String, String>> {
        let mut stats = HashMap::new();
        
        let counters_count = self.counters.read().await.len();
        let gauges_count = self.gauges.read().await.len();
        let histograms_count = self.histograms.read().await.len();
        
        stats.insert("counters_count".to_string(), counters_count.to_string());
        stats.insert("gauges_count".to_string(), gauges_count.to_string());
        stats.insert("histograms_count".to_string(), histograms_count.to_string());
        stats.insert("total_metrics".to_string(), (counters_count + gauges_count + histograms_count).to_string());
        
        Ok(stats)
    }

    /// Shutdown metrics collector
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down metrics collector");
        
        // Clear all metrics
        self.counters.write().await.clear();
        self.gauges.write().await.clear();
        self.histograms.write().await.clear();
        
        info!("Metrics collector shutdown complete");
        Ok(())
    }
}
