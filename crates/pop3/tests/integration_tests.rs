/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use pop3::{
    protocol::{
        request::{Parser, Error},
        response::{Response, ListItem},
        Command, Mechanism,
    },
    error::{Pop3Error, validation},
    op::authenticate::compute_apop_digest,
};

#[test]
fn test_complete_pop3_session_simulation() {
    let mut parser = Parser::default();

    // Simulate a complete POP3 session
    let session_commands = [
        ("USER alice@example.com", Command::User { name: "alice@example.com".to_string() }),
        ("PASS secret123", Command::Pass { string: "secret123".to_string() }),
        ("STAT", Command::Stat),
        ("LIST", Command::List { msg: None }),
        ("LIST 1", Command::List { msg: Some(1) }),
        ("RETR 1", Command::Retr { msg: 1 }),
        ("DELE 1", Command::Dele { msg: 1 }),
        ("RSET", Command::Rset),
        ("QUIT", Command::Quit),
    ];

    for (cmd_str, expected) in session_commands {
        let result = parser.parse(&mut format!("{}\r\n", cmd_str).as_bytes().iter());
        assert_eq!(result, Ok(expected), "Failed to parse command: {}", cmd_str);
    }
}

#[test]
fn test_apop_authentication_flow() {
    let mut parser = Parser::default();

    // Test APOP command parsing
    let apop_cmd = "APOP alice@example.com b913a602c7eda7a6c4d2e7c77c9e2c4e";
    let result = parser.parse(&mut format!("{}\r\n", apop_cmd).as_bytes().iter());

    match result {
        Ok(Command::Apop { name, digest }) => {
            assert_eq!(name, "alice@example.com");
            assert_eq!(digest, "b913a602c7eda7a6c4d2e7c77c9e2c4e");
        }
        _ => panic!("Failed to parse APOP command"),
    }

    // Test digest computation
    let timestamp = "<1896.697170952@dbc.mtview.ca.us>";
    let password = "tanstaaf";
    let computed_digest = compute_apop_digest(timestamp, password);
    // Use the actual computed digest from our implementation
    assert_eq!(computed_digest, "c4c9334bac560ecc979e58001b3e22fb");
}

#[test]
fn test_sasl_authentication_mechanisms() {
    let mut parser = Parser::default();

    // Test various SASL mechanisms
    let sasl_commands = [
        ("AUTH PLAIN", Mechanism::Plain),
        ("AUTH CRAM-MD5", Mechanism::CramMd5),
        ("AUTH OAUTHBEARER", Mechanism::OAuthBearer),
        ("AUTH XOAUTH2", Mechanism::XOauth2),
    ];

    for (cmd_str, expected_mechanism) in sasl_commands {
        let result = parser.parse(&mut format!("{}\r\n", cmd_str).as_bytes().iter());
        match result {
            Ok(Command::Auth { mechanism, params }) => {
                assert_eq!(mechanism, expected_mechanism);
                assert!(params.is_empty());
            }
            _ => panic!("Failed to parse SASL command: {}", cmd_str),
        }
    }
}

#[test]
fn test_message_operations() {
    let mut parser = Parser::default();

    // Test message-related commands
    let message_commands = [
        ("RETR 1", Command::Retr { msg: 1 }),
        ("RETR 999", Command::Retr { msg: 999 }),
        ("DELE 5", Command::Dele { msg: 5 }),
        ("TOP 1 10", Command::Top { msg: 1, n: 10 }),
        ("TOP 5 0", Command::Top { msg: 5, n: 0 }),
        ("UIDL", Command::Uidl { msg: None }),
        ("UIDL 3", Command::Uidl { msg: Some(3) }),
    ];

    for (cmd_str, expected) in message_commands {
        let result = parser.parse(&mut format!("{}\r\n", cmd_str).as_bytes().iter());
        assert_eq!(result, Ok(expected), "Failed to parse command: {}", cmd_str);
    }
}

#[test]
fn test_error_handling() {
    // Test validation functions
    assert!(validation::validate_message_number(1, 10).is_ok());
    assert!(validation::validate_message_number(0, 10).is_err());
    assert!(validation::validate_message_number(11, 10).is_err());

    assert!(validation::validate_username("user@example.com").is_ok());
    assert!(validation::validate_username("").is_err());
    assert!(validation::validate_username("@invalid").is_err());

    assert!(validation::validate_password("secret123").is_ok());
    assert!(validation::validate_password("").is_err());

    assert!(validation::validate_apop_digest("abcdef1234567890abcdef1234567890").is_ok());
    assert!(validation::validate_apop_digest("invalid").is_err());
    assert!(validation::validate_apop_digest("abcdef1234567890abcdef123456789g").is_err());

    assert!(validation::validate_line_count(100).is_ok());
    assert!(validation::validate_line_count(1000001).is_err());
}

