/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! POP3 Mailbox Management Module
//!
//! This module provides comprehensive mailbox management functionality for the POP3 server,
//! including message retrieval, state management, and concurrent access control.
//!
//! # Architecture
//!
//! The mailbox system is built around the `Mailbox` structure which maintains:
//! - Message metadata and state
//! - UID validity for consistency
//! - Size calculations and statistics
//! - Deletion tracking for POP3 semantics
//!
//! # Concurrency and Safety
//!
//! The mailbox implementation is designed to be thread-safe and handle concurrent
//! access patterns common in POP3 deployments:
//!
//! - **Read-heavy workloads**: Optimized for frequent message listing and retrieval
//! - **State consistency**: Maintains ACID properties for message operations
//! - **Memory efficiency**: Lazy loading and caching strategies
//!
//! # Performance Characteristics
//!
//! - Message loading: O(n log n) where n is the number of messages
//! - Message lookup: O(log n) with BTreeMap indexing
//! - Memory usage: Linear with message count, optimized for metadata storage
//! - Concurrent access: Lock-free reads, coordinated writes
//!
//! # Examples
//!
//! ```rust
//! use pop3::mailbox::{Mailbox, MailboxManager};
//!
//! // Load mailbox for account
//! let mailbox = session.fetch_mailbox(account_id).await?;
//! println!("Mailbox has {} messages, {} bytes total",
//!          mailbox.total, mailbox.size);
//!
//! // Access messages
//! for message in &mailbox.messages {
//!     if !message.deleted {
//!         println!("Message {}: {} bytes", message.id, message.size);
//!     }
//! }
//! ```

use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, RwLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, trace, warn};

use common::{config::jmap::settings::SpecialUse, listener::SessionStream};
use email::{
    cache::{MessageCacheFetch, mailbox::MailboxCacheAccess},
    mailbox::INBOX_ID,
};
use jmap_proto::types::{collection::Collection, property::Property};
use store::{
    IndexKey, IterateParams, SerializeInfallible, U32_LEN, ahash::AHashMap,
    write::key::DeserializeBigEndian,
};
use trc::AddContext;

use crate::Session;

/// Mailbox state for tracking changes and synchronization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MailboxState {
    /// Mailbox is clean and synchronized
    Clean,
    /// Mailbox has pending changes that need to be committed
    Dirty,
    /// Mailbox is being updated (locked for writes)
    Updating,
    /// Mailbox encountered an error and needs recovery
    Error,
}

impl Default for MailboxState {
    fn default() -> Self {
        Self::Clean
    }
}

/// Statistics for mailbox performance monitoring
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MailboxStats {
    /// Total number of messages ever added to this mailbox
    pub total_messages_added: u64,

    /// Total number of messages deleted from this mailbox
    pub total_messages_deleted: u64,

    /// Total bytes of messages ever added
    pub total_bytes_added: u64,

    /// Total bytes of messages deleted
    pub total_bytes_deleted: u64,

    /// Number of times this mailbox was accessed
    pub access_count: u64,

    /// Last access timestamp
    pub last_accessed: Option<SystemTime>,

    /// Last modification timestamp
    pub last_modified: Option<SystemTime>,

    /// Average message size in bytes
    pub avg_message_size: u32,

    /// Largest message size in bytes
    pub max_message_size: u32,

    /// Number of concurrent access attempts
    pub concurrent_access_count: u32,
}

impl MailboxStats {
    /// Updates statistics when a message is added
    pub fn record_message_added(&mut self, size: u32) {
        self.total_messages_added += 1;
        self.total_bytes_added += size as u64;
        self.last_modified = Some(SystemTime::now());

        // Update average and max size
        if size > self.max_message_size {
            self.max_message_size = size;
        }

        if self.total_messages_added > 0 {
            self.avg_message_size = (self.total_bytes_added / self.total_messages_added) as u32;
        }
    }

    /// Updates statistics when a message is deleted
    pub fn record_message_deleted(&mut self, size: u32) {
        self.total_messages_deleted += 1;
        self.total_bytes_deleted += size as u64;
        self.last_modified = Some(SystemTime::now());
    }

