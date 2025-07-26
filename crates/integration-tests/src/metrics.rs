/*!
 * Metrics Collection and Analysis Module
 * 
 * This module provides comprehensive metrics collection, analysis, and reporting
 * for integration tests, including performance metrics, resource usage, and
 * test execution statistics.
 * 
 * Features:
 * - Real-time metrics collection
 * - Performance analysis
 * - Resource usage monitoring
 * - Statistical analysis
 * - Report generation
 * - Metrics visualization data
 * 
 * Author: Stalwart Labs Ltd.
 * Created: 2024-07-26
 */

use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::{TestResult, PerformanceMetrics, Result};

/// Metrics collector for integration tests
pub struct MetricsCollector {
    metrics: Arc<RwLock<TestMetrics>>,
    start_time: Instant,
    collection_interval: Duration,
}

/// Comprehensive test metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMetrics {
    /// Test execution metrics
    pub execution: ExecutionMetrics,
    /// Performance metrics
    pub performance: PerformanceMetrics,
    /// Resource usage metrics
    pub resources: ResourceMetrics,
    /// Protocol-specific metrics
    pub protocols: ProtocolMetrics,
    /// Error and failure metrics
    pub errors: ErrorMetrics,
    /// Custom metrics
    pub custom: HashMap<String, f64>,
}

/// Test execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// Total tests executed
    pub total_tests: u64,
    /// Successful tests
    pub successful_tests: u64,
    /// Failed tests
    pub failed_tests: u64,
    /// Skipped tests
    pub skipped_tests: u64,
    /// Test execution rate (tests per second)
    pub execution_rate: f64,
    /// Average test duration
    pub avg_test_duration: Duration,
    /// Total execution time
    pub total_execution_time: Duration,
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// Memory usage statistics
    pub memory: MemoryMetrics,
    /// CPU usage statistics
    pub cpu: CpuMetrics,
    /// Network usage statistics
    pub network: NetworkMetrics,
    /// Disk I/O statistics
    pub disk: DiskMetrics,
}

/// Memory usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    /// Current memory usage (bytes)
    pub current_usage: u64,
    /// Peak memory usage (bytes)
    pub peak_usage: u64,
    /// Average memory usage (bytes)
    pub average_usage: u64,
    /// Memory usage samples
    pub samples: Vec<MemorySample>,
}

/// CPU usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMetrics {
    /// Current CPU usage (percentage)
    pub current_usage: f64,
    /// Peak CPU usage (percentage)
    pub peak_usage: f64,
    /// Average CPU usage (percentage)
    pub average_usage: f64,
    /// CPU usage samples
    pub samples: Vec<CpuSample>,
}

/// Network usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total packets sent
    pub packets_sent: u64,
    /// Total packets received
    pub packets_received: u64,
    /// Network throughput (bytes per second)
    pub throughput: f64,
}

/// Disk I/O metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskMetrics {
    /// Total bytes read
    pub bytes_read: u64,
    /// Total bytes written
    pub bytes_written: u64,
    /// Total read operations
    pub read_operations: u64,
    /// Total write operations
    pub write_operations: u64,
    /// Disk I/O rate (operations per second)
    pub io_rate: f64,
}

/// Protocol-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMetrics {
    /// SMTP metrics
    pub smtp: ProtocolStats,
    /// IMAP metrics
    pub imap: ProtocolStats,
    /// POP3 metrics
    pub pop3: ProtocolStats,
    /// JMAP metrics
    pub jmap: ProtocolStats,
}

/// Statistics for a specific protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolStats {
    /// Total operations
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Average response time
    pub avg_response_time: Duration,
    /// Operations per second
    pub ops_per_second: f64,
    /// Error rate (percentage)
    pub error_rate: f64,
}

/// Error and failure metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    /// Total errors
    pub total_errors: u64,
    /// Error rate (percentage)
    pub error_rate: f64,
    /// Error categories
    pub error_categories: HashMap<String, u64>,
    /// Most common errors
    pub common_errors: Vec<ErrorFrequency>,
}

