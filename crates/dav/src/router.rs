/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! High-Performance DAV Request Router
//!
//! This module provides an optimized request routing system for DAV operations,
//! featuring intelligent caching, request preprocessing, and performance monitoring.

use std::{
    collections::HashMap,
    sync::{Arc, atomic::{AtomicU64, Ordering}},
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

use crate::{
    DavError, DavMethod, DavResourceName,
    async_pool::{AsyncRequestPool, RequestPriority},
    monitoring::DavMetrics,
    security::DavSecurity,
    performance::DavPerformance,
};

/// High-performance DAV request router
///
/// Provides intelligent request routing with caching, preprocessing,
/// and performance optimization for maximum throughput.
#[derive(Debug, Clone)]
pub struct DavRouter {
    inner: Arc<DavRouterInner>,
    config: RouterConfig,
}

#[derive(Debug)]
struct DavRouterInner {
    /// Route cache for fast path resolution
    route_cache: RwLock<HashMap<String, CachedRoute>>,
    /// Request pool for async processing
    request_pool: AsyncRequestPool,
    /// Security manager
    security: DavSecurity,
    /// Performance optimizer
    performance: DavPerformance,
    /// Metrics collector
    metrics: DavMetrics,
    /// Router statistics
    stats: RouterStats,
}

/// Router configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RouterConfig {
    /// Enable route caching
    pub enable_route_cache: bool,
    /// Maximum cached routes
    pub max_cached_routes: usize,
    /// Route cache TTL
    pub route_cache_ttl: Duration,
    /// Enable request preprocessing
    pub enable_preprocessing: bool,
    /// Enable performance monitoring
    pub enable_monitoring: bool,
    /// Request timeout
    pub request_timeout: Duration,
    /// Enable request batching
    pub enable_batching: bool,
    /// Batch size
    pub batch_size: usize,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            enable_route_cache: true,
            max_cached_routes: 10000,
            route_cache_ttl: Duration::from_secs(300), // 5 minutes
            enable_preprocessing: true,
            enable_monitoring: true,
            request_timeout: Duration::from_secs(30),
            enable_batching: true,
            batch_size: 10,
        }
    }
}

#[derive(Debug, Clone)]
struct CachedRoute {
    resource: DavResourceName,
    method_permissions: HashMap<DavMethod, Vec<String>>,
    cached_at: Instant,
    access_count: u64,
    last_accessed: Instant,
}

#[derive(Debug, Default)]
struct RouterStats {
    total_requests: AtomicU64,
    cached_routes: AtomicU64,
    route_cache_hits: AtomicU64,
    route_cache_misses: AtomicU64,
    preprocessing_time: AtomicU64, // in nanoseconds
    routing_time: AtomicU64, // in nanoseconds
    failed_routes: AtomicU64,
}

/// Request routing information
#[derive(Debug, Clone)]
pub struct RouteInfo {
    pub resource: DavResourceName,
    pub method: DavMethod,
    pub path: String,
    pub priority: RequestPriority,
    pub estimated_duration: Duration,
    pub requires_auth: bool,
    pub cacheable: bool,
}

/// Request preprocessing result
#[derive(Debug, Clone)]
pub struct PreprocessResult {
    pub route_info: RouteInfo,
    pub optimizations: Vec<RouteOptimization>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum RouteOptimization {
    CacheHit(String),
    Compression(String),
    Batching(String),
    Prefetch(String),
}

impl DavRouter {
    /// Create a new high-performance DAV router
    pub fn new(
        request_pool: AsyncRequestPool,
        security: DavSecurity,
        performance: DavPerformance,
        metrics: DavMetrics,
        config: RouterConfig,
    ) -> Self {
        let config_clone = config.clone();
        let router = Self {
            inner: Arc::new(DavRouterInner {
                route_cache: RwLock::new(HashMap::new()),
                request_pool,
                security,
                performance,
                metrics,
                stats: RouterStats::default(),
            }),
            config,
        };

        info!(
            cache_enabled = config_clone.enable_route_cache,
            max_routes = config_clone.max_cached_routes,
            preprocessing = config_clone.enable_preprocessing,
            "DAV router initialized"
        );

        router
    }

