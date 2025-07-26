/*!
 * Stress Test Binary
 * 
 * This binary provides a command-line interface for running stress tests
 * against the Stalwart Mail Server.
 * 
 * Usage:
 *   cargo run --bin stress-test -- [OPTIONS]
 * 
 * Author: Stalwart Labs Ltd.
 * Created: 2024-07-26
 */

use std::time::{Duration, Instant};
use std::path::PathBuf;
use clap::{Parser, Subcommand};
use tracing::{info, warn, error};
use tokio;

use stalwart_integration_tests::{
    TestConfig, TestContext,
    stress::StressTestSuite,
    config::ConfigManager,
    metrics::MetricsCollector,
    utils::TestSetup,
};

/// Stress testing command-line interface
#[derive(Parser)]
#[command(name = "stress-test")]
#[command(about = "Stalwart Mail Server Stress Testing Tool")]
#[command(version = "1.0.0")]
struct Cli {
    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    
    /// Test environment
    #[arg(short, long, default_value = "testing")]
    environment: String,
    
    /// Output format
    #[arg(short, long, default_value = "text")]
    output: String,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
    
    /// Dry run (validate configuration only)
    #[arg(long)]
    dry_run: bool,
    
    #[command(subcommand)]
    command: Commands,
}

/// Available stress test commands
#[derive(Subcommand)]
enum Commands {
    /// Run all stress tests
    All {
        /// Maximum test duration in seconds
        #[arg(long, default_value = "3600")]
        max_duration: u64,
        
        /// Number of concurrent connections
        #[arg(long, default_value = "100")]
        concurrency: usize,
    },
    
    /// Run concurrent user stress tests
    ConcurrentUsers {
        /// Number of concurrent users
        #[arg(long, default_value = "50")]
        users: usize,
        
        /// Test duration in seconds
        #[arg(long, default_value = "300")]
        duration: u64,
    },
    
    /// Run high volume email stress tests
    HighVolume {
        /// Number of emails to send
        #[arg(long, default_value = "10000")]
        email_count: usize,
        
        /// Batch size for sending
        #[arg(long, default_value = "100")]
        batch_size: usize,
    },
    
    /// Run memory stress tests
    Memory {
        /// Maximum memory allocation in MB
        #[arg(long, default_value = "1024")]
        max_memory_mb: usize,
        
        /// Allocation step size in MB
        #[arg(long, default_value = "10")]
        step_size_mb: usize,
    },
    
    /// Run CPU stress tests
    Cpu {
        /// Number of CPU workers
        #[arg(long)]
        workers: Option<usize>,
        
        /// Work duration in seconds
        #[arg(long, default_value = "60")]
        duration: u64,
    },
    
    /// Run protocol-specific stress tests
    Protocol {
        /// Protocol to test (smtp, imap, pop3, jmap)
        #[arg(value_enum)]
        protocol: Protocol,
        
        /// Number of operations
        #[arg(long, default_value = "1000")]
        operations: usize,
    },
    
    /// Run endurance tests
    Endurance {
        /// Test duration in hours
        #[arg(long, default_value = "24")]
        hours: u64,
        
        /// Operation interval in seconds
        #[arg(long, default_value = "60")]
        interval: u64,
    },
}

/// Supported protocols for testing
#[derive(clap::ValueEnum, Clone, Debug)]
enum Protocol {
    Smtp,
    Imap,
    Pop3,
    Jmap,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Initialize test environment
    TestSetup::init_test_env()?;
    
    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .init();
    
    info!("Starting Stalwart Mail Server Stress Testing Tool");
    
    // Load configuration
    let config = load_configuration(&cli).await?;
    
    if cli.dry_run {
        info!("Dry run mode - validating configuration only");
        validate_configuration(&config)?;
        info!("Configuration validation successful");
        return Ok(());
    }
    
    // Create test context
    let context = TestContext::new(config);
    