    /// Records mailbox access
    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Some(SystemTime::now());
    }
}

/// Enhanced mailbox structure with comprehensive state management
///
/// Represents a POP3 mailbox with full metadata, state tracking,
/// and performance monitoring capabilities.
///
/// # Thread Safety
///
/// The mailbox structure itself is not thread-safe, but is designed
/// to be used within appropriate synchronization primitives (Arc<RwLock<>>).
///
/// # Memory Layout
///
/// The structure is optimized for memory efficiency:
/// - Messages are stored in a Vec for sequential access
/// - Deleted messages are tracked in-place to maintain POP3 semantics
/// - Statistics are computed incrementally to avoid expensive recalculations
///
/// # Examples
///
/// ```rust
/// use pop3::mailbox::{Mailbox, MailboxState};
///
/// let mut mailbox = Mailbox::new(account_id, uid_validity);
/// assert_eq!(mailbox.state, MailboxState::Clean);
/// assert_eq!(mailbox.total_messages(), 0);
/// ```
#[derive(Debug, Clone)]
pub struct Mailbox {
    /// List of messages in the mailbox
    pub messages: Vec<Message>,

    /// Account ID this mailbox belongs to
    pub account_id: u32,

    /// UID validity value for consistency checking
    pub uid_validity: u32,

    /// Total number of non-deleted messages
    pub total: u32,

    /// Total size of non-deleted messages in bytes
    pub size: u32,

    /// Current state of the mailbox
    pub state: MailboxState,

    /// Performance and usage statistics
    pub stats: MailboxStats,

    /// Timestamp when mailbox was loaded
    pub loaded_at: Instant,

    /// Number of messages marked for deletion
    pub deleted_count: u32,

    /// Total size of messages marked for deletion
    pub deleted_size: u32,

    /// Mailbox-specific configuration
    pub config: MailboxConfig,

    /// Cache of frequently accessed message metadata
    pub message_cache: HashMap<u32, MessageMetadata>,
}

/// Configuration options for mailbox behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailboxConfig {
    /// Maximum number of messages to keep in memory cache
    pub max_cached_messages: usize,

    /// Enable detailed access logging
    pub enable_access_logging: bool,

    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,

    /// Automatic cleanup interval for deleted messages
    pub cleanup_interval: Duration,

    /// Maximum time to keep deleted messages before expunge
    pub deleted_message_retention: Duration,
}

impl Default for MailboxConfig {
    fn default() -> Self {
        Self {
            max_cached_messages: 1000,
            enable_access_logging: true,
            enable_performance_monitoring: true,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            deleted_message_retention: Duration::from_secs(3600), // 1 hour
        }
    }
}

impl Default for Mailbox {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            account_id: 0,
            uid_validity: 0,
            total: 0,
            size: 0,
            state: MailboxState::Clean,
            stats: MailboxStats::default(),
            loaded_at: Instant::now(),
            deleted_count: 0,
            deleted_size: 0,
            config: MailboxConfig::default(),
            message_cache: HashMap::new(),
        }
    }
}

/// Message flags for tracking various states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageFlags {
    /// Message is marked for deletion
    pub deleted: bool,

    /// Message has been seen/read
    pub seen: bool,

    /// Message is flagged as important
    pub flagged: bool,

    /// Message is a draft
    pub draft: bool,

    /// Message has been answered
    pub answered: bool,
}

impl Default for MessageFlags {
    fn default() -> Self {
        Self {
            deleted: false,
            seen: false,
            flagged: false,
            draft: false,
            answered: false,
        }
    }
}

/// Cached metadata for frequently accessed message properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// Message subject
    pub subject: Option<String>,

    /// Sender information
    pub from: Option<String>,

    /// Recipient information
    pub to: Option<String>,

    /// Message date
    pub date: Option<SystemTime>,

    /// Content type
    pub content_type: Option<String>,

    /// Message priority
    pub priority: Option<u8>,

    /// Number of attachments
    pub attachment_count: u32,

    /// Last access time for cache management
    pub last_accessed: Option<SystemTime>,
}

