# A3Mailer High-Performance Server Development Rules

## Project Overview
**Goal**: Build A3Mailer - a million-level high-concurrency Rust high-performance, high-reliability email server with comprehensive testing, fault tolerance, and modern UI.

## Core Development Principles

### 1. Performance Requirements
- **Target**: Support 1M+ concurrent connections
- **Latency**: Sub-millisecond response times for critical operations
- **Throughput**: Handle 100K+ requests per second
- **Memory**: Efficient memory usage with minimal allocations
- **CPU**: Optimal CPU utilization across all cores

### 2. Reliability Standards
- **Uptime**: 99.99% availability target
- **Fault Tolerance**: Graceful degradation under load
- **Error Recovery**: Automatic recovery from transient failures
- **Data Integrity**: Zero data loss guarantee
- **Monitoring**: Comprehensive health checks and metrics

## Code Quality Standards

### 3. Documentation Requirements
```rust
/// Comprehensive function documentation with examples
///
/// # Purpose
/// Detailed explanation of what this function does and why it exists
///
/// # Arguments
/// * `param1` - Description of parameter 1 with type information
/// * `param2` - Description of parameter 2 with constraints
///
/// # Returns
/// Detailed description of return value and possible states
///
/// # Errors
/// All possible error conditions and their meanings
///
/// # Examples
/// ```rust
/// let result = function_name(param1, param2)?;
/// assert_eq!(result.status, ExpectedStatus::Success);
/// ```
///
/// # Performance Notes
/// Any performance considerations or complexity information
///
/// # Safety
/// Thread safety guarantees and concurrent access patterns
pub fn function_name(param1: Type1, param2: Type2) -> Result<ReturnType, ErrorType> {
    // Implementation with detailed inline comments
}
```

### 4. Error Handling and Logging
```rust
use tracing::{error, warn, info, debug, trace, instrument};

#[instrument(level = "debug", skip(sensitive_data))]
pub async fn critical_operation(
    id: u64,
    sensitive_data: &SecretData,
) -> Result<OperationResult, OperationError> {
    info!(operation_id = %id, "Starting critical operation");

    // Validate inputs with detailed logging
    if id == 0 {
        error!(operation_id = %id, "Invalid operation ID: cannot be zero");
        return Err(OperationError::InvalidInput("ID cannot be zero".into()));
    }

    // Log performance metrics at key points
    let start_time = std::time::Instant::now();

    match perform_operation(id, sensitive_data).await {
        Ok(result) => {
            let duration = start_time.elapsed();
            info!(
                operation_id = %id,
                duration_ms = %duration.as_millis(),
                "Operation completed successfully"
            );
            Ok(result)
        }
        Err(e) => {
            let duration = start_time.elapsed();
            error!(
                operation_id = %id,
                duration_ms = %duration.as_millis(),
                error = %e,
                "Operation failed"
            );
            Err(e)
        }
    }
}
```

### 5. Testing Requirements
Every file MUST include comprehensive tests at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;
    use tracing_test::traced_test;

    /// Test normal operation with valid inputs
    #[tokio::test]
    #[traced_test]
    async fn test_normal_operation() {
        // Arrange
        let input = create_valid_input();

        // Act
        let result = function_under_test(input).await;

        // Assert
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value.status, ExpectedStatus::Success);
    }

    /// Test boundary conditions
    #[tokio::test]
    #[traced_test]
    async fn test_boundary_conditions() {
        // Test minimum values
        let result = function_under_test(MIN_VALUE).await;
        assert!(result.is_ok());

        // Test maximum values
        let result = function_under_test(MAX_VALUE).await;
        assert!(result.is_ok());

        // Test edge cases
        let result = function_under_test(EDGE_CASE_VALUE).await;
        assert!(result.is_ok());
    }

    /// Test error conditions
    #[tokio::test]
    #[traced_test]
    async fn test_error_conditions() {
        // Test invalid inputs
        let result = function_under_test(INVALID_INPUT).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ExpectedError::InvalidInput);

        // Test resource exhaustion
        let result = function_under_test(RESOURCE_EXHAUSTION_INPUT).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ExpectedError::ResourceExhausted);
    }

    /// Test concurrent access and thread safety
    #[tokio::test]
    #[traced_test]
    async fn test_concurrent_access() {
        let shared_resource = Arc::new(SharedResource::new());
        let handles: Vec<_> = (0..100)
            .map(|i| {
                let resource = shared_resource.clone();
                tokio::spawn(async move {
                    function_under_test_concurrent(resource, i).await
                })
            })
            .collect();

        let results: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        // Verify all operations succeeded
        for result in results {
            assert!(result.is_ok());
        }
    }

    /// Performance benchmarks
    #[tokio::test]
    #[traced_test]
    async fn test_performance_benchmarks() {
        let start = std::time::Instant::now();
        let iterations = 10000;

        for i in 0..iterations {
            let _ = function_under_test(i).await;
        }

        let duration = start.elapsed();
        let ops_per_sec = iterations as f64 / duration.as_secs_f64();

        // Assert minimum performance requirements
        assert!(ops_per_sec > 1000.0, "Performance below threshold: {} ops/sec", ops_per_sec);
    }
}
```

## Frontend Development Standards

### 6. UI/UX Requirements
- **Default Mode**: Dark mode with warm background colors
- **Theme Support**: Light/Dark mode toggle with system preference detection
- **Framework**: Tailwind CSS for styling
- **Responsive**: Mobile-first responsive design
- **Accessibility**: WCAG 2.1 AA compliance
- **Internationalization**: Multi-language support from day one

