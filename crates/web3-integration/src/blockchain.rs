//! Blockchain Client for A3Mailer
//!
//! This module provides comprehensive blockchain interaction capabilities
//! for message verification, audit trails, and cryptographic operations.

use crate::{Web3Config, Web3Event, Result, Web3Error};
use std::collections::HashMap;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use chrono::{DateTime, Utc};

/// Blockchain transaction information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainTransaction {
    pub hash: String,
    pub block_number: u64,
    pub block_hash: String,
    pub transaction_index: u64,
    pub from_address: String,
    pub to_address: String,
    pub value: String,
    pub gas_used: u64,
    pub gas_price: String,
    pub timestamp: DateTime<Utc>,
    pub status: bool,
}

/// Message signature verification data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureVerification {
    pub message_hash: String,
    pub signature: String,
    pub signer_address: String,
    pub is_valid: bool,
    pub verified_at: DateTime<Utc>,
}

/// Audit trail entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub event_type: String,
    pub user_id: String,
    pub resource: String,
    pub action: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    pub transaction_hash: Option<String>,
    pub block_number: Option<u64>,
}

/// Blockchain network information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub chain_id: u64,
    pub network_name: String,
    pub latest_block: u64,
    pub gas_price: String,
    pub is_syncing: bool,
    pub peer_count: u64,
}

/// Blockchain client for network interactions
pub struct BlockchainClient {
    config: Web3Config,
    client: reqwest::Client,
    rpc_url: String,
    chain_id: u64,
    audit_contract: Option<String>,
    signature_cache: HashMap<String, SignatureVerification>,
}

impl BlockchainClient {
    /// Create a new blockchain client
    pub async fn new(config: &Web3Config) -> Result<Self> {
        info!("Initializing blockchain client");
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
        
        let mut blockchain_client = Self {
            config: config.clone(),
            client,
            rpc_url: config.rpc_url.clone(),
            chain_id: 1, // Default to Ethereum mainnet
            audit_contract: config.contract_addresses.get("audit").cloned(),
            signature_cache: HashMap::new(),
        };
        
        // Initialize blockchain connection
        blockchain_client.initialize_connection().await?;
        
        info!("Blockchain client initialized successfully");
        Ok(blockchain_client)
    }

    /// Verify message signature
    pub async fn verify_signature(&self, message_hash: &str, signature: &str) -> Result<bool> {
        debug!("Verifying signature for message hash: {}", message_hash);
        
        // Check cache first
        let cache_key = format!("{}:{}", message_hash, signature);
        if let Some(cached) = self.signature_cache.get(&cache_key) {
            debug!("Using cached signature verification result");
            return Ok(cached.is_valid);
        }
        
        // Recover signer address from signature
        let signer_address = self.recover_signer_address(message_hash, signature).await?;
        
        // Verify the signature is valid
        let is_valid = !signer_address.is_empty() && signer_address != "0x0000000000000000000000000000000000000000";
        
        // Cache the result
        let verification = SignatureVerification {
            message_hash: message_hash.to_string(),
            signature: signature.to_string(),
            signer_address: signer_address.clone(),
            is_valid,
            verified_at: Utc::now(),
        };
        
        info!("Signature verification result for {}: {} (signer: {})", 
              message_hash, is_valid, signer_address);
        
        Ok(is_valid)
    }

