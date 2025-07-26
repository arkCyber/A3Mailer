/*!
 * Integration Test Binary
 *
 * This binary provides a comprehensive command-line interface for running
 * integration tests against the Stalwart Mail Server.
 *
 * Usage:
 *   cargo run --bin integration-test -- [OPTIONS] [COMMAND]
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
    auth::AuthTestSuite,
    email::EmailTestSuite,
    stress::StressTestSuite,
    scenarios::ScenarioTestSuite,
    security::SecurityTestSuite,
    config::ConfigManager,
    metrics::MetricsCollector,
    utils::TestSetup,
};

/// Integration testing command-line interface
#[derive(Parser)]
#[command(name = "integration-test")]
#[command(about = "Stalwart Mail Server Integration Testing Tool")]
#[command(version = "1.0.0")]
struct Cli {
    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Test environment
    #[arg(short, long, default_value = "testing")]
    environment: String,

    /// Output format (text, json, csv, html)
    #[arg(short, long, default_value = "text")]
    output: String,

    /// Output file path
    #[arg(long)]
    output_file: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Dry run (validate configuration only)
    #[arg(long)]
    dry_run: bool,

    /// Continue on failure
    #[arg(long)]
    continue_on_failure: bool,

    /// Generate detailed report
    #[arg(long)]
    detailed_report: bool,

    #[command(subcommand)]
    command: Commands,
}

/// Available test commands
#[derive(Subcommand)]
enum Commands {
    /// Run all integration tests
    All {
        /// Include stress tests
        #[arg(long)]
        include_stress: bool,

        /// Include security tests
        #[arg(long)]
        include_security: bool,

        /// Maximum test duration in seconds
        #[arg(long, default_value = "7200")]
        max_duration: u64,
    },

    /// Run authentication tests
    Auth {
        /// Test specific authentication method
        #[arg(long)]
        method: Option<String>,
    },

    /// Run email communication tests
    Email {
        /// Test specific protocol (smtp, imap, pop3, jmap)
        #[arg(long)]
        protocol: Option<String>,

        /// Include attachment tests
        #[arg(long)]
        include_attachments: bool,

        /// Include bulk email tests
        #[arg(long)]
        include_bulk: bool,
    },

    /// Run stress tests
    Stress {
        /// Stress test type
        #[arg(value_enum)]
        test_type: StressTestType,

        /// Test intensity (low, medium, high)
        #[arg(long, default_value = "medium")]
        intensity: String,
    },

    /// Run scenario tests
    Scenarios {
        /// Specific scenario to run
        #[arg(long)]
        scenario: Option<String>,

        /// Number of users for scenario
        #[arg(long, default_value = "10")]
        users: usize,
    },

    /// Run security tests
    Security {
        /// Security test category
        #[arg(long)]
        category: Option<String>,

        /// Include compliance tests
        #[arg(long)]
        include_compliance: bool,

        /// Compliance framework (owasp, nist, iso27001)
        #[arg(long)]
        framework: Option<String>,
    },

    /// Generate configuration template
    GenerateConfig {
        /// Template type (basic, stress, corporate, development)
        #[arg(value_enum)]
        template: ConfigTemplate,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Validate configuration
    ValidateConfig {
        /// Configuration file to validate
        #[arg(value_name = "FILE")]
        config_file: PathBuf,
    },

    /// Show test information
    Info {
        /// Show available test suites
        #[arg(long)]
        suites: bool,

        /// Show configuration options
        #[arg(long)]
        config_options: bool,

        /// Show examples
        #[arg(long)]
        examples: bool,
    },
}

/// Stress test types
#[derive(clap::ValueEnum, Clone, Debug)]
enum StressTestType {
    ConcurrentUsers,
    HighVolume,
    Memory,
    Cpu,
    Protocol,
    Endurance,
}

/// Configuration templates
#[derive(clap::ValueEnum, Clone, Debug)]
enum ConfigTemplate {
    Basic,
    Stress,
    Corporate,
    Development,
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

    info!("Starting Stalwart Mail Server Integration Testing Tool");

    // Handle special commands that don't require configuration
    match &cli.command {
        Commands::GenerateConfig { template, output } => {
            return generate_config_template(template, output).await;
        },
        Commands::ValidateConfig { config_file } => {
            return validate_config_file(config_file).await;
        },
        Commands::Info { suites, config_options, examples } => {
            return show_info(*suites, *config_options, *examples).await;
        },
        _ => {}
    }

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

    // Execute tests based on command
    let start_time = Instant::now();
    let results = execute_tests(&cli, &context).await?;
    let total_duration = start_time.elapsed();

    // Generate and display results
    display_results(&results, total_duration, &cli).await?;

    // Generate detailed report if requested
    if cli.detailed_report {
        let metrics_report = metrics_collector.generate_report().await;
        let analysis = metrics_collector.analyze_metrics().await;

        generate_detailed_report(&results, &metrics_report, &analysis, &cli).await?;
    }

    // Cleanup
    TestSetup::cleanup_test_env()?;

    // Determine exit code based on test results
    let failed_tests = results.iter().filter(|r| !r.success).count();
    if failed_tests > 0 && !cli.continue_on_failure {
        error!("{} tests failed", failed_tests);
        std::process::exit(1);
    }

    info!("Integration testing completed successfully");
    Ok(())
}

/// Execute tests based on the command
async fn execute_tests(
    cli: &Cli,
    context: &TestContext,
) -> Result<Vec<stalwart_integration_tests::TestResult>, Box<dyn std::error::Error>> {
    match &cli.command {
        Commands::All { include_stress, include_security, max_duration: _ } => {
            info!("Running all integration tests");
            let mut all_results = Vec::new();

            // Authentication tests
            let auth_suite = AuthTestSuite::new(context.clone());
            all_results.extend(auth_suite.run_all_tests().await?);

            // Email tests
            let email_suite = EmailTestSuite::new(context.clone());
            all_results.extend(email_suite.run_all_tests().await?);

            // Scenario tests
            let scenario_suite = ScenarioTestSuite::new(context.clone());
            let scenario_results = scenario_suite.run_all_scenarios().await?;
            // Convert scenario results to test results
            for scenario_result in scenario_results {
                all_results.extend(scenario_result.test_results);
            }

            // Stress tests (if requested)
            if *include_stress {
                let stress_suite = StressTestSuite::new(context.clone());
                all_results.extend(stress_suite.run_all_tests().await?);
            }

            // Security tests (if requested)
            if *include_security {
                let security_suite = SecurityTestSuite::new(context.clone());
                let security_results = security_suite.run_all_tests().await?;
                // Convert security results to test results
                for security_result in security_results {
                    all_results.push(stalwart_integration_tests::TestResult {
                        test_id: security_result.test_id,
                        name: security_result.name,
                        success: security_result.secure,
                        duration: security_result.duration,
                        error: security_result.vulnerability_details.map(|v| v.description),
                        metadata: security_result.metadata,
                        timestamp: security_result.timestamp,
                    });
                }
            }

            Ok(all_results)
        },

        Commands::Auth { method: _ } => {
            info!("Running authentication tests");
            let auth_suite = AuthTestSuite::new(context.clone());
            auth_suite.run_all_tests().await
        },

        Commands::Email { protocol: _, include_attachments: _, include_bulk: _ } => {
            info!("Running email communication tests");
            let email_suite = EmailTestSuite::new(context.clone());
            email_suite.run_all_tests().await
        },

        Commands::Stress { test_type, intensity: _ } => {
            info!("Running stress tests: {:?}", test_type);
            let stress_suite = StressTestSuite::new(context.clone());
            match test_type {
                StressTestType::ConcurrentUsers => stress_suite.test_concurrent_users().await,
                StressTestType::HighVolume => stress_suite.test_high_volume_email().await,
                StressTestType::Memory => stress_suite.test_memory_stress().await,
                StressTestType::Cpu => stress_suite.test_cpu_stress().await,
                StressTestType::Protocol => stress_suite.test_protocol_stress().await,
                StressTestType::Endurance => stress_suite.test_endurance().await,
            }
        },

        Commands::Scenarios { scenario: _, users: _ } => {
            info!("Running scenario tests");
            let scenario_suite = ScenarioTestSuite::new(context.clone());
            let scenario_results = scenario_suite.run_all_scenarios().await?;

            // Convert scenario results to test results
            let mut all_results = Vec::new();
            for scenario_result in scenario_results {
                all_results.extend(scenario_result.test_results);
            }

            Ok(all_results)
        },

        Commands::Security { category: _, include_compliance: _, framework: _ } => {
            info!("Running security tests");
            let security_suite = SecurityTestSuite::new(context.clone());
            let security_results = security_suite.run_all_tests().await?;

            // Convert security results to test results
            let mut all_results = Vec::new();
            for security_result in security_results {
                all_results.push(stalwart_integration_tests::TestResult {
                    test_id: security_result.test_id,
                    name: security_result.name,
                    success: security_result.secure,
                    duration: security_result.duration,
                    error: security_result.vulnerability_details.map(|v| v.description),
                    metadata: security_result.metadata,
                    timestamp: security_result.timestamp,
                });
            }

            Ok(all_results)
        },

        _ => unreachable!("Command should have been handled earlier"),
    }
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
        "development" => stalwart_integration_tests::config::Environment::Development,
        "testing" => stalwart_integration_tests::config::Environment::Testing,
        "staging" => stalwart_integration_tests::config::Environment::Staging,
        "production" => stalwart_integration_tests::config::Environment::Production,
        "local" => stalwart_integration_tests::config::Environment::Local,
        _ => {
            warn!("Unknown environment '{}', using testing", cli.environment);
            stalwart_integration_tests::config::Environment::Testing
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

/// Generate configuration template
async fn generate_config_template(
    template: &ConfigTemplate,
    output_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_manager = ConfigManager::new();

    let template_name = match template {
        ConfigTemplate::Basic => "basic",
        ConfigTemplate::Stress => "stress",
        ConfigTemplate::Corporate => "corporate",
        ConfigTemplate::Development => "development",
    };

    let config_template = config_manager.generate_template(template_name);

    // Save template to file
    let content = toml::to_string_pretty(&config_template.config)?;
    std::fs::write(output_path, content)?;

    info!("Generated {} configuration template: {:?}", template_name, output_path);
    println!("Template: {}", config_template.name);
    println!("Description: {}", config_template.description);

    Ok(())
}

/// Validate configuration file
async fn validate_config_file(config_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut config_manager = ConfigManager::new();
    config_manager.load_from_file(config_path)?;

    let config = config_manager.get_config(&stalwart_integration_tests::config::Environment::Testing);
    validate_configuration(&config)?;

    println!("Configuration file is valid: {:?}", config_path);
    Ok(())
}

/// Show information about the testing tool
async fn show_info(
    show_suites: bool,
    show_config_options: bool,
    show_examples: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if show_suites {
        println!("Available Test Suites:");
        println!("  auth        - Authentication and authorization tests");
        println!("  email       - Email communication tests (SMTP, IMAP, POP3, JMAP)");
        println!("  stress      - Stress and performance tests");
        println!("  scenarios   - Real-world scenario tests");
        println!("  security    - Security vulnerability tests");
        println!();
    }

    if show_config_options {
        println!("Configuration Options:");
        println!("  Server Configuration:");
        println!("    host, smtp_port, imap_port, pop3_port, jmap_port, use_tls");
        println!("  Execution Configuration:");
        println!("    max_duration, concurrency, timeout, retry_attempts");
        println!("  User Configuration:");
        println!("    count, domain, default_password, quota");
        println!("  Email Configuration:");
        println!("    bulk_count, max_size, include_attachments, max_attachment_size");
        println!("  Performance Configuration:");
        println!("    target_eps, max_connections, stress_duration, ramp_up_duration");
        println!();
    }

    if show_examples {
        print_examples();
    }

    Ok(())
}

/// Display test results
async fn display_results(
    results: &[stalwart_integration_tests::TestResult],
    total_duration: Duration,
    cli: &Cli,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_content = match cli.output.as_str() {
        "json" => format_json_results(results, total_duration)?,
        "csv" => format_csv_results(results, total_duration)?,
        "html" => format_html_results(results, total_duration)?,
        "text" | _ => format_text_results(results, total_duration)?,
    };

    if let Some(output_file) = &cli.output_file {
        std::fs::write(output_file, &output_content)?;
        info!("Results written to: {:?}", output_file);
    } else {
        println!("{}", output_content);
    }

    Ok(())
}

/// Format results as text
fn format_text_results(
    results: &[stalwart_integration_tests::TestResult],
    total_duration: Duration,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = String::new();

    output.push_str("=== Integration Test Results ===\n");
    output.push_str(&format!("Total Duration: {:?}\n", total_duration));
    output.push_str(&format!("Total Tests: {}\n", results.len()));

    let successful = results.iter().filter(|r| r.success).count();
    let failed = results.len() - successful;

    output.push_str(&format!("Successful: {}\n", successful));
    output.push_str(&format!("Failed: {}\n", failed));
    output.push_str(&format!("Success Rate: {:.2}%\n", (successful as f64 / results.len() as f64) * 100.0));

    if !results.is_empty() {
        let avg_duration = Duration::from_nanos(
            (results.iter().map(|r| r.duration.as_nanos()).sum::<u128>() / results.len() as u128) as u64
        );
        output.push_str(&format!("Average Test Duration: {:?}\n", avg_duration));

        let throughput = results.len() as f64 / total_duration.as_secs_f64();
        output.push_str(&format!("Throughput: {:.2} tests/sec\n", throughput));
    }

    output.push_str("\n=== Individual Test Results ===\n");
    for result in results {
        let status = if result.success { "PASS" } else { "FAIL" };
        output.push_str(&format!("{}: {} ({:?})\n", status, result.name, result.duration));

        if let Some(error) = &result.error {
            output.push_str(&format!("  Error: {}\n", error));
        }
    }

    Ok(output)
}

/// Format results as JSON
fn format_json_results(
    results: &[stalwart_integration_tests::TestResult],
    total_duration: Duration,
) -> Result<String, Box<dyn std::error::Error>> {
    let summary = serde_json::json!({
        "total_duration_ms": total_duration.as_millis(),
        "total_tests": results.len(),
        "successful_tests": results.iter().filter(|r| r.success).count(),
        "failed_tests": results.iter().filter(|r| !r.success).count(),
        "results": results
    });

    Ok(serde_json::to_string_pretty(&summary)?)
}

/// Format results as CSV
fn format_csv_results(
    results: &[stalwart_integration_tests::TestResult],
    _total_duration: Duration,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = String::new();
    output.push_str("test_id,name,success,duration_ms,error,timestamp\n");

    for result in results {
        output.push_str(&format!(
            "{},{},{},{},{},{}\n",
            result.test_id,
            result.name,
            result.success,
            result.duration.as_millis(),
            result.error.as_deref().unwrap_or(""),
            result.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ));
    }

    Ok(output)
}

/// Format results as HTML
fn format_html_results(
    results: &[stalwart_integration_tests::TestResult],
    total_duration: Duration,
) -> Result<String, Box<dyn std::error::Error>> {
    let successful = results.iter().filter(|r| r.success).count();
    let failed = results.len() - successful;
    let success_rate = (successful as f64 / results.len() as f64) * 100.0;

    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
    html.push_str("<title>Integration Test Results</title>\n");
    html.push_str("<style>\n");
    html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
    html.push_str("table { border-collapse: collapse; width: 100%; }\n");
    html.push_str("th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n");
    html.push_str("th { background-color: #f2f2f2; }\n");
    html.push_str(".pass { color: green; }\n");
    html.push_str(".fail { color: red; }\n");
    html.push_str("</style>\n");
    html.push_str("</head>\n<body>\n");

    html.push_str("<h1>Integration Test Results</h1>\n");
    html.push_str(&format!("<p><strong>Total Duration:</strong> {:?}</p>\n", total_duration));
    html.push_str(&format!("<p><strong>Total Tests:</strong> {}</p>\n", results.len()));
    html.push_str(&format!("<p><strong>Successful:</strong> {}</p>\n", successful));
    html.push_str(&format!("<p><strong>Failed:</strong> {}</p>\n", failed));
    html.push_str(&format!("<p><strong>Success Rate:</strong> {:.2}%</p>\n", success_rate));

    html.push_str("<h2>Test Results</h2>\n");
    html.push_str("<table>\n");
    html.push_str("<tr><th>Status</th><th>Test Name</th><th>Duration</th><th>Error</th></tr>\n");

    for result in results {
        let status_class = if result.success { "pass" } else { "fail" };
        let status_text = if result.success { "PASS" } else { "FAIL" };
        let error_text = result.error.as_deref().unwrap_or("");

        html.push_str(&format!(
            "<tr><td class=\"{}\"><strong>{}</strong></td><td>{}</td><td>{:?}</td><td>{}</td></tr>\n",
            status_class, status_text, result.name, result.duration, error_text
        ));
    }

    html.push_str("</table>\n");
    html.push_str("</body>\n</html>\n");

    Ok(html)
}

/// Generate detailed report
async fn generate_detailed_report(
    results: &[stalwart_integration_tests::TestResult],
    metrics_report: &str,
    analysis: &stalwart_integration_tests::metrics::MetricsAnalysis,
    cli: &Cli,
) -> Result<(), Box<dyn std::error::Error>> {
    let report_path = cli.output_file.as_ref()
        .map(|p| p.with_extension("detailed.html"))
        .unwrap_or_else(|| PathBuf::from("detailed_report.html"));

    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
    html.push_str("<title>Detailed Integration Test Report</title>\n");
    html.push_str("<style>\n");
    html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
    html.push_str("h1, h2, h3 { color: #333; }\n");
    html.push_str("pre { background-color: #f5f5f5; padding: 10px; border-radius: 5px; }\n");
    html.push_str("table { border-collapse: collapse; width: 100%; margin: 10px 0; }\n");
    html.push_str("th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n");
    html.push_str("th { background-color: #f2f2f2; }\n");
    html.push_str(".pass { color: green; }\n");
    html.push_str(".fail { color: red; }\n");
    html.push_str(".warning { color: orange; }\n");
    html.push_str(".critical { color: red; font-weight: bold; }\n");
    html.push_str("</style>\n");
    html.push_str("</head>\n<body>\n");

    html.push_str("<h1>Detailed Integration Test Report</h1>\n");
    html.push_str(&format!("<p>Generated: {}</p>\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

    // Executive Summary
    html.push_str("<h2>Executive Summary</h2>\n");
    html.push_str(&format!("<p><strong>Overall Grade:</strong> {}</p>\n", analysis.summary.overall_grade));
    html.push_str(&format!("<p><strong>Success Rate:</strong> {:.2}%</p>\n", analysis.summary.success_rate));
    html.push_str(&format!("<p><strong>Performance Score:</strong> {:.2}</p>\n", analysis.summary.performance_score));
    html.push_str(&format!("<p><strong>Efficiency Score:</strong> {:.2}</p>\n", analysis.summary.efficiency_score));

    // Performance Insights
    if !analysis.insights.is_empty() {
        html.push_str("<h2>Performance Insights</h2>\n");
        html.push_str("<ul>\n");
        for insight in &analysis.insights {
            let severity_class = match insight.severity {
                stalwart_integration_tests::metrics::InsightSeverity::Info => "",
                stalwart_integration_tests::metrics::InsightSeverity::Warning => "warning",
                stalwart_integration_tests::metrics::InsightSeverity::Critical => "critical",
            };
            html.push_str(&format!(
                "<li class=\"{}\"><strong>{}:</strong> {} (Value: {:.2}, Threshold: {:.2})</li>\n",
                severity_class, insight.category, insight.message, insight.value, insight.threshold
            ));
        }
        html.push_str("</ul>\n");
    }

    // Recommendations
    if !analysis.recommendations.is_empty() {
        html.push_str("<h2>Recommendations</h2>\n");
        html.push_str("<ol>\n");
        for recommendation in &analysis.recommendations {
            html.push_str(&format!("<li>{}</li>\n", recommendation));
        }
        html.push_str("</ol>\n");
    }

    // Metrics Report
    html.push_str("<h2>Metrics Report</h2>\n");
    html.push_str("<pre>\n");
    html.push_str(metrics_report);
    html.push_str("</pre>\n");

    html.push_str("</body>\n</html>\n");

    std::fs::write(&report_path, html)?;
    info!("Detailed report generated: {:?}", report_path);

    Ok(())
}

/// Print usage examples
fn print_examples() {
    println!("Examples:");
    println!("  # Run all tests");
    println!("  cargo run --bin integration-test all");
    println!();
    println!("  # Run all tests including stress and security");
    println!("  cargo run --bin integration-test all --include-stress --include-security");
    println!();
    println!("  # Run only authentication tests");
    println!("  cargo run --bin integration-test auth");
    println!();
    println!("  # Run email tests with attachments and bulk testing");
    println!("  cargo run --bin integration-test email --include-attachments --include-bulk");
    println!();
    println!("  # Run concurrent user stress tests");
    println!("  cargo run --bin integration-test stress concurrent-users --intensity high");
    println!();
    println!("  # Run scenario tests with 50 users");
    println!("  cargo run --bin integration-test scenarios --users 50");
    println!();
    println!("  # Run security tests with OWASP compliance");
    println!("  cargo run --bin integration-test security --include-compliance --framework owasp");
    println!();
    println!("  # Generate basic configuration template");
    println!("  cargo run --bin integration-test generate-config basic -o basic-config.toml");
    println!();
    println!("  # Validate configuration file");
    println!("  cargo run --bin integration-test validate-config my-config.toml");
    println!();
    println!("  # Run tests with custom config and JSON output");
    println!("  cargo run --bin integration-test --config custom.toml --output json all");
    println!();
    println!("  # Generate detailed HTML report");
    println!("  cargo run --bin integration-test --detailed-report --output html all");
}
