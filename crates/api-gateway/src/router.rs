//! Request routing

/// Router
pub struct Router;

/// Route configuration
#[derive(Debug, Clone)]
pub struct Route {
    pub path: String,
    pub method: String,
}

/// Route configuration
pub struct RouteConfig;

impl Default for Route {
    fn default() -> Self {
        Self {
            path: "/api/*".to_string(),
            method: "GET".to_string(),
        }
    }
}