### 7. Dark Mode Color Palette
```css
/* Dark Mode - Warm Background Theme */
:root[data-theme="dark"] {
  --bg-primary: #1a1a1a;      /* Warm dark background */
  --bg-secondary: #2d2d2d;    /* Slightly lighter warm dark */
  --bg-tertiary: #3a3a3a;     /* Card backgrounds */
  --text-primary: #f5f5f5;    /* Primary text */
  --text-secondary: #d1d1d1;  /* Secondary text */
  --text-muted: #a1a1a1;      /* Muted text */
  --accent-primary: #ff6b35;   /* Warm orange accent */
  --accent-secondary: #ffa726; /* Warm amber accent */
  --success: #4caf50;          /* Success green */
  --warning: #ff9800;          /* Warning orange */
  --error: #f44336;            /* Error red */
  --border: #404040;           /* Border color */
}

/* Light Mode */
:root[data-theme="light"] {
  --bg-primary: #ffffff;
  --bg-secondary: #f8f9fa;
  --bg-tertiary: #e9ecef;
  --text-primary: #212529;
  --text-secondary: #495057;
  --text-muted: #6c757d;
  --accent-primary: #007bff;
  --accent-secondary: #6c757d;
  --success: #28a745;
  --warning: #ffc107;
  --error: #dc3545;
  --border: #dee2e6;
}
```

### 8. Internationalization Structure
```typescript
// i18n structure
interface Translations {
  common: {
    loading: string;
    error: string;
    success: string;
    cancel: string;
    save: string;
    delete: string;
  };
  navigation: {
    dashboard: string;
    settings: string;
    users: string;
    reports: string;
  };
  // ... more sections
}

// Supported languages
const SUPPORTED_LANGUAGES = ['en', 'zh', 'es', 'fr', 'de', 'ja'] as const;
```

## Architecture Standards

### 9. Fault Tolerance Design
- **Circuit Breakers**: Implement circuit breakers for external dependencies
- **Retry Logic**: Exponential backoff with jitter for transient failures
- **Graceful Degradation**: Fallback mechanisms for non-critical features
- **Health Checks**: Comprehensive health monitoring at all levels
- **Resource Limits**: Proper resource management and limits

### 10. Logging Standards
```rust
// Structured logging with context
use tracing::{info, warn, error, debug, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

// Log levels:
// ERROR: System errors, failures that require immediate attention
// WARN:  Potential issues, degraded performance, recoverable errors
// INFO:  Important business events, system state changes
// DEBUG: Detailed execution flow, performance metrics
// TRACE: Very detailed debugging information

// Required log fields for critical operations:
// - operation_id: Unique identifier for the operation
// - user_id: User performing the operation (if applicable)
// - duration_ms: Operation duration
// - status: Success/failure status
// - error_code: Specific error code (if applicable)
```

## Performance Optimization

### 11. Memory Management
- Use `Arc` and `Rc` judiciously for shared ownership
- Prefer `&str` over `String` when possible
- Use object pools for frequently allocated objects
- Implement proper drop handlers for cleanup
- Monitor memory usage with detailed metrics

### 12. Concurrency Patterns
- Use `tokio` for async operations
- Implement proper backpressure mechanisms
- Use channels for inter-task communication
- Avoid blocking operations in async contexts
- Implement proper cancellation handling

## Security Standards

### 13. Security Requirements
- Input validation on all external inputs
- Proper authentication and authorization
- Secure session management
- Rate limiting and DDoS protection
- Audit logging for security events
- Regular security dependency updates

## Deployment and Operations

### 14. Monitoring and Observability
- Prometheus metrics for all critical operations
- OpenTelemetry tracing for distributed operations
- Health check endpoints
- Performance dashboards
- Alerting for critical thresholds

### 15. Configuration Management
- Environment-based configuration
- Secure secret management
- Configuration validation
- Hot-reload capabilities where safe
- Documentation for all configuration options

## Code Review Standards

### 16. Review Checklist
- [ ] All functions have comprehensive documentation
- [ ] Test coverage includes normal, boundary, and error cases
- [ ] Logging is present at appropriate levels
- [ ] Error handling follows established patterns
- [ ] Performance implications are considered
- [ ] Security implications are reviewed
- [ ] Internationalization is considered
- [ ] Accessibility requirements are met (for UI)

## Continuous Integration

### 17. CI/CD Pipeline Requirements
- Automated testing on all platforms
- Performance regression testing
- Security vulnerability scanning
- Code coverage reporting (minimum 90%)
- Documentation generation
- Automated deployment to staging

## File Structure Standards

### 18. Rust File Template
```rust
//! Module documentation
//!
//! This module provides [brief description of functionality].
//!
//! # Architecture
//! [Detailed architecture explanation]
//!
//! # Performance Characteristics
//! [Performance notes, complexity analysis]
//!
//! # Thread Safety
//! [Concurrency guarantees and limitations]
//!
//! # Examples
//! ```rust
//! use crate::module_name::*;
//!
//! let instance = ModuleName::new()?;
//! let result = instance.operation().await?;
//! ```

use std::{
    collections::HashMap,
    sync::{Arc, atomic::{AtomicU64, Ordering}},
    time::{Duration, Instant},
};

use tokio::{
    sync::{RwLock, Semaphore},
    time::timeout,
};
use tracing::{error, warn, info, debug, trace, instrument};
use serde::{Deserialize, Serialize};

// Type definitions with comprehensive documentation
/// Represents the core data structure for [purpose]
///
/// # Fields
/// * `id` - Unique identifier for this instance
/// * `status` - Current operational status
/// * `metrics` - Performance and operational metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreStruct {
    pub id: u64,
    pub status: OperationStatus,
    pub metrics: PerformanceMetrics,
    pub created_at: Instant,
}

/// Operational status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationStatus {
    /// Initial state, not yet started
    Pending,
    /// Currently processing
    Running,
    /// Successfully completed
    Completed,
    /// Failed with error
    Failed,
    /// Cancelled by user or system
    Cancelled,
}

/// Performance metrics collection
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub operations_count: AtomicU64,
    pub total_duration_ms: AtomicU64,
    pub error_count: AtomicU64,
    pub last_operation_time: Option<Instant>,
}

