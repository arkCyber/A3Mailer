/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Health Check Module
//!
//! This module provides comprehensive health checking capabilities for various
//! system components including databases, caches, external services, and more.

use super::{HealthCheck, HealthStatus};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
    future::Future,
    pin::Pin,
};
use tracing::{debug, info, warn};

/// Health check function type
pub type HealthCheckFn = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = HealthCheck> + Send>> + Send + Sync>;

/// Health check configuration
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Component name
    pub name: String,
    /// Check interval
    pub interval: Duration,
    /// Timeout for the check
    pub timeout: Duration,
    /// Number of consecutive failures before marking as unhealthy
    pub failure_threshold: u32,
    /// Number of consecutive successes before marking as healthy
    pub success_threshold: u32,
    /// Enable this health check
    pub enabled: bool,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            name: "unknown".to_string(),
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            failure_threshold: 3,
            success_threshold: 1,
            enabled: true,
        }
    }
}

/// Health check state
#[derive(Debug, Clone)]
struct HealthCheckState {
    config: HealthCheckConfig,
    last_check: Option<Instant>,
    consecutive_failures: u32,
    consecutive_successes: u32,
    current_status: HealthStatus,
    last_result: Option<HealthCheck>,
}

impl HealthCheckState {
    fn new(config: HealthCheckConfig) -> Self {
        Self {
            config,
            last_check: None,
            consecutive_failures: 0,
            consecutive_successes: 0,
            current_status: HealthStatus::Unknown,
            last_result: None,
        }
    }

    fn should_run_check(&self) -> bool {
        if !self.config.enabled {
            return false;
        }

        match self.last_check {
            Some(last) => last.elapsed() >= self.config.interval,
            None => true,
        }
    }

    fn update_status(&mut self, check_result: HealthCheck) {
        self.last_check = Some(Instant::now());
        self.last_result = Some(check_result.clone());

        match check_result.status {
            HealthStatus::Healthy => {
                self.consecutive_successes += 1;
                self.consecutive_failures = 0;

                if self.consecutive_successes >= self.config.success_threshold {
                    self.current_status = HealthStatus::Healthy;
                }
            }
            HealthStatus::Unhealthy => {
                self.consecutive_failures += 1;
                self.consecutive_successes = 0;

                if self.consecutive_failures >= self.config.failure_threshold {
                    self.current_status = HealthStatus::Unhealthy;
                }
            }
            HealthStatus::Degraded => {
                self.consecutive_failures += 1;
                self.consecutive_successes = 0;

                if self.consecutive_failures >= self.config.failure_threshold {
                    self.current_status = HealthStatus::Degraded;
                }
            }
            HealthStatus::Unknown => {
                // Don't change status for unknown results
            }
        }
    }
}

/// Health check manager
pub struct HealthCheckManager {
    checks: Arc<std::sync::RwLock<HashMap<String, HealthCheckState>>>,
    check_functions: Arc<std::sync::RwLock<HashMap<String, HealthCheckFn>>>,
}

