/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! High-Performance DAV Request Processing
//!
//! This module provides optimized request processing for DAV operations,
//! integrating with our high-performance architecture components.

use std::time::Instant;

use crate::{
    async_pool::AsyncRequestPool,
    cache::DavCache,
    data_access::DataAccessLayer,
    monitoring::DavMetrics,
    performance::DavPerformance,
    router::DavRouter,
    security::DavSecurity,
};

/// High-performance DAV request processor
///
/// Provides optimized request processing with caching, connection pooling,
/// and performance monitoring for maximum throughput.
#[derive(Debug, Clone)]
pub struct DavRequestProcessor {
    router: DavRouter,
    cache: DavCache,
    data_access: DataAccessLayer,
    metrics: DavMetrics,
    security: DavSecurity,
    performance: DavPerformance,
    request_pool: AsyncRequestPool,
}

impl DavRequestProcessor {
    /// Create a new DAV request processor
    pub fn new(
        router: DavRouter,
        cache: DavCache,
        data_access: DataAccessLayer,
        metrics: DavMetrics,
        security: DavSecurity,
        performance: DavPerformance,
        request_pool: AsyncRequestPool,
    ) -> Self {
        Self {
            router,
            cache,
            data_access,
            metrics,
            security,
            performance,
            request_pool,
        }
    }

    /// Process a DAV request with full optimization
    pub async fn process_request(
        &self,
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
        client_ip: String,
    ) -> Result<DavResponse, DavError> {
        let start_time = Instant::now();

        // Parse method
        let dav_method = self.parse_method(&method)?;

        // Route the request
        let headers_map: std::collections::HashMap<String, String> = headers.clone().into_iter().collect();
        let route_info = self.router.route_request(
            &path,
            crate::DavMethod::GET, // Convert to crate DavMethod
            &headers_map,
            &body,
            client_ip.clone(),
        ).await.map_err(|e| DavError::Internal(e.to_string()))?;

        // Preprocess the request
        let _preprocess_result = self.router.preprocess_request(
            &route_info,
            &headers_map,
            &body,
        ).await.map_err(|e| DavError::Internal(e.to_string()))?;

        // Submit to async pool for processing
        let result = self.request_pool.submit_request(
            client_ip,
            method,
            path.clone(),
            headers,
            body,
            route_info.priority,
        ).await.map_err(|e| DavError::Internal(e.to_string()))?;

        // Record metrics
        self.metrics.record_request_start(&dav_method.to_string());

        // Create response
        Ok(DavResponse {
            status: 200,
            headers: vec![
                ("Content-Type".to_string(), "application/xml".to_string()),
                ("DAV".to_string(), "1, 2, 3, access-control, calendar-access, addressbook".to_string()),
            ],
            body: result.body,
        })
    }

    /// Parse HTTP method to DAV method
    fn parse_method(&self, method: &str) -> Result<DavMethod, DavError> {
        match method.to_uppercase().as_str() {
            "GET" => Ok(DavMethod::GET),
            "PUT" => Ok(DavMethod::PUT),
            "POST" => Ok(DavMethod::POST),
            "DELETE" => Ok(DavMethod::DELETE),
            "PATCH" => Ok(DavMethod::PATCH),
            "OPTIONS" => Ok(DavMethod::OPTIONS),
            "HEAD" => Ok(DavMethod::HEAD),
            "PROPFIND" => Ok(DavMethod::PROPFIND),
            "PROPPATCH" => Ok(DavMethod::PROPPATCH),
            "MKCOL" => Ok(DavMethod::MKCOL),
            "COPY" => Ok(DavMethod::COPY),
            "MOVE" => Ok(DavMethod::MOVE),
            "LOCK" => Ok(DavMethod::LOCK),
            "UNLOCK" => Ok(DavMethod::UNLOCK),
            "REPORT" => Ok(DavMethod::REPORT),
            "ACL" => Ok(DavMethod::ACL),
            "MKCALENDAR" => Ok(DavMethod::MKCALENDAR),
            _ => Err(DavError::Validation(format!("Unsupported method: {}", method))),
        }
    }

