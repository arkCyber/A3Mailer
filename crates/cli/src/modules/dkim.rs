/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! DKIM signature management module for Stalwart CLI
//!
//! This module provides functionality for creating and managing DKIM signatures,
//! including support for RSA and Ed25519 algorithms with comprehensive
//! error handling and validation.

use super::cli::{Client, DkimCommands};
use clap::ValueEnum;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
pub enum Algorithm {
    /// RSA
    #[default]
    Rsa,
    /// ED25519
    Ed25519,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
struct DkimSignature {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    pub algorithm: Algorithm,

    pub domain: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
}

impl DkimCommands {
    /// Execute DKIM management commands with comprehensive error handling and logging
    pub async fn exec(self, client: Client) {
        match self {
            DkimCommands::Create {
                signature_id,
                algorithm,
                domain,
                selector,
            } => {
                // Validate domain format
                if domain.is_empty() {
                    eprintln!("❌ Error: Domain cannot be empty");
                    std::process::exit(1);
                }

                if !is_valid_domain(&domain) {
                    eprintln!("❌ Error: Invalid domain format: {}", domain);
                    std::process::exit(1);
                }

                let signature_req = DkimSignature {
                    id: signature_id.clone(),
                    algorithm,
                    domain: domain.clone(),
                    selector: selector.clone(),
                };

                println!("Creating DKIM signature for domain '{}' with algorithm '{:?}'...",
                        domain, algorithm);

                if let Some(ref id) = signature_id {
                    println!("Using signature ID: {}", id);
                }

                if let Some(ref sel) = selector {
                    println!("Using selector: {}", sel);
                }

                client
                    .http_request::<Value, _>(Method::POST, "/api/dkim", Some(signature_req))
                    .await;

                println!("✓ Successfully created DKIM signature for domain '{}'", domain);
                println!("📝 Remember to publish the DNS TXT record for DKIM verification");
            }
            DkimCommands::GetPublicKey { signature_id } => {
                if signature_id.is_empty() {
                    eprintln!("❌ Error: Signature ID cannot be empty");
                    std::process::exit(1);
                }

                println!("Retrieving DKIM public key for signature ID '{}'...", signature_id);

                let response = client
                    .http_request::<Value, String>(
                        Method::GET,
                        &format!("/api/dkim/{}", signature_id),
                        None,
                    )
                    .await;

                println!("✓ DKIM Public Key Information:");
                println!("{}", serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|e| {
                        eprintln!("❌ Failed to format response: {}", e);
                        format!("{:?}", response)
                    }));
            }
        }
    }
}

