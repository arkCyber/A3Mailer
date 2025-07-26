//! Authentication module

/// Auth manager
pub struct AuthManager;

/// Auth configuration
pub struct AuthConfig;

/// Authentication method
#[derive(Debug, Clone)]
pub enum AuthMethod {
    ApiKey,
    Bearer,
    Basic,
}