// Implementation with comprehensive error handling
impl CoreStruct {
    /// Creates a new instance with default configuration
    ///
    /// # Returns
    /// A new `CoreStruct` instance with initialized metrics
    ///
    /// # Examples
    /// ```rust
    /// let instance = CoreStruct::new();
    /// assert_eq!(instance.status, OperationStatus::Pending);
    /// ```
    pub fn new() -> Self {
        info!("Creating new CoreStruct instance");

        Self {
            id: generate_unique_id(),
            status: OperationStatus::Pending,
            metrics: PerformanceMetrics::default(),
            created_at: Instant::now(),
        }
    }

    /// Performs the core operation with comprehensive error handling
    ///
    /// # Arguments
    /// * `input` - The input data to process
    /// * `timeout_duration` - Maximum time to wait for completion
    ///
    /// # Returns
    /// * `Ok(result)` - Operation completed successfully
    /// * `Err(error)` - Operation failed with specific error
    ///
    /// # Errors
    /// * `OperationError::Timeout` - Operation exceeded timeout
    /// * `OperationError::InvalidInput` - Input validation failed
    /// * `OperationError::ResourceExhausted` - System resources unavailable
    ///
    /// # Performance Notes
    /// This operation has O(n) complexity where n is the input size.
    /// Memory usage is bounded by the input size plus constant overhead.
    #[instrument(level = "debug", skip(self, input))]
    pub async fn perform_operation(
        &mut self,
        input: &InputData,
        timeout_duration: Duration,
    ) -> Result<OperationResult, OperationError> {
        let operation_id = self.id;
        let start_time = Instant::now();

        info!(
            operation_id = %operation_id,
            input_size = input.size(),
            timeout_ms = %timeout_duration.as_millis(),
            "Starting core operation"
        );

        // Update status to running
        self.status = OperationStatus::Running;

        // Validate input with detailed logging
        if let Err(validation_error) = self.validate_input(input) {
            error!(
                operation_id = %operation_id,
                error = %validation_error,
                "Input validation failed"
            );
            self.status = OperationStatus::Failed;
            self.metrics.error_count.fetch_add(1, Ordering::Relaxed);
            return Err(OperationError::InvalidInput(validation_error.to_string()));
        }

        // Perform operation with timeout
        let result = match timeout(timeout_duration, self.execute_core_logic(input)).await {
            Ok(Ok(result)) => {
                let duration = start_time.elapsed();
                info!(
                    operation_id = %operation_id,
                    duration_ms = %duration.as_millis(),
                    result_size = result.size(),
                    "Operation completed successfully"
                );

                self.status = OperationStatus::Completed;
                self.metrics.operations_count.fetch_add(1, Ordering::Relaxed);
                self.metrics.total_duration_ms.fetch_add(
                    duration.as_millis() as u64,
                    Ordering::Relaxed
                );
                self.metrics.last_operation_time = Some(start_time);

                Ok(result)
            }
            Ok(Err(execution_error)) => {
                let duration = start_time.elapsed();
                error!(
                    operation_id = %operation_id,
                    duration_ms = %duration.as_millis(),
                    error = %execution_error,
                    "Operation execution failed"
                );

                self.status = OperationStatus::Failed;
                self.metrics.error_count.fetch_add(1, Ordering::Relaxed);
                Err(execution_error)
            }
            Err(_timeout_error) => {
                let duration = start_time.elapsed();
                warn!(
                    operation_id = %operation_id,
                    duration_ms = %duration.as_millis(),
                    timeout_ms = %timeout_duration.as_millis(),
                    "Operation timed out"
                );

                self.status = OperationStatus::Failed;
                self.metrics.error_count.fetch_add(1, Ordering::Relaxed);
                Err(OperationError::Timeout(timeout_duration))
            }
        };

        result
    }

    /// Validates input data according to business rules
    ///
    /// # Arguments
    /// * `input` - Input data to validate
    ///
    /// # Returns
    /// * `Ok(())` - Input is valid
    /// * `Err(error)` - Input validation failed
    fn validate_input(&self, input: &InputData) -> Result<(), ValidationError> {
        debug!(operation_id = %self.id, "Validating input data");

        if input.is_empty() {
            return Err(ValidationError::EmptyInput);
        }

        if input.size() > MAX_INPUT_SIZE {
            return Err(ValidationError::InputTooLarge(input.size()));
        }

        if !input.is_well_formed() {
            return Err(ValidationError::MalformedInput);
        }

        debug!(operation_id = %self.id, "Input validation passed");
        Ok(())
    }

    /// Executes the core business logic
    ///
    /// # Arguments
    /// * `input` - Validated input data
    ///
    /// # Returns
    /// * `Ok(result)` - Processing completed successfully
    /// * `Err(error)` - Processing failed
    async fn execute_core_logic(&self, input: &InputData) -> Result<OperationResult, OperationError> {
        debug!(operation_id = %self.id, "Executing core logic");

        // Simulate processing with proper error handling
        match input.process().await {
            Ok(result) => {
                debug!(
                    operation_id = %self.id,
                    result_size = result.size(),
                    "Core logic execution completed"
                );
                Ok(result)
            }
            Err(e) => {
                error!(
                    operation_id = %self.id,
                    error = %e,
                    "Core logic execution failed"
                );
                Err(OperationError::ProcessingFailed(e.to_string()))
            }
        }
    }

    /// Gets current performance metrics
    ///
    /// # Returns
    /// A snapshot of current performance metrics
    pub fn get_metrics(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            operations_count: self.metrics.operations_count.load(Ordering::Relaxed),
            total_duration_ms: self.metrics.total_duration_ms.load(Ordering::Relaxed),
            error_count: self.metrics.error_count.load(Ordering::Relaxed),
            average_duration_ms: self.calculate_average_duration(),
            uptime: self.created_at.elapsed(),
        }
    }

    /// Calculates average operation duration
    fn calculate_average_duration(&self) -> f64 {
        let total_ops = self.metrics.operations_count.load(Ordering::Relaxed);
        let total_duration = self.metrics.total_duration_ms.load(Ordering::Relaxed);

        if total_ops > 0 {
            total_duration as f64 / total_ops as f64
        } else {
            0.0
        }
    }
}

// Helper functions with proper documentation
/// Generates a unique identifier for operations
///
/// # Returns
/// A unique 64-bit identifier
fn generate_unique_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

