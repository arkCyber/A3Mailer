//! Decentralized Identity (DID) management for A3Mailer
//!
//! This module provides DID resolution, verification, and management
//! capabilities for Web3-native user authentication.

use crate::{Web3Config, DidDocument, PublicKey, ServiceEndpoint, Result, Web3Error};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use tracing::{info, warn, error, debug};
use serde_json::Value;

/// DID Manager for handling decentralized identities
pub struct DidManager {
    config: Web3Config,
    resolver_client: reqwest::Client,
    cache: HashMap<String, (DidDocument, DateTime<Utc>)>,
}

impl DidManager {
    /// Create a new DID manager
    pub async fn new(config: &Web3Config) -> Result<Self> {
        info!("Initializing DID manager");
        
        let resolver_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
        
        Ok(Self {
            config: config.clone(),
            resolver_client,
            cache: HashMap::new(),
        })
    }

    /// Verify a DID exists and is valid
    pub async fn verify_did(&self, did: &str) -> Result<bool> {
        debug!("Verifying DID: {}", did);
        
        // Basic DID format validation
        if !self.is_valid_did_format(did) {
            return Ok(false);
        }
        
        // Try to resolve the DID
        match self.resolve_did(did).await {
            Ok(_) => {
                info!("DID verification successful: {}", did);
                Ok(true)
            }
            Err(e) => {
                warn!("DID verification failed for {}: {}", did, e);
                Ok(false)
            }
        }
    }

    /// Resolve a DID to get its document
    pub async fn resolve_did(&self, did: &str) -> Result<DidDocument> {
        debug!("Resolving DID: {}", did);
        
        // Check cache first
        if let Some((document, cached_at)) = self.cache.get(did) {
            let cache_age = Utc::now().signed_duration_since(*cached_at);
            if cache_age.num_minutes() < 60 { // Cache for 1 hour
                debug!("Returning cached DID document for: {}", did);
                return Ok(document.clone());
            }
        }
        
        // Resolve from network
        let document = self.resolve_did_from_network(did).await?;
        
        // Cache the result
        let mut cache = &mut self.cache;
        cache.insert(did.to_string(), (document.clone(), Utc::now()));
        
        info!("Successfully resolved DID: {}", did);
        Ok(document)
    }

    /// Resolve DID from network
    async fn resolve_did_from_network(&self, did: &str) -> Result<DidDocument> {
        let resolver_url = format!("{}/1.0/identifiers/{}", self.config.did_resolver_url, did);
        
        debug!("Resolving DID from: {}", resolver_url);
        
        let response = self.resolver_client
            .get(&resolver_url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(Web3Error::DidError(format!(
                "DID resolution failed with status: {}", 
                response.status()
            )));
        }
        
        let response_text = response.text().await
            .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
        
        let response_json: Value = serde_json::from_str(&response_text)
            .map_err(|e| Web3Error::SerializationError(e.to_string()))?;
        
        // Parse the DID document from the response
        self.parse_did_document(&response_json, did)
    }

    /// Parse DID document from JSON response
    fn parse_did_document(&self, json: &Value, did: &str) -> Result<DidDocument> {
        let did_document = json.get("didDocument")
            .or_else(|| json.get("document"))
            .ok_or_else(|| Web3Error::DidError("No DID document found in response".to_string()))?;
        
        // Extract public keys
        let public_keys = self.extract_public_keys(did_document)?;
        
        // Extract authentication methods
        let authentication = self.extract_authentication(did_document)?;
        
        // Extract service endpoints
        let service_endpoints = self.extract_service_endpoints(did_document)?;
        
        Ok(DidDocument {
            id: did.to_string(),
            public_keys,
            authentication,
            service_endpoints,
            created: Utc::now(), // TODO: Extract from document if available
            updated: Utc::now(), // TODO: Extract from document if available
        })
    }

    /// Extract public keys from DID document
    fn extract_public_keys(&self, document: &Value) -> Result<Vec<PublicKey>> {
        let mut public_keys = Vec::new();
        
        if let Some(keys) = document.get("publicKey").and_then(|k| k.as_array()) {
            for key in keys {
                if let (Some(id), Some(key_type), Some(controller)) = (
                    key.get("id").and_then(|v| v.as_str()),
                    key.get("type").and_then(|v| v.as_str()),
                    key.get("controller").and_then(|v| v.as_str()),
                ) {
                    let public_key_hex = key.get("publicKeyHex")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    
                    public_keys.push(PublicKey {
                        id: id.to_string(),
                        key_type: key_type.to_string(),
                        controller: controller.to_string(),
                        public_key_hex,
                    });
                }
            }
        }
        
        Ok(public_keys)
    }