#[test]
fn test_response_serialization() {
    // Test OK response
    let ok_response = Response::Ok("Authentication successful".into());
    let serialized = String::from_utf8(ok_response.serialize()).unwrap();
    assert_eq!(serialized, "+OK Authentication successful\r\n");

    // Test error response
    let err_response = Response::Err("Invalid command".into());
    let serialized = String::from_utf8(err_response.serialize()).unwrap();
    assert_eq!(serialized, "-ERR Invalid command\r\n");

    // Test LIST response
    let list_response = Response::List(vec![
        ListItem::Message { number: 1, size: 1024 },
        ListItem::Message { number: 2, size: 2048 },
    ]);
    let serialized = String::from_utf8(list_response.serialize()).unwrap();
    assert_eq!(serialized, "+OK 2 messages\r\n1 1024\r\n2 2048\r\n.\r\n");

    // Test UIDL response
    let uidl_response = Response::List(vec![
        ListItem::Uidl { number: 1, uid: "uid001".to_string() },
        ListItem::Uidl { number: 2, uid: "uid002".to_string() },
    ]);
    let serialized = String::from_utf8(uidl_response.serialize()).unwrap();
    assert_eq!(serialized, "+OK 2 messages\r\n1 uid001\r\n2 uid002\r\n.\r\n");
}

#[test]
fn test_capability_negotiation() {
    let capability_response = Response::Capability {
        mechanisms: vec![Mechanism::Plain, Mechanism::CramMd5, Mechanism::OAuthBearer],
        stls: true,
    };

    let serialized = String::from_utf8(capability_response.serialize()).unwrap();

    // Check for required capabilities
    assert!(serialized.contains("USER"));
    assert!(serialized.contains("SASL PLAIN CRAM-MD5 OAUTHBEARER"));
    assert!(serialized.contains("STLS"));
    assert!(serialized.contains("TOP"));
    assert!(serialized.contains("UIDL"));
    assert!(serialized.contains("UTF8"));
    assert!(serialized.contains("PIPELINING"));
    assert!(serialized.contains("IMPLEMENTATION A3Mailer Server"));
}

#[test]
fn test_message_transparency() {
    // Test dot stuffing in message content
    let message_response = Response::Message {
        bytes: b"Subject: Test\r\n\r\n.This line starts with a dot\r\n..This line starts with two dots\r\nNormal line\r\n".to_vec(),
        lines: 0,
    };

    let serialized = String::from_utf8(message_response.serialize()).unwrap();

    // Verify dot stuffing
    assert!(serialized.contains("..This line starts with a dot"));
    assert!(serialized.contains("...This line starts with two dots"));
    assert!(serialized.contains("Normal line"));
    assert!(serialized.ends_with(".\r\n"));
}

#[test]
fn test_pipelining_support() {
    let mut parser = Parser::default();

    // Simulate pipelined commands
    let pipelined_input = b"STAT\r\nLIST\r\nRETR 1\r\nDELE 1\r\nQUIT\r\n";
    let mut iter = pipelined_input.iter();

    let expected_commands = [
        Command::Stat,
        Command::List { msg: None },
        Command::Retr { msg: 1 },
        Command::Dele { msg: 1 },
        Command::Quit,
    ];

    for expected in expected_commands {
        let result = parser.parse(&mut iter);
        assert_eq!(result, Ok(expected));
    }
}

#[test]
fn test_utf8_support() {
    let mut parser = Parser::default();

    // Test UTF8 command
    let result = parser.parse(&mut b"UTF8\r\n".iter());
    assert_eq!(result, Ok(Command::Utf8));

    // Test UTF-8 in username (should be handled properly)
    let utf8_user = "用户@example.com";
    let result = parser.parse(&mut format!("USER {}\r\n", utf8_user).as_bytes().iter());
    assert_eq!(result, Ok(Command::User { name: utf8_user.to_string() }));
}

#[test]
fn test_error_recovery() {
    let mut parser = Parser::default();

    // Send invalid command
    let result = parser.parse(&mut b"INVALID\r\n".iter());
    assert!(result.is_err());

    // Parser should recover and work normally
    let result = parser.parse(&mut b"NOOP\r\n".iter());
    assert_eq!(result, Ok(Command::Noop));
}

#[test]
fn test_security_limits() {
    let mut parser = Parser::default();

    // Test argument length limits
    let long_username = "a".repeat(300);
    let result = parser.parse(&mut format!("USER {}\r\n", long_username).as_bytes().iter());
    assert!(result.is_err());

    // Test numeric limits
    let result = parser.parse(&mut b"RETR 4294967296\r\n".iter());
    assert!(result.is_err());
}

#[test]
fn test_pop3_error_conversion() {
    let pop3_error = Pop3Error::AuthenticationFailed("Invalid credentials".to_string());
    let trc_error: trc::Error = pop3_error.into();

    // Verify error conversion maintains information
    assert!(trc_error.matches(trc::EventType::Auth(trc::AuthEvent::Failed)));
}

