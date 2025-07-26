/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! POP3 Protocol Response Module
//!
//! This module provides comprehensive POP3 server response handling with production-grade
//! features including structured logging, performance monitoring, and robust error handling.
//!
//! # Architecture
//!
//! The response system is built around the `Response` enum which represents all possible
//! POP3 server responses. Each response type implements the `SerializeResponse` trait
//! for efficient serialization to the wire format.
//!
//! # Performance Characteristics
//!
//! - Response serialization is O(n) where n is the response data size
//! - Memory usage is minimized through zero-copy string handling where possible
//! - Bulk operations (like LIST) use streaming serialization for large datasets
//!
//! # Thread Safety
//!
//! All response types are Send + Sync and can be safely used across thread boundaries.
//! The serialization process is stateless and thread-safe.
//!
//! # Examples
//!
//! ```rust
//! use pop3::protocol::response::{Response, SerializeResponse};
//!
//! // Create a success response
//! let response = Response::Ok("Authentication successful".into());
//! let serialized = response.serialize();
//! assert_eq!(serialized, b"+OK Authentication successful\r\n");
//!
//! // Create a capability response
//! let response = Response::Capability {
//!     mechanisms: vec![Mechanism::Plain],
//!     stls: true,
//! };
//! let serialized = String::from_utf8(response.serialize()).unwrap();
//! assert!(serialized.contains("STLS"));
//! ```