/// Error frequency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorFrequency {
    /// Error message
    pub error: String,
    /// Frequency count
    pub count: u64,
    /// Percentage of total errors
    pub percentage: f64,
}

/// Memory usage sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySample {
    /// Sample timestamp
    pub timestamp: DateTime<Utc>,
    /// Memory usage in bytes
    pub usage: u64,
}

/// CPU usage sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSample {
    /// Sample timestamp
    pub timestamp: DateTime<Utc>,
    /// CPU usage percentage
    pub usage: f64,
}

/// Metrics analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsAnalysis {
    /// Analysis summary
    pub summary: AnalysisSummary,
    /// Performance insights
    pub insights: Vec<PerformanceInsight>,
    /// Recommendations
    pub recommendations: Vec<String>,
    /// Trend analysis
    pub trends: TrendAnalysis,
}

/// Analysis summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSummary {
    /// Overall test success rate
    pub success_rate: f64,
    /// Average performance score
    pub performance_score: f64,
    /// Resource efficiency score
    pub efficiency_score: f64,
    /// Overall grade
    pub overall_grade: String,
}

/// Performance insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceInsight {
    /// Insight category
    pub category: String,
    /// Insight message
    pub message: String,
    /// Severity level
    pub severity: InsightSeverity,
    /// Metric value
    pub value: f64,
    /// Threshold value
    pub threshold: f64,
}

/// Insight severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightSeverity {
    Info,
    Warning,
    Critical,
}

/// Trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    /// Performance trend
    pub performance_trend: TrendDirection,
    /// Resource usage trend
    pub resource_trend: TrendDirection,
    /// Error rate trend
    pub error_trend: TrendDirection,
    /// Throughput trend
    pub throughput_trend: TrendDirection,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
    Unknown,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        info!("Initializing metrics collector");
        
        Self {
            metrics: Arc::new(RwLock::new(TestMetrics::default())),
            start_time: Instant::now(),
            collection_interval: Duration::from_secs(1),
        }
    }

    /// Start metrics collection
    pub async fn start_collection(&self) -> Result<()> {
        info!("Starting metrics collection");
        
        let metrics = self.metrics.clone();
        let interval = self.collection_interval;
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;
                
                // Collect system metrics
                if let Err(e) = Self::collect_system_metrics(&metrics).await {
                    warn!("Failed to collect system metrics: {}", e);
                }
            }
        });
        
        Ok(())
    }

    /// Record test result
    pub async fn record_test_result(&self, result: &TestResult) {
        let mut metrics = self.metrics.write().await;
        
        metrics.execution.total_tests += 1;
        
        if result.success {
            metrics.execution.successful_tests += 1;
        } else {
            metrics.execution.failed_tests += 1;
            
            // Record error
            metrics.errors.total_errors += 1;
            if let Some(error) = &result.error {
                *metrics.errors.error_categories.entry(error.clone()).or_insert(0) += 1;
            }
        }
        
        // Update execution metrics
        let total_time = self.start_time.elapsed();
        metrics.execution.total_execution_time = total_time;
        metrics.execution.execution_rate = metrics.execution.total_tests as f64 / total_time.as_secs_f64();
        
        // Update average test duration
        let total_duration_nanos = metrics.execution.avg_test_duration.as_nanos() * (metrics.execution.total_tests - 1) as u128 + result.duration.as_nanos();
        metrics.execution.avg_test_duration = Duration::from_nanos((total_duration_nanos / metrics.execution.total_tests as u128) as u64);
        
        // Update error rate
        metrics.errors.error_rate = (metrics.errors.total_errors as f64 / metrics.execution.total_tests as f64) * 100.0;
    }

    /// Record protocol operation
    pub async fn record_protocol_operation(&self, protocol: &str, success: bool, duration: Duration) {
        let mut metrics = self.metrics.write().await;
        
        let protocol_stats = match protocol.to_lowercase().as_str() {
            "smtp" => &mut metrics.protocols.smtp,
            "imap" => &mut metrics.protocols.imap,
            "pop3" => &mut metrics.protocols.pop3,
            "jmap" => &mut metrics.protocols.jmap,
            _ => return,
        };
        
        protocol_stats.total_operations += 1;
        
        if success {
            protocol_stats.successful_operations += 1;
        } else {
            protocol_stats.failed_operations += 1;
        }
        
        // Update average response time
        let total_duration_nanos = protocol_stats.avg_response_time.as_nanos() * (protocol_stats.total_operations - 1) as u128 + duration.as_nanos();
        protocol_stats.avg_response_time = Duration::from_nanos((total_duration_nanos / protocol_stats.total_operations as u128) as u64);
        
        // Update error rate
        protocol_stats.error_rate = (protocol_stats.failed_operations as f64 / protocol_stats.total_operations as f64) * 100.0;
        
        // Update operations per second (simplified calculation)
        let elapsed = self.start_time.elapsed();
        protocol_stats.ops_per_second = protocol_stats.total_operations as f64 / elapsed.as_secs_f64();
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> TestMetrics {
        self.metrics.read().await.clone()
    }

    /// Analyze metrics and generate insights
    pub async fn analyze_metrics(&self) -> MetricsAnalysis {
        let metrics = self.get_metrics().await;
        
        // Calculate success rate
        let success_rate = if metrics.execution.total_tests > 0 {
            (metrics.execution.successful_tests as f64 / metrics.execution.total_tests as f64) * 100.0
        } else {
            0.0
        };
        
        // Calculate performance score (simplified)
        let performance_score = if metrics.performance.avg_response_time.as_millis() > 0 {
            (1000.0 / metrics.performance.avg_response_time.as_millis() as f64) * 100.0
        } else {
            100.0
        }.min(100.0);
        
        // Calculate efficiency score (simplified)
        let efficiency_score = if metrics.resources.memory.peak_usage > 0 {
            ((1024.0 * 1024.0 * 1024.0) / metrics.resources.memory.peak_usage as f64) * 100.0
        } else {
            100.0
        }.min(100.0);
        
        // Determine overall grade
        let overall_score = (success_rate + performance_score + efficiency_score) / 3.0;
        let overall_grade = match overall_score {
            90.0..=100.0 => "A",
            80.0..=89.9 => "B",
            70.0..=79.9 => "C",
            60.0..=69.9 => "D",
            _ => "F",
        }.to_string();
        
        // Generate insights
        let mut insights = Vec::new();
        
        if success_rate < 95.0 {
            insights.push(PerformanceInsight {
                category: "Reliability".to_string(),
                message: "Test success rate is below 95%".to_string(),
                severity: InsightSeverity::Warning,
                value: success_rate,
                threshold: 95.0,
            });
        }
        
        if metrics.performance.avg_response_time.as_millis() > 1000 {
            insights.push(PerformanceInsight {
                category: "Performance".to_string(),
                message: "Average response time exceeds 1 second".to_string(),
                severity: InsightSeverity::Critical,
                value: metrics.performance.avg_response_time.as_millis() as f64,
                threshold: 1000.0,
            });
        }
        
        // Generate recommendations
        let mut recommendations = Vec::new();
        
        if success_rate < 90.0 {
            recommendations.push("Investigate and fix failing tests to improve reliability".to_string());
        }
        
        if metrics.performance.avg_response_time.as_millis() > 500 {
            recommendations.push("Optimize performance to reduce response times".to_string());
        }
        
        if metrics.resources.memory.peak_usage > 1024 * 1024 * 1024 {
            recommendations.push("Consider optimizing memory usage to reduce resource consumption".to_string());
        }
        
        MetricsAnalysis {
            summary: AnalysisSummary {
                success_rate,
                performance_score,
                efficiency_score,
                overall_grade,
            },
            insights,
            recommendations,
            trends: TrendAnalysis {
                performance_trend: TrendDirection::Stable,
                resource_trend: TrendDirection::Stable,
                error_trend: TrendDirection::Stable,
                throughput_trend: TrendDirection::Stable,
            },
        }
    }

    /// Generate metrics report
    pub async fn generate_report(&self) -> String {
        let metrics = self.get_metrics().await;
        let analysis = self.analyze_metrics().await;
        
        let mut report = String::new();
        
        report.push_str("# Integration Test Metrics Report\n\n");
        report.push_str(&format!("Generated: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        
        // Executive Summary
        report.push_str("## Executive Summary\n\n");
        report.push_str(&format!("- Overall Grade: {}\n", analysis.summary.overall_grade));
        report.push_str(&format!("- Success Rate: {:.2}%\n", analysis.summary.success_rate));
        report.push_str(&format!("- Performance Score: {:.2}\n", analysis.summary.performance_score));
        report.push_str(&format!("- Efficiency Score: {:.2}\n\n", analysis.summary.efficiency_score));
        
        // Test Execution Metrics
        report.push_str("## Test Execution Metrics\n\n");
        report.push_str(&format!("- Total Tests: {}\n", metrics.execution.total_tests));
        report.push_str(&format!("- Successful Tests: {}\n", metrics.execution.successful_tests));
        report.push_str(&format!("- Failed Tests: {}\n", metrics.execution.failed_tests));
        report.push_str(&format!("- Execution Rate: {:.2} tests/sec\n", metrics.execution.execution_rate));
        report.push_str(&format!("- Average Test Duration: {:?}\n\n", metrics.execution.avg_test_duration));
        
        // Performance Metrics
        report.push_str("## Performance Metrics\n\n");
        report.push_str(&format!("- Total Operations: {}\n", metrics.performance.total_operations));
        report.push_str(&format!("- Operations per Second: {:.2}\n", metrics.performance.ops_per_second));
        report.push_str(&format!("- Average Response Time: {:?}\n", metrics.performance.avg_response_time));
        report.push_str(&format!("- 95th Percentile: {:?}\n", metrics.performance.p95_response_time));
        report.push_str(&format!("- 99th Percentile: {:?}\n\n", metrics.performance.p99_response_time));
        
        // Resource Usage
        report.push_str("## Resource Usage\n\n");
        report.push_str(&format!("- Peak Memory: {:.2} MB\n", metrics.resources.memory.peak_usage as f64 / 1024.0 / 1024.0));
        report.push_str(&format!("- Average Memory: {:.2} MB\n", metrics.resources.memory.average_usage as f64 / 1024.0 / 1024.0));
        report.push_str(&format!("- Peak CPU: {:.2}%\n", metrics.resources.cpu.peak_usage));
        report.push_str(&format!("- Average CPU: {:.2}%\n\n", metrics.resources.cpu.average_usage));
        
        // Protocol Statistics
        report.push_str("## Protocol Statistics\n\n");
        report.push_str(&format!("### SMTP\n"));
        report.push_str(&format!("- Operations: {}\n", metrics.protocols.smtp.total_operations));
        report.push_str(&format!("- Success Rate: {:.2}%\n", 100.0 - metrics.protocols.smtp.error_rate));
        report.push_str(&format!("- Avg Response Time: {:?}\n\n", metrics.protocols.smtp.avg_response_time));
        
        // Insights and Recommendations
        if !analysis.insights.is_empty() {
            report.push_str("## Performance Insights\n\n");
            for insight in &analysis.insights {
                report.push_str(&format!("- **{}**: {} (Value: {:.2}, Threshold: {:.2})\n", 
                    insight.category, insight.message, insight.value, insight.threshold));
            }
            report.push_str("\n");
        }
        
        if !analysis.recommendations.is_empty() {
            report.push_str("## Recommendations\n\n");
            for (i, recommendation) in analysis.recommendations.iter().enumerate() {
                report.push_str(&format!("{}. {}\n", i + 1, recommendation));
            }
        }
        
        report
    }

    /// Collect system metrics (placeholder implementation)
    async fn collect_system_metrics(metrics: &Arc<RwLock<TestMetrics>>) -> Result<()> {
        let mut metrics_guard = metrics.write().await;
        
        // Simulate memory usage collection
        let current_memory = Self::get_current_memory_usage();
        metrics_guard.resources.memory.current_usage = current_memory;
        if current_memory > metrics_guard.resources.memory.peak_usage {
            metrics_guard.resources.memory.peak_usage = current_memory;
        }
        
        // Add memory sample
        metrics_guard.resources.memory.samples.push(MemorySample {
            timestamp: Utc::now(),
            usage: current_memory,
        });
        
        // Simulate CPU usage collection
        let current_cpu = Self::get_current_cpu_usage();
        metrics_guard.resources.cpu.current_usage = current_cpu;
        if current_cpu > metrics_guard.resources.cpu.peak_usage {
            metrics_guard.resources.cpu.peak_usage = current_cpu;
        }
        
        // Add CPU sample
        metrics_guard.resources.cpu.samples.push(CpuSample {
            timestamp: Utc::now(),
            usage: current_cpu,
        });
        
        // Calculate averages
        if !metrics_guard.resources.memory.samples.is_empty() {
            let total_memory: u64 = metrics_guard.resources.memory.samples.iter().map(|s| s.usage).sum();
            metrics_guard.resources.memory.average_usage = total_memory / metrics_guard.resources.memory.samples.len() as u64;
        }
        
        if !metrics_guard.resources.cpu.samples.is_empty() {
            let total_cpu: f64 = metrics_guard.resources.cpu.samples.iter().map(|s| s.usage).sum();
            metrics_guard.resources.cpu.average_usage = total_cpu / metrics_guard.resources.cpu.samples.len() as f64;
        }
        
        Ok(())
    }

    /// Get current memory usage (placeholder)
    fn get_current_memory_usage() -> u64 {
        // In a real implementation, this would use system APIs
        // For now, simulate with random values
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen_range(50_000_000..200_000_000) // 50-200 MB
    }

    /// Get current CPU usage (placeholder)
    fn get_current_cpu_usage() -> f64 {
        // In a real implementation, this would use system APIs
        // For now, simulate with random values
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen_range(10.0..80.0) // 10-80%
    }
}

impl Default for TestMetrics {
    fn default() -> Self {
        Self {
            execution: ExecutionMetrics::default(),
            performance: PerformanceMetrics::default(),
            resources: ResourceMetrics::default(),
            protocols: ProtocolMetrics::default(),
            errors: ErrorMetrics::default(),
            custom: HashMap::new(),
        }
    }
}

impl Default for ExecutionMetrics {
    fn default() -> Self {
        Self {
            total_tests: 0,
            successful_tests: 0,
            failed_tests: 0,
            skipped_tests: 0,
            execution_rate: 0.0,
            avg_test_duration: Duration::ZERO,
            total_execution_time: Duration::ZERO,
        }
    }
}

impl Default for ResourceMetrics {
    fn default() -> Self {
        Self {
            memory: MemoryMetrics::default(),
            cpu: CpuMetrics::default(),
            network: NetworkMetrics::default(),
            disk: DiskMetrics::default(),
        }
    }
}

impl Default for MemoryMetrics {
    fn default() -> Self {
        Self {
            current_usage: 0,
            peak_usage: 0,
            average_usage: 0,
            samples: Vec::new(),
        }
    }
}

impl Default for CpuMetrics {
    fn default() -> Self {
        Self {
            current_usage: 0.0,
            peak_usage: 0.0,
            average_usage: 0.0,
            samples: Vec::new(),
        }
    }
}

impl Default for NetworkMetrics {
    fn default() -> Self {
        Self {
            bytes_sent: 0,
            bytes_received: 0,
            packets_sent: 0,
            packets_received: 0,
            throughput: 0.0,
        }
    }
}

impl Default for DiskMetrics {
    fn default() -> Self {
        Self {
            bytes_read: 0,
            bytes_written: 0,
            read_operations: 0,
            write_operations: 0,
            io_rate: 0.0,
        }
    }
}

impl Default for ProtocolMetrics {
    fn default() -> Self {
        Self {
            smtp: ProtocolStats::default(),
            imap: ProtocolStats::default(),
            pop3: ProtocolStats::default(),
            jmap: ProtocolStats::default(),
        }
    }
}

impl Default for ProtocolStats {
    fn default() -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            avg_response_time: Duration::ZERO,
            ops_per_second: 0.0,
            error_rate: 0.0,
        }
    }
}

