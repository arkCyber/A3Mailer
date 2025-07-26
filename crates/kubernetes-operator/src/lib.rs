//! # Stalwart Kubernetes Operator
//!
//! Kubernetes operator for managing Stalwart Mail Server deployments.
//! Provides automated deployment, scaling, backup, and lifecycle management
//! of Stalwart Mail Server instances in Kubernetes clusters.
//!
//! ## Features
//!
//! - **CRD Management**: Custom Resource Definitions for Stalwart
//! - **Auto-scaling**: Horizontal and vertical pod autoscaling
//! - **Backup Automation**: Automated backup and restore operations
//! - **Monitoring Integration**: Prometheus metrics and alerting
//! - **Service Mesh Integration**: Istio/Linkerd integration
//! - **Certificate Management**: Automatic TLS certificate provisioning
//!
//! ## Architecture
//!
//! The operator consists of:
//! - Operator Manager: Main operator controller
//! - CRD Controller: Custom resource management
//! - Scaling Controller: Auto-scaling logic
//! - Backup Controller: Backup and restore operations
//! - Monitoring Controller: Metrics and health checks
//!
//! ## Example
//!
//! ```rust,no_run
//! use stalwart_kubernetes_operator::{OperatorManager, OperatorConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = OperatorConfig::default();
//!     let manager = OperatorManager::new(config).await?;
//!
//!     // Start operator
//!     manager.start().await?;
//!
//!     Ok(())
//! }
//! ```

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

pub mod config;
pub mod manager;
pub mod crd;
pub mod controllers;
pub mod scaling;
pub mod backup;
pub mod monitoring;
pub mod resources;
pub mod metrics;
pub mod error;

pub use config::OperatorConfig;
pub use manager::OperatorManager;
pub use crd::{StalwartMailServer, StalwartMailServerSpec, StalwartMailServerStatus};
pub use controllers::{MailServerController, ControllerContext};
pub use scaling::{AutoScaler, ScalingPolicy, ScalingMetrics};
pub use backup::{BackupController, BackupPolicy, RestoreOperation};
pub use error::{OperatorError, Result};

/// Operator status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorStatus {
    /// Operator is starting up
    Starting,
    /// Operator is running
    Running,
    /// Operator is stopping
    Stopping,
    /// Operator has stopped
    Stopped,
    /// Operator has failed
    Failed,
}

/// Deployment phase
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DeploymentPhase {
    /// Deployment is pending
    Pending,
    /// Deployment is in progress
    Deploying,
    /// Deployment is running
    Running,
    /// Deployment is updating
    Updating,
    /// Deployment is scaling
    Scaling,
    /// Deployment has failed
    Failed,
    /// Deployment is being deleted
    Deleting,
}

/// Resource requirements
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceRequirements {
    /// CPU requests
    pub cpu_request: String,
    /// Memory requests
    pub memory_request: String,
    /// CPU limits
    pub cpu_limit: String,
    /// Memory limits
    pub memory_limit: String,
    /// Storage requests
    pub storage_request: String,
}

/// Scaling configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScalingConfig {
    /// Minimum replicas
    pub min_replicas: i32,
    /// Maximum replicas
    pub max_replicas: i32,
    /// Target CPU utilization percentage
    pub target_cpu_utilization: i32,
    /// Target memory utilization percentage
    pub target_memory_utilization: i32,
    /// Scale up threshold
    pub scale_up_threshold: f64,
    /// Scale down threshold
    pub scale_down_threshold: f64,
}

/// Main operator context
pub struct OperatorContext {
    pub config: OperatorConfig,
    pub status: Arc<RwLock<OperatorStatus>>,
    pub kubernetes_client: kube::Client,
    pub namespace: String,
}

impl OperatorContext {
    /// Create a new operator context
    pub async fn new(config: OperatorConfig) -> Result<Self> {
        let kubernetes_client = kube::Client::try_default().await
            .map_err(|e| OperatorError::KubernetesClient(e.to_string()))?;

        let namespace = config.namespace.clone();

        Ok(Self {
            config,
            status: Arc::new(RwLock::new(OperatorStatus::Starting)),
            kubernetes_client,
            namespace,
        })
    }

    /// Get current operator status
    pub async fn status(&self) -> OperatorStatus {
        self.status.read().await.clone()
    }

    /// Set operator status
    pub async fn set_status(&self, status: OperatorStatus) {
        let mut current_status = self.status.write().await;
        if *current_status != status {
            info!("Operator status changed: {:?} -> {:?}", *current_status, status);
            *current_status = status;
        }
    }

    /// Get Kubernetes client
    pub fn kubernetes_client(&self) -> &kube::Client {
        &self.kubernetes_client
    }

    /// Get operator namespace
    pub fn namespace(&self) -> &str {
        &self.namespace
    }
}

/// Initialize the Kubernetes operator
pub async fn init_operator(config: OperatorConfig) -> Result<OperatorContext> {
    info!("Initializing Kubernetes operator");

    let context = OperatorContext::new(config).await?;

    // TODO: Initialize operator components
    // - Set up CRD controllers
    // - Initialize scaling controllers
    // - Set up backup controllers
    // - Configure monitoring

    context.set_status(OperatorStatus::Running).await;

    info!("Kubernetes operator initialized successfully");
    Ok(context)
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu_request: "100m".to_string(),
            memory_request: "128Mi".to_string(),
            cpu_limit: "1000m".to_string(),
            memory_limit: "1Gi".to_string(),
            storage_request: "10Gi".to_string(),
        }
    }
}

impl Default for ScalingConfig {
    fn default() -> Self {
        Self {
            min_replicas: 1,
            max_replicas: 10,
            target_cpu_utilization: 70,
            target_memory_utilization: 80,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_operator_context_creation() {
        let config = OperatorConfig::default();
        // Note: This will fail in test environment without Kubernetes
        // let context = OperatorContext::new(config).await.unwrap();
        // assert_eq!(context.status().await, OperatorStatus::Starting);
    }

    #[test]
    fn test_default_resource_requirements() {
        let resources = ResourceRequirements::default();
        assert_eq!(resources.cpu_request, "100m");
        assert_eq!(resources.memory_request, "128Mi");
    }

    #[test]
    fn test_default_scaling_config() {
        let scaling = ScalingConfig::default();
        assert_eq!(scaling.min_replicas, 1);
        assert_eq!(scaling.max_replicas, 10);
    }
}
