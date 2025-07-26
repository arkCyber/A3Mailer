//! Custom Resource Definitions

use serde::{Deserialize, Serialize};

/// Stalwart Mail Server CRD
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StalwartMailServer {
    pub spec: StalwartMailServerSpec,
    pub status: Option<StalwartMailServerStatus>,
}

/// Stalwart Mail Server spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StalwartMailServerSpec {
    pub replicas: i32,
    pub image: String,
    pub version: String,
}

/// Stalwart Mail Server status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StalwartMailServerStatus {
    pub ready_replicas: i32,
    pub phase: String,
}
