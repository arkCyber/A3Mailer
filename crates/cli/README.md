# Stalwart CLI

A comprehensive command-line interface for managing A3Mailer Mail Server with production-grade features including robust error handling, comprehensive logging, and extensive testing.

## Features

- **Account Management**: Create, update, delete, and manage user accounts
- **Domain Management**: Manage domains and DNS records
- **DKIM Signatures**: Create and manage DKIM signatures for email authentication
- **Group Management**: Create and manage user groups and mailing lists
- **Queue Management**: Monitor and manage SMTP message queues
- **Report Management**: Handle DMARC and TLS reports
- **Import/Export**: Import and export JMAP accounts and mailboxes
- **Server Management**: Database maintenance, configuration management

## Installation

```bash
cargo build --release -p stalwart-cli
```

## Configuration

The CLI can be configured using command-line arguments or environment variables:

### Authentication

```bash
# Using command-line arguments
stalwart-cli --url https://mail.example.com --credentials admin:password <command>

# Using environment variables
export URL=https://mail.example.com
export CREDENTIALS=admin:password
stalwart-cli <command>

# Using OAuth (interactive)
stalwart-cli --url https://mail.example.com <command>
```

### Connection Options

- `--url`: Server base URL
- `--credentials`: Authentication credentials (username:password or token)
- `--timeout`: Connection timeout in seconds (default: 60)

## Commands

### Account Management

```bash
# Create a new user account
stalwart-cli account create john.doe password123 \
  --description "John Doe" \
  --quota 1000000000 \
  --addresses john@example.com,john.doe@example.com

# Update an existing account
stalwart-cli account update john.doe \
  --password newpassword123 \
  --quota 2000000000

# Add email addresses
stalwart-cli account add-email john.doe john.d@example.com

# Remove email addresses
stalwart-cli account remove-email john.doe old@example.com

# Delete an account
stalwart-cli account delete john.doe

# Display account information
stalwart-cli account display john.doe

# List all accounts
stalwart-cli account list --filter john --limit 10
```

### Domain Management

```bash
# Create a new domain
stalwart-cli domain create example.com

# Delete a domain
stalwart-cli domain delete example.com

# List DNS records for a domain
stalwart-cli domain dns-records example.com

# List all domains
stalwart-cli domain list --limit 50
```

### DKIM Management

```bash
# Create RSA DKIM signature
stalwart-cli dkim create rsa example.com \
  --signature-id default \
  --selector mail

# Create Ed25519 DKIM signature
stalwart-cli dkim create ed25519 example.com \
  --selector ed25519

# Get DKIM public key
stalwart-cli dkim get-public-key default
```

### Group Management

```bash
# Create a group
stalwart-cli group create developers \
  --email developers@example.com \
  --description "Development Team" \
  --members john.doe,jane.smith

# Update a group
stalwart-cli group update developers \
  --description "Software Development Team"

# Add members to a group
stalwart-cli group add-members developers alice.johnson

# Remove members from a group
stalwart-cli group remove-members developers john.doe

# Display group information
stalwart-cli group display developers

# List all groups
stalwart-cli group list
```

### Mailing List Management

```bash
# Create a mailing list
stalwart-cli list create announcements \
  --email announcements@example.com \
  --description "Company Announcements" \
  --members all-staff@example.com

# Add members to a list
stalwart-cli list add-members announcements new-employee@example.com

# Remove members from a list
stalwart-cli list remove-members announcements former-employee@example.com
```

### Queue Management

```bash
# List queued messages
stalwart-cli queue list \
  --sender user@example.com \
  --before "2024-01-01T00:00:00Z" \
  --page-size 50

# Show message status
stalwart-cli queue status message-id-1 message-id-2

# Retry message delivery
stalwart-cli queue retry \
  --domain example.com \
  --time "2024-01-01T12:00:00Z"

# Cancel message delivery
stalwart-cli queue cancel \
  --sender spam@example.com \
  --before "2024-01-01T00:00:00Z"
```

### Report Management

```bash
# List DMARC/TLS reports
stalwart-cli report list \
  --domain example.com \
  --format dmarc \
  --page-size 20

# Show report details
stalwart-cli report status report-id-1

# Cancel report delivery
stalwart-cli report cancel report-id-1 report-id-2
```

