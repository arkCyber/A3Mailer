# A3Mailer Integration Tests

This crate provides comprehensive integration and stress testing capabilities for the A3Mailer system. It includes a full suite of tests covering authentication, email protocols, performance, security, and real-world scenarios.

## Features

- **Authentication Testing**: Comprehensive authentication and authorization tests
- **Email Protocol Testing**: SMTP, IMAP, POP3, and JMAP protocol testing
- **Stress Testing**: High-load and performance testing
- **Security Testing**: Vulnerability assessment and security compliance
- **Scenario Testing**: Real-world usage scenario simulation
- **Metrics Collection**: Detailed performance and resource usage metrics
- **Flexible Configuration**: Environment-specific configurations and templates
- **Multiple Output Formats**: Text, JSON, CSV, and HTML reporting

## Installation

Build the A3Mailer integration testing tool:

```bash
cargo build -p stalwart-integration-tests --bin integration-test --release
```

## Quick Start

### Configuration Management

1. **Generate a configuration template**:
```bash
./target/release/integration-test generate-config basic --output my-config.toml
```

2. **Validate the configuration**:
```bash
./target/release/integration-test validate-config my-config.toml
```

### Running All Tests

```bash
# Run all integration tests with configuration
./target/release/integration-test --config my-config.toml all

# Run all tests with verbose output
./target/release/integration-test --config my-config.toml --verbose all

# Dry run (validate configuration only)
./target/release/integration-test --config my-config.toml --dry-run all
```

### Running Specific Test Suites

```bash
# Authentication tests
./target/release/integration-test --config config.toml auth

# Email communication tests
./target/release/integration-test --config config.toml email

# Stress tests
./target/release/integration-test --config config.toml stress

# Security tests
./target/release/integration-test --config config.toml security

# Scenario tests
./target/release/integration-test --config config.toml scenarios
```

### Configuration Templates

The tool provides several pre-configured templates:

- **basic**: Simple testing with minimal load
- **stress**: High-load performance testing
- **corporate**: Enterprise-level testing scenarios
- **development**: Development environment testing

Generate a template:
```bash
./target/release/integration-test generate-config [TEMPLATE] --output [FILE]
```

### Output Formats

```bash
# JSON output
./target/release/integration-test --config config.toml --output json --output-file results.json all

# CSV output
./target/release/integration-test --config config.toml --output csv --output-file results.csv all

# HTML report
./target/release/integration-test --config config.toml --output html --output-file report.html all
```

## Test Suites

### 1. Authentication Tests (`auth`)

Tests authentication and authorization mechanisms:
- Basic authentication
- Multi-factor authentication
- OAuth integration
- LDAP authentication
- Session management
- Permission validation

### 2. Email Communication Tests (`email`)

Tests email protocols and operations:
- **SMTP**: Email sending, delivery, authentication
- **IMAP**: Folder management, message retrieval, search
- **POP3**: Message download, deletion
- **JMAP**: Modern email API operations
- **Attachments**: File handling and size limits
- **Bulk Operations**: High-volume email processing

### 3. Stress Tests (`stress`)

Performance and load testing:
- **Concurrent Users**: Multiple simultaneous connections
- **High Volume**: Large-scale email processing
- **Memory Stress**: Memory allocation and usage testing
- **CPU Stress**: Processor-intensive operations
- **Protocol Stress**: Protocol-specific load testing
- **Endurance**: Long-running stability tests

### 4. Security Tests (`security`)

Security vulnerability assessment:
- **Authentication Security**: Brute force protection, password policies
- **Authorization**: Privilege escalation, access control bypass
- **Input Validation**: Injection attacks, malformed input
- **Encryption**: TLS configuration, certificate validation
- **Rate Limiting**: DoS protection, API limits
- **Compliance**: OWASP, NIST, ISO 27001, GDPR, HIPAA

### 5. Scenario Tests (`scenarios`)

Real-world usage scenarios:
- **Basic Email Workflow**: Send and receive operations
- **Corporate Environment**: Multi-department email patterns
- **High-Volume Server**: Large-scale deployment simulation
- **Multi-Domain Hosting**: Cross-domain email delivery
- **User Onboarding**: New user setup and first-time usage
- **Performance Degradation**: System behavior under stress

## Configuration

### Configuration Files

Generate configuration templates:

```bash
# Basic configuration
cargo run --bin integration-test generate-config basic -o basic-config.toml

# Stress testing configuration
cargo run --bin integration-test generate-config stress -o stress-config.toml

# Corporate environment configuration
cargo run --bin integration-test generate-config corporate -o corporate-config.toml

# Development configuration
cargo run --bin integration-test generate-config development -o dev-config.toml
```

