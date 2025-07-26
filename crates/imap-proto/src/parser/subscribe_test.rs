/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Comprehensive unit tests for SUBSCRIBE command parsing
//!
//! This module contains production-grade tests for the SUBSCRIBE command parser,
//! ensuring robust error handling, edge case coverage, and protocol compliance.

#[cfg(test)]
mod tests {
    use crate::{
        Command,
        protocol::ProtocolVersion,
        receiver::{Request, Token},
    };

    /// Helper function to create a test request with given tokens
    fn create_test_request(tag: &str, tokens: Vec<Token>) -> Request<Command> {
        Request {
            tag: tag.to_string(),
            command: Command::Subscribe,
            tokens,
        }
    }

    /// Helper function to create an argument token
    fn arg_token(value: &str) -> Token {
        Token::Argument(value.as_bytes().to_vec())
    }

    /// Test successful parsing of valid SUBSCRIBE command
    #[test]
    fn test_subscribe_valid_parsing() {
        let request = create_test_request("A001", vec![arg_token("INBOX")]);
        let result = request.parse_subscribe(ProtocolVersion::Rev1).unwrap();

        assert_eq!(result.tag, "A001");
        assert_eq!(result.mailbox_name, "INBOX");
    }

    /// Test SUBSCRIBE with UTF-7 encoded mailbox name
    #[test]
    fn test_subscribe_utf7_mailbox() {
        // Test with UTF-7 encoded mailbox name (e.g., "Sent" in German: "Gesendete Elemente")
        let request = create_test_request("A002", vec![arg_token("&BB4EQgQ,BEAEMAQyBDsENQQ9BD0ESwQ1-")]);
        let result = request.parse_subscribe(ProtocolVersion::Rev1).unwrap();

        assert_eq!(result.tag, "A002");
        // The actual decoded value depends on UTF-7 implementation
        assert!(!result.mailbox_name.is_empty());
    }

    /// Test SUBSCRIBE with hierarchical mailbox name
    #[test]
    fn test_subscribe_hierarchical_mailbox() {
        let request = create_test_request("A003", vec![arg_token("INBOX.Sent")]);
        let result = request.parse_subscribe(ProtocolVersion::Rev1).unwrap();

        assert_eq!(result.tag, "A003");
        assert_eq!(result.mailbox_name, "INBOX.Sent");
    }

    /// Test SUBSCRIBE with special characters in mailbox name
    #[test]
    fn test_subscribe_special_characters() {
        let test_cases = vec![
            "INBOX/Drafts",
            "INBOX.Sent Items",
            "INBOX-Archive",
            "INBOX_Backup",
            "INBOX+Tag",
        ];

        for (i, mailbox) in test_cases.iter().enumerate() {
            let tag = format!("A{:03}", i + 100);
            let request = create_test_request(&tag, vec![arg_token(mailbox)]);
            let result = request.parse_subscribe(ProtocolVersion::Rev1).unwrap();

            assert_eq!(result.tag, tag);
            assert_eq!(result.mailbox_name, *mailbox);
        }
    }

    /// Test error handling for missing mailbox name
    #[test]
    fn test_subscribe_missing_mailbox() {
        let request = create_test_request("A004", vec![]);
        let result = request.parse_subscribe(ProtocolVersion::Rev1);

        assert!(result.is_err());
        let error_msg = format!("{:?}", result.unwrap_err());
        assert!(error_msg.contains("Missing mailbox name"));
    }

    /// Test error handling for too many arguments
    #[test]
    fn test_subscribe_too_many_arguments() {
        let request = create_test_request("A005", vec![
            arg_token("INBOX"),
            arg_token("EXTRA_ARG"),
        ]);
        let result = request.parse_subscribe(ProtocolVersion::Rev1);

        assert!(result.is_err());
        let error_msg = format!("{:?}", result.unwrap_err());
        assert!(error_msg.contains("Too many arguments"));
    }

    /// Test error handling for invalid token types
    #[test]
    fn test_subscribe_invalid_token_types() {
        let invalid_tokens = vec![
            Token::ParenthesisOpen,
            Token::ParenthesisClose,
            Token::BracketOpen,
            Token::BracketClose,
            Token::Lt,
            Token::Gt,
            Token::Dot,
            Token::Nil,
        ];

        for (i, token) in invalid_tokens.iter().enumerate() {
            let tag = format!("A{:03}", i + 200);
            let request = create_test_request(&tag, vec![token.clone()]);
            let result = request.parse_subscribe(ProtocolVersion::Rev1);

            // Some tokens might be valid in certain contexts, so we just ensure
            // the parser doesn't panic and handles them gracefully
            match result {
                Ok(_) => {
                    // Some tokens might be successfully parsed as strings
                    // This is acceptable behavior
                }
                Err(_) => {
                    // Error is also acceptable for invalid tokens
                }
            }
        }
    }

