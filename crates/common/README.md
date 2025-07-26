# Stalwart Common Library

A production-grade common library providing core functionality for the Stalwart mail server, including authentication, configuration, monitoring, security, and error handling.

## Features

### 🔐 Authentication & Authorization
- **Access Token Management**: Comprehensive token creation, validation, and lifecycle management
- **SASL Support**: Multiple SASL mechanisms for secure authentication
- **Role-Based Permissions**: Granular permission system with role inheritance
- **Multi-tenant Support**: Tenant isolation and resource management

### ⚙️ Configuration Management
- **Hierarchical Configuration**: Nested configuration with inheritance
- **Environment Variable Support**: Flexible configuration from multiple sources
- **Validation**: Comprehensive configuration validation with detailed error messages
- **Hot Reloading**: Dynamic configuration updates without service restart

### 📊 Monitoring & Observability
- **Metrics Collection**: System and application metrics with Prometheus export
- **Health Checks**: Comprehensive health monitoring with configurable thresholds
- **Alerting**: Multi-level alerting system with deduplication
- **Performance Tracking**: Request tracing and performance analytics

### 🛡️ Security
- **Rate Limiting**: Multiple rate limiting algorithms (token bucket, sliding window, fixed window)
- **Input Validation**: Comprehensive input sanitization and validation
- **Audit Logging**: Detailed security event logging
- **Security Headers**: Automatic security header generation

### 🚨 Error Handling
- **Structured Errors**: Comprehensive error types with context information
- **Error Propagation**: Proper error chaining and context preservation
- **Retry Logic**: Intelligent retry mechanisms with backoff strategies
- **Logging Integration**: Seamless integration with tracing and logging systems

### 🗄️ Storage & Persistence
- **Multiple Backends**: Support for various storage backends
- **Connection Pooling**: Efficient database connection management
- **Transaction Support**: ACID transaction handling
- **Backup & Recovery**: Enterprise-grade backup and recovery features

## Architecture

### Core Components

```
common/
├── auth/           # Authentication and authorization
├── config/         # Configuration management
├── core/           # Core types and utilities
├── error/          # Error handling framework
├── monitoring/     # Metrics, health checks, and alerting
├── security/       # Security features and validation
├── storage/        # Storage abstraction layer
└── telemetry/      # Logging and tracing
```

### Error Handling Framework

The common library provides a comprehensive error handling framework:

```rust
use common::error::{CommonError, CommonResult, ErrorContext};

// Create structured errors
let error = CommonError::auth("Invalid credentials", AuthErrorType::InvalidCredentials)
    .with_context("During login attempt");

// Convert from standard errors
let result: CommonResult<String> = std::fs::read_to_string("config.toml")
    .with_context("Loading configuration file");

// Check if error is retryable
if error.is_retryable() {
    if let Some(delay) = error.retry_delay() {
        tokio::time::sleep(Duration::from_secs(delay)).await;
        // Retry operation
    }
}
```

### Monitoring Integration

```rust
use common::monitoring::{MetricsCollector, HealthChecker, AlertManager};

// Collect system metrics
let collector = SystemMetricsCollector::new();
let metrics = collector.collect_system_metrics().await?;

// Health monitoring
let health_checker = HealthChecker::new();
let health_status = health_checker.check_system_health().await?;

// Alert management
let alert_manager = AlertManager::new();
alert_manager.process_alert(Alert {
    severity: AlertSeverity::Warning,
    message: "High CPU usage detected".to_string(),
    component: "system".to_string(),
    // ...
}).await?;
```

### Security Features

```rust
use common::security::{RateLimiter, InputValidator, SecurityHeaders};

// Rate limiting
let rate_limiter = TokenBucketRateLimiter::new(100, Duration::from_secs(60));
if rate_limiter.check_rate_limit(&client_ip).await? {
    // Process request
}

// Input validation
let validator = InputValidator::new();
let email = validator.validate_email("user@example.com")?;
let domain = validator.validate_domain("example.com")?;

// Security headers
let headers = SecurityHeaders::new()
    .with_csp("default-src 'self'")
    .with_frame_options(FrameOptions::Deny)
    .generate();
```