    /// Extract authentication methods from DID document
    fn extract_authentication(&self, document: &Value) -> Result<Vec<String>> {
        let mut authentication = Vec::new();
        
        if let Some(auth) = document.get("authentication").and_then(|a| a.as_array()) {
            for auth_method in auth {
                if let Some(auth_str) = auth_method.as_str() {
                    authentication.push(auth_str.to_string());
                } else if let Some(auth_obj) = auth_method.as_object() {
                    if let Some(id) = auth_obj.get("id").and_then(|v| v.as_str()) {
                        authentication.push(id.to_string());
                    }
                }
            }
        }
        
        Ok(authentication)
    }

    /// Extract service endpoints from DID document
    fn extract_service_endpoints(&self, document: &Value) -> Result<Vec<ServiceEndpoint>> {
        let mut service_endpoints = Vec::new();
        
        if let Some(services) = document.get("service").and_then(|s| s.as_array()) {
            for service in services {
                if let (Some(id), Some(service_type), Some(endpoint)) = (
                    service.get("id").and_then(|v| v.as_str()),
                    service.get("type").and_then(|v| v.as_str()),
                    service.get("serviceEndpoint").and_then(|v| v.as_str()),
                ) {
                    service_endpoints.push(ServiceEndpoint {
                        id: id.to_string(),
                        service_type: service_type.to_string(),
                        service_endpoint: endpoint.to_string(),
                    });
                }
            }
        }
        
        Ok(service_endpoints)
    }

    /// Validate DID format
    fn is_valid_did_format(&self, did: &str) -> bool {
        // Basic DID format: did:method:method-specific-id
        let parts: Vec<&str> = did.split(':').collect();
        
        if parts.len() < 3 {
            return false;
        }
        
        if parts[0] != "did" {
            return false;
        }
        
        // Check for supported DID methods
        let supported_methods = ["ethr", "key", "web", "ion"];
        if !supported_methods.contains(&parts[1]) {
            warn!("Unsupported DID method: {}", parts[1]);
            return false;
        }
        
        // Method-specific validation
        match parts[1] {
            "ethr" => self.validate_ethr_did(&parts[2..]),
            "key" => self.validate_key_did(&parts[2..]),
            "web" => self.validate_web_did(&parts[2..]),
            "ion" => self.validate_ion_did(&parts[2..]),
            _ => false,
        }
    }

    /// Validate Ethereum DID format
    fn validate_ethr_did(&self, parts: &[&str]) -> bool {
        if parts.is_empty() {
            return false;
        }
        
        let identifier = parts[0];
        
        // Should be a valid Ethereum address (0x followed by 40 hex characters)
        if identifier.len() == 42 && identifier.starts_with("0x") {
            identifier[2..].chars().all(|c| c.is_ascii_hexdigit())
        } else {
            false
        }
    }

    /// Validate key DID format
    fn validate_key_did(&self, parts: &[&str]) -> bool {
        if parts.is_empty() {
            return false;
        }
        
        // Key DIDs should have a base58-encoded public key
        let identifier = parts[0];
        identifier.len() > 10 && identifier.chars().all(|c| {
            c.is_ascii_alphanumeric() && !"0OIl".contains(c)
        })
    }

    /// Validate web DID format
    fn validate_web_did(&self, parts: &[&str]) -> bool {
        if parts.is_empty() {
            return false;
        }
        
        // Web DIDs should have a valid domain name
        let domain = parts[0];
        domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
    }

    /// Validate ION DID format
    fn validate_ion_did(&self, parts: &[&str]) -> bool {
        if parts.is_empty() {
            return false;
        }
        
        // ION DIDs have a specific format with base64url encoding
        let identifier = parts[0];
        identifier.len() > 20 && identifier.chars().all(|c| {
            c.is_ascii_alphanumeric() || c == '-' || c == '_'
        })
    }

    /// Get DID manager status
    pub async fn get_status(&self) -> Result<String> {
        let cache_size = self.cache.len();
        Ok(format!("active (cached DIDs: {})", cache_size))
    }

    /// Clear DID cache
    pub async fn clear_cache(&mut self) {
        self.cache.clear();
        info!("DID cache cleared");
    }

    /// Get cached DID count
    pub fn get_cache_size(&self) -> usize {
        self.cache.len()
    }
}
