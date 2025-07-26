//! Circuit breaker module

/// Circuit breaker
pub struct CircuitBreaker;

/// Circuit breaker configuration
pub struct CircuitBreakerConfig;

/// Circuit state
#[derive(Debug, Clone)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}