    // Initialize metrics collector
    let metrics_collector = MetricsCollector::new();
    metrics_collector.start_collection().await?;
    
    // Create stress test suite
    let stress_suite = StressTestSuite::new(context);
    
    // Execute stress tests based on command
    let start_time = Instant::now();
    let results = match cli.command {
        Commands::All { max_duration, concurrency } => {
            info!("Running all stress tests with max_duration={}s, concurrency={}", max_duration, concurrency);
            stress_suite.run_all_tests().await?
        },
        
        Commands::ConcurrentUsers { users, duration } => {
            info!("Running concurrent users stress test with users={}, duration={}s", users, duration);
            stress_suite.test_concurrent_users().await?
        },
        
        Commands::HighVolume { email_count, batch_size } => {
            info!("Running high volume email stress test with email_count={}, batch_size={}", email_count, batch_size);
            stress_suite.test_high_volume_email().await?
        },
        
        Commands::Memory { max_memory_mb, step_size_mb } => {
            info!("Running memory stress test with max_memory={}MB, step_size={}MB", max_memory_mb, step_size_mb);
            stress_suite.test_memory_stress().await?
        },
        
        Commands::Cpu { workers, duration } => {
            let worker_count = workers.unwrap_or_else(|| num_cpus::get());
            info!("Running CPU stress test with workers={}, duration={}s", worker_count, duration);
            stress_suite.test_cpu_stress().await?
        },
        
        Commands::Protocol { protocol, operations } => {
            info!("Running protocol stress test for {:?} with operations={}", protocol, operations);
            stress_suite.test_protocol_stress().await?
        },
        
        Commands::Endurance { hours, interval } => {
            info!("Running endurance test for {}h with interval={}s", hours, interval);
            stress_suite.test_endurance().await?
        },
    };
    
    let total_duration = start_time.elapsed();
    
    // Generate and display results
    display_results(&results, total_duration, &cli.output).await?;
    
    // Generate metrics report
    let metrics_report = metrics_collector.generate_report().await;
    
    if cli.verbose {
        println!("\n{}", metrics_report);
    }
    
    // Cleanup
    TestSetup::cleanup_test_env()?;
    
    info!("Stress testing completed successfully");
    Ok(())
}

/// Load configuration from file or use defaults
async fn load_configuration(cli: &Cli) -> Result<TestConfig, Box<dyn std::error::Error>> {
    let mut config_manager = ConfigManager::new();
    
    if let Some(config_path) = &cli.config {
        info!("Loading configuration from: {:?}", config_path);
        config_manager.load_from_file(config_path)?;
    } else {
        info!("Using default configuration");
        // Try to load from environment variables
        if let Ok(env_config) = ConfigManager::from_environment() {
            return Ok(env_config);
        }
    }
    
    // Get environment-specific configuration
    let environment = match cli.environment.as_str() {
        "development" => crate::config::Environment::Development,
        "testing" => crate::config::Environment::Testing,
        "staging" => crate::config::Environment::Staging,
        "production" => crate::config::Environment::Production,
        "local" => crate::config::Environment::Local,
        _ => {
            warn!("Unknown environment '{}', using testing", cli.environment);
            crate::config::Environment::Testing
        }
    };
    
    let config = config_manager.get_config(&environment);
    Ok(config)
}

/// Validate configuration
fn validate_configuration(config: &TestConfig) -> Result<(), Box<dyn std::error::Error>> {
    let config_manager = ConfigManager::new();
    let validation_result = config_manager.validate_config(config);
    
    if !validation_result.valid {
        error!("Configuration validation failed:");
        for error in &validation_result.errors {
            error!("  - {}", error);
        }
        return Err("Invalid configuration".into());
    }
    
    if !validation_result.warnings.is_empty() {
        warn!("Configuration warnings:");
        for warning in &validation_result.warnings {
            warn!("  - {}", warning);
        }
    }
    
    info!("Configuration validation successful");
    Ok(())
}