// Constants with documentation
/// Maximum allowed input size in bytes
const MAX_INPUT_SIZE: usize = 1024 * 1024; // 1MB

/// Default operation timeout
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

// Error types with comprehensive documentation
/// Errors that can occur during operations
#[derive(Debug, thiserror::Error)]
pub enum OperationError {
    /// Operation exceeded the specified timeout
    #[error("Operation timed out after {0:?}")]
    Timeout(Duration),

    /// Input data failed validation
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// System resources are exhausted
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    /// Processing failed with error
    #[error("Processing failed: {0}")]
    ProcessingFailed(String),
}

/// Input validation errors
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    /// Input data is empty
    #[error("Input data is empty")]
    EmptyInput,

    /// Input data exceeds size limits
    #[error("Input too large: {0} bytes")]
    InputTooLarge(usize),

    /// Input data is malformed
    #[error("Input data is malformed")]
    MalformedInput,
}

// Additional types for completeness
#[derive(Debug, Clone)]
pub struct InputData {
    data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct OperationResult {
    data: Vec<u8>,
    metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub operations_count: u64,
    pub total_duration_ms: u64,
    pub error_count: u64,
    pub average_duration_ms: f64,
    pub uptime: Duration,
}

impl InputData {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn is_well_formed(&self) -> bool {
        // Implement validation logic
        !self.data.is_empty()
    }

    pub async fn process(&self) -> Result<OperationResult, Box<dyn std::error::Error + Send + Sync>> {
        // Simulate processing
        tokio::time::sleep(Duration::from_millis(10)).await;

        Ok(OperationResult {
            data: self.data.clone(),
            metadata: HashMap::new(),
        })
    }
}

impl OperationResult {
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

// COMPREHENSIVE TEST SUITE - ALWAYS AT THE BOTTOM OF EVERY FILE
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;
    use tracing_test::traced_test;
    use std::sync::Arc;
    use std::time::Duration;

    /// Test helper to create valid input data
    fn create_valid_input() -> InputData {
        InputData::new(b"valid test data".to_vec())
    }

    /// Test helper to create invalid input data
    fn create_invalid_input() -> InputData {
        InputData::new(Vec::new())
    }

    /// Test helper to create large input data
    fn create_large_input() -> InputData {
        InputData::new(vec![0u8; MAX_INPUT_SIZE + 1])
    }

    /// Test normal operation with valid inputs
    #[tokio::test]
    #[traced_test]
    async fn test_normal_operation() {
        // Arrange
        let mut instance = CoreStruct::new();
        let input = create_valid_input();
        let timeout = Duration::from_secs(5);

        // Act
        let result = instance.perform_operation(&input, timeout).await;

        // Assert
        assert!(result.is_ok(), "Operation should succeed with valid input");
        let operation_result = result.unwrap();
        assert_eq!(operation_result.size(), input.size());
        assert_eq!(instance.status, OperationStatus::Completed);

        // Verify metrics were updated
        let metrics = instance.get_metrics();
        assert_eq!(metrics.operations_count, 1);
        assert_eq!(metrics.error_count, 0);
        assert!(metrics.average_duration_ms > 0.0);
    }

    /// Test boundary conditions
    #[tokio::test]
    #[traced_test]
    async fn test_boundary_conditions() {
        let mut instance = CoreStruct::new();
        let timeout = Duration::from_secs(1);

        // Test minimum valid input (1 byte)
        let min_input = InputData::new(vec![42u8]);
        let result = instance.perform_operation(&min_input, timeout).await;
        assert!(result.is_ok(), "Should handle minimum valid input");

        // Test maximum valid input
        let max_input = InputData::new(vec![42u8; MAX_INPUT_SIZE]);
        let result = instance.perform_operation(&max_input, timeout).await;
        assert!(result.is_ok(), "Should handle maximum valid input");

        // Test minimum timeout
        let min_timeout = Duration::from_millis(1);
        let input = create_valid_input();
        let result = instance.perform_operation(&input, min_timeout).await;
        // This might timeout or succeed depending on system performance
        // We just verify it doesn't panic
        let _ = result;
    }

    /// Test error conditions
    #[tokio::test]
    #[traced_test]
    async fn test_error_conditions() {
        let mut instance = CoreStruct::new();
        let timeout = Duration::from_secs(5);

        // Test empty input
        let empty_input = create_invalid_input();
        let result = instance.perform_operation(&empty_input, timeout).await;
        assert!(result.is_err(), "Should fail with empty input");
        match result.unwrap_err() {
            OperationError::InvalidInput(_) => {}, // Expected
            other => panic!("Expected InvalidInput error, got: {:?}", other),
        }

        // Test oversized input
        let large_input = create_large_input();
        let result = instance.perform_operation(&large_input, timeout).await;
        assert!(result.is_err(), "Should fail with oversized input");
        match result.unwrap_err() {
            OperationError::InvalidInput(_) => {}, // Expected
            other => panic!("Expected InvalidInput error, got: {:?}", other),
        }

        // Test timeout condition
        let input = create_valid_input();
        let very_short_timeout = Duration::from_nanos(1);
        let result = instance.perform_operation(&input, very_short_timeout).await;
        // Should likely timeout, but we handle both cases
        if let Err(OperationError::Timeout(_)) = result {
            // Expected timeout
        } else {
            // Operation completed faster than expected, which is also valid
        }

        // Verify error metrics were updated
        let metrics = instance.get_metrics();
        assert!(metrics.error_count > 0, "Error count should be incremented");
    }

    /// Test concurrent access and thread safety
    #[tokio::test]
    #[traced_test]
    async fn test_concurrent_access() {
        let instance = Arc::new(tokio::sync::Mutex::new(CoreStruct::new()));
        let timeout = Duration::from_secs(5);

        // Spawn multiple concurrent operations
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let instance = instance.clone();
                let input = InputData::new(format!("test data {}", i).into_bytes());

                tokio::spawn(async move {
                    let mut guard = instance.lock().await;
                    guard.perform_operation(&input, timeout).await
                })
            })
            .collect();

