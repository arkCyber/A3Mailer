/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Basic usage example for the Stalwart POP3 server
//!
//! This example demonstrates how to:
//! - Create and configure a POP3 server
//! - Handle client connections
//! - Parse POP3 commands
//! - Generate appropriate responses

use pop3::{
    config::{Pop3Config, ServerConfig},
    security::SecurityConfig,
    protocol::{
        request::Parser,
        response::{Response, ListItem},
        Mechanism, Command,
    },
    error::validation,
    op::authenticate::compute_apop_digest,
    security::SecurityManager,
};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Stalwart POP3 Server - Basic Usage Example");
    println!("==========================================");

    // 1. Create a custom configuration
    let config = create_custom_config();
    println!("✓ Configuration created");

    // 2. Demonstrate command parsing
    demonstrate_command_parsing()?;
    println!("✓ Command parsing demonstrated");

    // 3. Demonstrate response generation
    demonstrate_response_generation()?;
    println!("✓ Response generation demonstrated");

    // 4. Demonstrate APOP authentication
    demonstrate_apop_authentication()?;
    println!("✓ APOP authentication demonstrated");

    // 5. Demonstrate security features
    demonstrate_security_features()?;
    println!("✓ Security features demonstrated");

    // 6. Demonstrate validation
    demonstrate_validation()?;
    println!("✓ Validation demonstrated");

    println!("\n🎉 All examples completed successfully!");
    Ok(())
}

fn create_custom_config() -> Pop3Config {
    Pop3Config {
        server: ServerConfig {
            greeting: "Welcome to My POP3 Server".to_string(),
            max_message_size: 25 * 1024 * 1024, // 25MB
            session_timeout: Duration::from_secs(1200), // 20 minutes
            unauth_timeout: Duration::from_secs(180), // 3 minutes
            enable_apop: true,
            enable_utf8: true,
            enable_stls: true,
        },
        security: SecurityConfig {
            max_auth_attempts: 5,
            auth_window: Duration::from_secs(600), // 10 minutes
            max_connections_per_ip: 10,
            max_commands_per_minute: 60,
            min_command_delay: Duration::from_millis(25),
            enable_security_logging: true,
            suspicious_threshold: 8,
        },
        ..Default::default()
    }
}

fn demonstrate_command_parsing() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Command Parsing Examples ---");

    let mut parser = Parser::default();

    let test_commands = [
        ("USER alice@example.com\r\n", "USER command"),
        ("PASS secretpassword\r\n", "PASS command"),
        ("APOP user@example.com b913a602c7eda7a6c4d2e7c77c9e2c4e\r\n", "APOP command"),
        ("AUTH PLAIN dGVzdAB0ZXN0AHRlc3Q=\r\n", "AUTH PLAIN command"),
        ("STAT\r\n", "STAT command"),
        ("LIST\r\n", "LIST all messages"),
        ("LIST 5\r\n", "LIST specific message"),
        ("RETR 1\r\n", "RETR command"),
        ("TOP 1 10\r\n", "TOP command"),
        ("DELE 2\r\n", "DELE command"),
        ("UIDL\r\n", "UIDL all messages"),
        ("NOOP\r\n", "NOOP command"),
        ("RSET\r\n", "RSET command"),
        ("CAPA\r\n", "CAPA command"),
        ("QUIT\r\n", "QUIT command"),
    ];

    for (cmd_str, description) in test_commands {
        match parser.parse(&mut cmd_str.as_bytes().iter()) {
            Ok(command) => {
                println!("  ✓ {}: {:?}", description, command);
            }
            Err(e) => {
                println!("  ✗ {}: Error - {:?}", description, e);
            }
        }
    }

    Ok(())
}

fn demonstrate_response_generation() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Response Generation Examples ---");

    // OK response
    let ok_response = Response::Ok("Authentication successful".into());
    let serialized = String::from_utf8(ok_response.serialize())?;
    println!("  OK Response: {}", serialized.trim());

    // Error response
    let err_response = Response::Err("Invalid command".into());
    let serialized = String::from_utf8(err_response.serialize())?;
    println!("  Error Response: {}", serialized.trim());

    // LIST response
    let list_response = Response::List(vec![
        ListItem::Message { number: 1, size: 1024 },
        ListItem::Message { number: 2, size: 2048 },
        ListItem::Message { number: 3, size: 512 },
    ]);
    let serialized = String::from_utf8(list_response.serialize())?;
    println!("  LIST Response:\n{}", indent_lines(&serialized));

    // UIDL response
    let uidl_response = Response::List(vec![
        ListItem::Uidl { number: 1, uid: "uid001".to_string() },
        ListItem::Uidl { number: 2, uid: "uid002".to_string() },
    ]);
    let serialized = String::from_utf8(uidl_response.serialize())?;
    println!("  UIDL Response:\n{}", indent_lines(&serialized));

    // Capability response
    let capa_response = Response::Capability {
        mechanisms: vec![Mechanism::Plain, Mechanism::CramMd5, Mechanism::OAuthBearer],
        stls: true,
    };
    let serialized = String::from_utf8(capa_response.serialize())?;
    println!("  CAPA Response:\n{}", indent_lines(&serialized));

    Ok(())
}

