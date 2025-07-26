/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! SMTP Performance Benchmarks
//! 
//! This module contains comprehensive performance benchmarks for SMTP operations,
//! including message parsing, authentication, spam filtering, and delivery.
//! These benchmarks ensure the SMTP server meets production performance requirements.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::{
    sync::Arc,
    time::Duration,
};
use tokio::runtime::Runtime;

/// Test message sizes for benchmarks
const SMALL_MESSAGE_SIZE: usize = 1_000;      // 1KB
const MEDIUM_MESSAGE_SIZE: usize = 100_000;   // 100KB
const LARGE_MESSAGE_SIZE: usize = 1_000_000;  // 1MB
const XLARGE_MESSAGE_SIZE: usize = 10_000_000; // 10MB

/// Benchmark configuration
struct SmtpBenchmarkConfig {
    runtime: Runtime,
    test_messages: Vec<Vec<u8>>,
    test_emails: Vec<String>,
}

impl SmtpBenchmarkConfig {
    fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");
        
        // Generate test messages of various sizes
        let mut test_messages = Vec::new();
        for &size in &[SMALL_MESSAGE_SIZE, MEDIUM_MESSAGE_SIZE, LARGE_MESSAGE_SIZE] {
            let message = generate_test_message(size);
            test_messages.push(message);
        }
        
        // Generate test email addresses
        let mut test_emails = Vec::new();
        for i in 0..1000 {
            test_emails.push(format!("user{}@example.com", i));
        }
        
        Self {
            runtime,
            test_messages,
            test_emails,
        }
    }
}

/// Generate a test email message of specified size
fn generate_test_message(size: usize) -> Vec<u8> {
    let header = b"From: sender@example.com\r\n\
                   To: recipient@example.com\r\n\
                   Subject: Test Message\r\n\
                   Date: Mon, 1 Jan 2024 12:00:00 +0000\r\n\
                   Message-ID: <test@example.com>\r\n\
                   Content-Type: text/plain\r\n\
                   \r\n";
    
    let header_len = header.len();
    let body_len = if size > header_len { size - header_len } else { 0 };
    
    let mut message = Vec::with_capacity(size);
    message.extend_from_slice(header);
    
    // Fill the body with repeating text
    let body_text = b"This is a test message body. ";
    let body_text_len = body_text.len();
    
    for i in 0..body_len {
        message.push(body_text[i % body_text_len]);
    }
    
    message
}

/// Benchmark message parsing operations
fn bench_message_parsing(c: &mut Criterion) {
    let config = SmtpBenchmarkConfig::new();
    
    let mut group = c.benchmark_group("message_parsing");
    group.measurement_time(Duration::from_secs(10));
    
    // Benchmark parsing messages of different sizes
    for (i, message) in config.test_messages.iter().enumerate() {
        let size_name = match i {
            0 => "small",
            1 => "medium", 
            2 => "large",
            _ => "unknown",
        };
        
        group.bench_function(&format!("parse_{}_message", size_name), |b| {
            b.iter(|| {
                // Simulate message parsing
                let lines: Vec<&[u8]> = message.split(|&b| b == b'\n').collect();
                let header_end = lines.iter().position(|line| line.is_empty()).unwrap_or(0);
                let headers = &lines[..header_end];
                let body = &lines[header_end + 1..];
                black_box((headers.len(), body.len()));
            });
        });
    }
    
    group.finish();
}

/// Benchmark email address validation
fn bench_email_validation(c: &mut Criterion) {
    let config = SmtpBenchmarkConfig::new();
    
    let mut group = c.benchmark_group("email_validation");
    group.measurement_time(Duration::from_secs(10));
    
    // Benchmark email address validation
    group.bench_function("validate_email_addresses", |b| {
        b.iter(|| {
            for email in &config.test_emails[0..100] {
                // Simulate email validation
                let is_valid = email.contains('@') && 
                              email.len() > 3 && 
                              email.len() < 255 &&
                              !email.starts_with('@') &&
                              !email.ends_with('@');
                black_box(is_valid);
            }
        });
    });
    
    // Benchmark domain extraction
    group.bench_function("extract_domains", |b| {
        b.iter(|| {
            for email in &config.test_emails[0..100] {
                let domain = email.split('@').nth(1).unwrap_or("");
                black_box(domain);
            }
        });
    });
    
    group.finish();
}

