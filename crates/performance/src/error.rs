//! Error Handling for A3Mailer Performance Module
//!
//! This module provides comprehensive error types and handling for the
//! performance optimization system.

use std::fmt;
use serde::{Deserialize, Serialize};

/// Result type for performance operations
pub type Result<T> = std::result::Result<T, PerformanceError>;

/// Performance-related errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceError {
    /// Cache-related errors
    CacheError(String),
    
    /// Connection pool errors
    PoolError(String),
    
    /// Load balancer errors
    LoadBalancerError(String),
    
    /// Memory management errors
    MemoryError(String),
    
    /// Configuration errors
    ConfigError(String),
    
    /// Network-related errors
    NetworkError(String),
    
    /// Timeout errors
    TimeoutError(String),
    
    /// Resource exhaustion errors
    ResourceExhaustedError(String),
    
    /// Serialization/deserialization errors
    SerializationError(String),
    
    /// I/O errors
    IoError(String),
    
    /// Generic performance errors
    GenericError(String),
}

impl fmt::Display for PerformanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PerformanceError::CacheError(msg) => write!(f, "Cache error: {}", msg),
            PerformanceError::PoolError(msg) => write!(f, "Connection pool error: {}", msg),
            PerformanceError::LoadBalancerError(msg) => write!(f, "Load balancer error: {}", msg),
            PerformanceError::MemoryError(msg) => write!(f, "Memory management error: {}", msg),
            PerformanceError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            PerformanceError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            PerformanceError::TimeoutError(msg) => write!(f, "Timeout error: {}", msg),
            PerformanceError::ResourceExhaustedError(msg) => write!(f, "Resource exhausted: {}", msg),
            PerformanceError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            PerformanceError::IoError(msg) => write!(f, "I/O error: {}", msg),
            PerformanceError::GenericError(msg) => write!(f, "Performance error: {}", msg),
        }
    }
}

impl std::error::Error for PerformanceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<std::io::Error> for PerformanceError {
    fn from(error: std::io::Error) -> Self {
        PerformanceError::IoError(error.to_string())
    }
}

impl From<serde_json::Error> for PerformanceError {
    fn from(error: serde_json::Error) -> Self {
        PerformanceError::SerializationError(error.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for PerformanceError {
    fn from(error: tokio::time::error::Elapsed) -> Self {
        PerformanceError::TimeoutError(error.to_string())
    }
}

impl From<reqwest::Error> for PerformanceError {
    fn from(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            PerformanceError::TimeoutError(error.to_string())
        } else if error.is_connect() {
            PerformanceError::NetworkError(error.to_string())
        } else {
            PerformanceError::GenericError(error.to_string())
        }
    }
}

/// Error context for better error reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub operation: String,
    pub component: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new(operation: &str, component: &str) -> Self {
        Self {
            operation: operation.to_string(),
            component: component.to_string(),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Add metadata to the error context
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// Enhanced error with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualError {
    pub error: PerformanceError,
    pub context: ErrorContext,
}

impl ContextualError {
    /// Create a new contextual error
    pub fn new(error: PerformanceError, context: ErrorContext) -> Self {
        Self { error, context }
    }
}

impl fmt::Display for ContextualError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (operation: {}, component: {}, timestamp: {})",
            self.error,
            self.context.operation,
            self.context.component,
            self.context.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        )
    }
}

impl std::error::Error for ContextualError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Error recovery strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// Retry the operation
    Retry {
        max_attempts: u32,
        delay_ms: u64,
        backoff_multiplier: f64,
    },
    
    /// Fallback to alternative method
    Fallback {
        alternative: String,
    },
    
    /// Circuit breaker pattern
    CircuitBreaker {
        failure_threshold: u32,
        recovery_timeout_ms: u64,
    },
    
    /// Graceful degradation
    Degrade {
        reduced_functionality: String,
    },
    
    /// Fail fast
    FailFast,
}

/// Error recovery manager
#[derive(Debug)]
pub struct ErrorRecoveryManager {
    strategies: std::collections::HashMap<String, RecoveryStrategy>,
    retry_counts: std::collections::HashMap<String, u32>,
}

impl ErrorRecoveryManager {
    /// Create a new error recovery manager
    pub fn new() -> Self {
        let mut strategies = std::collections::HashMap::new();
        
        // Default strategies for different error types
        strategies.insert(
            "cache".to_string(),
            RecoveryStrategy::Fallback {
                alternative: "direct_database_access".to_string(),
            }
        );
        
        strategies.insert(
            "pool".to_string(),
            RecoveryStrategy::Retry {
                max_attempts: 3,
                delay_ms: 100,
                backoff_multiplier: 2.0,
            }
        );
        
        strategies.insert(
            "load_balancer".to_string(),
            RecoveryStrategy::CircuitBreaker {
                failure_threshold: 5,
                recovery_timeout_ms: 30000,
            }
        );
        
        strategies.insert(
            "memory".to_string(),
            RecoveryStrategy::Degrade {
                reduced_functionality: "disable_caching".to_string(),
            }
        );
        
        Self {
            strategies,
            retry_counts: std::collections::HashMap::new(),
        }
    }

    /// Get recovery strategy for a component
    pub fn get_strategy(&self, component: &str) -> Option<&RecoveryStrategy> {
        self.strategies.get(component)
    }