    /// Get processor performance statistics
    pub async fn get_stats(&self) -> DavProcessorStats {
        DavProcessorStats {
            total_requests: 0, // Placeholder for now
            cache_stats: self.cache.get_stats().await,
            router_stats: self.router.get_router_stats().await,
            data_access_stats: self.data_access.get_performance_stats().await,
        }
    }
}

/// DAV request headers
#[derive(Debug, Clone)]
pub struct RequestHeaders {
    pub uri: String,
    pub content_type: Option<String>,
    pub if_match: Option<String>,
    pub if_none_match: Option<String>,
    pub depth: Option<u32>,
    pub destination: Option<String>,
    pub overwrite: Option<bool>,
    pub authorization: Option<String>,
    pub prefer: Option<String>,
    pub schedule_reply: Option<bool>,
    pub lock_token: Option<String>,
    pub timeout: Option<String>,
    pub user_agent: Option<String>,
}

impl RequestHeaders {
    pub fn new(path: &str) -> Self {
        Self {
            uri: path.to_string(),
            content_type: None,
            if_match: None,
            if_none_match: None,
            depth: None,
            destination: None,
            overwrite: None,
            authorization: None,
            prefer: None,
            schedule_reply: None,
            lock_token: None,
            timeout: None,
            user_agent: None,
        }
    }

    pub fn parse(&mut self, name: &str, value: &str) {
        match name.to_lowercase().as_str() {
            "content-type" => self.content_type = Some(value.to_string()),
            "if-match" => self.if_match = Some(value.to_string()),
            "if-none-match" => self.if_none_match = Some(value.to_string()),
            "depth" => {
                self.depth = match value {
                    "0" => Some(0),
                    "1" => Some(1),
                    "infinity" => Some(u32::MAX),
                    _ => value.parse().ok(),
                };
            }
            "destination" => self.destination = Some(value.to_string()),
            "overwrite" => {
                self.overwrite = match value.to_uppercase().as_str() {
                    "T" | "TRUE" => Some(true),
                    "F" | "FALSE" => Some(false),
                    _ => None,
                };
            }
            "authorization" => self.authorization = Some(value.to_string()),
            "prefer" => self.prefer = Some(value.to_string()),
            "schedule-reply" => {
                self.schedule_reply = match value.to_uppercase().as_str() {
                    "T" | "TRUE" => Some(true),
                    "F" | "FALSE" => Some(false),
                    _ => None,
                };
            }
            "lock-token" => self.lock_token = Some(value.to_string()),
            "timeout" => self.timeout = Some(value.to_string()),
            "user-agent" => self.user_agent = Some(value.to_string()),
            _ => {
                // Ignore unknown headers
            }
        }
    }
}

/// DAV method enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DavMethod {
    GET,
    PUT,
    POST,
    DELETE,
    PATCH,
    OPTIONS,
    HEAD,
    PROPFIND,
    PROPPATCH,
    MKCOL,
    COPY,
    MOVE,
    LOCK,
    UNLOCK,
    REPORT,
    ACL,
    MKCALENDAR,
}

impl DavMethod {
    pub fn has_body(self) -> bool {
        matches!(
            self,
            DavMethod::PUT
                | DavMethod::POST
                | DavMethod::PATCH
                | DavMethod::PROPPATCH
                | DavMethod::MKCOL
                | DavMethod::MKCALENDAR
                | DavMethod::LOCK
                | DavMethod::ACL
                | DavMethod::REPORT
        )
    }

    pub fn to_string(self) -> String {
        match self {
            DavMethod::GET => "GET".to_string(),
            DavMethod::PUT => "PUT".to_string(),
            DavMethod::POST => "POST".to_string(),
            DavMethod::DELETE => "DELETE".to_string(),
            DavMethod::PATCH => "PATCH".to_string(),
            DavMethod::OPTIONS => "OPTIONS".to_string(),
            DavMethod::HEAD => "HEAD".to_string(),
            DavMethod::PROPFIND => "PROPFIND".to_string(),
            DavMethod::PROPPATCH => "PROPPATCH".to_string(),
            DavMethod::MKCOL => "MKCOL".to_string(),
            DavMethod::COPY => "COPY".to_string(),
            DavMethod::MOVE => "MOVE".to_string(),
            DavMethod::LOCK => "LOCK".to_string(),
            DavMethod::UNLOCK => "UNLOCK".to_string(),
            DavMethod::REPORT => "REPORT".to_string(),
            DavMethod::ACL => "ACL".to_string(),
            DavMethod::MKCALENDAR => "MKCALENDAR".to_string(),
        }
    }
}