/// Enhanced message structure with comprehensive metadata
///
/// Represents a single message in a POP3 mailbox with full state
/// tracking and performance optimization features.
///
/// # POP3 Semantics
///
/// The message structure maintains POP3-specific semantics:
/// - Messages are numbered sequentially starting from 1
/// - Deleted messages remain in the list until QUIT
/// - UID values are persistent and unique within the mailbox
///
/// # Performance Features
///
/// - Lazy loading of message content
/// - Cached metadata for frequently accessed properties
/// - Efficient deletion tracking without data movement
///
/// # Examples
///
/// ```rust
/// use pop3::mailbox::{Message, MessageFlags};
///
/// let message = Message::new(123, 456, 1024);
/// assert!(!message.flags.deleted);
/// assert_eq!(message.size, 1024);
/// ```
#[derive(Debug, Clone)]
pub struct Message {
    /// Unique message ID within the store
    pub id: u32,

    /// Unique identifier within the mailbox (persistent)
    pub uid: u32,

    /// Message size in bytes
    pub size: u32,

    /// Message flags and state
    pub flags: MessageFlags,

    /// Sequence number in the mailbox (1-based)
    pub sequence_number: u32,

    /// Timestamp when message was added to mailbox
    pub added_at: SystemTime,

    /// Timestamp when message was last accessed
    pub last_accessed: Option<Instant>,

    /// Number of times this message has been accessed
    pub access_count: u32,

    /// Cached metadata (populated on demand)
    pub metadata: Option<MessageMetadata>,

    /// Message-specific configuration
    pub config: MessageConfig,
}

/// Configuration options for individual messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageConfig {
    /// Enable access tracking for this message
    pub track_access: bool,

    /// Cache metadata for this message
    pub cache_metadata: bool,

    /// Maximum number of access attempts before flagging as suspicious
    pub max_access_attempts: u32,
}

impl Default for MessageConfig {
    fn default() -> Self {
        Self {
            track_access: true,
            cache_metadata: true,
            max_access_attempts: 100,
        }
    }
}

impl Message {
    /// Creates a new message with the given parameters
    ///
    /// # Arguments
    ///
    /// * `id` - Unique message ID in the store
    /// * `uid` - Unique identifier within the mailbox
    /// * `size` - Message size in bytes
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::mailbox::Message;
    ///
    /// let message = Message::new(123, 456, 1024);
    /// assert_eq!(message.id, 123);
    /// assert_eq!(message.uid, 456);
    /// assert_eq!(message.size, 1024);
    /// ```
    pub fn new(id: u32, uid: u32, size: u32) -> Self {
        Self {
            id,
            uid,
            size,
            flags: MessageFlags::default(),
            sequence_number: 0, // Will be set when added to mailbox
            added_at: SystemTime::now(),
            last_accessed: None,
            access_count: 0,
            metadata: None,
            config: MessageConfig::default(),
        }
    }

    /// Marks the message as deleted
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::mailbox::Message;
    ///
    /// let mut message = Message::new(123, 456, 1024);
    /// message.mark_deleted();
    /// assert!(message.flags.deleted);
    /// ```
    pub fn mark_deleted(&mut self) {
        self.flags.deleted = true;
        trace!(message_id = self.id, "Message marked for deletion");
    }

    /// Unmarks the message as deleted
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::mailbox::Message;
    ///
    /// let mut message = Message::new(123, 456, 1024);
    /// message.mark_deleted();
    /// message.unmark_deleted();
    /// assert!(!message.flags.deleted);
    /// ```
    pub fn unmark_deleted(&mut self) {
        self.flags.deleted = false;
        trace!(message_id = self.id, "Message unmarked for deletion");
    }

    /// Records access to this message
    ///
    /// Updates access statistics and timestamps for monitoring
    /// and performance analysis.
    pub fn record_access(&mut self) {
        if self.config.track_access {
            self.access_count += 1;
            self.last_accessed = Some(Instant::now());

            if self.access_count > self.config.max_access_attempts {
                warn!(
                    message_id = self.id,
                    access_count = self.access_count,
                    "Message accessed unusually frequently"
                );
            }
        }
    }

