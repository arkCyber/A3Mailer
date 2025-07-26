/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use pop3::{
    protocol::{
        request::{Parser, Error},
        response::{Response, ListItem},
        Command, Mechanism,
    },
    error::validation,
    op::authenticate::compute_apop_digest,
};

fn bench_command_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("command_parsing");
    
    let commands = [
        ("USER alice@example.com", "Simple USER command"),
        ("PASS secretpassword123", "Simple PASS command"),
        ("APOP user@example.com b913a602c7eda7a6c4d2e7c77c9e2c4e", "APOP command"),
        ("AUTH PLAIN dGVzdAB0ZXN0AHRlc3Q=", "AUTH PLAIN command"),
        ("RETR 12345", "RETR command"),
        ("TOP 999 100", "TOP command"),
        ("LIST", "LIST command"),
        ("UIDL 5", "UIDL command"),
        ("STAT", "STAT command"),
        ("DELE 1", "DELE command"),
        ("NOOP", "NOOP command"),
        ("QUIT", "QUIT command"),
        ("CAPA", "CAPA command"),
    ];

    for (cmd, description) in commands {
        group.bench_with_input(
            BenchmarkId::new("parse", description),
            &cmd,
            |b, cmd| {
                b.iter(|| {
                    let mut parser = Parser::default();
                    let result = parser.parse(&mut format!("{}\r\n", cmd).as_bytes().iter());
                    black_box(result)
                });
            },
        );
    }
    
    group.finish();
}

fn bench_pipelined_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipelined_parsing");
    
    let pipeline_sizes = [1, 5, 10, 50, 100];
    
    for size in pipeline_sizes {
        let mut pipeline = String::new();
        for i in 0..size {
            pipeline.push_str(&format!("RETR {}\r\n", i + 1));
        }
        
        group.bench_with_input(
            BenchmarkId::new("pipeline", size),
            &pipeline,
            |b, pipeline| {
                b.iter(|| {
                    let mut parser = Parser::default();
                    let mut iter = pipeline.as_bytes().iter();
                    let mut commands = Vec::new();
                    
                    loop {
                        match parser.parse(&mut iter) {
                            Ok(cmd) => commands.push(cmd),
                            Err(Error::NeedsMoreData) => break,
                            Err(_) => break,
                        }
                    }
                    black_box(commands)
                });
            },
        );
    }
    
    group.finish();
}

fn bench_response_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("response_serialization");
    
    // Benchmark OK response
    group.bench_function("ok_response", |b| {
        let response = Response::Ok::<u32>("Authentication successful".into());
        b.iter(|| black_box(response.serialize()));
    });
    
    // Benchmark error response
    group.bench_function("error_response", |b| {
        let response = Response::Err::<u32>("Invalid command".into());
        b.iter(|| black_box(response.serialize()));
    });
    
    // Benchmark LIST response with various sizes
    let list_sizes = [1, 10, 100, 1000];
    for size in list_sizes {
        let list_items: Vec<ListItem> = (1..=size)
            .map(|i| ListItem::Message { number: i, size: i * 1024 })
            .collect();
        let response = Response::List(list_items);
        
        group.bench_with_input(
            BenchmarkId::new("list_response", size),
            &response,
            |b, response| {
                b.iter(|| black_box(response.serialize()));
            },
        );
    }
    
    // Benchmark message response with various sizes
    let message_sizes = [1024, 10240, 102400, 1024000]; // 1KB to 1MB
    for size in message_sizes {
        let message_data = vec![b'A'; size];
        let response = Response::Message::<u32> {
            bytes: message_data,
            lines: 0,
        };
        
        group.bench_with_input(
            BenchmarkId::new("message_response", size),
            &response,
            |b, response| {
                b.iter(|| black_box(response.serialize()));
            },
        );
    }
    
    group.finish();
}

fn bench_apop_digest(c: &mut Criterion) {
    let mut group = c.benchmark_group("apop_digest");
    
    let test_cases = [
        ("<1896.697170952@dbc.mtview.ca.us>", "tanstaaf"),
        ("<test.123@example.com>", "password"),
        ("<very.long.timestamp.12345678901234567890@example.com>", "verylongpassword123456789"),
    ];
    
    for (timestamp, password) in test_cases {
        group.bench_with_input(
            BenchmarkId::new("compute", format!("{}:{}", timestamp.len(), password.len())),
            &(timestamp, password),
            |b, (timestamp, password)| {
                b.iter(|| black_box(compute_apop_digest(timestamp, password)));
            },
        );
    }
    
    group.finish();
}

fn bench_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation");
    
    // Benchmark message number validation
    group.bench_function("message_number", |b| {
        b.iter(|| {
            for i in 1..=1000 {
                black_box(validation::validate_message_number(i, 1000));
            }
        });
    });
    
    // Benchmark username validation
    let usernames = [
        "user@example.com",
        "very.long.username.with.many.dots@example.com",
        "用户@example.com",
        "test+tag@example.com",
    ];
    
    for username in usernames {
        group.bench_with_input(
            BenchmarkId::new("username", username.len()),
            &username,
            |b, username| {
                b.iter(|| black_box(validation::validate_username(username)));
            },
        );
    }
    
    // Benchmark APOP digest validation
    let digests = [
        "b913a602c7eda7a6c4d2e7c77c9e2c4e",
        "abcdef1234567890abcdef1234567890",
        "0123456789abcdef0123456789abcdef",
    ];
    
    for digest in digests {
        group.bench_function("apop_digest", |b| {
            b.iter(|| black_box(validation::validate_apop_digest(digest)));
        });
    }
    
    group.finish();
}

fn bench_mechanism_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("mechanism_parsing");
    
    let mechanisms = [
        b"PLAIN",
        b"CRAM-MD5",
        b"DIGEST-MD5",
        b"SCRAM-SHA-1",
        b"SCRAM-SHA-256",
        b"OAUTHBEARER",
        b"XOAUTH2",
        b"GSSAPI",
        b"NTLM",
        b"EXTERNAL",
        b"ANONYMOUS",
    ];
    
    for mechanism in mechanisms {
        group.bench_with_input(
            BenchmarkId::new("parse", std::str::from_utf8(mechanism).unwrap()),
            mechanism,
            |b, mechanism| {
                b.iter(|| black_box(Mechanism::parse(mechanism)));
            },
        );
    }
    
    group.finish();
}

fn bench_large_mailbox_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_mailbox");
    
    let mailbox_sizes = [100, 1000, 10000];
    
    for size in mailbox_sizes {
        // Create large LIST response
        let list_items: Vec<ListItem> = (1..=size)
            .map(|i| ListItem::Message { number: i, size: i * 1024 })
            .collect();
        let response = Response::List(list_items);
        
        group.bench_with_input(
            BenchmarkId::new("list_serialization", size),
            &response,
            |b, response| {
                b.iter(|| black_box(response.serialize()));
            },
        );
        
        // Create large UIDL response
        let uidl_items: Vec<ListItem> = (1..=size)
            .map(|i| ListItem::Uidl { 
                number: i, 
                uid: format!("uid{:06}", i) 
            })
            .collect();
        let uidl_response = Response::List(uidl_items);
        
        group.bench_with_input(
            BenchmarkId::new("uidl_serialization", size),
            &uidl_response,
            |b, response| {
                b.iter(|| black_box(response.serialize()));
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_command_parsing,
    bench_pipelined_parsing,
    bench_response_serialization,
    bench_apop_digest,
    bench_validation,
    bench_mechanism_parsing,
    bench_large_mailbox_operations
);

criterion_main!(benches);