### Import/Export

```bash
# Import messages from Maildir
stalwart-cli import messages maildir \
  --account john.doe \
  --num-concurrent 4 \
  /path/to/maildir

# Import messages from mbox
stalwart-cli import messages mbox \
  --account john.doe \
  /path/to/mbox/file

# Import from stdin
cat messages.mbox | stalwart-cli import messages mbox --account john.doe -

# Export JMAP account
stalwart-cli export account john.doe \
  --num-concurrent 8 \
  /path/to/export/directory

# Import JMAP account
stalwart-cli import account john.doe \
  --num-concurrent 4 \
  /path/to/exported/account
```

### Server Management

```bash
# Perform database maintenance
stalwart-cli server database-maintenance

# Reload TLS certificates
stalwart-cli server reload-certificates

# Reload configuration
stalwart-cli server reload-config

# Add configuration key
stalwart-cli server add-config "smtp.timeout" "30s"

# Delete configuration key
stalwart-cli server delete-config "smtp.timeout"

# List configuration
stalwart-cli server list-config --prefix "smtp"
```

## Error Handling

The CLI provides comprehensive error handling with clear error messages:

- **Authentication Errors**: Clear messages for invalid credentials
- **Validation Errors**: Input validation with helpful suggestions
- **Network Errors**: Timeout and connection error handling
- **API Errors**: Detailed error messages from the server

## Logging

The CLI provides detailed logging for all operations:

- **Request Logging**: All HTTP requests are logged with timestamps
- **Progress Indicators**: Visual progress for long-running operations
- **Success Confirmations**: Clear success messages with relevant details
- **Error Context**: Detailed error information for troubleshooting

## Security Features

- **Credential Protection**: Secure handling of authentication credentials
- **TLS Support**: Full TLS support with certificate validation
- **OAuth Support**: Interactive OAuth authentication flow
- **Input Validation**: Comprehensive input validation and sanitization

## Performance

- **Concurrent Operations**: Configurable concurrency for bulk operations
- **Connection Pooling**: Efficient HTTP connection management
- **Timeout Management**: Configurable timeouts for all operations
- **Memory Efficiency**: Optimized memory usage for large operations

## Testing

The CLI includes comprehensive test coverage:

```bash
# Run all tests
cargo test -p stalwart-cli --lib

# Run specific module tests
cargo test -p stalwart-cli --lib account
cargo test -p stalwart-cli --lib dkim
cargo test -p stalwart-cli --lib client
```

## Examples

### Complete Account Setup

```bash
# Create domain
stalwart-cli domain create example.com

# Create DKIM signature
stalwart-cli dkim create rsa example.com --selector mail

# Create user account
stalwart-cli account create john.doe password123 \
  --description "John Doe" \
  --quota 5000000000 \
  --addresses john@example.com,john.doe@example.com

# Create group
stalwart-cli group create staff \
  --email staff@example.com \
  --members john.doe

# Import existing mailbox
stalwart-cli import messages maildir \
  --account john.doe \
  --num-concurrent 4 \
  /path/to/existing/maildir
```

### Bulk Operations

```bash
# Process multiple accounts
for user in alice bob charlie; do
  stalwart-cli account create $user password123 \
    --addresses $user@example.com
done

# Bulk queue management
stalwart-cli queue retry --domain problematic-domain.com
stalwart-cli queue cancel --sender spam@badactor.com
```

## Troubleshooting

### Common Issues

1. **Authentication Failed**: Verify credentials and admin privileges
2. **Connection Timeout**: Increase timeout or check network connectivity
3. **Invalid Domain**: Ensure domain format is correct (e.g., example.com)
4. **Permission Denied**: Verify account has necessary permissions

### Debug Mode

For detailed debugging information, set the log level:

```bash
RUST_LOG=debug stalwart-cli <command>
```

## Contributing

When contributing to the CLI:

1. Add comprehensive tests for new functionality
2. Include proper error handling and validation
3. Update documentation for new commands
4. Follow the existing code style and patterns
5. Ensure all tests pass before submitting

## License

This project is licensed under AGPL-3.0-only OR LicenseRef-SEL.