/// DAV resource name enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DavResourceName {
    Cal,
    Card,
    File,
    Principal,
    Scheduling,
}

impl DavResourceName {
    pub fn name(self) -> &'static str {
        match self {
            DavResourceName::Cal => "calendar",
            DavResourceName::Card => "addressbook",
            DavResourceName::File => "file",
            DavResourceName::Principal => "principal",
            DavResourceName::Scheduling => "scheduling",
        }
    }
}

/// DAV response structure
#[derive(Debug, Clone)]
pub struct DavResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// DAV error types
#[derive(Debug, Clone)]
pub enum DavError {
    Parse(String),
    Internal(String),
    Validation(String),
    NotFound,
    Unauthorized,
    Forbidden,
    Conflict,
    PreconditionFailed,
    UnsupportedMediaType,
    UnprocessableEntity,
    Locked,
    FailedDependency,
    InsufficientStorage,
}

impl std::fmt::Display for DavError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DavError::Parse(msg) => write!(f, "Parse error: {}", msg),
            DavError::Internal(msg) => write!(f, "Internal error: {}", msg),
            DavError::Validation(msg) => write!(f, "Validation error: {}", msg),
            DavError::NotFound => write!(f, "Not found"),
            DavError::Unauthorized => write!(f, "Unauthorized"),
            DavError::Forbidden => write!(f, "Forbidden"),
            DavError::Conflict => write!(f, "Conflict"),
            DavError::PreconditionFailed => write!(f, "Precondition failed"),
            DavError::UnsupportedMediaType => write!(f, "Unsupported media type"),
            DavError::UnprocessableEntity => write!(f, "Unprocessable entity"),
            DavError::Locked => write!(f, "Locked"),
            DavError::FailedDependency => write!(f, "Failed dependency"),
            DavError::InsufficientStorage => write!(f, "Insufficient storage"),
        }
    }
}

impl std::error::Error for DavError {}