/// Validate domain name format
fn is_valid_domain(domain: &str) -> bool {
    // Basic domain validation
    if domain.is_empty() || domain.len() > 253 {
        return false;
    }

    // Check for valid characters and structure
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() < 2 {
        return false;
    }

    for part in parts {
        if part.is_empty() || part.len() > 63 {
            return false;
        }

        // Check for valid characters (alphanumeric and hyphens)
        if !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return false;
        }

        // Cannot start or end with hyphen
        if part.starts_with('-') || part.ends_with('-') {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test Algorithm enum serialization
    #[test]
    fn test_algorithm_serialization() {
        let rsa = Algorithm::Rsa;
        let ed25519 = Algorithm::Ed25519;

        let rsa_json = serde_json::to_string(&rsa).unwrap();
        let ed25519_json = serde_json::to_string(&ed25519).unwrap();

        assert!(rsa_json.contains("Rsa"));
        assert!(ed25519_json.contains("Ed25519"));
    }

    /// Test DkimSignature creation
    #[test]
    fn test_dkim_signature_creation() {
        let signature = DkimSignature {
            id: Some("test_id".to_string()),
            algorithm: Algorithm::Rsa,
            domain: "example.com".to_string(),
            selector: Some("default".to_string()),
        };

        assert_eq!(signature.id, Some("test_id".to_string()));
        assert_eq!(signature.algorithm, Algorithm::Rsa);
        assert_eq!(signature.domain, "example.com");
        assert_eq!(signature.selector, Some("default".to_string()));
    }

    /// Test domain validation
    #[test]
    fn test_domain_validation() {
        // Valid domains
        let valid_domains = vec![
            "example.com",
            "sub.example.com",
            "test-domain.org",
            "a.b",
            "very-long-subdomain-name.example-domain.com",
        ];

        for domain in valid_domains {
            assert!(is_valid_domain(domain), "Domain should be valid: {}", domain);
        }

        // Invalid domains
        let long_label = "a".repeat(64);
        let long_domain = format!("{}.com", "a".repeat(64));
        let invalid_domains = vec![
            "",                    // Empty
            "example",             // No TLD
            ".example.com",        // Starts with dot
            "example.com.",        // Ends with dot
            "ex ample.com",        // Contains space
            "-example.com",        // Starts with hyphen
            "example-.com",        // Ends with hyphen
            "example..com",        // Double dot
            &long_label,           // Too long label
            &long_domain,          // Label too long
        ];

        for domain in invalid_domains {
            assert!(!is_valid_domain(domain), "Domain should be invalid: {}", domain);
        }
    }

    /// Test domain validation edge cases
    #[test]
    fn test_domain_validation_edge_cases() {
        // Maximum valid domain length (253 characters)
        let max_domain = format!("{}.{}.{}.{}.com",
                                "a".repeat(60),
                                "b".repeat(60),
                                "c".repeat(60),
                                "d".repeat(60));
        assert!(max_domain.len() <= 253);
        assert!(is_valid_domain(&max_domain));

        // Too long domain (over 253 characters)
        let too_long_domain = format!("{}.{}.{}.{}.{}.com",
                                     "a".repeat(60),
                                     "b".repeat(60),
                                     "c".repeat(60),
                                     "d".repeat(60),
                                     "e".repeat(60));
        assert!(too_long_domain.len() > 253);
        assert!(!is_valid_domain(&too_long_domain));
    }

    /// Test Algorithm default
    #[test]
    fn test_algorithm_default() {
        let default_algo = Algorithm::default();
        assert_eq!(default_algo, Algorithm::Rsa);
    }

    /// Test DkimSignature serialization
    #[test]
    fn test_dkim_signature_serialization() {
        let signature = DkimSignature {
            id: Some("test".to_string()),
            algorithm: Algorithm::Ed25519,
            domain: "example.com".to_string(),
            selector: Some("selector1".to_string()),
        };

        let json = serde_json::to_string(&signature).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("Ed25519"));
        assert!(json.contains("example.com"));
        assert!(json.contains("selector1"));
    }

    /// Test DkimSignature with optional fields
    #[test]
    fn test_dkim_signature_optional_fields() {
        let signature = DkimSignature {
            id: None,
            algorithm: Algorithm::Rsa,
            domain: "example.com".to_string(),
            selector: None,
        };

        let json = serde_json::to_string(&signature).unwrap();
        // Optional fields should not appear in JSON when None
        assert!(!json.contains("\"id\""));
        assert!(!json.contains("\"selector\""));
        assert!(json.contains("\"domain\""));
        assert!(json.contains("\"algorithm\""));
    }

    /// Performance test for domain validation
    #[test]
    fn test_domain_validation_performance() {
        let long_domain = "toolong.".repeat(50) + "com";
        let domains = vec![
            "example.com",
            "sub.example.org",
            "test-domain.net",
            "invalid..domain",
            &long_domain,
        ];

        let start = std::time::Instant::now();

        for _ in 0..1000 {
            for domain in &domains {
                let _ = is_valid_domain(domain);
            }
        }

        let elapsed = start.elapsed();

        // Should complete validation quickly
        assert!(elapsed.as_millis() < 100, "Domain validation too slow: {:?}", elapsed);
    }
}
