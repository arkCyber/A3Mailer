//! Load balancer configuration

use serde::{Deserialize, Serialize};
use crate::backend::Backend;

/// Load balancer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    pub enabled: bool,
    pub algorithm: String,
    pub health_check_interval: u64,
    pub backends: Vec<Backend>,
    pub health_check: HealthCheckConfig,
    pub session_affinity: Option<SessionAffinityConfig>,
    pub server: ServerConfig,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub timeout_seconds: u64,
    pub path: String,
    pub expected_status: u16,
}

/// Session affinity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAffinityConfig {
    pub enabled: bool,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub listen_address: String,
    pub listen_port: u16,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: "round_robin".to_string(),
            health_check_interval: 30,
            backends: vec![],
            health_check: HealthCheckConfig {
                enabled: true,
                interval_seconds: 30,
                timeout_seconds: 5,
                path: "/health".to_string(),
                expected_status: 200,
            },
            session_affinity: None,
            server: ServerConfig {
                listen_address: "0.0.0.0".to_string(),
                listen_port: 8080,
            },
        }
    }
}