    /// Route a DAV request with optimization
    pub async fn route_request(
        &self,
        path: &str,
        method: DavMethod,
        headers: &HashMap<String, String>,
        body: &[u8],
        client_ip: String,
    ) -> Result<RouteInfo, DavError> {
        let start_time = Instant::now();
        self.inner.stats.total_requests.fetch_add(1, Ordering::Relaxed);

        // Security checks
        let client_ip_addr = client_ip.parse()
            .map_err(|_| DavError::validation("Invalid IP address"))?;

        self.inner.security.check_rate_limit(client_ip_addr)?;
        self.inner.security.validate_path(path)?;
        self.inner.security.validate_body_size(body.len())?;

        // Check route cache
        let cached_route = if self.config.enable_route_cache {
            self.get_cached_route(path, method).await
        } else {
            None
        };

        let route_info = if let Some(cached) = cached_route {
            self.inner.stats.route_cache_hits.fetch_add(1, Ordering::Relaxed);

            debug!(
                path = path,
                method = ?method,
                cache_hit = true,
                "Route cache hit"
            );

            RouteInfo {
                resource: cached.resource,
                method,
                path: path.to_string(),
                priority: self.determine_priority(method, path, headers),
                estimated_duration: self.estimate_duration(method, &cached.resource),
                requires_auth: true, // Always require auth for cached routes
                cacheable: self.is_cacheable(method),
            }
        } else {
            self.inner.stats.route_cache_misses.fetch_add(1, Ordering::Relaxed);

            // Resolve route
            let resource = self.resolve_resource(path)?;
            let route_info = RouteInfo {
                resource,
                method,
                path: path.to_string(),
                priority: self.determine_priority(method, path, headers),
                estimated_duration: self.estimate_duration(method, &resource),
                requires_auth: true,
                cacheable: self.is_cacheable(method),
            };

            // Cache the route
            if self.config.enable_route_cache {
                self.cache_route(path, method, &route_info).await;
            }

            route_info
        };

        // Record routing time
        let routing_time = start_time.elapsed();
        let current_avg = self.inner.stats.routing_time.load(Ordering::Relaxed);
        let new_avg = (current_avg + routing_time.as_nanos() as u64) / 2;
        self.inner.stats.routing_time.store(new_avg, Ordering::Relaxed);

        debug!(
            path = path,
            method = ?method,
            resource = ?route_info.resource,
            priority = ?route_info.priority,
            routing_time_ms = routing_time.as_millis(),
            "Request routed successfully"
        );

        Ok(route_info)
    }

