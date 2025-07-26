/*!
 * Testing Utilities Module
 * 
 * This module provides utility functions and helpers for integration testing.
 * 
 * Features:
 * - Test data generation
 * - Email content generation
 * - Random data utilities
 * - File and attachment helpers
 * - Network utilities
 * - Time and date helpers
 * - Validation utilities
 * 
 * Author: Stalwart Labs Ltd.
 * Created: 2024-07-26
 */

use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::path::Path;
use tracing::{info, warn, error, debug};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

use crate::{TestUser, TestEmail, EmailAttachment, Result, TestError};

/// Test data generator
pub struct TestDataGenerator {
    rng: rand::rngs::ThreadRng,
}

/// Email content templates
pub struct EmailTemplates;

/// File utilities for testing
pub struct FileUtils;

/// Network utilities for testing
pub struct NetworkUtils;

/// Time utilities for testing
pub struct TimeUtils;

/// Validation utilities
pub struct ValidationUtils;

impl TestDataGenerator {
    /// Create a new test data generator
    pub fn new() -> Self {
        Self {
            rng: thread_rng(),
        }
    }

    /// Generate a random string of specified length
    pub fn random_string(&mut self, length: usize) -> String {
        (0..length)
            .map(|_| self.rng.sample(Alphanumeric) as char)
            .collect()
    }

    /// Generate a random email address
    pub fn random_email(&mut self, domain: &str) -> String {
        let username = self.random_string(8).to_lowercase();
        format!("{}@{}", username, domain)
    }

    /// Generate a random subject line
    pub fn random_subject(&mut self) -> String {
        let subjects = vec![
            "Important Update",
            "Meeting Reminder",
            "Project Status",
            "Weekly Report",
            "Action Required",
            "FYI: Information",
            "Urgent: Please Review",
            "Follow-up Required",
            "Monthly Summary",
            "Team Announcement",
        ];
        
        let base = subjects[self.rng.gen_range(0..subjects.len())];
        format!("{} - {}", base, self.random_string(6))
    }

    /// Generate random email body content
    pub fn random_email_body(&mut self, length: usize) -> String {
        let paragraphs = vec![
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.",
            "Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",
            "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.",
            "Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.",
            "Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium.",
            "Totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt.",
            "Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores.",
            "Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit.",
        ];
        
        let mut body = String::new();
        let target_length = length.max(100);
        
        while body.len() < target_length {
            let paragraph = paragraphs[self.rng.gen_range(0..paragraphs.len())];
            body.push_str(paragraph);
            body.push_str("\n\n");
        }
        
        body.truncate(target_length);
        body
    }

    /// Generate random binary data
    pub fn random_binary_data(&mut self, size: usize) -> Vec<u8> {
        (0..size).map(|_| self.rng.gen()).collect()
    }

    /// Generate a random filename
    pub fn random_filename(&mut self, extension: &str) -> String {
        let name = self.random_string(8).to_lowercase();
        format!("{}.{}", name, extension)
    }

    /// Generate random user data
    pub fn random_user(&mut self, domain: &str) -> TestUser {
        let user_id = Uuid::new_v4().to_string();
        let username = format!("user_{}", self.random_string(6).to_lowercase());
        
        TestUser {
            id: user_id,
            username: username.clone(),
            email: format!("{}@{}", username, domain),
            password: self.random_string(12),
            domain: domain.to_string(),
            created_at: Utc::now(),
        }
    }

    /// Generate random email
    pub fn random_email_message(&mut self, sender: &TestUser, recipients: Vec<String>) -> TestEmail {
        let email_id = Uuid::new_v4().to_string();
        let subject = self.random_subject();
        let body = self.random_email_body(500);
        
        TestEmail {
            id: email_id,
            from: sender.email.clone(),
            to: recipients,
            cc: Vec::new(),
            bcc: Vec::new(),
            subject,
            body: body.clone(),
            html_body: Some(format!("<html><body><p>{}</p></body></html>", body.replace('\n', "<br>"))),
            attachments: Vec::new(),
            headers: HashMap::new(),
            size: body.len(),
            created_at: Utc::now(),
        }
    }

