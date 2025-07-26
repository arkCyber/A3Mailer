//! IPFS Client for A3Mailer
//!
//! This module provides comprehensive IPFS (InterPlanetary File System) integration
//! for decentralized file storage and retrieval.

use crate::{Web3Config, IpfsResult, Result, Web3Error};
use std::collections::HashMap;
use std::io::Read;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use chrono::{DateTime, Utc};

/// IPFS file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsFile {
    pub hash: String,
    pub name: String,
    pub size: u64,
    pub content_type: String,
    pub uploaded_at: DateTime<Utc>,
    pub pinned: bool,
    pub links: Vec<String>,
}

/// IPFS pin status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinStatus {
    pub hash: String,
    pub status: String, // "pinned", "pinning", "failed"
    pub pinned_at: Option<DateTime<Utc>>,
    pub pin_service: String,
}

/// IPFS upload options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadOptions {
    pub pin: bool,
    pub wrap_with_directory: bool,
    pub chunk_size: Option<u64>,
    pub hash_algorithm: String,
    pub metadata: HashMap<String, String>,
}

impl Default for UploadOptions {
    fn default() -> Self {
        Self {
            pin: true,
            wrap_with_directory: false,
            chunk_size: Some(1024 * 1024), // 1MB chunks
            hash_algorithm: "sha2-256".to_string(),
            metadata: HashMap::new(),
        }
    }
}

/// IPFS client for decentralized storage operations
pub struct IpfsClient {
    config: Web3Config,
    client: reqwest::Client,
    api_url: String,
    gateway_url: String,
    pinning_service: Option<PinningService>,
    file_cache: HashMap<String, IpfsFile>,
}

/// Pinning service configuration
#[derive(Debug, Clone)]
struct PinningService {
    name: String,
    api_key: String,
    endpoint: String,
}

