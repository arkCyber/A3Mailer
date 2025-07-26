//! Web3 integration error types and handling

use std::fmt;

/// Web3 integration result type
pub type Result<T> = std::result::Result<T, Web3Error>;

/// Web3 integration errors
#[derive(Debug)]
pub enum Web3Error {
    /// DID-related errors
    DidError(String),
    
    /// Smart contract errors
    ContractError(String),
    
    /// IPFS-related errors
    IpfsError(String),
    
    /// Blockchain communication errors
    BlockchainError(String),
    
    /// Network connectivity errors
    NetworkError(String),
    
    /// Authentication/authorization errors
    AuthError(String),
    
    /// Configuration errors
    ConfigError(String),
    
    /// Serialization/deserialization errors
    SerializationError(String),
    
    /// Timeout errors
    TimeoutError(String),
    
    /// Generic errors
    Other(String),
}

impl fmt::Display for Web3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Web3Error::DidError(msg) => write!(f, "DID error: {}", msg),
            Web3Error::ContractError(msg) => write!(f, "Smart contract error: {}", msg),
            Web3Error::IpfsError(msg) => write!(f, "IPFS error: {}", msg),
            Web3Error::BlockchainError(msg) => write!(f, "Blockchain error: {}", msg),
            Web3Error::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Web3Error::AuthError(msg) => write!(f, "Authentication error: {}", msg),
            Web3Error::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            Web3Error::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Web3Error::TimeoutError(msg) => write!(f, "Timeout error: {}", msg),
            Web3Error::Other(msg) => write!(f, "Web3 error: {}", msg),
        }
    }
}

impl std::error::Error for Web3Error {}

impl From<serde_json::Error> for Web3Error {
    fn from(err: serde_json::Error) -> Self {
        Web3Error::SerializationError(err.to_string())
    }
}

impl From<reqwest::Error> for Web3Error {
    fn from(err: reqwest::Error) -> Self {
        Web3Error::NetworkError(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for Web3Error {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        Web3Error::TimeoutError(err.to_string())
    }
}

impl From<std::io::Error> for Web3Error {
    fn from(err: std::io::Error) -> Self {
        Web3Error::Other(err.to_string())
    }
}