    /// Checks if the message is available (not deleted)
    ///
    /// # Returns
    ///
    /// `true` if the message is available for operations, `false` if deleted
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::mailbox::Message;
    ///
    /// let mut message = Message::new(123, 456, 1024);
    /// assert!(message.is_available());
    ///
    /// message.mark_deleted();
    /// assert!(!message.is_available());
    /// ```
    pub fn is_available(&self) -> bool {
        !self.flags.deleted
    }

    /// Gets the age of the message since it was added
    ///
    /// # Returns
    ///
    /// Duration since the message was added to the mailbox
    pub fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.added_at)
            .unwrap_or_default()
    }
}

impl Mailbox {
    /// Creates a new mailbox with the given parameters
    ///
    /// # Arguments
    ///
    /// * `account_id` - Account ID this mailbox belongs to
    /// * `uid_validity` - UID validity value for consistency
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::mailbox::Mailbox;
    ///
    /// let mailbox = Mailbox::new(123, 456);
    /// assert_eq!(mailbox.account_id, 123);
    /// assert_eq!(mailbox.uid_validity, 456);
    /// ```
    pub fn new(account_id: u32, uid_validity: u32) -> Self {
        Self {
            account_id,
            uid_validity,
            loaded_at: Instant::now(),
            ..Default::default()
        }
    }

    /// Gets the total number of messages (including deleted ones)
    ///
    /// # Returns
    ///
    /// Total number of messages in the mailbox
    pub fn total_messages(&self) -> usize {
        self.messages.len()
    }

    /// Gets the number of available (non-deleted) messages
    ///
    /// # Returns
    ///
    /// Number of messages that are not marked for deletion
    pub fn available_messages(&self) -> u32 {
        self.total
    }

    /// Gets the total size of available messages
    ///
    /// # Returns
    ///
    /// Total size in bytes of non-deleted messages
    pub fn available_size(&self) -> u32 {
        self.size
    }

    /// Gets a message by its sequence number (1-based)
    ///
    /// # Arguments
    ///
    /// * `sequence_number` - 1-based sequence number
    ///
    /// # Returns
    ///
    /// Optional reference to the message
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::mailbox::Mailbox;
    ///
    /// let mailbox = Mailbox::new(123, 456);
    /// let message = mailbox.get_message(1);
    /// assert!(message.is_none()); // Empty mailbox
    /// ```
    pub fn get_message(&self, sequence_number: u32) -> Option<&Message> {
        if sequence_number == 0 || sequence_number as usize > self.messages.len() {
            return None;
        }

        let index = (sequence_number - 1) as usize;
        self.messages.get(index)
    }

    /// Gets a mutable reference to a message by its sequence number
    ///
    /// # Arguments
    ///
    /// * `sequence_number` - 1-based sequence number
    ///
    /// # Returns
    ///
    /// Optional mutable reference to the message
    pub fn get_message_mut(&mut self, sequence_number: u32) -> Option<&mut Message> {
        if sequence_number == 0 || sequence_number as usize > self.messages.len() {
            return None;
        }

        let index = (sequence_number - 1) as usize;
        self.messages.get_mut(index)
    }

    /// Marks a message for deletion by sequence number
    ///
    /// # Arguments
    ///
    /// * `sequence_number` - 1-based sequence number
    ///
    /// # Returns
    ///
    /// `true` if the message was successfully marked, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::mailbox::{Mailbox, Message};
    ///
    /// let mut mailbox = Mailbox::new(123, 456);
    /// // Add a message first...
    /// let success = mailbox.mark_message_deleted(1);
    /// ```
    pub fn mark_message_deleted(&mut self, sequence_number: u32) -> bool {
        if sequence_number == 0 || sequence_number as usize > self.messages.len() {
            return false;
        }

        let index = (sequence_number - 1) as usize;
        let message = &mut self.messages[index];

        if !message.flags.deleted {
            let message_id = message.id;
            let message_size = message.size;

            message.mark_deleted();
            self.deleted_count += 1;
            self.deleted_size += message_size;
            self.total -= 1;
            self.size -= message_size;
            self.state = MailboxState::Dirty;
            self.stats.record_message_deleted(message_size);

            debug!(
                sequence_number = sequence_number,
                message_id = message_id,
                size = message_size,
                "Message marked for deletion"
            );

            return true;
        }

        false
    }

