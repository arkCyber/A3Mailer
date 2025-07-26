//! Health Monitoring for A3Mailer
//!
//! This module provides comprehensive health checks for all system components
//! including AI services, Web3 integration, and core email functionality.

use crate::{MonitoringConfig, HealthStatus, HealthState, ComponentHealth, Result, MonitoringError};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub enabled: bool,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub critical_threshold: f64,
    pub warning_threshold: f64,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_seconds: 10,
            retry_attempts: 3,
            critical_threshold: 0.8,
            warning_threshold: 0.6,
        }
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub component: String,
    pub status: HealthState,
    pub message: String,
    pub response_time_ms: u64,
    pub details: HashMap<String, String>,
    pub checked_at: DateTime<Utc>,
}

/// Health monitor for system components
pub struct HealthMonitor {
    config: MonitoringConfig,
    health_checks: HashMap<String, HealthCheckConfig>,
    component_status: RwLock<HashMap<String, ComponentHealth>>,
    last_check_time: RwLock<DateTime<Utc>>,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub async fn new(config: &MonitoringConfig) -> Result<Self> {
        info!("Initializing health monitor");
        
        let mut health_checks = HashMap::new();
        
        // Configure health checks for different components
        health_checks.insert("database".to_string(), HealthCheckConfig::default());
        health_checks.insert("redis".to_string(), HealthCheckConfig::default());
        health_checks.insert("ai_service".to_string(), HealthCheckConfig {
            timeout_seconds: 5,
            ..HealthCheckConfig::default()
        });
        health_checks.insert("web3_service".to_string(), HealthCheckConfig {
            timeout_seconds: 15,
            ..HealthCheckConfig::default()
        });
        health_checks.insert("smtp_service".to_string(), HealthCheckConfig::default());
        health_checks.insert("imap_service".to_string(), HealthCheckConfig::default());
        health_checks.insert("storage".to_string(), HealthCheckConfig::default());
        
        let monitor = Self {
            config: config.clone(),
            health_checks,
            component_status: RwLock::new(HashMap::new()),
            last_check_time: RwLock::new(Utc::now()),
        };
        
        // Initialize component status
        monitor.initialize_component_status().await?;
        
        info!("Health monitor initialized successfully");
        Ok(monitor)
    }

    /// Run health checks for all components
    pub async fn run_health_checks(&self) -> Result<()> {
        debug!("Running health checks for all components");
        
        let start_time = Instant::now();
        let mut check_results = Vec::new();
        
        // Run health checks in parallel
        let mut check_futures = Vec::new();
        
        for (component, config) in &self.health_checks {
            if config.enabled {
                let component_name = component.clone();
                let config_clone = config.clone();
                let future = self.check_component_health(component_name, config_clone);
                check_futures.push(future);
            }
        }
        
        // Wait for all health checks to complete
        let results = futures::future::join_all(check_futures).await;
        
        // Update component status
        let mut component_status = self.component_status.write().await;
        for result in results {
            match result {
                Ok(health_result) => {
                    let component_health = ComponentHealth {
                        status: health_result.status.clone(),
                        message: health_result.message.clone(),
                        last_check: health_result.checked_at,
                        response_time_ms: health_result.response_time_ms,
                    };
                    component_status.insert(health_result.component.clone(), component_health);
                    check_results.push(health_result);
                }
                Err(e) => {
                    error!("Health check failed: {}", e);
                }
            }
        }
        
        // Update last check time
        {
            let mut last_check = self.last_check_time.write().await;
            *last_check = Utc::now();
        }
        
        let total_time = start_time.elapsed();
        info!("Health checks completed in {}ms ({} components)", 
              total_time.as_millis(), check_results.len());
        
        Ok(())
    }

    /// Get overall health status
    pub async fn get_health_status(&self) -> Result<HealthStatus> {
        let component_status = self.component_status.read().await;
        let last_updated = *self.last_check_time.read().await;
        
        // Calculate overall status
        let overall_status = self.calculate_overall_status(&component_status);
        
        Ok(HealthStatus {
            overall_status,
            components: component_status.clone(),
            last_updated,
        })
    }