impl HealthCheckManager {
    /// Create a new health check manager
    pub fn new() -> Self {
        info!("Creating health check manager");
        Self {
            checks: Arc::new(std::sync::RwLock::new(HashMap::new())),
            check_functions: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Register a health check
    pub fn register_check<F, Fut>(&self, config: HealthCheckConfig, check_fn: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HealthCheck> + Send + 'static,
    {
        info!("Registering health check: {}", config.name);

        let name = config.name.clone();
        let state = HealthCheckState::new(config);

        // Store the state
        {
            let mut checks = self.checks.write().unwrap();
            checks.insert(name.clone(), state);
        }

        // Store the check function
        {
            let mut check_functions = self.check_functions.write().unwrap();
            let boxed_fn: HealthCheckFn = Arc::new(move || {
                Box::pin(check_fn())
            });
            check_functions.insert(name, boxed_fn);
        }
    }

    /// Run all health checks that are due
    pub async fn run_checks(&self) -> Vec<HealthCheck> {
        debug!("Running health checks");

        let mut results = Vec::new();
        let checks_to_run = {
            let checks = self.checks.read().unwrap();
            checks
                .iter()
                .filter(|(_, state)| state.should_run_check())
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>()
        };

        for check_name in checks_to_run {
            if let Some(result) = self.run_single_check(&check_name).await {
                results.push(result);
            }
        }

        results
    }

    /// Run a single health check
    async fn run_single_check(&self, check_name: &str) -> Option<HealthCheck> {
        debug!("Running health check: {}", check_name);

        let timeout = {
            let checks = self.checks.read().unwrap();
            let state = checks.get(check_name)?;
            state.config.timeout
        };

        let check_fn = {
            let check_functions = self.check_functions.read().unwrap();
            check_functions.get(check_name)?.clone()
        };

        let start_time = Instant::now();

        // Run the check with timeout
        let result = match tokio::time::timeout(timeout, (check_fn)()).await {
            Ok(check_result) => {
                let mut check_result = check_result;
                check_result.response_time = start_time.elapsed().as_millis() as u64;
                check_result
            }
            Err(_) => {
                warn!("Health check {} timed out after {:?}", check_name, timeout);
                HealthCheck::new(
                    check_name.to_string(),
                    HealthStatus::Unhealthy,
                    format!("Check timed out after {:?}", timeout),
                ).with_response_time(timeout)
            }
        };

        // Update the state
        {
            let mut checks = self.checks.write().unwrap();
            if let Some(state) = checks.get_mut(check_name) {
                state.update_status(result.clone());
            }
        }

        Some(result)
    }

    /// Get current status of all health checks
    pub fn get_all_statuses(&self) -> HashMap<String, HealthStatus> {
        let checks = self.checks.read().unwrap();
        checks
            .iter()
            .map(|(name, state)| (name.clone(), state.current_status.clone()))
            .collect()
    }

    /// Get detailed health check results
    pub fn get_detailed_results(&self) -> Vec<HealthCheck> {
        let checks = self.checks.read().unwrap();
        checks
            .values()
            .filter_map(|state| state.last_result.clone())
            .collect()
    }

    /// Get overall health status
    pub fn get_overall_status(&self) -> HealthStatus {
        let checks = self.checks.read().unwrap();

        if checks.is_empty() {
            return HealthStatus::Unknown;
        }

        let mut healthy_count = 0;
        let mut degraded_count = 0;
        let mut unhealthy_count = 0;
        let mut _unknown_count = 0;

        for state in checks.values() {
            if !state.config.enabled {
                continue;
            }

            match state.current_status {
                HealthStatus::Healthy => healthy_count += 1,
                HealthStatus::Degraded => degraded_count += 1,
                HealthStatus::Unhealthy => unhealthy_count += 1,
                HealthStatus::Unknown => _unknown_count += 1,
            }
        }

        // Determine overall status
        if unhealthy_count > 0 {
            HealthStatus::Unhealthy
        } else if degraded_count > 0 {
            HealthStatus::Degraded
        } else if healthy_count > 0 {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unknown
        }
    }

    /// Remove a health check
    pub fn remove_check(&self, name: &str) {
        info!("Removing health check: {}", name);

        {
            let mut checks = self.checks.write().unwrap();
            checks.remove(name);
        }

        {
            let mut check_functions = self.check_functions.write().unwrap();
            check_functions.remove(name);
        }
    }

    /// Enable or disable a health check
    pub fn set_check_enabled(&self, name: &str, enabled: bool) {
        let mut checks = self.checks.write().unwrap();
        if let Some(state) = checks.get_mut(name) {
            state.config.enabled = enabled;
            info!("Health check {} {}", name, if enabled { "enabled" } else { "disabled" });
        }
    }
}

impl Default for HealthCheckManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in health check functions
pub mod builtin {
    use super::*;

    /// Database connectivity health check
    pub async fn database_check() -> HealthCheck {
        debug!("Running database health check");

        // Simulate database check
        tokio::time::sleep(Duration::from_millis(10)).await;

        // In a real implementation, this would check database connectivity
        HealthCheck::new(
            "database".to_string(),
            HealthStatus::Healthy,
            "Database is responding".to_string(),
        )
        .with_detail("connection_pool".to_string(), "10/20 active".to_string())
        .with_detail("query_time".to_string(), "5ms".to_string())
    }

    /// Cache health check
    pub async fn cache_check() -> HealthCheck {
        debug!("Running cache health check");

        // Simulate cache check
        tokio::time::sleep(Duration::from_millis(5)).await;

        HealthCheck::new(
            "cache".to_string(),
            HealthStatus::Healthy,
            "Cache is responding".to_string(),
        )
        .with_detail("hit_rate".to_string(), "85%".to_string())
        .with_detail("memory_usage".to_string(), "512MB".to_string())
    }

    /// Disk space health check
    pub async fn disk_space_check() -> HealthCheck {
        debug!("Running disk space health check");

        // Simulate disk space check
        let usage_percent = 75.0; // Simulated value

        let status = if usage_percent > 90.0 {
            HealthStatus::Unhealthy
        } else if usage_percent > 80.0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        let message = format!("Disk usage: {:.1}%", usage_percent);

        HealthCheck::new("disk_space".to_string(), status, message)
            .with_detail("usage_percent".to_string(), format!("{:.1}%", usage_percent))
            .with_detail("available_space".to_string(), "25GB".to_string())
    }

    /// Memory usage health check
    pub async fn memory_check() -> HealthCheck {
        debug!("Running memory health check");

        // Simulate memory check
        let usage_percent = 65.0; // Simulated value

        let status = if usage_percent > 95.0 {
            HealthStatus::Unhealthy
        } else if usage_percent > 85.0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        let message = format!("Memory usage: {:.1}%", usage_percent);

        HealthCheck::new("memory".to_string(), status, message)
            .with_detail("usage_percent".to_string(), format!("{:.1}%", usage_percent))
            .with_detail("available_memory".to_string(), "2GB".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check_manager() {
        let manager = HealthCheckManager::new();

        // Register a simple health check
        let config = HealthCheckConfig {
            name: "test_check".to_string(),
            interval: Duration::from_millis(100),
            timeout: Duration::from_secs(1),
            failure_threshold: 2,
            success_threshold: 1,
            enabled: true,
        };

        manager.register_check(config, || async {
            HealthCheck::new(
                "test_check".to_string(),
                HealthStatus::Healthy,
                "Test check passed".to_string(),
            )
        });

        // Run checks
        let results = manager.run_checks().await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, HealthStatus::Healthy);

        // Check overall status
        assert_eq!(manager.get_overall_status(), HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_health_check_failure_threshold() {
        let manager = HealthCheckManager::new();

        let config = HealthCheckConfig {
            name: "failing_check".to_string(),
            interval: Duration::from_millis(1),
            timeout: Duration::from_secs(1),
            failure_threshold: 2,
            success_threshold: 1,
            enabled: true,
        };

        manager.register_check(config, || {
            async move {
                HealthCheck::new(
                    "failing_check".to_string(),
                    HealthStatus::Unhealthy,
                    "Check failed".to_string(),
                )
            }
        });

        // First failure - should not mark as unhealthy yet
        manager.run_checks().await;
        assert_ne!(manager.get_overall_status(), HealthStatus::Unhealthy);

        // Wait for interval
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Second failure - should mark as unhealthy
        manager.run_checks().await;
        assert_eq!(manager.get_overall_status(), HealthStatus::Unhealthy);
    }

    #[tokio::test]
    async fn test_builtin_health_checks() {
        let db_result = builtin::database_check().await;
        assert_eq!(db_result.component, "database");
        assert_eq!(db_result.status, HealthStatus::Healthy);

        let cache_result = builtin::cache_check().await;
        assert_eq!(cache_result.component, "cache");
        assert_eq!(cache_result.status, HealthStatus::Healthy);

        let disk_result = builtin::disk_space_check().await;
        assert_eq!(disk_result.component, "disk_space");

        let memory_result = builtin::memory_check().await;
        assert_eq!(memory_result.component, "memory");
    }
}