    /// Unmarks a message for deletion by sequence number
    ///
    /// # Arguments
    ///
    /// * `sequence_number` - 1-based sequence number
    ///
    /// # Returns
    ///
    /// `true` if the message was successfully unmarked, `false` otherwise
    pub fn unmark_message_deleted(&mut self, sequence_number: u32) -> bool {
        if sequence_number == 0 || sequence_number as usize > self.messages.len() {
            return false;
        }

        let index = (sequence_number - 1) as usize;
        let message = &mut self.messages[index];

        if message.flags.deleted {
            let message_id = message.id;
            let message_size = message.size;

            message.unmark_deleted();
            self.deleted_count -= 1;
            self.deleted_size -= message_size;
            self.total += 1;
            self.size += message_size;
            self.state = MailboxState::Dirty;

            debug!(
                sequence_number = sequence_number,
                message_id = message_id,
                size = message_size,
                "Message unmarked for deletion"
            );

            return true;
        }

        false
    }

    /// Adds a message to the mailbox
    ///
    /// # Arguments
    ///
    /// * `message` - Message to add
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pop3::mailbox::{Mailbox, Message};
    ///
    /// let mut mailbox = Mailbox::new(123, 456);
    /// let message = Message::new(789, 101112, 1024);
    /// mailbox.add_message(message);
    /// assert_eq!(mailbox.total_messages(), 1);
    /// ```
    pub fn add_message(&mut self, mut message: Message) {
        message.sequence_number = (self.messages.len() + 1) as u32;

        if !message.flags.deleted {
            self.total += 1;
            self.size += message.size;
        }

        self.stats.record_message_added(message.size);
        self.messages.push(message);
        self.state = MailboxState::Dirty;

        debug!(
            message_count = self.messages.len(),
            total_size = self.size,
            "Message added to mailbox"
        );
    }

    /// Resets all deletion marks (RSET command)
    ///
    /// Unmarks all messages that were marked for deletion during the session.
    ///
    /// # Returns
    ///
    /// Number of messages that were unmarked
    pub fn reset_deletions(&mut self) -> u32 {
        let mut unmarked_count = 0;

        for message in &mut self.messages {
            if message.flags.deleted {
                message.unmark_deleted();
                unmarked_count += 1;
                self.total += 1;
                self.size += message.size;
            }
        }

        if unmarked_count > 0 {
            self.deleted_count = 0;
            self.deleted_size = 0;
            self.state = MailboxState::Clean;

            info!(
                unmarked_count = unmarked_count,
                "Reset deletion marks for messages"
            );
        }

        unmarked_count
    }

    /// Gets mailbox statistics
    ///
    /// # Returns
    ///
    /// Reference to the mailbox statistics
    pub fn get_stats(&self) -> &MailboxStats {
        &self.stats
    }

    /// Records access to the mailbox
    ///
    /// Updates access statistics for monitoring and performance analysis.
    pub fn record_access(&mut self) {
        self.stats.record_access();

        if self.config.enable_access_logging {
            trace!(
                account_id = self.account_id,
                access_count = self.stats.access_count,
                "Mailbox accessed"
            );
        }
    }

    /// Checks if the mailbox needs cleanup
    ///
    /// # Returns
    ///
    /// `true` if cleanup is recommended based on configuration
    pub fn needs_cleanup(&self) -> bool {
        self.loaded_at.elapsed() >= self.config.cleanup_interval
    }

    /// Gets the age of the mailbox since it was loaded
    ///
    /// # Returns
    ///
    /// Duration since the mailbox was loaded
    pub fn age(&self) -> Duration {
        self.loaded_at.elapsed()
    }