    /// Generate random attachment
    pub fn random_attachment(&mut self, max_size: usize) -> EmailAttachment {
        let extensions = vec!["txt", "pdf", "doc", "jpg", "png", "zip"];
        let content_types = vec![
            "text/plain",
            "application/pdf",
            "application/msword",
            "image/jpeg",
            "image/png",
            "application/zip",
        ];
        
        let ext_index = self.rng.gen_range(0..extensions.len());
        let filename = self.random_filename(extensions[ext_index]);
        let content_type = content_types[ext_index].to_string();
        let size = self.rng.gen_range(1024..max_size.max(2048));
        let data = self.random_binary_data(size);
        
        EmailAttachment {
            filename,
            content_type,
            data,
            size,
        }
    }
}

impl EmailTemplates {
    /// Get a welcome email template
    pub fn welcome_email(user_name: &str, company_name: &str) -> (String, String) {
        let subject = format!("Welcome to {}!", company_name);
        let body = format!(
            "Dear {},\n\nWelcome to {}! We're excited to have you on board.\n\nBest regards,\nThe {} Team",
            user_name, company_name, company_name
        );
        (subject, body)
    }

    /// Get a meeting invitation template
    pub fn meeting_invitation(meeting_title: &str, date_time: &str, location: &str) -> (String, String) {
        let subject = format!("Meeting Invitation: {}", meeting_title);
        let body = format!(
            "You are invited to attend:\n\nMeeting: {}\nDate & Time: {}\nLocation: {}\n\nPlease confirm your attendance.",
            meeting_title, date_time, location
        );
        (subject, body)
    }

    /// Get a project update template
    pub fn project_update(project_name: &str, status: &str, next_steps: &str) -> (String, String) {
        let subject = format!("Project Update: {}", project_name);
        let body = format!(
            "Project: {}\nStatus: {}\n\nNext Steps:\n{}\n\nPlease review and provide feedback.",
            project_name, status, next_steps
        );
        (subject, body)
    }

    /// Get a system notification template
    pub fn system_notification(system_name: &str, message: &str, severity: &str) -> (String, String) {
        let subject = format!("[{}] System Notification: {}", severity.to_uppercase(), system_name);
        let body = format!(
            "System: {}\nSeverity: {}\n\nMessage:\n{}\n\nPlease take appropriate action if required.",
            system_name, severity, message
        );
        (subject, body)
    }
}

impl FileUtils {
    /// Create a temporary file with specified content
    pub fn create_temp_file(content: &[u8], extension: &str) -> Result<std::path::PathBuf> {
        use std::io::Write;
        
        let temp_dir = std::env::temp_dir();
        let filename = format!("test_{}_{}.{}", 
            Uuid::new_v4().to_string()[..8].to_string(),
            chrono::Utc::now().timestamp(),
            extension
        );
        let file_path = temp_dir.join(filename);
        
        let mut file = std::fs::File::create(&file_path)
            .map_err(|e| format!("Failed to create temp file: {}", e))?;
        
        file.write_all(content)
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
        
        Ok(file_path)
    }

    /// Read file content as bytes
    pub fn read_file_bytes(path: &Path) -> Result<Vec<u8>> {
        std::fs::read(path)
            .map_err(|e| format!("Failed to read file: {}", e).into())
    }

    /// Get file size
    pub fn get_file_size(path: &Path) -> Result<u64> {
        let metadata = std::fs::metadata(path)
            .map_err(|e| format!("Failed to get file metadata: {}", e))?;
        Ok(metadata.len())
    }

    /// Delete file if it exists
    pub fn delete_file_if_exists(path: &Path) -> Result<()> {
        if path.exists() {
            std::fs::remove_file(path)
                .map_err(|e| format!("Failed to delete file: {}", e))?;
        }
        Ok(())
    }

    /// Create directory if it doesn't exist
    pub fn create_dir_if_not_exists(path: &Path) -> Result<()> {
        if !path.exists() {
            std::fs::create_dir_all(path)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        Ok(())
    }
}

impl NetworkUtils {
    /// Check if a port is available
    pub fn is_port_available(port: u16) -> bool {
        use std::net::{TcpListener, SocketAddr};
        
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        TcpListener::bind(addr).is_ok()
    }