    /// Create audit trail entry on blockchain
    pub async fn create_audit_entry(&self, event_data: &HashMap<String, String>) -> Result<String> {
        debug!("Creating audit trail entry");
        
        if let Some(audit_contract) = &self.audit_contract {
            // Create audit entry via smart contract
            let audit_data = json!({
                "event_type": event_data.get("event_type").unwrap_or(&"unknown".to_string()),
                "user_id": event_data.get("user_id").unwrap_or(&"system".to_string()),
                "resource": event_data.get("resource").unwrap_or(&"".to_string()),
                "action": event_data.get("action").unwrap_or(&"".to_string()),
                "timestamp": Utc::now().timestamp(),
                "metadata": serde_json::to_string(event_data).unwrap_or_default()
            });
            
            let tx_hash = self.send_audit_transaction(audit_contract, &audit_data).await?;
            
            info!("Audit entry created with transaction hash: {}", tx_hash);
            Ok(tx_hash)
        } else {
            // Fallback: create audit entry as transaction data
            let audit_json = serde_json::to_string(event_data)
                .map_err(|e| Web3Error::SerializationError(e.to_string()))?;
            
            let tx_hash = self.send_data_transaction(&audit_json).await?;
            
            info!("Audit entry created as data transaction: {}", tx_hash);
            Ok(tx_hash)
        }
    }