        // Wait for all operations to complete
        let results: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .expect("All tasks should complete");

        // Verify all operations succeeded
        for (i, result) in results.into_iter().enumerate() {
            assert!(result.is_ok(), "Operation {} should succeed", i);
        }

        // Verify final metrics
        let guard = instance.lock().await;
        let metrics = guard.get_metrics();
        assert_eq!(metrics.operations_count, 10, "Should have processed 10 operations");
    }

    /// Test performance benchmarks
    #[tokio::test]
    #[traced_test]
    async fn test_performance_benchmarks() {
        let mut instance = CoreStruct::new();
        let input = create_valid_input();
        let timeout = Duration::from_secs(30);
        let iterations = 100;

        let start = std::time::Instant::now();

        for _ in 0..iterations {
            let result = instance.perform_operation(&input, timeout).await;
            assert!(result.is_ok(), "All benchmark operations should succeed");
        }

        let duration = start.elapsed();
        let ops_per_sec = iterations as f64 / duration.as_secs_f64();

        // Assert minimum performance requirements
        assert!(
            ops_per_sec > 10.0,
            "Performance below threshold: {:.2} ops/sec (expected > 10 ops/sec)",
            ops_per_sec
        );

        // Log performance metrics for analysis
        println!("Performance benchmark results:");
        println!("  Operations: {}", iterations);
        println!("  Duration: {:.2}s", duration.as_secs_f64());
        println!("  Ops/sec: {:.2}", ops_per_sec);
        println!("  Avg latency: {:.2}ms", duration.as_millis() as f64 / iterations as f64);
    }

    /// Test metrics collection accuracy
    #[tokio::test]
    #[traced_test]
    async fn test_metrics_accuracy() {
        let mut instance = CoreStruct::new();
        let timeout = Duration::from_secs(5);

        // Perform successful operations
        for i in 0..5 {
            let input = InputData::new(format!("test {}", i).into_bytes());
            let result = instance.perform_operation(&input, timeout).await;
            assert!(result.is_ok());
        }

        // Perform failed operations
        for _ in 0..3 {
            let invalid_input = create_invalid_input();
            let result = instance.perform_operation(&invalid_input, timeout).await;
            assert!(result.is_err());
        }

        // Verify metrics
        let metrics = instance.get_metrics();
        assert_eq!(metrics.operations_count, 5, "Should count successful operations");
        assert_eq!(metrics.error_count, 3, "Should count failed operations");
        assert!(metrics.total_duration_ms > 0, "Should track total duration");
        assert!(metrics.average_duration_ms > 0.0, "Should calculate average duration");
        assert!(metrics.uptime > Duration::from_millis(0), "Should track uptime");
    }

    /// Test input validation edge cases
    #[tokio::test]
    #[traced_test]
    async fn test_input_validation_edge_cases() {
        let instance = CoreStruct::new();

        // Test exactly at size limit
        let max_size_input = InputData::new(vec![42u8; MAX_INPUT_SIZE]);
        assert!(instance.validate_input(&max_size_input).is_ok());

        // Test one byte over limit
        let over_size_input = InputData::new(vec![42u8; MAX_INPUT_SIZE + 1]);
        assert!(instance.validate_input(&over_size_input).is_err());

        // Test empty input
        let empty_input = InputData::new(Vec::new());
        assert!(instance.validate_input(&empty_input).is_err());

        // Test single byte input
        let single_byte_input = InputData::new(vec![42u8]);
        assert!(instance.validate_input(&single_byte_input).is_ok());
    }

    /// Test error propagation and logging
    #[tokio::test]
    #[traced_test]
    async fn test_error_propagation() {
        let mut instance = CoreStruct::new();
        let timeout = Duration::from_secs(5);

        // Test that validation errors are properly propagated
        let invalid_input = create_invalid_input();
        let result = instance.perform_operation(&invalid_input, timeout).await;

        assert!(result.is_err());
        assert_eq!(instance.status, OperationStatus::Failed);

        // Verify error is of correct type
        match result.unwrap_err() {
            OperationError::InvalidInput(msg) => {
                assert!(msg.contains("Input data is empty"));
            }
            other => panic!("Expected InvalidInput error, got: {:?}", other),
        }
    }

    /// Test unique ID generation
    #[test]
    fn test_unique_id_generation() {
        let mut ids = std::collections::HashSet::new();

        // Generate many IDs and ensure they're unique
        for _ in 0..1000 {
            let id = generate_unique_id();
            assert!(ids.insert(id), "Generated duplicate ID: {}", id);
        }
    }

    /// Test status transitions
    #[tokio::test]
    #[traced_test]
    async fn test_status_transitions() {
        let mut instance = CoreStruct::new();

        // Initial status should be Pending
        assert_eq!(instance.status, OperationStatus::Pending);

        // Start operation with valid input
        let input = create_valid_input();
        let timeout = Duration::from_secs(5);

        let result = instance.perform_operation(&input, timeout).await;
        assert!(result.is_ok());
        assert_eq!(instance.status, OperationStatus::Completed);

        // Try operation with invalid input
        let invalid_input = create_invalid_input();
        let result = instance.perform_operation(&invalid_input, timeout).await;
        assert!(result.is_err());
        assert_eq!(instance.status, OperationStatus::Failed);
    }
}
```

## Frontend Development Standards

### 19. React/TypeScript Component Template
```typescript
/**
 * Component documentation
 *
 * This component provides [brief description of functionality].
 *
 * @component
 * @example
 * ```tsx
 * <ComponentName
 *   prop1="value1"
 *   prop2={value2}
 *   onAction={handleAction}
 * />
 * ```
 */

import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useTheme } from '@/hooks/useTheme';
import { cn } from '@/lib/utils';
import { logger } from '@/lib/logger';