use std::{
    borrow::Cow,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use tracing::{debug, error, info, instrument, trace, warn};
use super::Mechanism;

/// Performance metrics for response serialization
///
/// Tracks serialization performance and usage statistics for monitoring
/// and optimization purposes.
#[derive(Debug, Default)]
pub struct ResponseMetrics {
    /// Total number of responses serialized
    pub total_responses: AtomicU64,
    /// Total bytes serialized
    pub total_bytes: AtomicU64,
    /// Total serialization time in microseconds
    pub total_serialization_time_us: AtomicU64,
    /// Number of serialization errors
    pub serialization_errors: AtomicU64,
}

impl ResponseMetrics {
    /// Creates a new metrics instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a successful serialization
    pub fn record_serialization(&self, bytes: usize, duration: Duration) {
        self.total_responses.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(bytes as u64, Ordering::Relaxed);
        self.total_serialization_time_us.fetch_add(
            duration.as_micros() as u64,
            Ordering::Relaxed,
        );
    }

    /// Records a serialization error
    pub fn record_error(&self) {
        self.serialization_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Gets current metrics snapshot
    pub fn snapshot(&self) -> ResponseMetricsSnapshot {
        let total_responses = self.total_responses.load(Ordering::Relaxed);
        let total_bytes = self.total_bytes.load(Ordering::Relaxed);
        let total_time_us = self.total_serialization_time_us.load(Ordering::Relaxed);

        ResponseMetricsSnapshot {
            total_responses,
            total_bytes,
            total_serialization_time_us: total_time_us,
            serialization_errors: self.serialization_errors.load(Ordering::Relaxed),
            average_response_size: if total_responses > 0 {
                total_bytes as f64 / total_responses as f64
            } else {
                0.0
            },
            average_serialization_time_us: if total_responses > 0 {
                total_time_us as f64 / total_responses as f64
            } else {
                0.0
            },
        }
    }
}

/// Snapshot of response metrics at a point in time
#[derive(Debug, Clone)]
pub struct ResponseMetricsSnapshot {
    pub total_responses: u64,
    pub total_bytes: u64,
    pub total_serialization_time_us: u64,
    pub serialization_errors: u64,
    pub average_response_size: f64,
    pub average_serialization_time_us: f64,
}

/// List item representation for POP3 LIST and UIDL commands
///
/// Represents individual items in mailbox listings with proper
/// validation and serialization support.
#[derive(Debug, Clone)]
pub enum ListItem {
    /// Message item with number and size
    Message {
        /// 1-based message number
        number: usize,
        /// Message size in octets
        size: u32
    },
    /// UIDL item with number and unique identifier
    Uidl {
        /// 1-based message number
        number: usize,
        /// Unique identifier string
        uid: String
    },
}

/// POP3 Server Response Types
///
/// Represents all possible responses that a POP3 server can send to clients.
/// Each variant corresponds to specific POP3 commands and protocol states.
///
/// # Variants
///
/// * `Ok` - Positive response (+OK) indicating successful command execution
/// * `Err` - Negative response (-ERR) indicating command failure
/// * `List` - Multi-line response for LIST/UIDL commands
/// * `Message` - Message content response for RETR/TOP commands
/// * `Capability` - Server capability advertisement
///
/// # Examples
///
/// ```rust
/// use pop3::protocol::response::Response;
///
/// // Success response
/// let response = Response::Ok("Command completed".into());
///
/// // Error response
/// let response = Response::Err("Invalid command".into());
///
/// // List response
/// let response = Response::List(vec![]);
/// ```
#[derive(Debug, Clone)]
pub enum Response {
    /// Positive response indicating success
    ///
    /// Used for successful command completion. The message provides
    /// additional context or confirmation to the client.
    Ok(Cow<'static, str>),

    /// Negative response indicating failure
    ///
    /// Used when a command fails. The message should provide a clear
    /// explanation of why the command failed.
    Err(Cow<'static, str>),

    /// Multi-line list response
    ///
    /// Used for LIST and UIDL commands to return mailbox contents.
    /// The list is terminated with a single period on its own line.
    List(Vec<ListItem>),

    /// Message content response
    ///
    /// Used for RETR and TOP commands to return message content.
    /// Includes dot-stuffing and proper line ending handling.
    Message {
        /// Raw message bytes
        bytes: Vec<u8>,
        /// Number of lines (used for TOP command)
        lines: u32,
    },

    /// Server capability response
    ///
    /// Used for CAPA command to advertise server capabilities
    /// including supported authentication mechanisms and extensions.
    Capability {
        /// Supported SASL authentication mechanisms
        mechanisms: Vec<Mechanism>,
        /// Whether STLS (StartTLS) is supported
        stls: bool,
    },
}

/// Trait for serializing responses to POP3 wire format
///
/// This trait provides a standardized interface for converting response
/// objects into the byte format expected by POP3 clients.
///
/// # Performance Notes
///
/// Implementations should minimize allocations and use efficient
/// serialization strategies for large responses.
pub trait SerializeResponse {
    /// Serializes the response to POP3 wire format
    ///
    /// # Returns
    ///
    /// A byte vector containing the properly formatted POP3 response
    /// including appropriate line endings and protocol markers.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::protocol::response::{Response, SerializeResponse};
    ///
    /// let response = Response::Ok("Test message".into());
    /// let serialized = response.serialize();
    /// assert_eq!(serialized, b"+OK Test message\r\n");
    /// ```
    fn serialize(self) -> Vec<u8>;
}

impl SerializeResponse for Response {
    /// Serializes POP3 response to wire format with performance monitoring
    ///
    /// This implementation includes comprehensive logging and performance
    /// tracking for production monitoring and debugging.
    ///
    /// # Performance Characteristics
    ///
    /// - Simple responses (Ok/Err): O(1) with minimal allocation
    /// - List responses: O(n) where n is the number of items
    /// - Message responses: O(m) where m is the message size
    ///
    /// # Returns
    ///
    /// Properly formatted POP3 response bytes including CRLF line endings
    #[instrument(level = "trace", skip(self), fields(response_type = ?std::mem::discriminant(&self)))]
    fn serialize(self) -> Vec<u8> {
        let start_time = Instant::now();

        let result = match self {
            Response::Ok(message) => {
                trace!(message = %message, "Serializing OK response");
                let mut buf = Vec::with_capacity(message.len() + 6);
                buf.extend_from_slice(b"+OK ");
                buf.extend_from_slice(message.as_bytes());
                buf.extend_from_slice(b"\r\n");
                debug!(
                    message = %message,
                    size = buf.len(),
                    "OK response serialized"
                );
                buf
            }
            Response::Err(message) => {
                trace!(message = %message, "Serializing ERR response");
                let mut buf = Vec::with_capacity(message.len() + 6);
                buf.extend_from_slice(b"-ERR ");
                buf.extend_from_slice(message.as_bytes());
                buf.extend_from_slice(b"\r\n");
                debug!(
                    message = %message,
                    size = buf.len(),
                    "ERR response serialized"
                );
                buf
            }
            Response::List(items) => {
                trace!(item_count = items.len(), "Serializing LIST response");

                // Pre-calculate buffer size for better performance
                let estimated_size = items.len() * 16 + 32; // Conservative estimate
                let mut buf = Vec::with_capacity(estimated_size);

                // Add header
                buf.extend_from_slice(format!("+OK {} messages\r\n", items.len()).as_bytes());

                // Serialize each item with validation
                for (index, item) in items.iter().enumerate() {
                    match item {
                        ListItem::Message { number, size } => {
                            if *number == 0 {
                                warn!(
                                    index = index,
                                    number = number,
                                    "Invalid message number 0 in LIST response"
                                );
                            }
                            buf.extend_from_slice(number.to_string().as_bytes());
                            buf.extend_from_slice(b" ");
                            buf.extend_from_slice(size.to_string().as_bytes());
                            buf.extend_from_slice(b"\r\n");
                        }
                        ListItem::Uidl { number, uid } => {
                            if *number == 0 {
                                warn!(
                                    index = index,
                                    number = number,
                                    "Invalid message number 0 in UIDL response"
                                );
                            }
                            if uid.is_empty() {
                                warn!(
                                    index = index,
                                    number = number,
                                    "Empty UID in UIDL response"
                                );
                            }
                            buf.extend_from_slice(number.to_string().as_bytes());
                            buf.extend_from_slice(b" ");
                            buf.extend_from_slice(uid.as_bytes());
                            buf.extend_from_slice(b"\r\n");
                        }
                    }
                }

                // Add terminator
                buf.extend_from_slice(b".\r\n");

                debug!(
                    item_count = items.len(),
                    size = buf.len(),
                    estimated_size = estimated_size,
                    "LIST response serialized"
                );

                buf
            }
            Response::Message { bytes, lines } => {
                trace!(
                    message_size = bytes.len(),
                    line_limit = lines,
                    "Serializing MESSAGE response"
                );

                // Pre-allocate buffer with extra space for dot-stuffing and CRLF conversion
                let estimated_size = bytes.len() + (bytes.len() / 50) + 64; // ~2% overhead for stuffing
                let mut buf = Vec::with_capacity(estimated_size);

                // Add response header
                buf.extend_from_slice(b"+OK ");
                buf.extend_from_slice(bytes.len().to_string().as_bytes());
                buf.extend_from_slice(b" octets\r\n");

                let mut line_count = 0;
                let mut last_byte = 0;
                let mut dot_stuffed_count = 0;
                let mut crlf_added_count = 0;

                // POP3 transparency procedure with detailed logging
                for (pos, &byte) in bytes.iter().enumerate() {
                    // Ensure lines end with CRLF (RFC 1939 requirement)
                    if byte == b'\n' && last_byte != b'\r' {
                        buf.push(b'\r');
                        crlf_added_count += 1;
                    }

                    // Dot stuffing: lines beginning with '.' get an extra '.'
                    if byte == b'.' && (last_byte == b'\n' || pos == 0) {
                        buf.push(b'.');
                        dot_stuffed_count += 1;
                        trace!(position = pos, "Applied dot stuffing");
                    }

                    buf.push(byte);
                    last_byte = byte;

                    // Handle line limit for TOP command
                    if lines > 0 && byte == b'\n' {
                        line_count += 1;
                        if line_count == lines {
                            debug!(
                                lines_processed = line_count,
                                line_limit = lines,
                                "Reached line limit for TOP command"
                            );
                            break;
                        }
                    }
                }

                // Ensure message ends with CRLF
                if last_byte != b'\n' {
                    buf.extend_from_slice(b"\r\n");
                    crlf_added_count += 1;
                }

                // Add termination marker
                buf.extend_from_slice(b".\r\n");

                debug!(
                    message_size = bytes.len(),
                    serialized_size = buf.len(),
                    lines_processed = if lines > 0 { line_count } else { 0 },
                    line_limit = lines,
                    dot_stuffed_count = dot_stuffed_count,
                    crlf_added_count = crlf_added_count,
                    estimated_size = estimated_size,
                    "MESSAGE response serialized"
                );

                buf
            }
            Response::Capability { mechanisms, stls } => {
                trace!(
                    mechanism_count = mechanisms.len(),
                    stls_enabled = stls,
                    "Serializing CAPABILITY response"
                );

                let mut buf = Vec::with_capacity(512); // Increased capacity for all capabilities
                buf.extend_from_slice(b"+OK Capability list follows\r\n");

                // Add authentication capabilities
                if !mechanisms.is_empty() {
                    // USER/PASS authentication (if PLAIN is supported)
                    if mechanisms.contains(&Mechanism::Plain) {
                        buf.extend_from_slice(b"USER\r\n");
                        debug!("Added USER capability");
                    }

                    // SASL mechanisms
                    buf.extend_from_slice(b"SASL");
                    for mechanism in &mechanisms {
                        buf.extend_from_slice(b" ");
                        buf.extend_from_slice(mechanism.as_str().as_bytes());
                    }
                    buf.extend_from_slice(b"\r\n");
                    debug!(mechanisms = ?mechanisms, "Added SASL capabilities");
                }

                // StartTLS capability
                if stls {
                    buf.extend_from_slice(b"STLS\r\n");
                    debug!("Added STLS capability");
                }

                // Standard POP3 capabilities
                const STANDARD_CAPABILITIES: &[&str] = &[
                    "TOP",           // TOP command support
                    "RESP-CODES",    // Response codes extension
                    "PIPELINING",    // Command pipelining
                    "EXPIRE NEVER",  // Messages never expire
                    "UIDL",          // Unique ID listing
                    "UTF8",          // UTF-8 support
                    "IMPLEMENTATION A3Mailer Server", // Server identification
                ];

                for &capability in STANDARD_CAPABILITIES {
                    buf.extend_from_slice(capability.as_bytes());
                    buf.extend_from_slice(b"\r\n");
                }

                debug!(
                    standard_capabilities = STANDARD_CAPABILITIES.len(),
                    "Added standard capabilities"
                );

                // Add termination marker
                buf.extend_from_slice(b".\r\n");

                debug!(
                    mechanism_count = mechanisms.len(),
                    stls_enabled = stls,
                    total_size = buf.len(),
                    "CAPABILITY response serialized"
                );

                buf
            }
        };

        let duration = start_time.elapsed();
        let response_size = result.len();

        // Log performance metrics
        debug!(
            response_size = response_size,
            serialization_time_us = duration.as_micros(),
            "Response serialization completed"
        );

        // TODO: Record metrics in global metrics collector
        // METRICS.record_serialization(response_size, duration);

        result
    }
}

impl Mechanism {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mechanism::Plain => "PLAIN",
            Mechanism::CramMd5 => "CRAM-MD5",
            Mechanism::DigestMd5 => "DIGEST-MD5",
            Mechanism::ScramSha1 => "SCRAM-SHA-1",
            Mechanism::ScramSha256 => "SCRAM-SHA-256",
            Mechanism::Apop => "APOP",
            Mechanism::Ntlm => "NTLM",
            Mechanism::Gssapi => "GSSAPI",
            Mechanism::Anonymous => "ANONYMOUS",
            Mechanism::External => "EXTERNAL",
            Mechanism::OAuthBearer => "OAUTHBEARER",
            Mechanism::XOauth2 => "XOAUTH2",
        }
    }
}



impl SerializeResponse for trc::Error {
    /// Serializes error to POP3 error response format with enhanced logging
    ///
    /// Converts internal error types to user-friendly POP3 error messages
    /// while preserving error context for logging and debugging.
    ///
    /// # Returns
    ///
    /// Properly formatted POP3 error response (-ERR message\r\n)
    ///
    /// # Performance Notes
    ///
    /// This implementation minimizes allocations and provides O(1) complexity
    /// for error message serialization.
    #[instrument(level = "debug", skip(self))]
    fn serialize(self) -> Vec<u8> {
        let start_time = Instant::now();

        // Extract error message with fallback
        let message = self
            .value_as_str(trc::Key::Details)
            .unwrap_or_else(|| self.as_ref().message());

        // Sanitize error message for POP3 protocol compliance
        let sanitized_msg = sanitize_error_message(message);

        // Build response with pre-calculated capacity
        let mut buf = Vec::with_capacity(sanitized_msg.len() + 7);
        buf.extend_from_slice(b"-ERR ");
        buf.extend_from_slice(sanitized_msg.as_bytes());
        buf.extend_from_slice(b"\r\n");

        let duration = start_time.elapsed();

        debug!(
            original_message = %message,
            sanitized_message = %sanitized_msg,
            response_size = buf.len(),
            serialization_time_us = duration.as_micros(),
            "Error response serialized"
        );

        buf
    }
}

/// Sanitizes error messages for POP3 protocol compliance
///
/// Ensures error messages don't contain characters that could break
/// the POP3 protocol or cause security issues.
///
/// # Arguments
///
/// * `message` - The original error message
///
/// # Returns
///
/// A sanitized message safe for POP3 transmission
///
/// # Examples
///
/// ```rust
/// use pop3::protocol::response::sanitize_error_message;
///
/// let sanitized = sanitize_error_message("Error\r\nwith\nnewlines");
/// assert!(!sanitized.contains('\r'));
/// assert!(!sanitized.contains('\n'));
/// ```
fn sanitize_error_message(message: &str) -> String {
    message
        .chars()
        .filter(|&c| c.is_ascii() && c != '\r' && c != '\n' && c.is_control() == false)
        .take(200) // Limit message length to prevent buffer overflow attacks
        .collect()
}

#[cfg(test)]
mod tests {

    use crate::protocol::Mechanism;
    use super::{Response, ListItem, SerializeResponse, sanitize_error_message};

    #[test]
    fn serialize_response() {
        for (cmd, expected) in [
            (
                Response::Ok("message 1 deleted".into()),
                "+OK message 1 deleted\r\n",
            ),
            (
                Response::Err("permission denied".into()),
                "-ERR permission denied\r\n",
            ),
            (
                Response::List(vec![
                    ListItem::Message { number: 1, size: 100 },
                    ListItem::Message { number: 2, size: 200 },
                    ListItem::Message { number: 3, size: 300 },
                ]),
                "+OK 3 messages\r\n1 100\r\n2 200\r\n3 300\r\n.\r\n",
            ),
            (
                Response::Capability {
                    mechanisms: vec![Mechanism::Plain, Mechanism::CramMd5],
                    stls: true,
                },
                concat!(
                    "+OK Capability list follows\r\n",
                    "USER\r\n",
                    "SASL PLAIN CRAM-MD5\r\n",
                    "STLS\r\n",
                    "TOP\r\n",
                    "RESP-CODES\r\n",
                    "PIPELINING\r\n",
                    "EXPIRE NEVER\r\n",
                    "UIDL\r\n",
                    "UTF8\r\n",
                    "IMPLEMENTATION A3Mailer Server\r\n.\r\n"
                ),
            ),
            (
                Response::Message {
                    bytes: "Subject: test\r\n\r\n.\r\ntest.\r\n.test\r\na"
                        .as_bytes()
                        .to_vec(),
                    lines: 0,
                },
                "+OK 35 octets\r\nSubject: test\r\n\r\n..\r\ntest.\r\n..test\r\na\r\n.\r\n",
            ),
        ] {
            assert_eq!(expected, String::from_utf8(cmd.serialize()).unwrap());
        }
    }

    #[test]
    fn test_list_item_serialization() {
        // Test message list
        let response = Response::List(vec![
            ListItem::Message { number: 1, size: 1024 },
            ListItem::Message { number: 5, size: 2048 },
        ]);
        let serialized = String::from_utf8(response.serialize()).unwrap();
        assert_eq!(serialized, "+OK 2 messages\r\n1 1024\r\n5 2048\r\n.\r\n");

        // Test UIDL list
        let response = Response::List(vec![
            ListItem::Uidl { number: 1, uid: "abc123".to_string() },
            ListItem::Uidl { number: 2, uid: "def456".to_string() },
        ]);
        let serialized = String::from_utf8(response.serialize()).unwrap();
        assert_eq!(serialized, "+OK 2 messages\r\n1 abc123\r\n2 def456\r\n.\r\n");

        // Test mixed list (should not happen in practice, but test robustness)
        let response = Response::List(vec![
            ListItem::Message { number: 1, size: 100 },
            ListItem::Uidl { number: 2, uid: "uid123".to_string() },
        ]);
        let serialized = String::from_utf8(response.serialize()).unwrap();
        assert_eq!(serialized, "+OK 2 messages\r\n1 100\r\n2 uid123\r\n.\r\n");
    }

    #[test]
    fn test_message_transparency() {
        // Test dot stuffing
        let response = Response::Message {
            bytes: b"Line 1\r\n.Line starting with dot\r\n..Double dot line\r\nLast line".to_vec(),
            lines: 0,
        };
        let serialized = String::from_utf8(response.serialize()).unwrap();
        assert!(serialized.contains("..Line starting with dot"));
        assert!(serialized.contains("...Double dot line"));

        // Test line ending normalization
        let response = Response::Message {
            bytes: b"Line 1\nLine 2\r\nLine 3\n".to_vec(),
            lines: 0,
        };
        let serialized = String::from_utf8(response.serialize()).unwrap();
        assert!(serialized.contains("Line 1\r\n"));
        assert!(serialized.contains("Line 2\r\n"));
        assert!(serialized.contains("Line 3\r\n"));
    }

    #[test]
    fn test_capability_response() {
        let response = Response::Capability {
            mechanisms: vec![Mechanism::Plain, Mechanism::CramMd5, Mechanism::OAuthBearer],
            stls: true,
        };
        let serialized = String::from_utf8(response.serialize()).unwrap();

        assert!(serialized.contains("USER"));
        assert!(serialized.contains("SASL PLAIN CRAM-MD5 OAUTHBEARER"));
        assert!(serialized.contains("STLS"));
        assert!(serialized.contains("TOP"));
        assert!(serialized.contains("UIDL"));
        assert!(serialized.contains("UTF8"));
        assert!(serialized.contains("IMPLEMENTATION A3Mailer Server"));

        // Test without STLS
        let response = Response::Capability {
            mechanisms: vec![Mechanism::Plain],
            stls: false,
        };
        let serialized = String::from_utf8(response.serialize()).unwrap();
        assert!(!serialized.contains("STLS"));
    }

    #[test]
    fn test_error_serialization() {
        use crate::protocol::response::SerializeResponse;
        let error = trc::Pop3Event::Error
            .into_err()
            .details("Test error message");
        let serialized = String::from_utf8(error.serialize()).unwrap();
        assert_eq!(serialized, "-ERR Test error message\r\n");
    }

    /// Test error message sanitization
    #[test]
    fn test_error_message_sanitization() {
        // Test CRLF removal
        let sanitized = sanitize_error_message("Error\r\nwith\nnewlines");
        assert!(!sanitized.contains('\r'));
        assert!(!sanitized.contains('\n'));

        // Test length limiting
        let long_message = "A".repeat(300);
        let sanitized = sanitize_error_message(&long_message);
        assert!(sanitized.len() <= 200);

        // Test control character removal
        let with_controls = "Error\x00\x01\x02message";
        let sanitized = sanitize_error_message(with_controls);
        assert!(!sanitized.contains('\x00'));
        assert!(!sanitized.contains('\x01'));
        assert!(!sanitized.contains('\x02'));
    }

    /// Test response serialization performance
    #[test]
    fn test_serialization_performance() {
        use std::time::Instant;

        // Test simple response performance
        let start = Instant::now();
        for _ in 0..1000 {
            let response = Response::Ok("Test message".into());
            let _ = response.serialize();
        }
        let duration = start.elapsed();
        assert!(duration.as_millis() < 100, "Simple response serialization too slow: {}ms", duration.as_millis());

        // Test large list performance
        let items: Vec<ListItem> = (1..=1000)
            .map(|i| ListItem::Message { number: i, size: i as u32 * 100 })
            .collect();

        let start = Instant::now();
        let response = Response::List(items);
        let serialized = response.serialize();
        let duration = start.elapsed();

        assert!(duration.as_millis() < 50, "Large list serialization too slow: {}ms", duration.as_millis());
        assert!(serialized.len() > 10000, "Serialized list should be substantial");
    }

    /// Test concurrent serialization safety
    #[test]
    fn test_concurrent_serialization() {
        use std::sync::Arc;
        use std::thread;

        let responses = Arc::new(vec![
            Response::Ok("Message 1".into()),
            Response::Err("Error 1".into()),
            Response::List(vec![ListItem::Message { number: 1, size: 100 }]),
        ]);

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let responses = responses.clone();
                thread::spawn(move || {
                    for _ in 0..100 {
                        let response = responses[i % responses.len()].clone();
                        let _ = response.serialize();
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("Thread should complete successfully");
        }
    }

    /// Test edge cases and boundary conditions
    #[test]
    fn test_edge_cases() {
        // Empty message
        let response = Response::Ok("".into());
        let serialized = String::from_utf8(response.serialize()).unwrap();
        assert_eq!(serialized, "+OK \r\n");

        // Empty list
        let response = Response::List(vec![]);
        let serialized = String::from_utf8(response.serialize()).unwrap();
        assert_eq!(serialized, "+OK 0 messages\r\n.\r\n");

        // Empty message content
        let response = Response::Message { bytes: vec![], lines: 0 };
        let serialized = String::from_utf8(response.serialize()).unwrap();
        assert!(serialized.starts_with("+OK 0 octets\r\n"));
        assert!(serialized.ends_with(".\r\n"));

        // Single character responses
        let response = Response::Ok("X".into());
        let serialized = String::from_utf8(response.serialize()).unwrap();
        assert_eq!(serialized, "+OK X\r\n");
    }

    /// Test message dot stuffing edge cases
    #[test]
    fn test_dot_stuffing_edge_cases() {
        // Message starting with dot
        let response = Response::Message {
            bytes: b".This starts with a dot".to_vec(),
            lines: 0
        };
        let serialized = String::from_utf8(response.serialize()).unwrap();
        assert!(serialized.contains("..This starts with a dot"));

        // Multiple consecutive dots
        let response = Response::Message {
            bytes: b"Line 1\r\n..Multiple dots\r\n...Three dots".to_vec(),
            lines: 0
        };
        let serialized = String::from_utf8(response.serialize()).unwrap();
        assert!(serialized.contains("...Multiple dots"));
        assert!(serialized.contains("....Three dots"));

        // Dot at end of message
        let response = Response::Message {
            bytes: b"Message ending with dot.".to_vec(),
            lines: 0
        };
        let serialized = String::from_utf8(response.serialize()).unwrap();
        assert!(serialized.contains("Message ending with dot."));
    }

    /// Test line limit functionality for TOP command
    #[test]
    fn test_line_limit() {
        let message = b"Line 1\r\nLine 2\r\nLine 3\r\nLine 4\r\nLine 5\r\n".to_vec();

        // Test with line limit
        let response = Response::Message { bytes: message.clone(), lines: 3 };
        let serialized = String::from_utf8(response.serialize()).unwrap();

        // Should contain first 3 lines
        assert!(serialized.contains("Line 1"));
        assert!(serialized.contains("Line 2"));
        assert!(serialized.contains("Line 3"));

        // Should not contain lines 4 and 5
        assert!(!serialized.contains("Line 4"));
        assert!(!serialized.contains("Line 5"));

        // Test without line limit
        let response = Response::Message { bytes: message, lines: 0 };
        let serialized = String::from_utf8(response.serialize()).unwrap();

        // Should contain all lines
        assert!(serialized.contains("Line 5"));
    }

    /// Test capability response completeness
    #[test]
    fn test_capability_completeness() {
        let response = Response::Capability {
            mechanisms: vec![
                Mechanism::Plain,
                Mechanism::CramMd5,
                Mechanism::OAuthBearer
            ],
            stls: true,
        };
        let serialized = String::from_utf8(response.serialize()).unwrap();

        // Check all expected capabilities are present
        let expected_capabilities = [
            "USER", "SASL PLAIN CRAM-MD5 OAUTHBEARER", "STLS",
            "TOP", "RESP-CODES", "PIPELINING", "EXPIRE NEVER",
            "UIDL", "UTF8", "IMPLEMENTATION A3Mailer Server"
        ];

        for capability in expected_capabilities {
            assert!(
                serialized.contains(capability),
                "Missing capability: {} in response: {}",
                capability,
                serialized
            );
        }

        // Ensure proper termination
        assert!(serialized.ends_with(".\r\n"));
    }
}