/// Display test results
async fn display_results(
    results: &[crate::TestResult],
    total_duration: Duration,
    output_format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match output_format {
        "json" => display_json_results(results, total_duration).await?,
        "csv" => display_csv_results(results, total_duration).await?,
        "text" | _ => display_text_results(results, total_duration).await?,
    }
    
    Ok(())
}

/// Display results in text format
async fn display_text_results(
    results: &[crate::TestResult],
    total_duration: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Stress Test Results ===");
    println!("Total Duration: {:?}", total_duration);
    println!("Total Tests: {}", results.len());
    
    let successful = results.iter().filter(|r| r.success).count();
    let failed = results.len() - successful;
    
    println!("Successful: {}", successful);
    println!("Failed: {}", failed);
    println!("Success Rate: {:.2}%", (successful as f64 / results.len() as f64) * 100.0);
    
    if !results.is_empty() {
        let avg_duration = Duration::from_nanos(
            results.iter().map(|r| r.duration.as_nanos()).sum::<u128>() / results.len() as u128
        );
        println!("Average Test Duration: {:?}", avg_duration);
        
        let throughput = results.len() as f64 / total_duration.as_secs_f64();
        println!("Throughput: {:.2} tests/sec", throughput);
    }
    
    println!("\n=== Individual Test Results ===");
    for result in results {
        let status = if result.success { "PASS" } else { "FAIL" };
        println!("{}: {} ({:?})", status, result.name, result.duration);
        
        if let Some(error) = &result.error {
            println!("  Error: {}", error);
        }
        
        if !result.metadata.is_empty() {
            println!("  Metadata:");
            for (key, value) in &result.metadata {
                println!("    {}: {}", key, value);
            }
        }
    }
    
    Ok(())
}

/// Display results in JSON format
async fn display_json_results(
    results: &[crate::TestResult],
    total_duration: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let summary = serde_json::json!({
        "total_duration_ms": total_duration.as_millis(),
        "total_tests": results.len(),
        "successful_tests": results.iter().filter(|r| r.success).count(),
        "failed_tests": results.iter().filter(|r| !r.success).count(),
        "results": results
    });
    
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

/// Display results in CSV format
async fn display_csv_results(
    results: &[crate::TestResult],
    _total_duration: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("test_id,name,success,duration_ms,error,timestamp");
    
    for result in results {
        println!(
            "{},{},{},{},{},{}",
            result.test_id,
            result.name,
            result.success,
            result.duration.as_millis(),
            result.error.as_deref().unwrap_or(""),
            result.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );
    }
    
    Ok(())
}

/// Print usage examples
fn print_examples() {
    println!("Examples:");
    println!("  # Run all stress tests");
    println!("  cargo run --bin stress-test all");
    println!();
    println!("  # Run concurrent user test with 100 users for 5 minutes");
    println!("  cargo run --bin stress-test concurrent-users --users 100 --duration 300");
    println!();
    println!("  # Run high volume email test");
    println!("  cargo run --bin stress-test high-volume --email-count 5000 --batch-size 50");
    println!();
    println!("  # Run memory stress test");
    println!("  cargo run --bin stress-test memory --max-memory-mb 512");
    println!();
    println!("  # Run CPU stress test");
    println!("  cargo run --bin stress-test cpu --workers 4 --duration 120");
    println!();
    println!("  # Run SMTP protocol stress test");
    println!("  cargo run --bin stress-test protocol smtp --operations 2000");
    println!();
    println!("  # Run endurance test for 2 hours");
    println!("  cargo run --bin stress-test endurance --hours 2 --interval 30");
    println!();
    println!("  # Use custom configuration file");
    println!("  cargo run --bin stress-test --config stress-config.toml all");
    println!();
    println!("  # Output results in JSON format");
    println!("  cargo run --bin stress-test --output json all");
    println!();
    println!("  # Dry run to validate configuration");
    println!("  cargo run --bin stress-test --dry-run all");
}