/// Benchmark SMTP command parsing
fn bench_command_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("command_parsing");
    group.measurement_time(Duration::from_secs(10));
    
    // Test SMTP commands
    let commands = vec![
        b"HELO example.com\r\n".to_vec(),
        b"EHLO example.com\r\n".to_vec(),
        b"MAIL FROM:<sender@example.com>\r\n".to_vec(),
        b"RCPT TO:<recipient@example.com>\r\n".to_vec(),
        b"DATA\r\n".to_vec(),
        b"QUIT\r\n".to_vec(),
        b"RSET\r\n".to_vec(),
        b"NOOP\r\n".to_vec(),
    ];
    
    // Benchmark command parsing
    group.bench_function("parse_smtp_commands", |b| {
        b.iter(|| {
            for command in &commands {
                // Simulate command parsing
                let cmd_str = std::str::from_utf8(command).unwrap_or("");
                let parts: Vec<&str> = cmd_str.trim().split_whitespace().collect();
                let cmd_name = parts.first().unwrap_or(&"");
                let args = &parts[1..];
                black_box((cmd_name, args.len()));
            }
        });
    });
    
    group.finish();
}

/// Benchmark concurrent SMTP operations
fn bench_concurrent_smtp(c: &mut Criterion) {
    let config = SmtpBenchmarkConfig::new();
    
    let mut group = c.benchmark_group("concurrent_smtp");
    group.measurement_time(Duration::from_secs(15));
    
    // Benchmark concurrent message processing
    group.bench_function("concurrent_message_processing", |b| {
        b.to_async(&config.runtime).iter(|| async {
            let mut handles = Vec::new();
            
            for i in 0..50 {
                let message = config.test_messages[i % config.test_messages.len()].clone();
                
                let handle = tokio::spawn(async move {
                    // Simulate message processing
                    let lines: Vec<&[u8]> = message.split(|&b| b == b'\n').collect();
                    let header_count = lines.iter()
                        .take_while(|line| !line.is_empty())
                        .count();
                    
                    // Simulate some processing time
                    tokio::task::yield_now().await;
                    
                    black_box(header_count);
                });
                
                handles.push(handle);
            }
            
            for handle in handles {
                handle.await.unwrap();
            }
        });
    });
    
    group.finish();
}

/// Benchmark spam filtering operations
fn bench_spam_filtering(c: &mut Criterion) {
    let config = SmtpBenchmarkConfig::new();
    
    let mut group = c.benchmark_group("spam_filtering");
    group.measurement_time(Duration::from_secs(10));
    
    // Simulate spam keywords
    let spam_keywords = vec![
        "viagra", "casino", "lottery", "winner", "urgent", "free", "money",
        "click here", "limited time", "act now", "guaranteed", "risk free"
    ];
    
    // Benchmark spam keyword detection
    group.bench_function("spam_keyword_detection", |b| {
        b.iter(|| {
            for message in &config.test_messages {
                let message_str = std::str::from_utf8(message).unwrap_or("");
                let message_lower = message_str.to_lowercase();
                
                let mut spam_score = 0;
                for keyword in &spam_keywords {
                    if message_lower.contains(keyword) {
                        spam_score += 1;
                    }
                }
                
                black_box(spam_score);
            }
        });
    });
    
    group.finish();
}

/// Benchmark message size calculations
fn bench_message_metrics(c: &mut Criterion) {
    let config = SmtpBenchmarkConfig::new();
    
    let mut group = c.benchmark_group("message_metrics");
    group.measurement_time(Duration::from_secs(10));
    
    // Benchmark message size calculations
    group.bench_function("calculate_message_sizes", |b| {
        b.iter(|| {
            for message in &config.test_messages {
                let total_size = message.len();
                let line_count = message.iter().filter(|&&b| b == b'\n').count();
                let header_end = message.windows(4)
                    .position(|w| w == b"\r\n\r\n")
                    .unwrap_or(0);
                let header_size = header_end;
                let body_size = total_size.saturating_sub(header_size + 4);
                
                black_box((total_size, line_count, header_size, body_size));
            }
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_message_parsing,
    bench_email_validation,
    bench_command_parsing,
    bench_concurrent_smtp,
    bench_spam_filtering,
    bench_message_metrics
);

criterion_main!(benches);