    /// Find an available port in a range
    pub fn find_available_port(start: u16, end: u16) -> Option<u16> {
        for port in start..=end {
            if Self::is_port_available(port) {
                return Some(port);
            }
        }
        None
    }

    /// Get local IP address
    pub fn get_local_ip() -> Result<String> {
        use std::net::{UdpSocket, SocketAddr};
        
        // Connect to a remote address to determine local IP
        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| format!("Failed to bind UDP socket: {}", e))?;
        
        socket.connect("8.8.8.8:80")
            .map_err(|e| format!("Failed to connect UDP socket: {}", e))?;
        
        let local_addr = socket.local_addr()
            .map_err(|e| format!("Failed to get local address: {}", e))?;
        
        Ok(local_addr.ip().to_string())
    }

    /// Validate email address format
    pub fn is_valid_email(email: &str) -> bool {
        // Simple email validation regex
        let email_regex = regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        email_regex.is_match(email)
    }

    /// Validate domain name format
    pub fn is_valid_domain(domain: &str) -> bool {
        // Simple domain validation
        let domain_regex = regex::Regex::new(r"^[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        domain_regex.is_match(domain)
    }
}

impl TimeUtils {
    /// Get current timestamp as string
    pub fn current_timestamp() -> String {
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
    }

    /// Get current timestamp as milliseconds
    pub fn current_timestamp_millis() -> u64 {
        Utc::now().timestamp_millis() as u64
    }

    /// Format duration as human-readable string
    pub fn format_duration(duration: Duration) -> String {
        let total_secs = duration.as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;
        let millis = duration.subsec_millis();
        
        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else if seconds > 0 {
            format!("{}.{}s", seconds, millis / 100)
        } else {
            format!("{}ms", millis)
        }
    }

    /// Sleep for a random duration within a range
    pub async fn random_sleep(min_ms: u64, max_ms: u64) {
        let mut rng = thread_rng();
        let sleep_ms = rng.gen_range(min_ms..=max_ms);
        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
    }

    /// Create a timeout future
    pub async fn with_timeout<T, F>(duration: Duration, future: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        tokio::time::timeout(duration, future)
            .await
            .map_err(|_| "Operation timed out".into())?
    }
}

impl ValidationUtils {
    /// Validate test result
    pub fn validate_test_result(result: &crate::TestResult, expected_success: bool) -> bool {
        result.success == expected_success
    }

    /// Validate email content
    pub fn validate_email_content(email: &TestEmail) -> Vec<String> {
        let mut errors = Vec::new();
        
        if email.from.is_empty() {
            errors.push("Email sender is empty".to_string());
        }
        
        if email.to.is_empty() {
            errors.push("Email recipients list is empty".to_string());
        }
        
        if email.subject.is_empty() {
            errors.push("Email subject is empty".to_string());
        }
        
        if email.body.is_empty() {
            errors.push("Email body is empty".to_string());
        }
        
        // Validate email addresses
        if !NetworkUtils::is_valid_email(&email.from) {
            errors.push(format!("Invalid sender email: {}", email.from));
        }
        
        for recipient in &email.to {
            if !NetworkUtils::is_valid_email(recipient) {
                errors.push(format!("Invalid recipient email: {}", recipient));
            }
        }
        
        errors
    }

    /// Validate user data
    pub fn validate_user_data(user: &TestUser) -> Vec<String> {
        let mut errors = Vec::new();
        
        if user.username.is_empty() {
            errors.push("Username is empty".to_string());
        }
        
        if user.email.is_empty() {
            errors.push("Email is empty".to_string());
        }
        
        if user.password.is_empty() {
            errors.push("Password is empty".to_string());
        }
        
        if user.domain.is_empty() {
            errors.push("Domain is empty".to_string());
        }
        
        // Validate email format
        if !NetworkUtils::is_valid_email(&user.email) {
            errors.push(format!("Invalid email format: {}", user.email));
        }
        
        // Validate domain format
        if !NetworkUtils::is_valid_domain(&user.domain) {
            errors.push(format!("Invalid domain format: {}", user.domain));
        }
        
        errors
    }

