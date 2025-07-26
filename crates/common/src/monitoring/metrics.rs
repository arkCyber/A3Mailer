/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Metrics Module
//!
//! This module provides comprehensive metrics collection and export capabilities
//! including Prometheus-compatible metrics, custom metrics, and aggregation.

use super::MetricType;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};
use serde::{Serialize, Deserialize};
use tracing::{debug, info};

/// Metric value types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(HistogramData),
    Summary(SummaryData),
}

/// Histogram data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistogramData {
    pub buckets: Vec<HistogramBucket>,
    pub sum: f64,
    pub count: u64,
}

/// Histogram bucket
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistogramBucket {
    pub upper_bound: f64,
    pub count: u64,
}

/// Summary data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SummaryData {
    pub quantiles: Vec<Quantile>,
    pub sum: f64,
    pub count: u64,
}

/// Quantile data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quantile {
    pub quantile: f64,
    pub value: f64,
}

/// Metric definition
#[derive(Debug, Clone)]
pub struct MetricDefinition {
    pub name: String,
    pub description: String,
    pub metric_type: MetricType,
    pub labels: Vec<String>,
    pub unit: Option<String>,
}

/// Metric sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSample {
    pub name: String,
    pub labels: HashMap<String, String>,
    pub value: MetricValue,
    pub timestamp: u64,
}