impl Default for ErrorMetrics {
    fn default() -> Self {
        Self {
            total_errors: 0,
            error_rate: 0.0,
            error_categories: HashMap::new(),
            common_errors: Vec::new(),
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TestResult;

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        let metrics = collector.get_metrics().await;
        
        assert_eq!(metrics.execution.total_tests, 0);
        assert_eq!(metrics.execution.successful_tests, 0);
    }

    #[tokio::test]
    async fn test_record_test_result() {
        let collector = MetricsCollector::new();
        
        let result = TestResult {
            test_id: "test-1".to_string(),
            name: "Test 1".to_string(),
            success: true,
            duration: Duration::from_millis(100),
            error: None,
            metadata: HashMap::new(),
            timestamp: Utc::now(),
        };
        
        collector.record_test_result(&result).await;
        
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.execution.total_tests, 1);
        assert_eq!(metrics.execution.successful_tests, 1);
        assert_eq!(metrics.execution.failed_tests, 0);
    }

    #[tokio::test]
    async fn test_record_protocol_operation() {
        let collector = MetricsCollector::new();
        
        collector.record_protocol_operation("smtp", true, Duration::from_millis(50)).await;
        collector.record_protocol_operation("smtp", false, Duration::from_millis(100)).await;
        
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.protocols.smtp.total_operations, 2);
        assert_eq!(metrics.protocols.smtp.successful_operations, 1);
        assert_eq!(metrics.protocols.smtp.failed_operations, 1);
        assert_eq!(metrics.protocols.smtp.error_rate, 50.0);
    }

    #[tokio::test]
    async fn test_metrics_analysis() {
        let collector = MetricsCollector::new();
        
        // Record some test results
        for i in 0..10 {
            let result = TestResult {
                test_id: format!("test-{}", i),
                name: format!("Test {}", i),
                success: i < 8, // 80% success rate
                duration: Duration::from_millis(100),
                error: if i >= 8 { Some("Test error".to_string()) } else { None },
                metadata: HashMap::new(),
                timestamp: Utc::now(),
            };
            collector.record_test_result(&result).await;
        }
        
        let analysis = collector.analyze_metrics().await;
        assert_eq!(analysis.summary.success_rate, 80.0);
        assert!(!analysis.insights.is_empty());
    }

    #[tokio::test]
    async fn test_report_generation() {
        let collector = MetricsCollector::new();
        
        let result = TestResult {
            test_id: "test-1".to_string(),
            name: "Test 1".to_string(),
            success: true,
            duration: Duration::from_millis(100),
            error: None,
            metadata: HashMap::new(),
            timestamp: Utc::now(),
        };
        
        collector.record_test_result(&result).await;
        
        let report = collector.generate_report().await;
        assert!(report.contains("Integration Test Metrics Report"));
        assert!(report.contains("Executive Summary"));
        assert!(report.contains("Test Execution Metrics"));
    }
}