// Type definitions with comprehensive documentation
interface ComponentProps {
  /** Primary identifier for the component */
  id?: string;
  /** CSS class names to apply */
  className?: string;
  /** Whether the component is disabled */
  disabled?: boolean;
  /** Loading state indicator */
  loading?: boolean;
  /** Error state information */
  error?: string | null;
  /** Success callback function */
  onSuccess?: (result: OperationResult) => void;
  /** Error callback function */
  onError?: (error: Error) => void;
  /** Children elements */
  children?: React.ReactNode;
}

interface OperationResult {
  id: string;
  status: 'success' | 'error' | 'pending';
  data?: any;
  message?: string;
}

/**
 * Main component implementation with comprehensive error handling
 *
 * @param props - Component properties
 * @returns JSX element
 */
export const ComponentName: React.FC<ComponentProps> = ({
  id = 'component-default',
  className,
  disabled = false,
  loading = false,
  error = null,
  onSuccess,
  onError,
  children,
}) => {
  // Hooks for internationalization and theming
  const { t } = useTranslation();
  const { theme, toggleTheme } = useTheme();

  // State management with proper typing
  const [internalState, setInternalState] = useState<{
    data: any[];
    isProcessing: boolean;
    lastUpdate: Date | null;
  }>({
    data: [],
    isProcessing: false,
    lastUpdate: null,
  });

  // Memoized computed values
  const computedClassName = useMemo(() => cn(
    // Base styles
    'relative flex flex-col',
    'transition-all duration-200 ease-in-out',

    // Theme-specific styles
    theme === 'dark' ? [
      'bg-gray-900 text-gray-100',
      'border-gray-700',
    ] : [
      'bg-white text-gray-900',
      'border-gray-200',
    ],

    // State-specific styles
    {
      'opacity-50 cursor-not-allowed': disabled,
      'animate-pulse': loading,
      'border-red-500 bg-red-50 dark:bg-red-900/20': error,
    },

    // Custom className
    className,
  ), [theme, disabled, loading, error, className]);

  // Event handlers with proper error handling
  const handleOperation = useCallback(async () => {
    if (disabled || loading) {
      logger.warn('Operation attempted while component is disabled or loading', {
        componentId: id,
        disabled,
        loading,
      });
      return;
    }

    setInternalState(prev => ({ ...prev, isProcessing: true }));

    try {
      logger.info('Starting component operation', { componentId: id });

      // Simulate async operation
      await new Promise(resolve => setTimeout(resolve, 1000));

      const result: OperationResult = {
        id: `operation-${Date.now()}`,
        status: 'success',
        data: { timestamp: new Date().toISOString() },
        message: t('operation.success'),
      };

      setInternalState(prev => ({
        ...prev,
        isProcessing: false,
        lastUpdate: new Date(),
      }));

      logger.info('Component operation completed successfully', {
        componentId: id,
        resultId: result.id,
      });

      onSuccess?.(result);

    } catch (err) {
      const error = err instanceof Error ? err : new Error('Unknown error');

      setInternalState(prev => ({ ...prev, isProcessing: false }));

      logger.error('Component operation failed', {
        componentId: id,
        error: error.message,
        stack: error.stack,
      });

      onError?.(error);
    }
  }, [id, disabled, loading, onSuccess, onError, t]);

  // Effect for component lifecycle logging
  useEffect(() => {
    logger.debug('Component mounted', { componentId: id });

    return () => {
      logger.debug('Component unmounted', { componentId: id });
    };
  }, [id]);

  // Effect for error state changes
  useEffect(() => {
    if (error) {
      logger.warn('Component error state changed', {
        componentId: id,
        error,
      });
    }
  }, [id, error]);

  // Render error state
  if (error) {
    return (
      <div className={cn(computedClassName, 'border-2 border-red-500')}>
        <div className="flex items-center space-x-2 p-4">
          <svg
            className="w-5 h-5 text-red-500"
            fill="currentColor"
            viewBox="0 0 20 20"
            aria-hidden="true"
          >
            <path
              fillRule="evenodd"
              d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
              clipRule="evenodd"
            />
          </svg>
          <span className="text-sm font-medium">
            {t('error.generic')}: {error}
          </span>
        </div>
      </div>
    );
  }

  // Render loading state
  if (loading || internalState.isProcessing) {
    return (
      <div className={cn(computedClassName, 'animate-pulse')}>
        <div className="flex items-center justify-center p-8">
          <div className="flex space-x-2">
            <div className="w-2 h-2 bg-blue-500 rounded-full animate-bounce"></div>
            <div className="w-2 h-2 bg-blue-500 rounded-full animate-bounce" style={{ animationDelay: '0.1s' }}></div>
            <div className="w-2 h-2 bg-blue-500 rounded-full animate-bounce" style={{ animationDelay: '0.2s' }}></div>
          </div>
          <span className="ml-3 text-sm text-gray-600 dark:text-gray-400">
            {t('loading.generic')}
          </span>
        </div>
      </div>
    );
  }

  // Main render
  return (
    <div className={computedClassName} id={id}>
      {/* Header section */}
      <div className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
        <h2 className="text-lg font-semibold">
          {t('component.title')}
        </h2>

        <div className="flex items-center space-x-2">
          {/* Theme toggle button */}
          <button
            onClick={toggleTheme}
            className={cn(
              'p-2 rounded-md transition-colors',
              'hover:bg-gray-100 dark:hover:bg-gray-800',
              'focus:outline-none focus:ring-2 focus:ring-blue-500'
            )}
            aria-label={t('theme.toggle')}
          >
            {theme === 'dark' ? (
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M10 2a1 1 0 011 1v1a1 1 0 11-2 0V3a1 1 0 011-1zm4 8a4 4 0 11-8 0 4 4 0 018 0zm-.464 4.95l.707.707a1 1 0 001.414-1.414l-.707-.707a1 1 0 00-1.414 1.414zm2.12-10.607a1 1 0 010 1.414l-.706.707a1 1 0 11-1.414-1.414l.707-.707a1 1 0 011.414 0zM17 11a1 1 0 100-2h-1a1 1 0 100 2h1zm-7 4a1 1 0 011 1v1a1 1 0 11-2 0v-1a1 1 0 011-1zM5.05 6.464A1 1 0 106.465 5.05l-.708-.707a1 1 0 00-1.414 1.414l.707.707zm1.414 8.486l-.707.707a1 1 0 01-1.414-1.414l.707-.707a1 1 0 011.414 1.414zM4 11a1 1 0 100-2H3a1 1 0 000 2h1z" clipRule="evenodd" />
              </svg>
            ) : (
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                <path d="M17.293 13.293A8 8 0 016.707 2.707a8.001 8.001 0 1010.586 10.586z" />
              </svg>
            )}
          </button>

          {/* Action button */}
          <button
            onClick={handleOperation}
            disabled={disabled || internalState.isProcessing}
            className={cn(
              'px-4 py-2 text-sm font-medium rounded-md',
              'transition-colors duration-200',
              'focus:outline-none focus:ring-2 focus:ring-blue-500',
              disabled || internalState.isProcessing
                ? 'bg-gray-300 text-gray-500 cursor-not-allowed dark:bg-gray-700 dark:text-gray-400'
                : 'bg-blue-600 text-white hover:bg-blue-700 dark:bg-blue-500 dark:hover:bg-blue-600'
            )}
          >
            {internalState.isProcessing ? t('button.processing') : t('button.action')}
          </button>
        </div>
      </div>

      {/* Content section */}
      <div className="flex-1 p-4">
        {children || (
          <div className="text-center text-gray-500 dark:text-gray-400">
            {t('content.empty')}
          </div>
        )}
      </div>

      {/* Footer section with status */}
      {internalState.lastUpdate && (
        <div className="px-4 py-2 text-xs text-gray-500 dark:text-gray-400 border-t border-gray-200 dark:border-gray-700">
          {t('status.lastUpdate')}: {internalState.lastUpdate.toLocaleString()}
        </div>
      )}
    </div>
  );
};