fn demonstrate_apop_authentication() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- APOP Authentication Examples ---");

    // Example from RFC 1939
    let timestamp = "<1896.697170952@dbc.mtview.ca.us>";
    let password = "tanstaaf";
    let expected_digest = "b913a602c7eda7a6c4d2e7c77c9e2c4e";

    let computed_digest = compute_apop_digest(timestamp, password);
    println!("  Timestamp: {}", timestamp);
    println!("  Password: {}", password);
    println!("  Expected digest: {}", expected_digest);
    println!("  Computed digest: {}", computed_digest);
    println!("  Match: {}", computed_digest == expected_digest);

    // Custom example
    let custom_timestamp = "<test.123@example.com>";
    let custom_password = "mysecretpassword";
    let custom_digest = compute_apop_digest(custom_timestamp, custom_password);
    println!("\n  Custom example:");
    println!("  Timestamp: {}", custom_timestamp);
    println!("  Password: {}", custom_password);
    println!("  Digest: {}", custom_digest);

    Ok(())
}

fn demonstrate_security_features() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Security Features Examples ---");

    let security_config = SecurityConfig::default();
    let security_manager = SecurityManager::new(security_config);

    // Simulate IP address
    let test_ip = "192.168.1.100".parse()?;

    // Check initial authentication
    match security_manager.check_auth_allowed(test_ip) {
        Ok(()) => println!("  ✓ Initial authentication check passed"),
        Err(e) => println!("  ✗ Initial authentication check failed: {}", e),
    }

    // Simulate failed authentication attempts
    for i in 1..=3 {
        security_manager.record_auth_attempt(test_ip, false);
        println!("  Recorded failed auth attempt #{}", i);
    }

    // Check if still allowed after max attempts
    match security_manager.check_auth_allowed(test_ip) {
        Ok(()) => println!("  ✓ Authentication still allowed"),
        Err(e) => println!("  ✗ Authentication blocked: {}", e),
    }

    // Simulate successful authentication (should clear attempts)
    security_manager.record_auth_attempt(test_ip, true);
    println!("  Recorded successful authentication");

    match security_manager.check_auth_allowed(test_ip) {
        Ok(()) => println!("  ✓ Authentication allowed after successful login"),
        Err(e) => println!("  ✗ Authentication still blocked: {}", e),
    }

    // Get security statistics
    let stats = security_manager.get_stats();
    println!("  Security Stats: {:?}", stats);

    Ok(())
}

fn demonstrate_validation() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Validation Examples ---");

    // Message number validation
    match validation::validate_message_number(5, 10) {
        Ok(index) => println!("  ✓ Message number 5 -> index {}", index),
        Err(e) => println!("  ✗ Message number validation failed: {}", e),
    }

    match validation::validate_message_number(0, 10) {
        Ok(index) => println!("  ✓ Message number 0 -> index {}", index),
        Err(e) => println!("  ✗ Message number 0 validation failed: {}", e),
    }

    // Username validation
    let usernames = ["user@example.com", "", "@invalid", "user@"];
    for username in usernames {
        match validation::validate_username(username) {
            Ok(()) => println!("  ✓ Username '{}' is valid", username),
            Err(e) => println!("  ✗ Username '{}' is invalid: {}", username, e),
        }
    }

    // APOP digest validation
    let digests = ["b913a602c7eda7a6c4d2e7c77c9e2c4e", "invalid", "too_short"];
    for digest in digests {
        match validation::validate_apop_digest(digest) {
            Ok(()) => println!("  ✓ APOP digest '{}' is valid", digest),
            Err(e) => println!("  ✗ APOP digest '{}' is invalid: {}", digest, e),
        }
    }

    Ok(())
}

fn indent_lines(text: &str) -> String {
    text.lines()
        .map(|line| format!("    {}", line))
        .collect::<Vec<_>>()
        .join("\n")
}