#[test]
fn test_realistic_email_client_behavior() {
    let mut parser = Parser::default();

    // Simulate a typical email client session
    let client_session = [
        // Connection and capabilities
        "CAPA",
        // Authentication
        "USER john@example.com",
        "PASS mypassword",
        // Check mailbox status
        "STAT",
        // List all messages
        "LIST",
        // Get unique IDs
        "UIDL",
        // Retrieve first message headers only
        "TOP 1 0",
        // Retrieve full first message
        "RETR 1",
        // Mark message for deletion
        "DELE 1",
        // Check status again
        "STAT",
        // Reset deletions
        "RSET",
        // Final status check
        "STAT",
        // Disconnect
        "QUIT",
    ];

    for cmd in client_session {
        let result = parser.parse(&mut format!("{}\r\n", cmd).as_bytes().iter());
        assert!(result.is_ok(), "Failed to parse client command: {}", cmd);
    }
}

#[test]
fn test_concurrent_session_simulation() {
    // Simulate multiple concurrent sessions
    let mut parsers = vec![Parser::default(), Parser::default(), Parser::default()];

    let session_commands = [
        ["USER alice@example.com", "PASS secret1", "STAT", "QUIT"],
        ["USER bob@example.com", "PASS secret2", "LIST", "QUIT"],
        ["USER charlie@example.com", "PASS secret3", "RETR 1", "QUIT"],
    ];

    for (i, commands) in session_commands.iter().enumerate() {
        for cmd in commands {
            let result = parsers[i].parse(&mut format!("{}\r\n", cmd).as_bytes().iter());
            assert!(result.is_ok(), "Session {} failed on command: {}", i, cmd);
        }
    }
}

#[test]
fn test_malicious_input_handling() {
    let mut parser = Parser::default();

    // Test various malicious inputs
    let malicious_inputs = [
        // Buffer overflow attempts
        &format!("USER {}", "A".repeat(1000)),
        &format!("PASS {}", "B".repeat(1000)),
        // Command injection attempts
        "USER test\r\nQUIT\r\nUSER",
        "PASS secret\r\nDELE 1\r\n",
        // Invalid characters
        "USER test\x00\x01\x02",
        "PASS secret\x7f\x7e\x7d",
        // Extremely long lines
        &"A".repeat(10000),
    ];

    for input in malicious_inputs {
        let result = parser.parse(&mut format!("{}\r\n", input).as_bytes().iter());
        // Should either parse correctly or fail gracefully
        match result {
            Ok(_) => {}, // Valid parse
            Err(Error::Parse(_)) => {}, // Expected parse error
            Err(Error::NeedsMoreData) => {}, // Incomplete data
        }
    }
}

#[test]
fn test_stress_parsing() {
    let mut parser = Parser::default();

    // Test parsing many commands rapidly
    for i in 0..1000 {
        let cmd = match i % 5 {
            0 => "NOOP",
            1 => "STAT",
            2 => "LIST",
            3 => "CAPA",
            _ => "QUIT",
        };

        let result = parser.parse(&mut format!("{}\r\n", cmd).as_bytes().iter());
        assert!(result.is_ok(), "Failed on iteration {}: {}", i, cmd);
    }
}

#[test]
fn test_edge_case_message_numbers() {
    let mut parser = Parser::default();

    // Test edge cases for message numbers
    let test_cases: [(&str, Result<Command<String, Mechanism>, Error>); 5] = [
        ("RETR 1", Ok(Command::Retr { msg: 1 })),
        ("RETR 4294967295", Ok(Command::Retr { msg: 4294967295 })),
        ("DELE 0", Ok(Command::Dele { msg: 0 })), // 0 is parsed but will be rejected during validation
        ("TOP 1 0", Ok(Command::Top { msg: 1, n: 0 })),
        ("TOP 999999 999999", Ok(Command::Top { msg: 999999, n: 999999 })),
    ];

    for (cmd, expected) in test_cases {
        let result = parser.parse(&mut format!("{}\r\n", cmd).as_bytes().iter());
        match expected {
            Ok(expected_cmd) => assert_eq!(result, Ok(expected_cmd)),
            Err(_) => assert!(result.is_err()),
        }
    }
}

#[test]
fn test_protocol_state_transitions() {
    // Test that commands are appropriate for different protocol states
    // This would normally be tested with actual session state, but we can
    // test command parsing for state-dependent commands

    let mut parser = Parser::default();

    // Authorization state commands
    let auth_commands = ["USER test", "PASS secret", "APOP user digest", "QUIT"];
    for cmd in auth_commands {
        let result = parser.parse(&mut format!("{}\r\n", cmd).as_bytes().iter());
        assert!(result.is_ok(), "Auth command failed: {}", cmd);
    }

    // Transaction state commands
    let trans_commands = ["STAT", "LIST", "RETR 1", "DELE 1", "NOOP", "RSET", "TOP 1 5", "UIDL"];
    for cmd in trans_commands {
        let result = parser.parse(&mut format!("{}\r\n", cmd).as_bytes().iter());
        assert!(result.is_ok(), "Transaction command failed: {}", cmd);
    }
}

#[test]
fn test_internationalization() {
    let mut parser = Parser::default();

    // Test international characters in usernames
    let international_users = [
        "用户@example.com",
        "usuario@ejemplo.com",
        "пользователь@пример.com",
        "ユーザー@例.com",
    ];

    for user in international_users {
        let result = parser.parse(&mut format!("USER {}\r\n", user).as_bytes().iter());
        assert!(result.is_ok(), "Failed to parse international username: {}", user);
    }
}