// Default export with display name for debugging
ComponentName.displayName = 'ComponentName';
export default ComponentName;
```

### 20. Internationalization Configuration
```typescript
// i18n/index.ts - Main i18n configuration
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import Backend from 'i18next-http-backend';
import LanguageDetector from 'i18next-browser-languagedetector';

// Import translation files
import enTranslations from './locales/en.json';
import zhTranslations from './locales/zh.json';
import esTranslations from './locales/es.json';
import frTranslations from './locales/fr.json';
import deTranslations from './locales/de.json';
import jaTranslations from './locales/ja.json';

// Supported languages configuration
export const SUPPORTED_LANGUAGES = [
  { code: 'en', name: 'English', nativeName: 'English' },
  { code: 'zh', name: 'Chinese', nativeName: '中文' },
  { code: 'es', name: 'Spanish', nativeName: 'Español' },
  { code: 'fr', name: 'French', nativeName: 'Français' },
  { code: 'de', name: 'German', nativeName: 'Deutsch' },
  { code: 'ja', name: 'Japanese', nativeName: '日本語' },
] as const;

export type SupportedLanguage = typeof SUPPORTED_LANGUAGES[number]['code'];

// Translation resources
const resources = {
  en: { translation: enTranslations },
  zh: { translation: zhTranslations },
  es: { translation: esTranslations },
  fr: { translation: frTranslations },
  de: { translation: deTranslations },
  ja: { translation: jaTranslations },
};

// Initialize i18n
i18n
  .use(Backend)
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: 'en',
    debug: process.env.NODE_ENV === 'development',

    interpolation: {
      escapeValue: false, // React already escapes values
    },

    detection: {
      order: ['localStorage', 'navigator', 'htmlTag'],
      caches: ['localStorage'],
    },

    backend: {
      loadPath: '/locales/{{lng}}.json',
    },
  });