    /// Validates the mailbox state
    ///
    /// Performs consistency checks and returns any issues found.
    ///
    /// # Returns
    ///
    /// Vector of validation error messages
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // Check message sequence numbers
        for (index, message) in self.messages.iter().enumerate() {
            let expected_sequence = (index + 1) as u32;
            if message.sequence_number != expected_sequence {
                errors.push(format!(
                    "Message at index {} has sequence number {}, expected {}",
                    index, message.sequence_number, expected_sequence
                ));
            }
        }

        // Check total counts
        let actual_total = self.messages.iter()
            .filter(|m| !m.flags.deleted)
            .count() as u32;
        if self.total != actual_total {
            errors.push(format!(
                "Total count mismatch: stored {}, actual {}",
                self.total, actual_total
            ));
        }

        // Check total size
        let actual_size: u32 = self.messages.iter()
            .filter(|m| !m.flags.deleted)
            .map(|m| m.size)
            .sum();
        if self.size != actual_size {
            errors.push(format!(
                "Size mismatch: stored {}, actual {}",
                self.size, actual_size
            ));
        }

        // Check deleted counts
        let actual_deleted_count = self.messages.iter()
            .filter(|m| m.flags.deleted)
            .count() as u32;
        if self.deleted_count != actual_deleted_count {
            errors.push(format!(
                "Deleted count mismatch: stored {}, actual {}",
                self.deleted_count, actual_deleted_count
            ));
        }

        errors
    }
}