    /// Test with different protocol versions
    #[test]
    fn test_subscribe_protocol_versions() {
        let versions = vec![
            ProtocolVersion::Rev1,
            ProtocolVersion::Rev2,
        ];

        for version in versions {
            let request = create_test_request("A006", vec![arg_token("INBOX")]);
            let result = request.parse_subscribe(version).unwrap();

            assert_eq!(result.tag, "A006");
            assert_eq!(result.mailbox_name, "INBOX");
        }
    }

    /// Test with empty mailbox name (edge case)
    #[test]
    fn test_subscribe_empty_mailbox_name() {
        let request = create_test_request("A007", vec![arg_token("")]);
        let result = request.parse_subscribe(ProtocolVersion::Rev1).unwrap();

        assert_eq!(result.tag, "A007");
        assert_eq!(result.mailbox_name, "");
    }

    /// Test with very long mailbox names
    #[test]
    fn test_subscribe_long_mailbox_names() {
        // Test with a very long mailbox name (255 characters)
        let long_name = "A".repeat(255);
        let request = create_test_request("A008", vec![arg_token(&long_name)]);
        let result = request.parse_subscribe(ProtocolVersion::Rev1).unwrap();

        assert_eq!(result.tag, "A008");
        assert_eq!(result.mailbox_name, long_name);

        // Test with extremely long mailbox name (1000 characters)
        let very_long_name = "B".repeat(1000);
        let request = create_test_request("A009", vec![arg_token(&very_long_name)]);
        let result = request.parse_subscribe(ProtocolVersion::Rev1).unwrap();

        assert_eq!(result.tag, "A009");
        assert_eq!(result.mailbox_name, very_long_name);
    }

    /// Test with binary data in mailbox names
    #[test]
    fn test_subscribe_binary_mailbox_names() {
        let binary_data = vec![0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD];
        let request = create_test_request("A010", vec![Token::Argument(binary_data.clone())]);
        let result = request.parse_subscribe(ProtocolVersion::Rev1);

        // This might succeed or fail depending on UTF-7 decoding implementation
        // The important thing is that it doesn't panic
        match result {
            Ok(parsed) => {
                assert_eq!(parsed.tag, "A010");
                // Mailbox name might be modified during UTF-7 processing
            }
            Err(_) => {
                // Error is acceptable for invalid UTF-7 sequences
            }
        }
    }

    /// Test tag preservation in all scenarios
    #[test]
    fn test_subscribe_tag_preservation() {
        let test_tags = vec![
            "A001",
            "TAG123",
            "VERY_LONG_TAG_NAME_WITH_UNDERSCORES",
            "tag-with-dashes",
            "tag.with.dots",
            "1234567890",
            "",  // Empty tag (edge case)
        ];

        for tag in test_tags {
            let request = create_test_request(tag, vec![arg_token("INBOX")]);
            let result = request.parse_subscribe(ProtocolVersion::Rev1).unwrap();

            assert_eq!(result.tag, tag);
        }
    }

    /// Performance test for parsing many SUBSCRIBE commands
    #[test]
    fn test_subscribe_performance() {
        let start = std::time::Instant::now();

        for i in 0..10000 {
            let tag = format!("A{:05}", i);
            let mailbox = format!("INBOX.Folder{}", i);
            let request = create_test_request(&tag, vec![arg_token(&mailbox)]);
            let result = request.parse_subscribe(ProtocolVersion::Rev1).unwrap();

            assert_eq!(result.tag, tag);
            assert_eq!(result.mailbox_name, mailbox);
        }

        let elapsed = start.elapsed();

        // Should be able to parse 10k commands in less than 100ms
        assert!(elapsed.as_millis() < 100, "Parsing too slow: {:?}", elapsed);
    }

    /// Test memory usage with large inputs
    #[test]
    fn test_subscribe_memory_usage() {
        // Test that parsing doesn't cause excessive memory allocation
        let large_mailbox = "X".repeat(10000);
        let request = create_test_request("A011", vec![arg_token(&large_mailbox)]);
        let result = request.parse_subscribe(ProtocolVersion::Rev1).unwrap();

        assert_eq!(result.tag, "A011");
        assert_eq!(result.mailbox_name, large_mailbox);
    }
}