    /// Preprocess a request for optimization
    pub async fn preprocess_request(
        &self,
        route_info: &RouteInfo,
        headers: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<PreprocessResult, DavError> {
        if !self.config.enable_preprocessing {
            return Ok(PreprocessResult {
                route_info: route_info.clone(),
                optimizations: vec![],
                warnings: vec![],
            });
        }

        let start_time = Instant::now();
        let mut optimizations = Vec::new();
        let mut warnings = Vec::new();

        // Check for cache opportunities
        if route_info.cacheable {
            if let Some(etag) = headers.get("if-none-match") {
                optimizations.push(RouteOptimization::CacheHit(
                    format!("ETag match possible: {}", etag)
                ));
            }
        }

        // Check for compression opportunities
        if self.inner.performance.should_compress(body.len(),
            headers.get("content-type").map(|s| s.as_str()).unwrap_or("")) {
            optimizations.push(RouteOptimization::Compression(
                format!("Content can be compressed ({}% savings estimated)",
                    self.estimate_compression_savings(body))
            ));
        }

        // Check for batching opportunities
        if self.config.enable_batching && route_info.method == DavMethod::PROPFIND {
            optimizations.push(RouteOptimization::Batching(
                "Request can be batched with similar requests".to_string()
            ));
        }

        // Check for prefetch opportunities
        if route_info.method == DavMethod::GET && route_info.path.ends_with(".ics") {
            optimizations.push(RouteOptimization::Prefetch(
                "Related calendar resources can be prefetched".to_string()
            ));
        }

        // Performance warnings
        if body.len() > 1024 * 1024 { // 1MB
            warnings.push(format!("Large request body: {} bytes", body.len()));
        }

        if route_info.estimated_duration > Duration::from_millis(500) {
            warnings.push(format!("Slow operation estimated: {:?}", route_info.estimated_duration));
        }

        // Record preprocessing time
        let preprocessing_time = start_time.elapsed();
        let current_avg = self.inner.stats.preprocessing_time.load(Ordering::Relaxed);
        let new_avg = (current_avg + preprocessing_time.as_nanos() as u64) / 2;
        self.inner.stats.preprocessing_time.store(new_avg, Ordering::Relaxed);

        debug!(
            path = %route_info.path,
            method = ?route_info.method,
            optimizations = optimizations.len(),
            warnings = warnings.len(),
            preprocessing_time_ms = preprocessing_time.as_millis(),
            "Request preprocessing completed"
        );

        Ok(PreprocessResult {
            route_info: route_info.clone(),
            optimizations,
            warnings,
        })
    }

    /// Get router performance statistics
    pub async fn get_router_stats(&self) -> RouterPerformanceStats {
        let route_cache = self.inner.route_cache.read().await;

        RouterPerformanceStats {
            total_requests: self.inner.stats.total_requests.load(Ordering::Relaxed),
            cached_routes: route_cache.len(),
            route_cache_hits: self.inner.stats.route_cache_hits.load(Ordering::Relaxed),
            route_cache_misses: self.inner.stats.route_cache_misses.load(Ordering::Relaxed),
            cache_hit_rate: {
                let hits = self.inner.stats.route_cache_hits.load(Ordering::Relaxed);
                let misses = self.inner.stats.route_cache_misses.load(Ordering::Relaxed);
                if hits + misses > 0 {
                    hits as f64 / (hits + misses) as f64
                } else {
                    0.0
                }
            },
            average_routing_time: Duration::from_nanos(
                self.inner.stats.routing_time.load(Ordering::Relaxed)
            ),
            average_preprocessing_time: Duration::from_nanos(
                self.inner.stats.preprocessing_time.load(Ordering::Relaxed)
            ),
            failed_routes: self.inner.stats.failed_routes.load(Ordering::Relaxed),
        }
    }

    async fn get_cached_route(&self, path: &str, method: DavMethod) -> Option<CachedRoute> {
        let mut cache = self.inner.route_cache.write().await;
        let cache_key = format!("{}:{:?}", path, method);

        if let Some(cached) = cache.get_mut(&cache_key) {
            if cached.cached_at.elapsed() < self.config.route_cache_ttl {
                cached.access_count += 1;
                cached.last_accessed = Instant::now();
                Some(cached.clone())
            } else {
                cache.remove(&cache_key);
                None
            }
        } else {
            None
        }
    }

    async fn cache_route(&self, path: &str, method: DavMethod, route_info: &RouteInfo) {
        let mut cache = self.inner.route_cache.write().await;

        if cache.len() >= self.config.max_cached_routes {
            // Remove oldest entry
            if let Some((oldest_key, _)) = cache.iter()
                .min_by_key(|(_, route)| route.last_accessed)
                .map(|(k, v)| (k.clone(), v.clone()))
            {
                cache.remove(&oldest_key);
            }
        }

        let cache_key = format!("{}:{:?}", path, method);
        let cached_route = CachedRoute {
            resource: route_info.resource,
            method_permissions: HashMap::new(), // Would be populated with actual permissions
            cached_at: Instant::now(),
            access_count: 0,
            last_accessed: Instant::now(),
        };

        cache.insert(cache_key, cached_route);
        self.inner.stats.cached_routes.fetch_add(1, Ordering::Relaxed);
    }

    fn resolve_resource(&self, path: &str) -> Result<DavResourceName, DavError> {
        // Simplified resource resolution - in production this would be more sophisticated
        if path.contains("/calendar/") || path.contains("/cal/") {
            Ok(DavResourceName::Cal)
        } else if path.contains("/addressbook/") || path.contains("/card/") {
            Ok(DavResourceName::Card)
        } else if path.contains("/files/") || path.contains("/file/") {
            Ok(DavResourceName::File)
        } else if path.contains("/principal/") {
            Ok(DavResourceName::Principal)
        } else if path.contains("/scheduling/") {
            Ok(DavResourceName::Scheduling)
        } else {
            // Default to file for unknown paths
            Ok(DavResourceName::File)
        }
    }

    fn determine_priority(&self, method: DavMethod, path: &str, headers: &HashMap<String, String>) -> RequestPriority {
        // Admin paths get high priority
        if path.contains("/admin/") {
            return RequestPriority::High;
        }

        // Critical operations
        if matches!(method, DavMethod::LOCK | DavMethod::UNLOCK) {
            return RequestPriority::High;
        }

        // Bulk operations get lower priority
        if headers.get("depth").map(|d| d == "infinity").unwrap_or(false) {
            return RequestPriority::Low;
        }

        // Default priority
        RequestPriority::Normal
    }

    fn estimate_duration(&self, method: DavMethod, resource: &DavResourceName) -> Duration {
        match (method, resource) {
            (DavMethod::GET, _) => Duration::from_millis(10),
            (DavMethod::PROPFIND, _) => Duration::from_millis(50),
            (DavMethod::PUT, DavResourceName::File) => Duration::from_millis(200),
            (DavMethod::PUT, _) => Duration::from_millis(100),
            (DavMethod::DELETE, _) => Duration::from_millis(30),
            (DavMethod::COPY | DavMethod::MOVE, _) => Duration::from_millis(150),
            (DavMethod::REPORT, _) => Duration::from_millis(100),
            _ => Duration::from_millis(50),
        }
    }

    fn is_cacheable(&self, method: DavMethod) -> bool {
        matches!(method, DavMethod::GET | DavMethod::PROPFIND | DavMethod::REPORT)
    }

    fn estimate_compression_savings(&self, body: &[u8]) -> u32 {
        // Simplified compression estimation
        if body.len() < 1024 {
            return 0;
        }

        // Estimate based on content type and size
        let text_ratio = body.iter().filter(|&&b| b.is_ascii_graphic() || b.is_ascii_whitespace()).count();
        let text_percentage = (text_ratio * 100) / body.len();

        if text_percentage > 80 {
            60 // 60% compression for text content
        } else if text_percentage > 50 {
            30 // 30% compression for mixed content
        } else {
            10 // 10% compression for binary content
        }
    }
}

/// Router performance statistics
#[derive(Debug, Clone)]
pub struct RouterPerformanceStats {
    pub total_requests: u64,
    pub cached_routes: usize,
    pub route_cache_hits: u64,
    pub route_cache_misses: u64,
    pub cache_hit_rate: f64,
    pub average_routing_time: Duration,
    pub average_preprocessing_time: Duration,
    pub failed_routes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        async_pool::{AsyncRequestPool, AsyncPoolConfig},
        security::{DavSecurity, SecurityConfig},
        performance::{DavPerformance, PerformanceConfig},
        monitoring::DavMetrics,
    };