## Testing

The common library includes comprehensive test coverage with 82 tests covering all major functionality:

```bash
# Run all tests
cargo test -p common --lib

# Run specific module tests
cargo test -p common --lib error
cargo test -p common --lib monitoring
cargo test -p common --lib security
```

### Test Categories

- **Unit Tests**: Individual function and method testing
- **Integration Tests**: Component interaction testing
- **Performance Tests**: Performance benchmarking and validation
- **Security Tests**: Security feature validation
- **Error Handling Tests**: Error propagation and handling validation

## Performance

### Benchmarks

- **Error Creation**: < 1ms for 1000 error instances
- **Metrics Collection**: < 100ms for 10,000 metrics
- **Rate Limiting**: < 10μs per rate limit check
- **Input Validation**: < 1ms for complex validation rules

### Memory Usage

- **Efficient Data Structures**: Optimized memory usage with minimal allocations
- **Connection Pooling**: Reuse of expensive resources
- **Lazy Loading**: On-demand resource initialization

## Configuration

### Basic Configuration

```toml
[auth]
token_expiry = "1h"
max_concurrent_sessions = 100

[monitoring]
metrics_interval = "30s"
health_check_interval = "10s"

[security]
rate_limit_requests = 1000
rate_limit_window = "1m"

[storage]
connection_pool_size = 10
connection_timeout = "30s"
```

### Environment Variables

```bash
STALWART_AUTH_TOKEN_EXPIRY=3600
STALWART_MONITORING_METRICS_INTERVAL=30
STALWART_SECURITY_RATE_LIMIT=1000
```

## Production Deployment

### Requirements

- **Rust**: 1.70+ (MSRV)
- **Memory**: Minimum 512MB, recommended 2GB+
- **CPU**: 2+ cores recommended for production workloads
- **Storage**: SSD recommended for optimal performance

### Monitoring

The library provides comprehensive monitoring capabilities:

- **Prometheus Metrics**: Built-in Prometheus metrics export
- **Health Endpoints**: HTTP health check endpoints
- **Structured Logging**: JSON-formatted logs with correlation IDs
- **Distributed Tracing**: OpenTelemetry integration

### Security Considerations

- **Input Validation**: All inputs are validated and sanitized
- **Rate Limiting**: Built-in protection against abuse
- **Audit Logging**: Comprehensive security event logging
- **Secure Defaults**: Security-first default configurations

## Development

### Adding New Features

1. **Error Handling**: All new features must include proper error handling
2. **Testing**: Comprehensive test coverage required (>90%)
3. **Documentation**: All public APIs must be documented
4. **Performance**: Performance impact must be measured and documented

### Code Quality

- **Linting**: Use `cargo clippy` for code quality checks
- **Formatting**: Use `cargo fmt` for consistent formatting
- **Testing**: Use `cargo test` for comprehensive testing
- **Documentation**: Use `cargo doc` for documentation generation

## Contributing

1. **Fork** the repository
2. **Create** a feature branch
3. **Add** comprehensive tests
4. **Update** documentation
5. **Submit** a pull request

### Guidelines

- Follow Rust best practices and idioms
- Maintain backward compatibility
- Add comprehensive error handling
- Include performance benchmarks for new features
- Update documentation for all changes

## License

This project is licensed under AGPL-3.0-only OR LicenseRef-SEL.

## Support

For support and questions:

- **Documentation**: Check the inline documentation
- **Issues**: Report bugs and feature requests on GitHub
- **Community**: Join the Stalwart community discussions

---

**Note**: This library is designed for production use and follows enterprise-grade standards for reliability, security, and performance.
