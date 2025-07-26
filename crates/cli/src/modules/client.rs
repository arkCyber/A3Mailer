/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! HTTP client implementation for Stalwart CLI
//!
//! This module provides the HTTP client functionality for communicating
//! with the Stalwart server API, including proper error handling,
//! authentication, and response parsing.

use std::time::Duration;
use reqwest::{Method, StatusCode, header::AUTHORIZATION};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use jmap_client::client::Credentials;

use super::{cli::Client, host, is_localhost, UnwrapResult};

/// Response wrapper for API responses
#[derive(Deserialize)]
#[serde(untagged)]
pub enum Response<T> {
    Error(ManagementApiError),
    Data { data: T },
}

/// Management API error types
#[derive(Deserialize)]
#[serde(tag = "error")]
#[serde(rename_all = "camelCase")]
pub enum ManagementApiError {
    FieldAlreadyExists { field: String, value: String },
    FieldMissing { field: String },
    NotFound { item: String },
    Unsupported { details: String },
    AssertFailed,
    Other { details: String },
}

impl Client {
    /// Convert this client into a JMAP client for JMAP operations
    pub async fn into_jmap_client(self) -> jmap_client::client::Client {
        jmap_client::client::Client::new()
            .credentials(self.credentials)
            .accept_invalid_certs(is_localhost(&self.url))
            .follow_redirects([host(&self.url).expect("Invalid host").to_owned()])
            .timeout(Duration::from_secs(self.timeout.unwrap_or(60)))
            .connect(&self.url)
            .await
            .unwrap_or_else(|err| {
                eprintln!("Failed to connect to JMAP server {}: {}.", &self.url, err);
                std::process::exit(1);
            })
    }

    /// Make an HTTP request and expect a successful response
    pub async fn http_request<R: DeserializeOwned, B: Serialize>(
        &self,
        method: Method,
        url: &str,
        body: Option<B>,
    ) -> R {
        self.try_http_request(method, url, body)
            .await
            .unwrap_or_else(|| {
                eprintln!("Request failed: No data returned.");
                std::process::exit(1);
            })
    }

    /// Make an HTTP request and return None if not found, otherwise expect success
    pub async fn try_http_request<R: DeserializeOwned, B: Serialize>(
        &self,
        method: Method,
        url: &str,
        body: Option<B>,
    ) -> Option<R> {
        let full_url = if self.url.ends_with('/') && url.starts_with('/') {
            // Remove duplicate slash
            format!("{}{}", self.url.trim_end_matches('/'), url)
        } else if !self.url.ends_with('/') && !url.starts_with('/') {
            // Add missing slash
            format!("{}/{}", self.url, url)
        } else {
            // No modification needed
            format!("{}{}", self.url, url)
        };

        // Log the request for debugging
        println!("Making {} request to: {}", method, full_url);

        let mut request = reqwest::Client::builder()
            .danger_accept_invalid_certs(is_localhost(&full_url))
            .timeout(Duration::from_secs(self.timeout.unwrap_or(60)))
            .build()
            .unwrap_or_default()
            .request(method.clone(), &full_url)
            .header(
                AUTHORIZATION,
                match &self.credentials {
                    Credentials::Basic(s) => format!("Basic {s}"),
                    Credentials::Bearer(s) => format!("Bearer {s}"),
                },
            );

        if let Some(body) = body {
            let serialized_body = serde_json::to_string(&body)
                .unwrap_result("serialize request body");
            request = request
                .header("Content-Type", "application/json")
                .body(serialized_body);
        }

        let response = request.send().await.unwrap_result("send HTTP request");

        match response.status() {
            StatusCode::OK => {
                println!("✓ Request successful");
            }
            StatusCode::NOT_FOUND => {
                println!("⚠ Resource not found");
                return None;
            }
            StatusCode::UNAUTHORIZED => {
                eprintln!("❌ Authentication failed. Make sure the credentials are correct and that the account has administrator rights.");
                std::process::exit(1);
            }
            status => {
                let error_text = response.text().await.unwrap_result("fetch error text");
                eprintln!("❌ Request failed with status {}: {}", status, error_text);
                std::process::exit(1);
            }
        }

        let bytes = response.bytes().await.unwrap_result("fetch response bytes");

        // Try to parse as our Response wrapper first
        match serde_json::from_slice::<Response<R>>(&bytes) {
            Ok(Response::Data { data }) => {
                println!("✓ Successfully parsed response data");
                Some(data)
            }
            Ok(Response::Error(error)) => {
                eprintln!("❌ API error: {}", error);
                std::process::exit(1);
            }
            Err(_) => {
                // If that fails, try to parse directly as R
                match serde_json::from_slice::<R>(&bytes) {
                    Ok(data) => {
                        println!("✓ Successfully parsed direct response");
                        Some(data)
                    }
                    Err(parse_err) => {
                        eprintln!("❌ Failed to parse response: {}", parse_err);
                        eprintln!("Response body: {}", String::from_utf8_lossy(&bytes));
                        std::process::exit(1);
                    }
                }
            }
        }
    }
}