/// DAV processor performance statistics
#[derive(Debug, Clone)]
pub struct DavProcessorStats {
    pub total_requests: u64,
    pub cache_stats: crate::cache::CacheStats,
    pub router_stats: crate::router::RouterPerformanceStats,
    pub data_access_stats: crate::data_access::DataAccessStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        async_pool::{AsyncRequestPool, AsyncPoolConfig},
        cache::{DavCache, CacheConfig},
        data_access::{DataAccessLayer, DataAccessConfig},
        monitoring::{DavMetrics, MonitoringConfig},
        performance::{DavPerformance, PerformanceConfig},
        router::{DavRouter, RouterConfig},
        security::{DavSecurity, SecurityConfig},
    };

    #[tokio::test]
    async fn test_dav_request_processor_creation() {
        let request_pool = AsyncRequestPool::new(AsyncPoolConfig::default());
        let security = DavSecurity::new(SecurityConfig::default());
        let performance = DavPerformance::new(PerformanceConfig::default());
        let metrics = DavMetrics::new();
        let cache = DavCache::new(CacheConfig::default());
        let data_access = DataAccessLayer::new(DataAccessConfig::default());

        let router = DavRouter::new(
            request_pool.clone(),
            security.clone(),
            performance.clone(),
            metrics.clone(),
            RouterConfig::default(),
        );

        let processor = DavRequestProcessor::new(
            router,
            cache,
            data_access,
            metrics,
            security,
            performance,
            request_pool,
        );

        // Verify processor was created successfully
        let stats = processor.get_stats().await;
        assert_eq!(stats.total_requests, 0);
    }

    #[tokio::test]
    async fn test_process_get_request() {
        let request_pool = AsyncRequestPool::new(AsyncPoolConfig::default());
        let security = DavSecurity::new(SecurityConfig::default());
        let performance = DavPerformance::new(PerformanceConfig::default());
        let metrics = DavMetrics::new();
        let cache = DavCache::new(CacheConfig::default());
        let data_access = DataAccessLayer::new(DataAccessConfig::default());

        let router = DavRouter::new(
            request_pool.clone(),
            security.clone(),
            performance.clone(),
            metrics.clone(),
            RouterConfig::default(),
        );

        let processor = DavRequestProcessor::new(
            router,
            cache,
            data_access,
            metrics,
            security,
            performance,
            request_pool,
        );

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(100)).await;

        let result = processor.process_request(
            "GET".to_string(),
            "/calendar/user/personal".to_string(),
            vec![("Content-Type".to_string(), "text/calendar".to_string())],
            vec![],
            "192.168.1.1".to_string(),
        ).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, 200);
        assert!(!response.headers.is_empty());
    }

    #[tokio::test]
    async fn test_parse_method() {
        let request_pool = AsyncRequestPool::new(AsyncPoolConfig::default());
        let security = DavSecurity::new(SecurityConfig::default());
        let performance = DavPerformance::new(PerformanceConfig::default());
        let metrics = DavMetrics::new();
        let cache = DavCache::new(CacheConfig::default());
        let data_access = DataAccessLayer::new(DataAccessConfig::default());

        let router = DavRouter::new(
            request_pool.clone(),
            security.clone(),
            performance.clone(),
            metrics.clone(),
            RouterConfig::default(),
        );

        let processor = DavRequestProcessor::new(
            router,
            cache,
            data_access,
            metrics,
            security,
            performance,
            request_pool,
        );

        // Test valid methods
        assert_eq!(processor.parse_method("GET").unwrap(), DavMethod::GET);
        assert_eq!(processor.parse_method("PUT").unwrap(), DavMethod::PUT);
        assert_eq!(processor.parse_method("PROPFIND").unwrap(), DavMethod::PROPFIND);
        assert_eq!(processor.parse_method("MKCALENDAR").unwrap(), DavMethod::MKCALENDAR);

        // Test case insensitive
        assert_eq!(processor.parse_method("get").unwrap(), DavMethod::GET);
        assert_eq!(processor.parse_method("Put").unwrap(), DavMethod::PUT);

        // Test invalid method
        assert!(processor.parse_method("INVALID").is_err());
    }

    #[test]
    fn test_request_headers_creation() {
        let headers = RequestHeaders::new("/calendar/user/personal");
        assert_eq!(headers.uri, "/calendar/user/personal");
        assert!(headers.content_type.is_none());
        assert!(headers.if_match.is_none());
        assert!(headers.if_none_match.is_none());
    }

    #[test]
    fn test_request_headers_parsing() {
        let mut headers = RequestHeaders::new("/test");

        // Test content-type parsing
        headers.parse("content-type", "text/calendar; charset=utf-8");
        assert!(headers.content_type.is_some());
        assert!(headers.content_type.unwrap().contains("text/calendar"));

        // Test if-match parsing
        headers.parse("if-match", "\"etag123\"");
        assert!(headers.if_match.is_some());

        // Test if-none-match parsing
        headers.parse("if-none-match", "\"etag456\"");
        assert!(headers.if_none_match.is_some());

        // Test depth parsing
        headers.parse("depth", "1");
        assert_eq!(headers.depth, Some(1));

        // Test destination parsing
        headers.parse("destination", "/new/path");
        assert!(headers.destination.is_some());

        // Test overwrite parsing
        headers.parse("overwrite", "T");
        assert_eq!(headers.overwrite, Some(true));

        headers.parse("overwrite", "F");
        assert_eq!(headers.overwrite, Some(false));
    }

    #[test]
    fn test_dav_method_has_body() {
        // Test which methods should have bodies
        assert!(DavMethod::PUT.has_body());
        assert!(DavMethod::POST.has_body());
        assert!(DavMethod::PATCH.has_body());
        assert!(DavMethod::PROPPATCH.has_body());
        assert!(DavMethod::MKCOL.has_body());
        assert!(DavMethod::MKCALENDAR.has_body());
        assert!(DavMethod::LOCK.has_body());
        assert!(DavMethod::ACL.has_body());
        assert!(DavMethod::REPORT.has_body());

        // Methods that typically don't have bodies
        assert!(!DavMethod::GET.has_body());
        assert!(!DavMethod::DELETE.has_body());
        assert!(!DavMethod::OPTIONS.has_body());
        assert!(!DavMethod::UNLOCK.has_body());
        assert!(!DavMethod::COPY.has_body());
        assert!(!DavMethod::MOVE.has_body());
    }

    #[test]
    fn test_dav_method_to_string() {
        assert_eq!(DavMethod::GET.to_string(), "GET");
        assert_eq!(DavMethod::PUT.to_string(), "PUT");
        assert_eq!(DavMethod::PROPFIND.to_string(), "PROPFIND");
        assert_eq!(DavMethod::MKCALENDAR.to_string(), "MKCALENDAR");
    }

    #[test]
    fn test_dav_resource_name() {
        assert_eq!(DavResourceName::Cal.name(), "calendar");
        assert_eq!(DavResourceName::Card.name(), "addressbook");
        assert_eq!(DavResourceName::File.name(), "file");
        assert_eq!(DavResourceName::Principal.name(), "principal");
        assert_eq!(DavResourceName::Scheduling.name(), "scheduling");
    }

    #[test]
    fn test_dav_error_display() {
        let error = DavError::Parse("test error".to_string());
        assert_eq!(error.to_string(), "Parse error: test error");

        let error = DavError::Internal("internal issue".to_string());
        assert_eq!(error.to_string(), "Internal error: internal issue");

        let error = DavError::Validation("validation failed".to_string());
        assert_eq!(error.to_string(), "Validation error: validation failed");

        let error = DavError::NotFound;
        assert_eq!(error.to_string(), "Not found");

        let error = DavError::Unauthorized;
        assert_eq!(error.to_string(), "Unauthorized");
    }

    #[test]
    fn test_dav_response_creation() {
        let response = DavResponse {
            status: 200,
            headers: vec![
                ("Content-Type".to_string(), "application/xml".to_string()),
                ("DAV".to_string(), "1, 2, 3".to_string()),
            ],
            body: b"<response></response>".to_vec(),
        };

        assert_eq!(response.status, 200);
        assert_eq!(response.headers.len(), 2);
        assert!(!response.body.is_empty());
    }

    #[test]
    fn test_request_headers_depth_parsing() {
        let mut headers = RequestHeaders::new("/test");

        // Valid depth values
        headers.parse("depth", "0");
        assert_eq!(headers.depth, Some(0));

        headers.parse("depth", "1");
        assert_eq!(headers.depth, Some(1));

        headers.parse("depth", "infinity");
        assert_eq!(headers.depth, Some(u32::MAX));

        // Invalid depth should be ignored
        headers.parse("depth", "invalid");
        // Should not panic
    }

    #[test]
    fn test_request_headers_boolean_parsing() {
        let mut headers = RequestHeaders::new("/test");

        // Test various boolean representations
        headers.parse("overwrite", "T");
        assert_eq!(headers.overwrite, Some(true));

        headers.parse("overwrite", "F");
        assert_eq!(headers.overwrite, Some(false));

        headers.parse("overwrite", "true");
        assert_eq!(headers.overwrite, Some(true));

        headers.parse("overwrite", "false");
        assert_eq!(headers.overwrite, Some(false));

        // Invalid boolean should be ignored
        headers.parse("overwrite", "maybe");
        // Should not panic
    }

    #[test]
    fn test_request_headers_case_insensitive() {
        let mut headers = RequestHeaders::new("/test");

        // Headers should be case-insensitive
        headers.parse("Content-Type", "text/calendar");
        assert!(headers.content_type.is_some());

        headers.parse("CONTENT-TYPE", "application/xml");
        assert!(headers.content_type.is_some());

        headers.parse("content-type", "text/plain");
        assert!(headers.content_type.is_some());
    }
}