impl IpfsClient {
    /// Create a new IPFS client
    pub async fn new(config: &Web3Config) -> Result<Self> {
        info!("Initializing IPFS client");
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60)) // Longer timeout for file operations
            .build()
            .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
        
        // Extract IPFS configuration
        let api_url = config.contract_addresses
            .get("ipfs_api")
            .cloned()
            .unwrap_or_else(|| "http://localhost:5001".to_string());
        
        let gateway_url = config.ipfs_gateway.clone();
        
        // Initialize pinning service if configured
        let pinning_service = if let Some(api_key) = config.contract_addresses.get("pinata_api_key") {
            Some(PinningService {
                name: "pinata".to_string(),
                api_key: api_key.clone(),
                endpoint: "https://api.pinata.cloud".to_string(),
            })
        } else {
            None
        };
        
        let mut client = Self {
            config: config.clone(),
            client,
            api_url,
            gateway_url,
            pinning_service,
            file_cache: HashMap::new(),
        };
        
        // Test IPFS connection
        client.test_connection().await?;
        
        info!("IPFS client initialized successfully");
        Ok(client)
    }

    /// Store data on IPFS
    pub async fn store_data(&self, data: &[u8]) -> Result<IpfsResult> {
        self.store_data_with_options(data, &UploadOptions::default()).await
    }

    /// Store data on IPFS with custom options
    pub async fn store_data_with_options(&self, data: &[u8], options: &UploadOptions) -> Result<IpfsResult> {
        debug!("Storing {} bytes on IPFS", data.len());
        
        // Prepare multipart form data
        let form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(data.to_vec())
                .file_name("data")
                .mime_str("application/octet-stream")
                .map_err(|e| Web3Error::IpfsError(e.to_string()))?);
        
        // Add IPFS API parameters
        let mut url = format!("{}/api/v0/add", self.api_url);
        let mut params = vec![
            ("pin", options.pin.to_string()),
            ("wrap-with-directory", options.wrap_with_directory.to_string()),
            ("hash", options.hash_algorithm.clone()),
        ];
        
        if let Some(chunk_size) = options.chunk_size {
            params.push(("chunker", format!("size-{}", chunk_size)));
        }
        
        // Build URL with parameters
        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&"));
        }
        
        // Send request to IPFS API
        let response = self.client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(Web3Error::IpfsError(format!(
                "IPFS upload failed with status: {}", 
                response.status()
            )));
        }
        
        let response_text = response.text().await
            .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
        
        // Parse IPFS response (can be multiple JSON objects)
        let mut hash = String::new();
        let mut size = 0u64;
        let mut links = Vec::new();
        
        for line in response_text.lines() {
            if let Ok(json_obj) = serde_json::from_str::<Value>(line) {
                if let Some(file_hash) = json_obj["Hash"].as_str() {
                    hash = file_hash.to_string();
                }
                if let Some(file_size) = json_obj["Size"].as_u64() {
                    size = file_size;
                }
                if let Some(name) = json_obj["Name"].as_str() {
                    if !name.is_empty() {
                        links.push(name.to_string());
                    }
                }
            }
        }
        
        if hash.is_empty() {
            return Err(Web3Error::IpfsError("No hash returned from IPFS".to_string()));
        }
        
        // Pin to external service if configured
        if options.pin && self.pinning_service.is_some() {
            if let Err(e) = self.pin_to_service(&hash).await {
                warn!("Failed to pin to external service: {}", e);
            }
        }
        
        let result = IpfsResult {
            hash: hash.clone(),
            size,
            links,
        };
        
        info!("Data stored on IPFS with hash: {} (size: {} bytes)", hash, size);
        Ok(result)
    }

    /// Retrieve data from IPFS
    pub async fn retrieve_data(&self, hash: &str) -> Result<Vec<u8>> {
        debug!("Retrieving data from IPFS: {}", hash);
        
        // Try gateway first for better performance
        let gateway_url = format!("{}/ipfs/{}", self.gateway_url, hash);
        
        let response = self.client
            .get(&gateway_url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await;
        
        match response {
            Ok(resp) if resp.status().is_success() => {
                let data = resp.bytes().await
                    .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
                
                info!("Retrieved {} bytes from IPFS gateway", data.len());
                return Ok(data.to_vec());
            }
            _ => {
                debug!("Gateway failed, trying local IPFS API");
            }
        }
        
        // Fallback to local IPFS API
        let api_url = format!("{}/api/v0/cat?arg={}", self.api_url, hash);
        
        let response = self.client
            .post(&api_url)
            .send()
            .await
            .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(Web3Error::IpfsError(format!(
                "IPFS retrieval failed with status: {}", 
                response.status()
            )));
        }
        
        let data = response.bytes().await
            .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
        
        info!("Retrieved {} bytes from IPFS API", data.len());
        Ok(data.to_vec())
    }

    /// Get file information from IPFS
    pub async fn get_file_info(&self, hash: &str) -> Result<IpfsFile> {
        debug!("Getting file info for: {}", hash);
        
        // Check cache first
        if let Some(cached_file) = self.file_cache.get(hash) {
            return Ok(cached_file.clone());
        }
        
        let url = format!("{}/api/v0/object/stat?arg={}", self.api_url, hash);
        
        let response = self.client
            .post(&url)
            .send()
            .await
            .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(Web3Error::IpfsError(format!(
                "IPFS stat failed with status: {}", 
                response.status()
            )));
        }
        
        let stat_data: Value = response.json().await
            .map_err(|e| Web3Error::SerializationError(e.to_string()))?;
        
        let file_info = IpfsFile {
            hash: hash.to_string(),
            name: hash.to_string(), // Use hash as name if no name available
            size: stat_data["CumulativeSize"].as_u64().unwrap_or(0),
            content_type: "application/octet-stream".to_string(), // Default content type
            uploaded_at: Utc::now(),
            pinned: self.is_pinned(hash).await.unwrap_or(false),
            links: self.get_file_links(hash).await.unwrap_or_default(),
        };
        
        info!("Retrieved file info for: {} (size: {} bytes)", hash, file_info.size);
        Ok(file_info)
    }

    /// Pin file to IPFS
    pub async fn pin_file(&self, hash: &str) -> Result<PinStatus> {
        debug!("Pinning file: {}", hash);
        
        let url = format!("{}/api/v0/pin/add?arg={}", self.api_url, hash);
        
        let response = self.client
            .post(&url)
            .send()
            .await
            .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(Web3Error::IpfsError(format!(
                "IPFS pin failed with status: {}", 
                response.status()
            )));
        }
        
        // Also pin to external service if configured
        if let Some(_) = &self.pinning_service {
            if let Err(e) = self.pin_to_service(hash).await {
                warn!("Failed to pin to external service: {}", e);
            }
        }
        
        let pin_status = PinStatus {
            hash: hash.to_string(),
            status: "pinned".to_string(),
            pinned_at: Some(Utc::now()),
            pin_service: "local".to_string(),
        };
        
        info!("File pinned successfully: {}", hash);
        Ok(pin_status)
    }

    /// Unpin file from IPFS
    pub async fn unpin_file(&self, hash: &str) -> Result<()> {
        debug!("Unpinning file: {}", hash);
        
        let url = format!("{}/api/v0/pin/rm?arg={}", self.api_url, hash);
        
        let response = self.client
            .post(&url)
            .send()
            .await
            .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(Web3Error::IpfsError(format!(
                "IPFS unpin failed with status: {}", 
                response.status()
            )));
        }
        
        info!("File unpinned successfully: {}", hash);
        Ok(())
    }

    /// List pinned files
    pub async fn list_pinned_files(&self) -> Result<Vec<String>> {
        debug!("Listing pinned files");
        
        let url = format!("{}/api/v0/pin/ls", self.api_url);
        
        let response = self.client
            .post(&url)
            .send()
            .await
            .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(Web3Error::IpfsError(format!(
                "IPFS pin list failed with status: {}", 
                response.status()
            )));
        }
        
        let pin_data: Value = response.json().await
            .map_err(|e| Web3Error::SerializationError(e.to_string()))?;
        
        let mut pinned_files = Vec::new();
        if let Some(keys) = pin_data["Keys"].as_object() {
            for (hash, _) in keys {
                pinned_files.push(hash.clone());
            }
        }
        
        info!("Found {} pinned files", pinned_files.len());
        Ok(pinned_files)
    }

    /// Test IPFS connection
    async fn test_connection(&self) -> Result<()> {
        debug!("Testing IPFS connection");
        
        let url = format!("{}/api/v0/version", self.api_url);
        
        let response = self.client
            .post(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| Web3Error::NetworkError(format!("IPFS connection test failed: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(Web3Error::IpfsError(format!(
                "IPFS not available, status: {}", 
                response.status()
            )));
        }
        
        let version_data: Value = response.json().await
            .map_err(|e| Web3Error::SerializationError(e.to_string()))?;
        
        let version = version_data["Version"].as_str().unwrap_or("unknown");
        info!("IPFS connection successful, version: {}", version);
        
        Ok(())
    }

    /// Check if file is pinned
    async fn is_pinned(&self, hash: &str) -> Result<bool> {
        let url = format!("{}/api/v0/pin/ls?arg={}", self.api_url, hash);
        
        let response = self.client
            .post(&url)
            .send()
            .await
            .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
        
        Ok(response.status().is_success())
    }

    /// Get file links
    async fn get_file_links(&self, hash: &str) -> Result<Vec<String>> {
        let url = format!("{}/api/v0/object/links?arg={}", self.api_url, hash);
        
        let response = self.client
            .post(&url)
            .send()
            .await
            .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Ok(Vec::new());
        }
        
        let links_data: Value = response.json().await
            .map_err(|e| Web3Error::SerializationError(e.to_string()))?;
        
        let mut links = Vec::new();
        if let Some(links_array) = links_data["Links"].as_array() {
            for link in links_array {
                if let Some(hash) = link["Hash"].as_str() {
                    links.push(hash.to_string());
                }
            }
        }
        
        Ok(links)
    }

    /// Pin to external pinning service
    async fn pin_to_service(&self, hash: &str) -> Result<()> {
        if let Some(service) = &self.pinning_service {
            debug!("Pinning {} to external service: {}", hash, service.name);
            
            let pin_data = json!({
                "hashToPin": hash,
                "pinataMetadata": {
                    "name": format!("a3mailer-{}", hash),
                    "keyvalues": {
                        "service": "a3mailer",
                        "timestamp": Utc::now().to_rfc3339()
                    }
                }
            });
            
            let response = self.client
                .post(&format!("{}/pinning/pinByHash", service.endpoint))
                .header("Authorization", format!("Bearer {}", service.api_key))
                .json(&pin_data)
                .send()
                .await
                .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
            
            if response.status().is_success() {
                info!("Successfully pinned {} to {}", hash, service.name);
            } else {
                warn!("Failed to pin {} to {}: {}", hash, service.name, response.status());
            }
        }
        
        Ok(())
    }

    /// Get IPFS client status
    pub async fn get_status(&self) -> Result<String> {
        let pinned_count = self.list_pinned_files().await.map(|files| files.len()).unwrap_or(0);
        let cache_size = self.file_cache.len();
        
        Ok(format!("active (pinned: {}, cached: {})", pinned_count, cache_size))
    }

    /// Shutdown IPFS client
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down IPFS client");
        // Cleanup operations would go here
        info!("IPFS client shutdown complete");
        Ok(())
    }
}