    #[tokio::test]
    async fn test_route_caching() {
        let request_pool = AsyncRequestPool::new(AsyncPoolConfig::default());
        let security = DavSecurity::new(SecurityConfig::default());
        let performance = DavPerformance::new(PerformanceConfig::default());
        let metrics = DavMetrics::new();

        let router = DavRouter::new(
            request_pool,
            security,
            performance,
            metrics,
            RouterConfig::default(),
        );

        let headers = HashMap::new();

        // First request should miss cache
        let route1 = router.route_request(
            "/calendar/user/personal",
            DavMethod::GET,
            &headers,
            &[],
            "192.168.1.1".to_string(),
        ).await.unwrap();

        // Second request should hit cache
        let route2 = router.route_request(
            "/calendar/user/personal",
            DavMethod::GET,
            &headers,
            &[],
            "192.168.1.1".to_string(),
        ).await.unwrap();

        assert_eq!(route1.resource, route2.resource);
        assert_eq!(route1.method, route2.method);

        let stats = router.get_router_stats().await;
        assert!(stats.route_cache_hits > 0);
    }

    #[tokio::test]
    async fn test_request_preprocessing() {
        let request_pool = AsyncRequestPool::new(AsyncPoolConfig::default());
        let security = DavSecurity::new(SecurityConfig::default());
        let performance = DavPerformance::new(PerformanceConfig::default());
        let metrics = DavMetrics::new();

        let router = DavRouter::new(
            request_pool,
            security,
            performance,
            metrics,
            RouterConfig::default(),
        );

        let route_info = RouteInfo {
            resource: DavResourceName::Cal,
            method: DavMethod::GET,
            path: "/calendar/test.ics".to_string(),
            priority: RequestPriority::Normal,
            estimated_duration: Duration::from_millis(10),
            requires_auth: true,
            cacheable: true,
        };

        let headers = HashMap::new();
        let body = b"large body content that should trigger compression";

        let result = router.preprocess_request(&route_info, &headers, body).await.unwrap();

        assert!(!result.optimizations.is_empty());
        assert_eq!(result.route_info.resource, DavResourceName::Cal);
    }

    #[tokio::test]
    async fn test_resource_resolution() {
        let request_pool = AsyncRequestPool::new(AsyncPoolConfig::default());
        let security = DavSecurity::new(SecurityConfig::default());
        let performance = DavPerformance::new(PerformanceConfig::default());
        let metrics = DavMetrics::new();

        let router = DavRouter::new(
            request_pool,
            security,
            performance,
            metrics,
            RouterConfig::default(),
        );

        // Test calendar resource
        assert_eq!(
            router.resolve_resource("/calendar/user/personal").unwrap(),
            DavResourceName::Cal
        );

        // Test addressbook resource
        assert_eq!(
            router.resolve_resource("/addressbook/user/contacts").unwrap(),
            DavResourceName::Card
        );

        // Test file resource
        assert_eq!(
            router.resolve_resource("/files/user/documents").unwrap(),
            DavResourceName::File
        );

        // Test principal resource
        assert_eq!(
            router.resolve_resource("/principal/user").unwrap(),
            DavResourceName::Principal
        );
    }
}