export default i18n;
```

```json
// i18n/locales/en.json - English translations
{
  "common": {
    "loading": "Loading...",
    "error": "Error",
    "success": "Success",
    "cancel": "Cancel",
    "save": "Save",
    "delete": "Delete",
    "edit": "Edit",
    "create": "Create",
    "update": "Update",
    "confirm": "Confirm",
    "close": "Close",
    "back": "Back",
    "next": "Next",
    "previous": "Previous",
    "search": "Search",
    "filter": "Filter",
    "sort": "Sort",
    "refresh": "Refresh"
  },
  "navigation": {
    "dashboard": "Dashboard",
    "settings": "Settings",
    "users": "Users",
    "reports": "Reports",
    "mail": "Mail",
    "calendar": "Calendar",
    "contacts": "Contacts",
    "files": "Files"
  },
  "theme": {
    "toggle": "Toggle theme",
    "light": "Light mode",
    "dark": "Dark mode",
    "system": "System preference"
  },
  "button": {
    "action": "Execute Action",
    "processing": "Processing...",
    "submit": "Submit",
    "reset": "Reset"
  },
  "error": {
    "generic": "An error occurred",
    "network": "Network error",
    "timeout": "Request timeout",
    "unauthorized": "Unauthorized access",
    "forbidden": "Access forbidden",
    "notFound": "Resource not found",
    "serverError": "Server error",
    "validation": "Validation error"
  },
  "operation": {
    "success": "Operation completed successfully",
    "failed": "Operation failed",
    "timeout": "Operation timed out",
    "cancelled": "Operation cancelled"
  },
  "status": {
    "lastUpdate": "Last updated",
    "online": "Online",
    "offline": "Offline",
    "connecting": "Connecting",
    "connected": "Connected",
    "disconnected": "Disconnected"
  },
  "content": {
    "empty": "No content available",
    "noResults": "No results found",
    "noData": "No data to display"
  },
  "component": {
    "title": "Component Title"
  }
}
```

### 21. Project Structure Standards
```
a3mailer-server/
├── crates/                          # Rust backend modules
│   ├── main/                        # Main application entry point
│   ├── http/                        # HTTP server and routing
│   ├── smtp/                        # SMTP protocol implementation
│   ├── imap/                        # IMAP protocol implementation
│   ├── jmap/                        # JMAP protocol implementation
│   ├── pop3/                        # POP3 protocol implementation
│   ├── dav/                         # WebDAV/CalDAV/CardDAV
│   ├── store/                       # Data storage abstraction
│   ├── directory/                   # User directory services
│   ├── common/                      # Shared utilities and types
│   ├── spam-filter/                 # Spam filtering engine
│   ├── groupware/                   # Collaboration features
│   ├── services/                    # Background services
│   ├── trc/                         # Tracing and telemetry
│   └── utils/                       # Utility functions
├── frontend/                        # React/TypeScript frontend
│   ├── src/
│   │   ├── components/              # Reusable UI components
│   │   │   ├── ui/                  # Base UI components
│   │   │   ├── forms/               # Form components
│   │   │   ├── layout/              # Layout components
│   │   │   └── features/            # Feature-specific components
│   │   ├── pages/                   # Page components
│   │   │   ├── dashboard/           # Dashboard pages
│   │   │   ├── mail/                # Mail interface
│   │   │   ├── calendar/            # Calendar interface
│   │   │   ├── contacts/            # Contacts interface
│   │   │   ├── settings/            # Settings pages
│   │   │   └── admin/               # Admin interface
│   │   ├── hooks/                   # Custom React hooks
│   │   ├── lib/                     # Utility libraries
│   │   │   ├── api/                 # API client functions
│   │   │   ├── auth/                # Authentication utilities
│   │   │   ├── utils/               # General utilities
│   │   │   └── logger/              # Frontend logging
│   │   ├── stores/                  # State management (Zustand/Redux)
│   │   ├── types/                   # TypeScript type definitions
│   │   ├── styles/                  # Global styles and Tailwind config
│   │   └── i18n/                    # Internationalization
│   │       ├── locales/             # Translation files
│   │       │   ├── en.json
│   │       │   ├── zh.json
│   │       │   ├── es.json
│   │       │   ├── fr.json
│   │       │   ├── de.json
│   │       │   └── ja.json
│   │       └── index.ts             # i18n configuration
│   ├── public/                      # Static assets
│   ├── package.json                 # Node.js dependencies
│   ├── tailwind.config.js           # Tailwind CSS configuration
│   ├── vite.config.ts               # Vite build configuration
│   └── tsconfig.json                # TypeScript configuration
├── tests/                           # Integration and E2E tests
│   ├── src/                         # Test source code
│   ├── resources/                   # Test resources and fixtures
│   └── Cargo.toml                   # Test dependencies
├── docs/                            # Documentation
│   ├── api/                         # API documentation
│   ├── deployment/                  # Deployment guides
│   ├── development/                 # Development guides
│   └── user/                        # User documentation
├── scripts/                         # Build and deployment scripts
│   ├── build.sh                     # Build script
│   ├── test.sh                      # Test script
│   ├── deploy.sh                    # Deployment script
│   └── setup-dev.sh                 # Development environment setup
├── docker/                          # Docker configuration
│   ├── Dockerfile                   # Production Docker image
│   ├── Dockerfile.dev               # Development Docker image
│   └── docker-compose.yml           # Docker Compose configuration
├── .github/                         # GitHub Actions workflows
│   └── workflows/                   # CI/CD workflows
├── DEVELOPMENT_RULES.md             # This file
├── README.md                        # Project overview
├── SECURITY.md                      # Security policy
├── CONTRIBUTING.md                  # Contribution guidelines
├── LICENSE                          # License information
├── Cargo.toml                       # Rust workspace configuration
└── Cargo.lock                       # Rust dependency lock file
```

## Final Implementation Checklist

### 22. Pre-Development Checklist
- [ ] Development environment setup complete
- [ ] All required tools installed (Rust, Node.js, Docker)
- [ ] IDE configured with proper extensions
- [ ] Git hooks configured for code quality
- [ ] Documentation templates ready

### 23. Per-Feature Development Checklist
- [ ] Feature requirements documented
- [ ] API design reviewed and approved
- [ ] Database schema changes planned
- [ ] Security implications assessed
- [ ] Performance impact analyzed
- [ ] Internationalization considered
- [ ] Accessibility requirements defined

### 24. Code Quality Checklist
- [ ] All functions have comprehensive documentation
- [ ] Error handling implemented throughout
- [ ] Logging added at appropriate levels
- [ ] Input validation implemented
- [ ] Unit tests written and passing
- [ ] Integration tests written and passing
- [ ] Performance tests written and passing
- [ ] Security tests written and passing

### 25. UI/UX Checklist
- [ ] Dark mode implementation complete
- [ ] Light mode implementation complete
- [ ] Responsive design verified
- [ ] Accessibility standards met (WCAG 2.1 AA)
- [ ] Internationalization implemented
- [ ] User experience tested
- [ ] Performance optimized (Core Web Vitals)

### 26. Deployment Checklist
- [ ] Production build tested
- [ ] Security scan completed
- [ ] Performance benchmarks met
- [ ] Documentation updated
- [ ] Monitoring configured
- [ ] Backup procedures tested
- [ ] Rollback procedures tested

## Success Metrics

### 27. Performance Targets
- **Latency**: < 1ms for critical operations
- **Throughput**: > 100K requests/second
- **Concurrency**: Support 1M+ concurrent connections
- **Memory**: < 1GB RAM for 100K active users
- **CPU**: < 50% utilization under normal load

### 28. Quality Targets
- **Test Coverage**: > 90% code coverage
- **Bug Rate**: < 1 bug per 1000 lines of code
- **Security**: Zero critical vulnerabilities
- **Accessibility**: WCAG 2.1 AA compliance
- **Performance**: Core Web Vitals in green zone

### 29. Operational Targets
- **Uptime**: 99.99% availability
- **Recovery Time**: < 5 minutes for critical failures
- **Backup**: Daily automated backups with verification
- **Monitoring**: 100% coverage of critical components
- **Documentation**: 100% API documentation coverage

---

**Remember**: These rules are designed to ensure we build a production-ready, enterprise-grade mail server that can handle millions of users while maintaining the highest standards of code quality, security, and user experience.