    /// Record a retry attempt
    pub fn record_retry(&mut self, operation_id: &str) -> u32 {
        let count = self.retry_counts.entry(operation_id.to_string()).or_insert(0);
        *count += 1;
        *count
    }

    /// Reset retry count for an operation
    pub fn reset_retry_count(&mut self, operation_id: &str) {
        self.retry_counts.remove(operation_id);
    }

    /// Check if retry limit is exceeded
    pub fn is_retry_limit_exceeded(&self, operation_id: &str, max_attempts: u32) -> bool {
        self.retry_counts.get(operation_id).unwrap_or(&0) >= &max_attempts
    }
}

impl Default for ErrorRecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro for creating contextual errors
#[macro_export]
macro_rules! context_error {
    ($error:expr, $operation:expr, $component:expr) => {
        ContextualError::new(
            $error,
            ErrorContext::new($operation, $component)
        )
    };
    
    ($error:expr, $operation:expr, $component:expr, $($key:expr => $value:expr),*) => {
        {
            let mut context = ErrorContext::new($operation, $component);
            $(
                context = context.with_metadata($key, $value);
            )*
            ContextualError::new($error, context)
        }
    };
}

/// Macro for retry logic with exponential backoff
#[macro_export]
macro_rules! retry_with_backoff {
    ($operation:expr, $max_attempts:expr, $initial_delay_ms:expr, $backoff_multiplier:expr) => {
        {
            let mut attempts = 0;
            let mut delay = std::time::Duration::from_millis($initial_delay_ms);
            
            loop {
                attempts += 1;
                
                match $operation {
                    Ok(result) => break Ok(result),
                    Err(error) => {
                        if attempts >= $max_attempts {
                            break Err(error);
                        }
                        
                        tokio::time::sleep(delay).await;
                        delay = std::time::Duration::from_millis(
                            (delay.as_millis() as f64 * $backoff_multiplier) as u64
                        );
                    }
                }
            }
        }
    };
}

/// Performance error metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    pub total_errors: u64,
    pub errors_by_type: std::collections::HashMap<String, u64>,
    pub errors_by_component: std::collections::HashMap<String, u64>,
    pub recovery_attempts: u64,
    pub successful_recoveries: u64,
    pub error_rate: f64,
}

impl ErrorMetrics {
    /// Create new error metrics
    pub fn new() -> Self {
        Self {
            total_errors: 0,
            errors_by_type: std::collections::HashMap::new(),
            errors_by_component: std::collections::HashMap::new(),
            recovery_attempts: 0,
            successful_recoveries: 0,
            error_rate: 0.0,
        }
    }

    /// Record an error
    pub fn record_error(&mut self, error_type: &str, component: &str) {
        self.total_errors += 1;
        
        *self.errors_by_type.entry(error_type.to_string()).or_insert(0) += 1;
        *self.errors_by_component.entry(component.to_string()).or_insert(0) += 1;
    }

    /// Record a recovery attempt
    pub fn record_recovery_attempt(&mut self, successful: bool) {
        self.recovery_attempts += 1;
        if successful {
            self.successful_recoveries += 1;
        }
    }

    /// Calculate error rate
    pub fn calculate_error_rate(&mut self, total_operations: u64) {
        if total_operations > 0 {
            self.error_rate = (self.total_errors as f64 / total_operations as f64) * 100.0;
        }
    }

    /// Get recovery success rate
    pub fn recovery_success_rate(&self) -> f64 {
        if self.recovery_attempts > 0 {
            (self.successful_recoveries as f64 / self.recovery_attempts as f64) * 100.0
        } else {
            0.0
        }
    }
}

impl Default for ErrorMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Error reporting utilities
pub mod reporting {
    use super::*;
    use tracing::{error, warn, info};

    /// Report error with appropriate log level
    pub fn report_error(error: &PerformanceError, context: Option<&ErrorContext>) {
        match error {
            PerformanceError::CacheError(_) | 
            PerformanceError::PoolError(_) => {
                warn!("Performance warning: {}", error);
            }
            
            PerformanceError::LoadBalancerError(_) |
            PerformanceError::MemoryError(_) |
            PerformanceError::ResourceExhaustedError(_) => {
                error!("Performance error: {}", error);
            }
            
            PerformanceError::TimeoutError(_) |
            PerformanceError::NetworkError(_) => {
                warn!("Transient error: {}", error);
            }
            
            _ => {
                info!("Performance info: {}", error);
            }
        }

        if let Some(ctx) = context {
            info!("Error context: operation={}, component={}, timestamp={}", 
                  ctx.operation, ctx.component, ctx.timestamp);
        }
    }

    /// Generate error summary report
    pub fn generate_error_summary(metrics: &ErrorMetrics) -> String {
        format!(
            "Error Summary:\n\
             Total Errors: {}\n\
             Error Rate: {:.2}%\n\
             Recovery Success Rate: {:.2}%\n\
             Top Error Types: {:?}\n\
             Top Error Components: {:?}",
            metrics.total_errors,
            metrics.error_rate,
            metrics.recovery_success_rate(),
            metrics.errors_by_type,
            metrics.errors_by_component
        )
    }
}