impl MetricSample {
    /// Create a new metric sample
    pub fn new(name: String, value: MetricValue) -> Self {
        Self {
            name,
            labels: HashMap::new(),
            value,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Add a label
    pub fn with_label(mut self, key: String, value: String) -> Self {
        self.labels.insert(key, value);
        self
    }

    /// Add multiple labels
    pub fn with_labels(mut self, labels: HashMap<String, String>) -> Self {
        self.labels.extend(labels);
        self
    }
}

/// Metrics registry
pub struct MetricsRegistry {
    definitions: Arc<RwLock<HashMap<String, MetricDefinition>>>,
    samples: Arc<RwLock<HashMap<String, MetricSample>>>,
    counters: Arc<RwLock<HashMap<String, u64>>>,
    gauges: Arc<RwLock<HashMap<String, f64>>>,
    histograms: Arc<RwLock<HashMap<String, HistogramCollector>>>,
    summaries: Arc<RwLock<HashMap<String, SummaryCollector>>>,
}

impl MetricsRegistry {
    /// Create a new metrics registry
    pub fn new() -> Self {
        info!("Creating metrics registry");
        Self {
            definitions: Arc::new(RwLock::new(HashMap::new())),
            samples: Arc::new(RwLock::new(HashMap::new())),
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
            summaries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a metric definition
    pub fn register_metric(&self, definition: MetricDefinition) {
        debug!("Registering metric: {}", definition.name);
        let mut definitions = self.definitions.write().unwrap();
        definitions.insert(definition.name.clone(), definition);
    }

    /// Increment a counter
    pub fn increment_counter(&self, name: &str, value: u64) {
        self.increment_counter_with_labels(name, value, HashMap::new());
    }

    /// Increment a counter with labels
    pub fn increment_counter_with_labels(&self, name: &str, value: u64, labels: HashMap<String, String>) {
        debug!("Incrementing counter: {} by {}", name, value);

        let key = self.make_metric_key(name, &labels);
        let mut counters = self.counters.write().unwrap();
        *counters.entry(key.clone()).or_insert(0) += value;

        // Update sample
        let sample = MetricSample::new(name.to_string(), MetricValue::Counter(*counters.get(&key).unwrap()))
            .with_labels(labels);

        let mut samples = self.samples.write().unwrap();
        samples.insert(key, sample);
    }

    /// Set a gauge value
    pub fn set_gauge(&self, name: &str, value: f64) {
        self.set_gauge_with_labels(name, value, HashMap::new());
    }

    /// Set a gauge value with labels
    pub fn set_gauge_with_labels(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        debug!("Setting gauge: {} to {}", name, value);

        let key = self.make_metric_key(name, &labels);
        let mut gauges = self.gauges.write().unwrap();
        gauges.insert(key.clone(), value);

        // Update sample
        let sample = MetricSample::new(name.to_string(), MetricValue::Gauge(value))
            .with_labels(labels);

        let mut samples = self.samples.write().unwrap();
        samples.insert(key, sample);
    }

    /// Record a histogram value
    pub fn record_histogram(&self, name: &str, value: f64) {
        self.record_histogram_with_labels(name, value, HashMap::new());
    }

    /// Record a histogram value with labels
    pub fn record_histogram_with_labels(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        debug!("Recording histogram: {} value {}", name, value);

        let key = self.make_metric_key(name, &labels);
        let mut histograms = self.histograms.write().unwrap();

        let histogram = histograms.entry(key.clone()).or_insert_with(|| {
            HistogramCollector::new(vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 25.0, 50.0, 100.0])
        });

        histogram.observe(value);

        // Update sample
        let sample = MetricSample::new(name.to_string(), MetricValue::Histogram(histogram.data()))
            .with_labels(labels);

        let mut samples = self.samples.write().unwrap();
        samples.insert(key, sample);
    }

    /// Record a summary value
    pub fn record_summary(&self, name: &str, value: f64) {
        self.record_summary_with_labels(name, value, HashMap::new());
    }

    /// Record a summary value with labels
    pub fn record_summary_with_labels(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        debug!("Recording summary: {} value {}", name, value);

        let key = self.make_metric_key(name, &labels);
        let mut summaries = self.summaries.write().unwrap();

        let summary = summaries.entry(key.clone()).or_insert_with(|| {
            SummaryCollector::new(vec![0.5, 0.9, 0.95, 0.99])
        });

        summary.observe(value);

        // Update sample
        let sample = MetricSample::new(name.to_string(), MetricValue::Summary(summary.data()))
            .with_labels(labels);

        let mut samples = self.samples.write().unwrap();
        samples.insert(key, sample);
    }

    /// Get all metric samples
    pub fn get_all_samples(&self) -> Vec<MetricSample> {
        let samples = self.samples.read().unwrap();
        samples.values().cloned().collect()
    }

    /// Get samples for a specific metric
    pub fn get_metric_samples(&self, name: &str) -> Vec<MetricSample> {
        let samples = self.samples.read().unwrap();
        samples
            .values()
            .filter(|sample| sample.name == name)
            .cloned()
            .collect()
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();
        let definitions = self.definitions.read().unwrap();
        let samples = self.samples.read().unwrap();

        for sample in samples.values() {
            if let Some(definition) = definitions.get(&sample.name) {
                // Add HELP and TYPE comments
                output.push_str(&format!("# HELP {} {}\n", sample.name, definition.description));
                output.push_str(&format!("# TYPE {} {}\n", sample.name,
                    match definition.metric_type {
                        MetricType::Counter => "counter",
                        MetricType::Gauge => "gauge",
                        MetricType::Histogram => "histogram",
                        MetricType::Summary => "summary",
                    }
                ));
            }

            // Add metric line
            let labels_str = if sample.labels.is_empty() {
                String::new()
            } else {
                let labels: Vec<String> = sample.labels
                    .iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect();
                format!("{{{}}}", labels.join(","))
            };

            match &sample.value {
                MetricValue::Counter(value) => {
                    output.push_str(&format!("{}{} {}\n", sample.name, labels_str, value));
                }
                MetricValue::Gauge(value) => {
                    output.push_str(&format!("{}{} {}\n", sample.name, labels_str, value));
                }
                MetricValue::Histogram(data) => {
                    for bucket in &data.buckets {
                        let bucket_labels = if sample.labels.is_empty() {
                            format!("{{le=\"{}\"}}", bucket.upper_bound)
                        } else {
                            let mut labels = sample.labels.clone();
                            labels.insert("le".to_string(), bucket.upper_bound.to_string());
                            let labels_vec: Vec<String> = labels
                                .iter()
                                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                                .collect();
                            format!("{{{}}}", labels_vec.join(","))
                        };
                        output.push_str(&format!("{}_bucket{} {}\n", sample.name, bucket_labels, bucket.count));
                    }
                    output.push_str(&format!("{}_sum{} {}\n", sample.name, labels_str, data.sum));
                    output.push_str(&format!("{}_count{} {}\n", sample.name, labels_str, data.count));
                }
                MetricValue::Summary(data) => {
                    for quantile in &data.quantiles {
                        let quantile_labels = if sample.labels.is_empty() {
                            format!("{{quantile=\"{}\"}}", quantile.quantile)
                        } else {
                            let mut labels = sample.labels.clone();
                            labels.insert("quantile".to_string(), quantile.quantile.to_string());
                            let labels_vec: Vec<String> = labels
                                .iter()
                                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                                .collect();
                            format!("{{{}}}", labels_vec.join(","))
                        };
                        output.push_str(&format!("{}{} {}\n", sample.name, quantile_labels, quantile.value));
                    }
                    output.push_str(&format!("{}_sum{} {}\n", sample.name, labels_str, data.sum));
                    output.push_str(&format!("{}_count{} {}\n", sample.name, labels_str, data.count));
                }
            }
        }

        output
    }

    /// Make a unique key for a metric with labels
    fn make_metric_key(&self, name: &str, labels: &HashMap<String, String>) -> String {
        if labels.is_empty() {
            name.to_string()
        } else {
            let mut sorted_labels: Vec<_> = labels.iter().collect();
            sorted_labels.sort_by_key(|(k, _)| *k);
            let labels_str: Vec<String> = sorted_labels
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            format!("{}[{}]", name, labels_str.join(","))
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        info!("Resetting all metrics");

        let mut counters = self.counters.write().unwrap();
        let mut gauges = self.gauges.write().unwrap();
        let mut histograms = self.histograms.write().unwrap();
        let mut summaries = self.summaries.write().unwrap();
        let mut samples = self.samples.write().unwrap();

        counters.clear();
        gauges.clear();
        histograms.clear();
        summaries.clear();
        samples.clear();
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Histogram collector
pub struct HistogramCollector {
    buckets: Vec<f64>,
    counts: Vec<u64>,
    sum: f64,
    count: u64,
}

impl HistogramCollector {
    /// Create a new histogram collector
    pub fn new(buckets: Vec<f64>) -> Self {
        let counts = vec![0; buckets.len()];
        Self {
            buckets,
            counts,
            sum: 0.0,
            count: 0,
        }
    }

    /// Observe a value
    pub fn observe(&mut self, value: f64) {
        self.sum += value;
        self.count += 1;

        for (i, &bucket) in self.buckets.iter().enumerate() {
            if value <= bucket {
                self.counts[i] += 1;
            }
        }
    }

    /// Get histogram data
    pub fn data(&self) -> HistogramData {
        let buckets = self.buckets
            .iter()
            .zip(self.counts.iter())
            .map(|(&upper_bound, &count)| HistogramBucket { upper_bound, count })
            .collect();

        HistogramData {
            buckets,
            sum: self.sum,
            count: self.count,
        }
    }
}

/// Summary collector
pub struct SummaryCollector {
    quantiles: Vec<f64>,
    values: Vec<f64>,
    sum: f64,
    count: u64,
}

impl SummaryCollector {
    /// Create a new summary collector
    pub fn new(quantiles: Vec<f64>) -> Self {
        Self {
            quantiles,
            values: Vec::new(),
            sum: 0.0,
            count: 0,
        }
    }

    /// Observe a value
    pub fn observe(&mut self, value: f64) {
        self.values.push(value);
        self.sum += value;
        self.count += 1;

        // Keep only recent values to prevent memory growth
        if self.values.len() > 10000 {
            self.values.drain(0..1000);
        }
    }

    /// Get summary data
    pub fn data(&self) -> SummaryData {
        if self.values.is_empty() {
            return SummaryData {
                quantiles: self.quantiles
                    .iter()
                    .map(|&q| Quantile { quantile: q, value: 0.0 })
                    .collect(),
                sum: self.sum,
                count: self.count,
            };
        }

        let mut sorted_values = self.values.clone();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let quantiles = self.quantiles
            .iter()
            .map(|&q| {
                let index = ((sorted_values.len() as f64 * q) as usize).min(sorted_values.len() - 1);
                let value = sorted_values.get(index).copied().unwrap_or(0.0);
                Quantile { quantile: q, value }
            })
            .collect();

        SummaryData {
            quantiles,
            sum: self.sum,
            count: self.count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registry() {
        let registry = MetricsRegistry::new();

        // Register a counter
        registry.register_metric(MetricDefinition {
            name: "test_counter".to_string(),
            description: "A test counter".to_string(),
            metric_type: MetricType::Counter,
            labels: vec!["method".to_string()],
            unit: None,
        });

        // Increment counter
        let mut labels = HashMap::new();
        labels.insert("method".to_string(), "GET".to_string());
        registry.increment_counter_with_labels("test_counter", 5, labels);

        // Check samples
        let samples = registry.get_metric_samples("test_counter");
        assert_eq!(samples.len(), 1);

        if let MetricValue::Counter(value) = &samples[0].value {
            assert_eq!(*value, 5);
        } else {
            panic!("Expected counter value");
        }
    }

    #[test]
    fn test_histogram_collector() {
        let mut histogram = HistogramCollector::new(vec![1.0, 5.0, 10.0]);

        histogram.observe(0.5);
        histogram.observe(2.0);
        histogram.observe(7.0);

        let data = histogram.data();
        assert_eq!(data.count, 3);
        assert_eq!(data.sum, 9.5);
        assert_eq!(data.buckets[0].count, 1); // <= 1.0
        assert_eq!(data.buckets[1].count, 2); // <= 5.0
        assert_eq!(data.buckets[2].count, 3); // <= 10.0
    }

    #[test]
    fn test_prometheus_export() {
        let registry = MetricsRegistry::new();

        registry.register_metric(MetricDefinition {
            name: "http_requests_total".to_string(),
            description: "Total HTTP requests".to_string(),
            metric_type: MetricType::Counter,
            labels: vec!["method".to_string(), "status".to_string()],
            unit: None,
        });

        let mut labels = HashMap::new();
        labels.insert("method".to_string(), "GET".to_string());
        labels.insert("status".to_string(), "200".to_string());
        registry.increment_counter_with_labels("http_requests_total", 42, labels);

        let prometheus_output = registry.export_prometheus();
        println!("Prometheus output: {}", prometheus_output);
        assert!(prometheus_output.contains("# HELP http_requests_total Total HTTP requests"));
        assert!(prometheus_output.contains("# TYPE http_requests_total counter"));
        // The label order might be different, so check for both possible orders
        assert!(prometheus_output.contains("http_requests_total{method=\"GET\",status=\"200\"} 42") ||
                prometheus_output.contains("http_requests_total{status=\"200\",method=\"GET\"} 42"));
    }
}