### Environment Variables

Configure tests using environment variables:

```bash
export STALWART_TEST_HOST=localhost
export STALWART_TEST_SMTP_PORT=587
export STALWART_TEST_IMAP_PORT=143
export STALWART_TEST_DOMAIN=test.local
export STALWART_TEST_USER_COUNT=10
export STALWART_TEST_CONCURRENCY=5
```

### Configuration Validation

```bash
# Validate configuration file
cargo run --bin integration-test validate-config my-config.toml
```

## Output Formats

### Text Output (Default)

```bash
cargo run --bin integration-test all
```

### JSON Output

```bash
cargo run --bin integration-test --output json all
```

### CSV Output

```bash
cargo run --bin integration-test --output csv all
```

### HTML Report

```bash
cargo run --bin integration-test --output html --output-file report.html all
```

### Detailed Report

```bash
cargo run --bin integration-test --detailed-report all
```

## Advanced Usage

### Custom Configuration

```bash
# Use custom configuration file
cargo run --bin integration-test --config my-config.toml all

# Specify environment
cargo run --bin integration-test --environment production all
```

### Verbose Output

```bash
# Enable verbose logging
cargo run --bin integration-test --verbose all
```

### Dry Run

```bash
# Validate configuration without running tests
cargo run --bin integration-test --dry-run all
```

### Continue on Failure

```bash
# Continue running tests even if some fail
cargo run --bin integration-test --continue-on-failure all
```

## Examples

### Basic Testing

```bash
# Quick smoke test
cargo run --bin integration-test auth
cargo run --bin integration-test email

# Full integration test
cargo run --bin integration-test all
```

### Performance Testing

```bash
# Concurrent user stress test
cargo run --bin stress-test concurrent-users --users 100 --duration 300

# High volume email test
cargo run --bin stress-test high-volume --email-count 10000 --batch-size 100

# Memory stress test
cargo run --bin stress-test memory --max-memory-mb 1024

# CPU stress test
cargo run --bin stress-test cpu --workers 8 --duration 120
```

### Security Testing

```bash
# Basic security scan
cargo run --bin integration-test security

# OWASP compliance test
cargo run --bin integration-test security --include-compliance --framework owasp

# Full security assessment
cargo run --bin integration-test security --include-compliance
```

### Scenario Testing

```bash
# Corporate environment simulation
cargo run --bin integration-test scenarios --scenario corporate --users 100

# User onboarding workflow
cargo run --bin integration-test scenarios --scenario onboarding --users 5
```

## Metrics and Reporting

The test suite automatically collects comprehensive metrics:

- **Execution Metrics**: Test counts, success rates, durations
- **Performance Metrics**: Response times, throughput, operations per second
- **Resource Metrics**: Memory usage, CPU utilization, network I/O
- **Protocol Metrics**: Protocol-specific statistics and error rates

### Metrics Analysis

The system provides automatic analysis with:
- Performance insights and recommendations
- Trend analysis
- Compliance scoring
- Resource efficiency assessment

## Development

### Adding New Tests

1. Create test functions in the appropriate module
2. Add test registration to the suite
3. Update configuration if needed
4. Add documentation and examples

### Test Structure

```rust
use stalwart_integration_tests::*;

#[tokio::test]
async fn test_my_feature() -> Result<()> {
    let config = TestConfig::default();
    let context = TestContext::new(config);

    // Your test logic here

    Ok(())
}
```

### Running Tests in Development

```bash
# Run unit tests
cargo test

# Run integration tests
cargo test --test integration

# Run benchmarks
cargo bench
```

## Troubleshooting

### Common Issues

1. **Connection Refused**: Ensure Stalwart server is running
2. **Authentication Failed**: Check credentials in configuration
3. **Timeout Errors**: Increase timeout values in configuration
4. **Memory Issues**: Reduce concurrency or test data size

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug cargo run --bin integration-test --verbose all
```

### Configuration Issues

```bash
# Validate configuration
cargo run --bin integration-test validate-config config.toml

# Show configuration options
cargo run --bin integration-test info --config-options
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

This project is licensed under the AGPL-3.0 license. See LICENSE file for details.

## Support

For support and questions:
- GitHub Issues: [stalwartlabs/mail-server](https://github.com/stalwartlabs/mail-server)
- Documentation: [stalw.art/docs](https://stalw.art/docs)
- Community: [stalw.art/community](https://stalw.art/community)