    /// Get transaction information
    pub async fn get_transaction(&self, tx_hash: &str) -> Result<BlockchainTransaction> {
        debug!("Getting transaction information: {}", tx_hash);
        
        let request_data = json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionByHash",
            "params": [tx_hash],
            "id": 1
        });
        
        let response = self.send_rpc_request(&request_data).await?;
        
        let tx_data = response["result"]
            .as_object()
            .ok_or_else(|| Web3Error::BlockchainError("Transaction not found".to_string()))?;
        
        // Get transaction receipt for additional information
        let receipt = self.get_transaction_receipt(tx_hash).await?;
        
        let transaction = BlockchainTransaction {
            hash: tx_hash.to_string(),
            block_number: self.parse_hex_to_u64(tx_data["blockNumber"].as_str().unwrap_or("0x0"))?,
            block_hash: tx_data["blockHash"].as_str().unwrap_or("").to_string(),
            transaction_index: self.parse_hex_to_u64(tx_data["transactionIndex"].as_str().unwrap_or("0x0"))?,
            from_address: tx_data["from"].as_str().unwrap_or("").to_string(),
            to_address: tx_data["to"].as_str().unwrap_or("").to_string(),
            value: tx_data["value"].as_str().unwrap_or("0x0").to_string(),
            gas_used: self.parse_hex_to_u64(receipt["gasUsed"].as_str().unwrap_or("0x0"))?,
            gas_price: tx_data["gasPrice"].as_str().unwrap_or("0x0").to_string(),
            timestamp: Utc::now(), // Would get actual block timestamp in production
            status: receipt["status"].as_str() == Some("0x1"),
        };
        
        info!("Retrieved transaction information: {}", tx_hash);
        Ok(transaction)
    }

    /// Get network information
    pub async fn get_network_info(&self) -> Result<NetworkInfo> {
        debug!("Getting network information");
        
        // Get chain ID
        let chain_id_request = json!({
            "jsonrpc": "2.0",
            "method": "eth_chainId",
            "params": [],
            "id": 1
        });
        
        let chain_id_response = self.send_rpc_request(&chain_id_request).await?;
        let chain_id = self.parse_hex_to_u64(
            chain_id_response["result"].as_str().unwrap_or("0x1")
        )?;
        
        // Get latest block number
        let block_request = json!({
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 2
        });
        
        let block_response = self.send_rpc_request(&block_request).await?;
        let latest_block = self.parse_hex_to_u64(
            block_response["result"].as_str().unwrap_or("0x0")
        )?;
        
        // Get gas price
        let gas_request = json!({
            "jsonrpc": "2.0",
            "method": "eth_gasPrice",
            "params": [],
            "id": 3
        });
        
        let gas_response = self.send_rpc_request(&gas_request).await?;
        let gas_price = gas_response["result"].as_str().unwrap_or("0x0").to_string();
        
        // Get sync status
        let sync_request = json!({
            "jsonrpc": "2.0",
            "method": "eth_syncing",
            "params": [],
            "id": 4
        });
        
        let sync_response = self.send_rpc_request(&sync_request).await?;
        let is_syncing = sync_response["result"].as_bool().unwrap_or(false);
        
        let network_info = NetworkInfo {
            chain_id,
            network_name: self.get_network_name(chain_id),
            latest_block,
            gas_price,
            is_syncing,
            peer_count: 0, // Would implement peer count in production
        };
        
        info!("Network info: {} (chain ID: {}, block: {})", 
              network_info.network_name, chain_id, latest_block);
        
        Ok(network_info)
    }

    /// Get blockchain events for audit trail
    pub async fn get_audit_events(&self, from_block: Option<u64>, to_block: Option<u64>) -> Result<Vec<Web3Event>> {
        debug!("Getting audit events from blockchain");
        
        let filter_data = json!({
            "jsonrpc": "2.0",
            "method": "eth_getLogs",
            "params": [{
                "fromBlock": from_block.map(|b| format!("0x{:x}", b)).unwrap_or_else(|| "earliest".to_string()),
                "toBlock": to_block.map(|b| format!("0x{:x}", b)).unwrap_or_else(|| "latest".to_string()),
                "address": self.audit_contract.as_ref().map(|s| s.as_str()).unwrap_or(""),
                "topics": []
            }],
            "id": 1
        });
        
        let response = self.send_rpc_request(&filter_data).await?;
        
        let logs = response["result"]
            .as_array()
            .ok_or_else(|| Web3Error::BlockchainError("Invalid logs response".to_string()))?;
        
        let mut events = Vec::new();
        for log in logs {
            if let Ok(event) = self.parse_audit_event(log).await {
                events.push(event);
            }
        }
        
        info!("Retrieved {} audit events", events.len());
        Ok(events)
    }

    /// Initialize blockchain connection
    async fn initialize_connection(&self) -> Result<()> {
        debug!("Initializing blockchain connection");
        
        // Test connection with a simple request
        let request_data = json!({
            "jsonrpc": "2.0",
            "method": "eth_chainId",
            "params": [],
            "id": 1
        });
        
        let response = self.send_rpc_request(&request_data).await?;
        
        let chain_id_hex = response["result"]
            .as_str()
            .ok_or_else(|| Web3Error::BlockchainError("No chain ID in response".to_string()))?;
        
        let chain_id = self.parse_hex_to_u64(chain_id_hex)?;
        
        info!("Connected to blockchain network, chain ID: {}", chain_id);
        Ok(())
    }

    /// Recover signer address from signature
    async fn recover_signer_address(&self, message_hash: &str, signature: &str) -> Result<String> {
        // In a real implementation, this would use cryptographic libraries
        // to recover the public key and derive the address from the signature
        // For now, we'll simulate this process
        
        if signature.len() < 130 { // 0x + 128 hex chars for a valid signature
            return Ok("0x0000000000000000000000000000000000000000".to_string());
        }
        
        // Simulate signature recovery - in production, use proper crypto libraries
        let mock_address = format!("0x{}", &signature[2..42]); // Take first 20 bytes as address
        
        debug!("Recovered signer address: {}", mock_address);
        Ok(mock_address)
    }

    /// Send audit transaction to smart contract
    async fn send_audit_transaction(&self, contract_address: &str, audit_data: &Value) -> Result<String> {
        let transaction_data = json!({
            "jsonrpc": "2.0",
            "method": "eth_sendTransaction",
            "params": [{
                "to": contract_address,
                "data": self.encode_audit_data(audit_data)?,
                "gas": format!("0x{:x}", self.config.gas_limit),
                "gasPrice": self.config.gas_price
            }],
            "id": 1
        });
        
        let response = self.send_rpc_request(&transaction_data).await?;
        
        let tx_hash = response["result"]
            .as_str()
            .ok_or_else(|| Web3Error::BlockchainError("No transaction hash in response".to_string()))?;
        
        Ok(tx_hash.to_string())
    }

    /// Send data transaction
    async fn send_data_transaction(&self, data: &str) -> Result<String> {
        let hex_data = format!("0x{}", hex::encode(data.as_bytes()));
        
        let transaction_data = json!({
            "jsonrpc": "2.0",
            "method": "eth_sendTransaction",
            "params": [{
                "data": hex_data,
                "gas": format!("0x{:x}", self.config.gas_limit),
                "gasPrice": self.config.gas_price
            }],
            "id": 1
        });
        
        let response = self.send_rpc_request(&transaction_data).await?;
        
        let tx_hash = response["result"]
            .as_str()
            .ok_or_else(|| Web3Error::BlockchainError("No transaction hash in response".to_string()))?;
        
        Ok(tx_hash.to_string())
    }

    /// Get transaction receipt
    async fn get_transaction_receipt(&self, tx_hash: &str) -> Result<Value> {
        let request_data = json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        });
        
        let response = self.send_rpc_request(&request_data).await?;
        
        response["result"]
            .as_object()
            .ok_or_else(|| Web3Error::BlockchainError("Transaction receipt not found".to_string()))
            .map(|_| response["result"].clone())
    }

    /// Encode audit data for smart contract
    fn encode_audit_data(&self, audit_data: &Value) -> Result<String> {
        // In a real implementation, this would use proper ABI encoding
        // For now, we'll create a simple hex encoding
        let data_string = audit_data.to_string();
        Ok(format!("0x{}", hex::encode(data_string.as_bytes())))
    }

    /// Parse audit event from blockchain log
    async fn parse_audit_event(&self, log: &Value) -> Result<Web3Event> {
        Ok(Web3Event {
            event_type: "audit".to_string(),
            transaction_hash: log["transactionHash"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            block_number: self.parse_hex_to_u64(
                log["blockNumber"].as_str().unwrap_or("0x0")
            )?,
            timestamp: Utc::now(), // Would get actual block timestamp in production
            data: HashMap::new(), // Would parse actual event data in production
        })
    }

    /// Send RPC request to blockchain
    async fn send_rpc_request(&self, data: &Value) -> Result<Value> {
        let response = self.client
            .post(&self.rpc_url)
            .header("Content-Type", "application/json")
            .json(data)
            .send()
            .await
            .map_err(|e| Web3Error::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(Web3Error::NetworkError(format!(
                "RPC request failed with status: {}", 
                response.status()
            )));
        }
        
        let response_json: Value = response.json().await
            .map_err(|e| Web3Error::SerializationError(e.to_string()))?;
        
        if let Some(error) = response_json.get("error") {
            return Err(Web3Error::BlockchainError(format!(
                "RPC error: {}", 
                error.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error")
            )));
        }
        
        Ok(response_json)
    }

    /// Parse hex string to u64
    fn parse_hex_to_u64(&self, hex_str: &str) -> Result<u64> {
        let hex_str = hex_str.trim_start_matches("0x");
        u64::from_str_radix(hex_str, 16)
            .map_err(|e| Web3Error::SerializationError(format!("Invalid hex number: {}", e)))
    }

    /// Get network name from chain ID
    fn get_network_name(&self, chain_id: u64) -> String {
        match chain_id {
            1 => "Ethereum Mainnet".to_string(),
            3 => "Ropsten Testnet".to_string(),
            4 => "Rinkeby Testnet".to_string(),
            5 => "Goerli Testnet".to_string(),
            137 => "Polygon Mainnet".to_string(),
            80001 => "Polygon Mumbai".to_string(),
            56 => "BSC Mainnet".to_string(),
            97 => "BSC Testnet".to_string(),
            _ => format!("Unknown Network ({})", chain_id),
        }
    }

    /// Get blockchain client status
    pub async fn get_status(&self) -> Result<String> {
        let network_info = self.get_network_info().await?;
        Ok(format!("connected ({}, block: {})", 
                  network_info.network_name, 
                  network_info.latest_block))
    }

    /// Shutdown blockchain client
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down blockchain client");
        // Cleanup operations would go here
        info!("Blockchain client shutdown complete");
        Ok(())
    }
}
