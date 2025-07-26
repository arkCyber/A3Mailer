# Stalwart POP3 Server

A high-performance, production-ready POP3 server implementation in Rust, part of the A3Mailer Mail Server suite.

## Features

### Core POP3 Protocol Support
- **RFC 1939 Compliance**: Full implementation of the POP3 protocol
- **APOP Authentication**: Secure challenge-response authentication using MD5
- **SASL Support**: Multiple SASL mechanisms including PLAIN, CRAM-MD5, OAUTHBEARER, XOAUTH2
- **STARTTLS**: Secure connections with TLS encryption
- **UTF8 Support**: International character support
- **Pipelining**: Command pipelining for improved performance

### Security Features
- **Rate Limiting**: Configurable limits on authentication attempts and command rates
- **Connection Limits**: Per-IP connection restrictions
- **Suspicious Activity Detection**: Automatic detection of malicious patterns
- **Security Logging**: Comprehensive security event logging
- **Brute Force Protection**: Automatic IP blocking after failed attempts

### Performance Optimizations
- **Async/Await**: Fully asynchronous implementation using Tokio
- **Connection Pooling**: Efficient resource management
- **Response Caching**: Optional caching for improved performance
- **Memory Efficient**: Optimized for large mailboxes and messages
- **Concurrent Sessions**: Support for thousands of concurrent connections

### Production Ready
- **Comprehensive Testing**: Unit tests, integration tests, and benchmarks
- **Error Handling**: Robust error handling and recovery
- **Configuration Management**: Flexible configuration system
- **Monitoring**: Built-in metrics and health checks
- **Documentation**: Extensive documentation and examples

## Quick Start

### Basic Usage

```rust
use pop3::{Pop3SessionManager, config::Pop3Config};
use std::sync::Arc;

// Create configuration
let config = Pop3Config::production();

// Create session manager
let inner = Arc::new(/* your server inner */);
let manager = Pop3SessionManager::with_security_config(inner, config.security);

// Handle incoming connections
// (Integration with your server framework)
```

### Configuration

```rust
use pop3::config::{Pop3Config, ServerConfig, SecurityConfig};
use std::time::Duration;

let config = Pop3Config {
    server: ServerConfig {
        greeting: "My POP3 Server".to_string(),
        max_message_size: 50 * 1024 * 1024, // 50MB
        session_timeout: Duration::from_secs(1800),
        enable_apop: true,
        enable_utf8: true,
        enable_stls: true,
        ..Default::default()
    },
    security: SecurityConfig {
        max_auth_attempts: 3,
        auth_window: Duration::from_secs(900),
        max_connections_per_ip: 5,
        enable_security_logging: true,
        ..Default::default()
    },
    ..Default::default()
};
```

### APOP Authentication

```rust
use pop3::op::authenticate::compute_apop_digest;

// Server generates timestamp in greeting
let timestamp = "<1896.697170952@dbc.mtview.ca.us>";
let password = "tanstaaf";

// Compute expected digest
let expected_digest = compute_apop_digest(timestamp, password);

// Client sends: APOP username digest
// Server verifies digest matches expected value
```

## Protocol Support

### Supported Commands

| Command | Description | Status |
|---------|-------------|--------|
| USER    | Specify username | ✅ |
| PASS    | Specify password | ✅ |
| APOP    | APOP authentication | ✅ |
| AUTH    | SASL authentication | ✅ |
| STAT    | Get mailbox statistics | ✅ |
| LIST    | List messages | ✅ |
| RETR    | Retrieve message | ✅ |
| DELE    | Mark message for deletion | ✅ |
| NOOP    | No operation | ✅ |
| RSET    | Reset session | ✅ |
| TOP     | Get message headers + lines | ✅ |
| UIDL    | Get unique message IDs | ✅ |
| CAPA    | Get server capabilities | ✅ |
| STLS    | Start TLS | ✅ |
| UTF8    | Enable UTF8 mode | ✅ |
| QUIT    | End session | ✅ |

### SASL Mechanisms

- **PLAIN**: Simple username/password authentication
- **CRAM-MD5**: Challenge-response using MD5
- **DIGEST-MD5**: Digest authentication
- **SCRAM-SHA-1**: Salted Challenge Response Authentication Mechanism
- **SCRAM-SHA-256**: SCRAM with SHA-256
- **OAUTHBEARER**: OAuth 2.0 bearer tokens
- **XOAUTH2**: Extended OAuth 2.0
- **GSSAPI**: Generic Security Services API
- **NTLM**: NT LAN Manager authentication
- **EXTERNAL**: External authentication
- **ANONYMOUS**: Anonymous access

## Configuration

### Environment-Specific Configurations

```rust
// Production configuration
let prod_config = Pop3Config::production();

// Development configuration  
let dev_config = Pop3Config::development();

// Custom configuration
let custom_config = Pop3Config::default();
```

### Configuration File

```toml
[server]
greeting = "Stalwart POP3 Server"
max_message_size = 104857600  # 100MB
session_timeout = "1h"
unauth_timeout = "3m"
enable_apop = true
enable_utf8 = true
enable_stls = true

[security]
max_auth_attempts = 3
auth_window = "15m"
max_connections_per_ip = 5
max_commands_per_minute = 30
min_command_delay = "50ms"
enable_security_logging = true
suspicious_threshold = 5

[protocol]
max_line_length = 4096
max_arguments = 5
enable_pipelining = true
max_pipelined_commands = 50
enable_top = true
enable_uidl = true
max_top_lines = 500

[performance]
max_concurrent_sessions = 500
connection_pool_size = 50
io_buffer_size = 4096
enable_response_caching = true
cache_ttl = "10m"
max_cached_responses = 500
```

## Security

### Rate Limiting

The server implements multiple layers of rate limiting:

- **Authentication attempts**: Limit failed login attempts per IP
- **Command rate**: Limit commands per minute per session
- **Connection limits**: Maximum concurrent connections per IP
- **Minimum delays**: Anti-spam delays between commands

### Suspicious Activity Detection

Automatic detection of:
- Brute force authentication attempts
- Command flooding attacks
- Unusual connection patterns
- Protocol violations

### Security Logging

Comprehensive logging of security events:
- Failed authentication attempts
- Rate limit violations
- Suspicious activity detection
- Connection refused events

## Performance

### Benchmarks

Run benchmarks with:

```bash
cargo bench --package pop3
```

### Optimization Tips

1. **Enable response caching** for read-heavy workloads
2. **Tune connection pool size** based on expected load
3. **Adjust I/O buffer sizes** for your network conditions
4. **Configure appropriate timeouts** for your use case
5. **Enable pipelining** for better throughput

## Testing

### Unit Tests

```bash
cargo test --package pop3
```

### Integration Tests

```bash
cargo test --package pop3 --test integration_tests
```

### Benchmarks

```bash
cargo bench --package pop3
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

This project is licensed under the AGPL-3.0-only OR LicenseRef-SEL license.

## Support

For support and questions:
- GitHub Issues: [Report bugs and request features](https://github.com/stalwartlabs/stalwart-mail-server/issues)
- Documentation: [Full documentation](https://stalw.art/docs)
- Community: [Join our community](https://stalw.art/community)