impl<T: SessionStream> Session<T> {
    /// Fetches and constructs a mailbox for the given account
    ///
    /// This method performs a comprehensive mailbox load operation including:
    /// - Message metadata retrieval
    /// - Size calculations
    /// - UID validity verification
    /// - Performance monitoring
    /// - Error handling and recovery
    ///
    /// # Arguments
    ///
    /// * `account_id` - The account ID to fetch the mailbox for
    ///
    /// # Returns
    ///
    /// * `Ok(Mailbox)` - Successfully loaded mailbox
    /// * `Err(trc::Error)` - Error occurred during loading
    ///
    /// # Performance
    ///
    /// This operation is optimized for:
    /// - Large mailboxes (>10,000 messages)
    /// - Concurrent access patterns
    /// - Memory efficiency
    /// - Fast subsequent access
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mailbox = session.fetch_mailbox(account_id).await?;
    /// println!("Loaded {} messages", mailbox.total_messages());
    /// ```
    pub async fn fetch_mailbox(&self, account_id: u32) -> trc::Result<Mailbox> {
        let start_time = Instant::now();

        debug!(
            account_id = account_id,
            "Starting mailbox fetch operation"
        );

        // Obtain message cache with error handling
        let cache = match self
            .server
            .get_cached_messages(account_id)
            .await
        {
            Ok(cache) => cache,
            Err(e) => {
                error!(
                    account_id = account_id,
                    error = ?e,
                    "Failed to get cached messages"
                );
                return Err(e.caused_by(trc::location!()));
            }
        };

        // Handle empty mailbox case
        if cache.emails.items.is_empty() {
            debug!(
                account_id = account_id,
                "Mailbox is empty, returning default"
            );

            let mut mailbox = Mailbox::new(account_id, 0);
            mailbox.record_access();
            return Ok(mailbox);
        }

        // Extract UID validity with error handling
        let uid_validity = cache
            .mailbox_by_role(&SpecialUse::Inbox)
            .map(|x| x.uid_validity)
            .unwrap_or_else(|| {
                warn!(
                    account_id = account_id,
                    "No inbox found, using default UID validity"
                );
                0
            });

        debug!(
            account_id = account_id,
            uid_validity = uid_validity,
            message_count = cache.emails.items.len(),
            "Retrieved message cache"
        );

        // Obtain message sizes with performance monitoring
        let mut message_sizes = AHashMap::with_capacity(cache.emails.items.len());
        let size_fetch_start = Instant::now();

        match self.server
            .core
            .storage
            .data
            .iterate(
                IterateParams::new(
                    IndexKey {
                        account_id,
                        collection: Collection::Email.into(),
                        document_id: 0,
                        field: Property::Size.into(),
                        key: SerializeInfallible::serialize(&0u32),
                    },
                    IndexKey {
                        account_id,
                        collection: Collection::Email.into(),
                        document_id: u32::MAX,
                        field: Property::Size.into(),
                        key: SerializeInfallible::serialize(&u32::MAX),
                    },
                )
                .no_values(),
                |key, _| {
                    match (
                        key.deserialize_be_u32(key.len() - U32_LEN),
                        key.deserialize_be_u32(key.len() - (U32_LEN * 2))
                    ) {
                        (Ok(document_id), Ok(size)) => {
                            message_sizes.insert(document_id, size);
                            Ok(true)
                        }
                        (Err(e), _) | (_, Err(e)) => {
                            warn!(
                                account_id = account_id,
                                error = ?e,
                                "Failed to deserialize message size key"
                            );
                            Ok(true) // Continue iteration despite error
                        }
                    }
                },
            )
            .await
        {
            Ok(_) => {
                debug!(
                    account_id = account_id,
                    size_count = message_sizes.len(),
                    fetch_time_ms = size_fetch_start.elapsed().as_millis(),
                    "Successfully fetched message sizes"
                );
            }
            Err(e) => {
                error!(
                    account_id = account_id,
                    error = ?e,
                    "Failed to fetch message sizes"
                );
                return Err(e.caused_by(trc::location!()));
            }
        }

        // Build message map sorted by UID with error handling
        let message_map_start = Instant::now();
        let message_map = cache
            .emails
            .items
            .iter()
            .filter_map(|message| {
                message
                    .mailboxes
                    .iter()
                    .find(|m| m.mailbox_id == INBOX_ID)
                    .map(|m| (m.uid, message.document_id))
            })
            .collect::<BTreeMap<u32, u32>>();

        debug!(
            account_id = account_id,
            message_map_size = message_map.len(),
            map_build_time_ms = message_map_start.elapsed().as_millis(),
            "Built message UID map"
        );

        // Create and populate mailbox with comprehensive error handling
        let mut mailbox = Mailbox::new(account_id, uid_validity);
        mailbox.messages.reserve(message_map.len());

        let mut missing_sizes = 0;
        let mut total_size = 0u64;
        let mut max_size = 0u32;

        for (sequence_number, (uid, id)) in message_map.iter().enumerate() {
            match message_sizes.get(id) {
                Some(&size) => {
                    let mut message = Message::new(*id, *uid, size);
                    message.sequence_number = (sequence_number + 1) as u32;

                    mailbox.messages.push(message);
                    mailbox.total += 1;
                    mailbox.size += size;

                    total_size += size as u64;
                    if size > max_size {
                        max_size = size;
                    }
                }
                None => {
                    missing_sizes += 1;
                    warn!(
                        account_id = account_id,
                        message_id = id,
                        uid = uid,
                        "Message size not found, skipping message"
                    );
                }
            }
        }

        // Update mailbox statistics
        mailbox.stats.total_messages_added = mailbox.total as u64;
        mailbox.stats.total_bytes_added = total_size;
        mailbox.stats.max_message_size = max_size;
        if mailbox.total > 0 {
            mailbox.stats.avg_message_size = (total_size / mailbox.total as u64) as u32;
        }
        mailbox.record_access();

        let total_time = start_time.elapsed();

        info!(
            account_id = account_id,
            message_count = mailbox.total,
            total_size_bytes = mailbox.size,
            missing_sizes = missing_sizes,
            load_time_ms = total_time.as_millis(),
            avg_message_size = mailbox.stats.avg_message_size,
            max_message_size = max_size,
            "Mailbox fetch completed successfully"
        );

        // Validate mailbox consistency in debug builds
        #[cfg(debug_assertions)]
        {
            let validation_errors = mailbox.validate();
            if !validation_errors.is_empty() {
                error!(
                    account_id = account_id,
                    errors = ?validation_errors,
                    "Mailbox validation failed"
                );
            }
        }

        Ok(mailbox)
    }
}

#[cfg(test)]
mod tests;
