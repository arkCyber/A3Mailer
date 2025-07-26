# Contributing to A3Mailer 🤝

Thank you for your interest in contributing to **A3Mailer** - the world's first AI-Powered Web3-Native Mail Server! We welcome contributions from developers of all skill levels.

## 🤖⛓️ What is A3Mailer?

**A3** represents the fusion of cutting-edge technologies:
- **A** = **Artificial Intelligence** - Smart threat detection, automated content analysis, and intelligent routing
- **3** = **Web3** - Blockchain integration, decentralized identity, and cryptographic security

This guide will help you contribute to the future of email communication.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Coding Standards](#coding-standards)
- [Testing Guidelines](#testing-guidelines)
- [Documentation](#documentation)
- [Pull Request Process](#pull-request-process)
- [Community](#community)

## 📜 Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## 🚀 Getting Started

### Prerequisites

- **Rust** (latest stable version)
- **Git**
- **Docker** (for testing)
- **PostgreSQL** or **SQLite** (for database testing)

### Quick Setup

```bash
# Clone the repository
git clone https://github.com/a3mailer/a3mailer.git
cd a3mailer

# Install Rust if you haven't already
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install required tools
cargo install cargo-audit cargo-deny cargo-outdated

# Build the project
cargo build

# Run tests
cargo test
```

## 🛠️ Development Setup

### Environment Configuration

1. **Copy the example configuration:**
   ```bash
   cp resources/config/spamfilter.toml config.toml
   ```

2. **Set up your development database:**
   ```bash
   # For PostgreSQL
   createdb a3mailer_dev

   # For SQLite (default)
   # No setup required
   ```

3. **Run the development server:**
   ```bash
   cargo run --bin a3mailer -- --config config.toml
   ```

### IDE Setup

We recommend using **VS Code** with the following extensions:
- **rust-analyzer** - Rust language support
- **CodeLLDB** - Debugging support
- **Better TOML** - Configuration file support
- **GitLens** - Git integration

## 🤝 How to Contribute

### 🐛 Reporting Bugs

1. **Check existing issues** to avoid duplicates
2. **Use the bug report template** when creating new issues
3. **Provide detailed information:**
   - A3Mailer version
   - Operating system
   - Steps to reproduce
   - Expected vs actual behavior
   - Relevant logs

### 💡 Suggesting Features

1. **Check the roadmap** and existing feature requests
2. **Use the feature request template**
3. **Provide clear use cases** and benefits
4. **Consider implementation complexity**

### 🔧 Code Contributions

1. **Fork the repository**
2. **Create a feature branch:**
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes**
4. **Write tests** for new functionality
5. **Update documentation** if needed
6. **Commit your changes:**
   ```bash
   git commit -m "feat: add amazing new feature"
   ```
7. **Push to your fork:**
   ```bash
   git push origin feature/your-feature-name
   ```
8. **Create a Pull Request**

## 📝 Coding Standards

### Rust Guidelines

- **Follow Rust conventions** and idioms
- **Use `rustfmt`** for code formatting:
  ```bash
  cargo fmt
  ```
- **Use `clippy`** for linting:
  ```bash
  cargo clippy -- -D warnings
  ```
- **Write comprehensive documentation:**
  ```rust
  /// Processes incoming email messages with spam filtering
  ///
  /// # Arguments
  ///
  /// * `message` - The email message to process
  /// * `config` - Spam filter configuration
  ///
  /// # Returns
  ///
  /// Returns `Ok(FilterResult)` on success, `Err(FilterError)` on failure
  ///
  /// # Examples
  ///
  /// ```rust
  /// let result = process_message(&message, &config)?;
  /// ```
  pub fn process_message(message: &Message, config: &Config) -> Result<FilterResult, FilterError> {
      // Implementation
  }
  ```

### Code Organization

- **Keep functions small** and focused
- **Use meaningful names** for variables and functions
- **Organize code into logical modules**
- **Minimize dependencies** between modules
- **Handle errors gracefully** with proper error types

### Performance Considerations

- **Avoid unnecessary allocations**
- **Use async/await** for I/O operations
- **Profile performance-critical code**
- **Consider memory usage** in data structures
- **Use appropriate data structures** for the task

## 🧪 Testing Guidelines

### Test Types

1. **Unit Tests** - Test individual functions and modules
2. **Integration Tests** - Test component interactions
3. **End-to-End Tests** - Test complete workflows
4. **Performance Tests** - Benchmark critical paths

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spam_detection() {
        // Arrange
        let message = create_test_message();
        let config = SpamConfig::default();

        // Act
        let result = detect_spam(&message, &config).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().score, 0.1);
    }

    #[test]
    fn test_email_parsing() {
        let raw_email = "From: test@example.com\r\n\r\nTest message";
        let parsed = parse_email(raw_email).unwrap();
        assert_eq!(parsed.from, "test@example.com");
    }
}
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_spam_detection

# Run tests with output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration

# Run with coverage
cargo tarpaulin --out Html
```

## 📚 Documentation

### Code Documentation

- **Document all public APIs** with rustdoc comments
- **Include examples** in documentation
- **Explain complex algorithms** and business logic
- **Document error conditions** and edge cases

### User Documentation

- **Update relevant guides** when adding features
- **Include configuration examples**
- **Provide migration guides** for breaking changes
- **Add troubleshooting information**

### Building Documentation

```bash
# Build API documentation
cargo doc --open

# Check documentation links
cargo doc --no-deps
```

## 🔄 Pull Request Process

### Before Submitting

1. **Ensure all tests pass:**
   ```bash
   cargo test
   ```

2. **Check code formatting:**
   ```bash
   cargo fmt --check
   ```

3. **Run clippy:**
   ```bash
   cargo clippy -- -D warnings
   ```

4. **Update documentation** if needed

5. **Add changelog entry** for user-facing changes

### PR Requirements

- **Clear title** describing the change
- **Detailed description** of what and why
- **Link to related issues**
- **Screenshots** for UI changes
- **Breaking change notes** if applicable

### Review Process

1. **Automated checks** must pass
2. **Code review** by maintainers
3. **Testing** in development environment
4. **Documentation review** if applicable
5. **Final approval** and merge

## 💬 Community

### Getting Help

- **[GitHub Discussions](https://github.com/a3mailer/a3mailer/discussions)** - Q&A and general discussion
- **[Discord](https://discord.gg/a3mailer)** - Real-time chat
- **[Reddit](https://www.reddit.com/r/a3mailer/)** - Community discussions

### Communication Guidelines

- **Be respectful** and inclusive
- **Stay on topic** in discussions
- **Search before asking** to avoid duplicates
- **Provide context** when asking for help
- **Help others** when you can

## 🏆 Recognition

Contributors are recognized in several ways:

- **Contributor list** in the repository
- **Release notes** mention significant contributions
- **Special badges** for long-term contributors
- **Swag and merchandise** for notable contributions

## 📞 Contact

- **Maintainers:** [@a3mailer-team](https://github.com/orgs/a3mailer/teams/maintainers)
- **Security:** security@a3mailer.com
- **General:** hello@a3mailer.com

---

Thank you for contributing to A3Mailer! 🚀