    /// Check health of a specific component
    async fn check_component_health(&self, component: String, config: HealthCheckConfig) -> Result<HealthCheckResult> {
        debug!("Checking health of component: {}", component);
        
        let start_time = Instant::now();
        let mut attempts = 0;
        let mut last_error = None;
        
        while attempts < config.retry_attempts {
            attempts += 1;
            
            match self.perform_health_check(&component, &config).await {
                Ok(result) => {
                    let response_time = start_time.elapsed().as_millis() as u64;
                    return Ok(HealthCheckResult {
                        component: component.clone(),
                        status: result.0,
                        message: result.1,
                        response_time_ms: response_time,
                        details: result.2,
                        checked_at: Utc::now(),
                    });
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempts < config.retry_attempts {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        }
        
        // All attempts failed
        let response_time = start_time.elapsed().as_millis() as u64;
        let error_message = last_error
            .map(|e| e.to_string())
            .unwrap_or_else(|| "Health check failed".to_string());
        
        Ok(HealthCheckResult {
            component: component.clone(),
            status: HealthState::Unhealthy,
            message: error_message,
            response_time_ms: response_time,
            details: HashMap::new(),
            checked_at: Utc::now(),
        })
    }

    /// Perform actual health check for a component
    async fn perform_health_check(&self, component: &str, config: &HealthCheckConfig) -> Result<(HealthState, String, HashMap<String, String>)> {
        let timeout = Duration::from_secs(config.timeout_seconds);
        
        match component {
            "database" => self.check_database_health(timeout).await,
            "redis" => self.check_redis_health(timeout).await,
            "ai_service" => self.check_ai_service_health(timeout).await,
            "web3_service" => self.check_web3_service_health(timeout).await,
            "smtp_service" => self.check_smtp_service_health(timeout).await,
            "imap_service" => self.check_imap_service_health(timeout).await,
            "storage" => self.check_storage_health(timeout).await,
            _ => Ok((HealthState::Unknown, format!("Unknown component: {}", component), HashMap::new())),
        }
    }

    /// Check database health
    async fn check_database_health(&self, _timeout: Duration) -> Result<(HealthState, String, HashMap<String, String>)> {
        debug!("Checking database health");
        
        // In a real implementation, this would:
        // 1. Test database connection
        // 2. Execute a simple query
        // 3. Check connection pool status
        // 4. Verify database schema
        
        let mut details = HashMap::new();
        details.insert("connection_pool_size".to_string(), "10".to_string());
        details.insert("active_connections".to_string(), "3".to_string());
        details.insert("schema_version".to_string(), "1.0.0".to_string());
        
        // Simulate database check
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        Ok((HealthState::Healthy, "Database is healthy".to_string(), details))
    }

    /// Check Redis health
    async fn check_redis_health(&self, _timeout: Duration) -> Result<(HealthState, String, HashMap<String, String>)> {
        debug!("Checking Redis health");
        
        let mut details = HashMap::new();
        details.insert("memory_usage".to_string(), "45MB".to_string());
        details.insert("connected_clients".to_string(), "5".to_string());
        details.insert("keyspace_hits".to_string(), "1250".to_string());
        
        // Simulate Redis check
        tokio::time::sleep(Duration::from_millis(30)).await;
        
        Ok((HealthState::Healthy, "Redis is healthy".to_string(), details))
    }

    /// Check AI service health
    async fn check_ai_service_health(&self, _timeout: Duration) -> Result<(HealthState, String, HashMap<String, String>)> {
        debug!("Checking AI service health");
        
        let mut details = HashMap::new();
        details.insert("models_loaded".to_string(), "3".to_string());
        details.insert("inference_queue_size".to_string(), "2".to_string());
        details.insert("avg_inference_time_ms".to_string(), "8.5".to_string());
        details.insert("gpu_utilization".to_string(), "65%".to_string());
        
        // Simulate AI service check
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        Ok((HealthState::Healthy, "AI service is healthy".to_string(), details))
    }

    /// Check Web3 service health
    async fn check_web3_service_health(&self, _timeout: Duration) -> Result<(HealthState, String, HashMap<String, String>)> {
        debug!("Checking Web3 service health");
        
        let mut details = HashMap::new();
        details.insert("blockchain_connection".to_string(), "connected".to_string());
        details.insert("latest_block".to_string(), "18500000".to_string());
        details.insert("did_cache_size".to_string(), "150".to_string());
        details.insert("ipfs_peers".to_string(), "25".to_string());
        
        // Simulate Web3 service check
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        Ok((HealthState::Healthy, "Web3 service is healthy".to_string(), details))
    }

    /// Check SMTP service health
    async fn check_smtp_service_health(&self, _timeout: Duration) -> Result<(HealthState, String, HashMap<String, String>)> {
        debug!("Checking SMTP service health");
        
        let mut details = HashMap::new();
        details.insert("active_connections".to_string(), "15".to_string());
        details.insert("queue_size".to_string(), "3".to_string());
        details.insert("messages_per_minute".to_string(), "120".to_string());
        
        // Simulate SMTP service check
        tokio::time::sleep(Duration::from_millis(40)).await;
        
        Ok((HealthState::Healthy, "SMTP service is healthy".to_string(), details))
    }

    /// Check IMAP service health
    async fn check_imap_service_health(&self, _timeout: Duration) -> Result<(HealthState, String, HashMap<String, String>)> {
        debug!("Checking IMAP service health");
        
        let mut details = HashMap::new();
        details.insert("active_sessions".to_string(), "8".to_string());
        details.insert("idle_connections".to_string(), "12".to_string());
        details.insert("sync_operations".to_string(), "5".to_string());
        
        // Simulate IMAP service check
        tokio::time::sleep(Duration::from_millis(35)).await;
        
        Ok((HealthState::Healthy, "IMAP service is healthy".to_string(), details))
    }

    /// Check storage health
    async fn check_storage_health(&self, _timeout: Duration) -> Result<(HealthState, String, HashMap<String, String>)> {
        debug!("Checking storage health");
        
        let mut details = HashMap::new();
        details.insert("disk_usage".to_string(), "65%".to_string());
        details.insert("available_space".to_string(), "500GB".to_string());
        details.insert("io_operations_per_sec".to_string(), "450".to_string());
        
        // Simulate storage check
        tokio::time::sleep(Duration::from_millis(60)).await;
        
        // Check if disk usage is too high
        let disk_usage = 65.0;
        let status = if disk_usage > 90.0 {
            HealthState::Unhealthy
        } else if disk_usage > 80.0 {
            HealthState::Degraded
        } else {
            HealthState::Healthy
        };
        
        let message = format!("Storage is {} ({}% used)", 
                             match status {
                                 HealthState::Healthy => "healthy",
                                 HealthState::Degraded => "degraded",
                                 HealthState::Unhealthy => "unhealthy",
                                 HealthState::Unknown => "unknown",
                             }, disk_usage);
        
        Ok((status, message, details))
    }

    /// Initialize component status
    async fn initialize_component_status(&self) -> Result<()> {
        debug!("Initializing component status");
        
        let mut component_status = self.component_status.write().await;
        
        for component in self.health_checks.keys() {
            component_status.insert(component.clone(), ComponentHealth {
                status: HealthState::Unknown,
                message: "Not checked yet".to_string(),
                last_check: Utc::now(),
                response_time_ms: 0,
            });
        }
        
        info!("Component status initialized for {} components", component_status.len());
        Ok(())
    }

    /// Calculate overall system health status
    fn calculate_overall_status(&self, components: &HashMap<String, ComponentHealth>) -> HealthState {
        if components.is_empty() {
            return HealthState::Unknown;
        }
        
        let mut healthy_count = 0;
        let mut degraded_count = 0;
        let mut unhealthy_count = 0;
        let mut unknown_count = 0;
        
        for component in components.values() {
            match component.status {
                HealthState::Healthy => healthy_count += 1,
                HealthState::Degraded => degraded_count += 1,
                HealthState::Unhealthy => unhealthy_count += 1,
                HealthState::Unknown => unknown_count += 1,
            }
        }
        
        let total_count = components.len();
        
        // If any component is unhealthy, system is unhealthy
        if unhealthy_count > 0 {
            return HealthState::Unhealthy;
        }
        
        // If more than 20% of components are degraded, system is degraded
        if degraded_count as f64 / total_count as f64 > 0.2 {
            return HealthState::Degraded;
        }
        
        // If any component is degraded, system is degraded
        if degraded_count > 0 {
            return HealthState::Degraded;
        }
        
        // If more than 50% of components are unknown, system is unknown
        if unknown_count as f64 / total_count as f64 > 0.5 {
            return HealthState::Unknown;
        }
        
        // Otherwise, system is healthy
        HealthState::Healthy
    }

    /// Get health monitor statistics
    pub async fn get_stats(&self) -> Result<HashMap<String, String>> {
        let mut stats = HashMap::new();
        
        let component_status = self.component_status.read().await;
        let last_check = *self.last_check_time.read().await;
        
        let mut healthy_count = 0;
        let mut degraded_count = 0;
        let mut unhealthy_count = 0;
        let mut unknown_count = 0;
        
        for component in component_status.values() {
            match component.status {
                HealthState::Healthy => healthy_count += 1,
                HealthState::Degraded => degraded_count += 1,
                HealthState::Unhealthy => unhealthy_count += 1,
                HealthState::Unknown => unknown_count += 1,
            }
        }
        
        stats.insert("total_components".to_string(), component_status.len().to_string());
        stats.insert("healthy_components".to_string(), healthy_count.to_string());
        stats.insert("degraded_components".to_string(), degraded_count.to_string());
        stats.insert("unhealthy_components".to_string(), unhealthy_count.to_string());
        stats.insert("unknown_components".to_string(), unknown_count.to_string());
        stats.insert("last_check".to_string(), last_check.to_rfc3339());
        
        Ok(stats)
    }

    /// Shutdown health monitor
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down health monitor");
        
        // Clear component status
        self.component_status.write().await.clear();
        
        info!("Health monitor shutdown complete");
        Ok(())
    }
}