    /// Validate attachment data
    pub fn validate_attachment(attachment: &EmailAttachment) -> Vec<String> {
        let mut errors = Vec::new();
        
        if attachment.filename.is_empty() {
            errors.push("Attachment filename is empty".to_string());
        }
        
        if attachment.content_type.is_empty() {
            errors.push("Attachment content type is empty".to_string());
        }
        
        if attachment.data.is_empty() {
            errors.push("Attachment data is empty".to_string());
        }
        
        if attachment.size != attachment.data.len() {
            errors.push("Attachment size mismatch".to_string());
        }
        
        errors
    }

    /// Calculate success rate from test results
    pub fn calculate_success_rate(results: &[crate::TestResult]) -> f64 {
        if results.is_empty() {
            return 0.0;
        }
        
        let successful = results.iter().filter(|r| r.success).count();
        successful as f64 / results.len() as f64
    }

    /// Calculate average duration from test results
    pub fn calculate_average_duration(results: &[crate::TestResult]) -> Duration {
        if results.is_empty() {
            return Duration::ZERO;
        }
        
        let total_nanos: u128 = results.iter().map(|r| r.duration.as_nanos()).sum();
        Duration::from_nanos((total_nanos / results.len() as u128) as u64)
    }
}

/// Utility functions for test setup and teardown
pub struct TestSetup;

impl TestSetup {
    /// Initialize test environment
    pub fn init_test_env() -> Result<()> {
        // Initialize logging
        tracing_subscriber::fmt()
            .with_env_filter("debug")
            .try_init()
            .map_err(|e| format!("Failed to initialize logging: {}", e))?;
        
        info!("Test environment initialized");
        Ok(())
    }

    /// Cleanup test environment
    pub fn cleanup_test_env() -> Result<()> {
        // Cleanup temporary files
        let temp_dir = std::env::temp_dir();
        let test_files: Vec<_> = std::fs::read_dir(&temp_dir)
            .map_err(|e| format!("Failed to read temp directory: {}", e))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_name()
                    .to_string_lossy()
                    .starts_with("test_")
            })
            .collect();
        
        for file in test_files {
            if let Err(e) = std::fs::remove_file(file.path()) {
                warn!("Failed to cleanup test file: {}", e);
            }
        }
        
        info!("Test environment cleaned up");
        Ok(())
    }

    /// Create test configuration directory
    pub fn create_test_config_dir() -> Result<std::path::PathBuf> {
        let config_dir = std::env::temp_dir().join("stalwart_test_config");
        FileUtils::create_dir_if_not_exists(&config_dir)?;
        Ok(config_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_generator() {
        let mut generator = TestDataGenerator::new();
        
        let random_str = generator.random_string(10);
        assert_eq!(random_str.len(), 10);
        
        let email = generator.random_email("test.com");
        assert!(email.contains("@test.com"));
        
        let subject = generator.random_subject();
        assert!(!subject.is_empty());
    }

    #[test]
    fn test_email_templates() {
        let (subject, body) = EmailTemplates::welcome_email("John", "Acme Corp");
        assert!(subject.contains("Welcome"));
        assert!(body.contains("John"));
        assert!(body.contains("Acme Corp"));
    }

    #[test]
    fn test_network_utils() {
        assert!(NetworkUtils::is_valid_email("test@example.com"));
        assert!(!NetworkUtils::is_valid_email("invalid-email"));
        
        assert!(NetworkUtils::is_valid_domain("example.com"));
        assert!(!NetworkUtils::is_valid_domain("invalid"));
    }

    #[test]
    fn test_time_utils() {
        let timestamp = TimeUtils::current_timestamp();
        assert!(!timestamp.is_empty());
        
        let duration = Duration::from_secs(125);
        let formatted = TimeUtils::format_duration(duration);
        assert!(formatted.contains("2m"));
    }

    #[tokio::test]
    async fn test_file_utils() {
        let content = b"test content";
        let temp_file = FileUtils::create_temp_file(content, "txt").unwrap();
        
        assert!(temp_file.exists());
        
        let read_content = FileUtils::read_file_bytes(&temp_file).unwrap();
        assert_eq!(read_content, content);
        
        let size = FileUtils::get_file_size(&temp_file).unwrap();
        assert_eq!(size, content.len() as u64);
        
        FileUtils::delete_file_if_exists(&temp_file).unwrap();
        assert!(!temp_file.exists());
    }
}