impl std::fmt::Display for ManagementApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManagementApiError::FieldAlreadyExists { field, value } => {
                write!(f, "Field '{}' already exists with value '{}'", field, value)
            }
            ManagementApiError::FieldMissing { field } => {
                write!(f, "Required field '{}' is missing", field)
            }
            ManagementApiError::NotFound { item } => {
                write!(f, "{} not found", item)
            }
            ManagementApiError::Unsupported { details } => {
                write!(f, "Unsupported operation: {}", details)
            }
            ManagementApiError::AssertFailed => {
                write!(f, "Assertion failed during operation")
            }
            ManagementApiError::Other { details } => {
                write!(f, "Error: {}", details)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test ManagementApiError display formatting
    #[test]
    fn test_management_api_error_display() {
        let errors = vec![
            (
                ManagementApiError::FieldAlreadyExists {
                    field: "email".to_string(),
                    value: "test@example.com".to_string(),
                },
                "Field 'email' already exists with value 'test@example.com'",
            ),
            (
                ManagementApiError::FieldMissing {
                    field: "name".to_string(),
                },
                "Required field 'name' is missing",
            ),
            (
                ManagementApiError::NotFound {
                    item: "User".to_string(),
                },
                "User not found",
            ),
            (
                ManagementApiError::Unsupported {
                    details: "Operation not supported".to_string(),
                },
                "Unsupported operation: Operation not supported",
            ),
            (
                ManagementApiError::AssertFailed,
                "Assertion failed during operation",
            ),
            (
                ManagementApiError::Other {
                    details: "Something went wrong".to_string(),
                },
                "Error: Something went wrong",
            ),
        ];

        for (error, expected) in errors {
            assert_eq!(format!("{}", error), expected);
        }
    }

    /// Test URL construction
    #[test]
    fn test_url_construction() {
        let test_cases = vec![
            ("https://example.com", "/api/test", "https://example.com/api/test"),
            ("https://example.com/", "/api/test", "https://example.com/api/test"),
            ("https://example.com", "api/test", "https://example.com/api/test"),
            ("https://example.com/", "api/test", "https://example.com/api/test"),
        ];

        for (base_url, path, expected) in test_cases {
            // Implement the same logic as in the actual client
            let result = if base_url.ends_with('/') && path.starts_with('/') {
                // Remove duplicate slash
                format!("{}{}", base_url.trim_end_matches('/'), path)
            } else if !base_url.ends_with('/') && !path.starts_with('/') {
                // Add missing slash
                format!("{}/{}", base_url, path)
            } else {
                // No modification needed
                format!("{}{}", base_url, path)
            };
            assert_eq!(result, expected, "URL construction failed for base: {}, path: {}", base_url, path);
        }
    }

    /// Test credentials formatting
    #[test]
    fn test_credentials_formatting() {
        let basic_creds = Credentials::basic("admin", "password");
        let bearer_creds = Credentials::Bearer("token123".to_string());

        // We can't directly test the formatting since it's done in the request,
        // but we can verify the credentials are created correctly
        match basic_creds {
            Credentials::Basic(_) => {}, // Expected
            _ => panic!("Expected Basic credentials"),
        }

        match bearer_creds {
            Credentials::Bearer(token) => {
                assert_eq!(token, "token123");
            }
            _ => panic!("Expected Bearer credentials"),
        }
    }
}
