//! # Stalwart API Gateway
//!
//! API gateway functionality for Stalwart Mail Server.
//! Provides a unified entry point for all API requests with features like
//! rate limiting, load balancing, authentication, caching, and request/response transformation.
//!
//! ## Features
//!
//! - **Rate Limiting**: Configurable rate limiting per client/endpoint
//! - **Load Balancing**: Multiple load balancing algorithms
//! - **Authentication**: JWT, OAuth2, API key authentication
//! - **Authorization**: Role-based access control
//! - **Caching**: Response caching for improved performance
//! - **Compression**: Request/response compression
//! - **SSL Termination**: TLS termination and certificate management
//! - **Request/Response Transformation**: Modify requests and responses
//! - **Circuit Breaker**: Fault tolerance and resilience
//!
//! ## Architecture
//!
//! The API gateway consists of:
//! - Gateway Manager: Main gateway orchestrator
//! - Router: Request routing and load balancing
//! - Auth Manager: Authentication and authorization
//! - Rate Limiter: Request rate limiting
//! - Cache Manager: Response caching
//! - Transform Manager: Request/response transformation
//!
//! ## Example
//!
//! ```rust,no_run
//! use stalwart_api_gateway::{ApiGateway, GatewayConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = GatewayConfig::default();
//!     let gateway = ApiGateway::new(config).await?;
//!
//!     // Start API gateway
//!     gateway.start().await?;
//!
//!     Ok(())
//! }
//! ```

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

pub mod config;
pub mod gateway;
pub mod router;
pub mod auth;
pub mod rate_limiter;
pub mod cache;
pub mod transform;
pub mod load_balancer;
pub mod circuit_breaker;
pub mod middleware;
pub mod metrics;
pub mod error;

pub use config::GatewayConfig;
pub use gateway::ApiGateway;
pub use router::{Router, Route, RouteConfig};
pub use auth::{AuthManager, AuthConfig, AuthMethod};
pub use rate_limiter::{RateLimiter, RateLimitConfig, RateLimitRule};
pub use cache::{CacheManager, CacheConfig, CachePolicy};
pub use transform::{TransformManager, RequestTransform, ResponseTransform};
pub use load_balancer::{LoadBalancer, LoadBalancingAlgorithm, Backend};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use error::{GatewayError, Result};

/// Gateway status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayStatus {
    /// Gateway is starting
    Starting,
    /// Gateway is running
    Running,
    /// Gateway is stopping
    Stopping,
    /// Gateway is stopped
    Stopped,
    /// Gateway has failed
    Failed,
}

/// Request context
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Request ID
    pub request_id: uuid::Uuid,
    /// Client IP address
    pub client_ip: std::net::IpAddr,
    /// User agent
    pub user_agent: Option<String>,
    /// Authentication info
    pub auth_info: Option<AuthInfo>,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Authentication information
#[derive(Debug, Clone)]
pub struct AuthInfo {
    /// User ID
    pub user_id: String,
    /// User roles
    pub roles: Vec<String>,
    /// Authentication method used
    pub auth_method: AuthMethod,
    /// Token expiration
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Additional claims
    pub claims: HashMap<String, serde_json::Value>,
}

/// Backend service configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackendService {
    /// Service name
    pub name: String,
    /// Service URL
    pub url: String,
    /// Health check endpoint
    pub health_check: Option<String>,
    /// Service weight for load balancing
    pub weight: u32,
    /// Service timeout
    pub timeout: std::time::Duration,
    /// Service metadata
    pub metadata: HashMap<String, String>,
}

/// Route matching criteria
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RouteMatcher {
    /// Path pattern
    pub path: String,
    /// HTTP method
    pub method: Option<String>,
    /// Host header
    pub host: Option<String>,
    /// Custom headers
    pub headers: HashMap<String, String>,
    /// Query parameters
    pub query_params: HashMap<String, String>,
}

/// Main API gateway context
pub struct GatewayContext {
    pub config: GatewayConfig,
    pub status: Arc<RwLock<GatewayStatus>>,
    pub routes: Arc<RwLock<Vec<Route>>>,
    pub backends: Arc<RwLock<Vec<BackendService>>>,
}

impl GatewayContext {
    /// Create a new gateway context
    pub fn new(config: GatewayConfig) -> Self {
        Self {
            config,
            status: Arc::new(RwLock::new(GatewayStatus::Starting)),
            routes: Arc::new(RwLock::new(Vec::new())),
            backends: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get current gateway status
    pub async fn status(&self) -> GatewayStatus {
        self.status.read().await.clone()
    }

    /// Set gateway status
    pub async fn set_status(&self, status: GatewayStatus) {
        let mut current_status = self.status.write().await;
        if *current_status != status {
            info!("Gateway status changed: {:?} -> {:?}", *current_status, status);
            *current_status = status;
        }
    }

    /// Add a route
    pub async fn add_route(&self, route: Route) {
        let mut routes = self.routes.write().await;
        routes.push(route);
    }

    /// Add a backend service
    pub async fn add_backend(&self, backend: BackendService) {
        let mut backends = self.backends.write().await;
        backends.push(backend);
    }

    /// Get all routes
    pub async fn routes(&self) -> Vec<Route> {
        self.routes.read().await.clone()
    }

    /// Get all backends
    pub async fn backends(&self) -> Vec<BackendService> {
        self.backends.read().await.clone()
    }
}

/// Initialize the API gateway
pub async fn init_api_gateway(config: GatewayConfig) -> Result<GatewayContext> {
    info!("Initializing API gateway");

    let context = GatewayContext::new(config);

    // TODO: Initialize gateway components
    // - Set up router
    // - Initialize authentication
    // - Configure rate limiting
    // - Set up caching
    // - Initialize load balancer

    context.set_status(GatewayStatus::Running).await;

    info!("API gateway initialized successfully");
    Ok(context)
}

impl Default for BackendService {
    fn default() -> Self {
        Self {
            name: "stalwart-backend".to_string(),
            url: "http://localhost:8080".to_string(),
            health_check: Some("/health".to_string()),
            weight: 100,
            timeout: std::time::Duration::from_secs(30),
            metadata: HashMap::new(),
        }
    }
}

impl Default for RouteMatcher {
    fn default() -> Self {
        Self {
            path: "/api/*".to_string(),
            method: None,
            host: None,
            headers: HashMap::new(),
            query_params: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gateway_context_creation() {
        let config = GatewayConfig::default();
        let context = GatewayContext::new(config);

        assert_eq!(context.status().await, GatewayStatus::Starting);
    }

    #[tokio::test]
    async fn test_route_addition() {
        let config = GatewayConfig::default();
        let context = GatewayContext::new(config);

        let route = Route::default();
        context.add_route(route).await;

        let routes = context.routes().await;
        assert_eq!(routes.len(), 1);
    }

    #[tokio::test]
    async fn test_backend_addition() {
        let config = GatewayConfig::default();
        let context = GatewayContext::new(config);

        let backend = BackendService::default();
        context.add_backend(backend).await;

        let backends = context.backends().await;
        assert_eq!(backends.len(), 1);
        assert_eq!(backends[0].name, "stalwart-backend");
    }

    #[test]
    fn test_default_backend_service() {
        let backend = BackendService::default();
        assert_eq!(backend.name, "stalwart-backend");
        assert_eq!(backend.url, "http://localhost:8080");
        assert_eq!(backend.weight, 100);
    }
}
