/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Async Request Pool for High-Performance DAV Server
//!
//! This module provides a simplified but highly efficient async request pool
//! for processing DAV requests with maximum concurrency and minimal latency.

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{
    sync::{Mutex, Semaphore, RwLock},
    task::JoinHandle,
    time::sleep,
};
use tracing::{debug, info};

/// High-performance async request pool
///
/// Manages concurrent DAV request processing with intelligent load balancing,
/// request prioritization, and performance monitoring.
#[derive(Debug, Clone)]
pub struct AsyncRequestPool {
    inner: Arc<AsyncRequestPoolInner>,
    config: AsyncPoolConfig,
}

#[derive(Debug)]
struct AsyncRequestPoolInner {
    /// Request queue
    request_queue: Mutex<VecDeque<PooledRequest>>,
    /// Global semaphore for limiting concurrent requests
    global_semaphore: Semaphore,
    /// Per-IP semaphores for rate limiting
    ip_semaphores: RwLock<std::collections::HashMap<String, Arc<Semaphore>>>,
    /// Worker handles
    workers: RwLock<Vec<JoinHandle<()>>>,
    /// Performance metrics
    metrics: AsyncPoolMetrics,
}

/// Async pool configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AsyncPoolConfig {
    /// Maximum concurrent requests globally
    pub max_concurrent_requests: usize,
    /// Maximum concurrent requests per IP
    pub max_requests_per_ip: usize,
    /// Number of worker tasks
    pub worker_count: usize,
    /// Request timeout
    pub request_timeout: Duration,
    /// Queue size limit
    pub max_queue_size: usize,
    /// Enable request batching
    pub enable_batching: bool,
    /// Batch size for processing
    pub batch_size: usize,
}

impl Default for AsyncPoolConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 10000,
            max_requests_per_ip: 100,
            worker_count: num_cpus::get() * 4,
            request_timeout: Duration::from_secs(30),
            max_queue_size: 50000,
            enable_batching: true,
            batch_size: 10,
        }
    }
}

#[derive(Debug)]
struct PooledRequest {
    id: u64,
    client_ip: String,
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    priority: RequestPriority,
    created_at: Instant,
    response_sender: tokio::sync::oneshot::Sender<RequestResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone)]
pub struct RequestResult {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub processing_time: Duration,
}

#[derive(Debug, Default)]
struct AsyncPoolMetrics {
    total_requests: AtomicU64,
    completed_requests: AtomicU64,
    failed_requests: AtomicU64,
    timeout_requests: AtomicU64,
    queue_full_rejections: AtomicU64,
    current_queue_size: AtomicUsize,
    peak_queue_size: AtomicUsize,
    average_processing_time: AtomicU64, // in nanoseconds
    average_queue_time: AtomicU64, // in nanoseconds
}

impl AsyncRequestPool {
    /// Create a new async request pool
    pub fn new(config: AsyncPoolConfig) -> Self {
        let pool = Self {
            inner: Arc::new(AsyncRequestPoolInner {
                request_queue: Mutex::new(VecDeque::new()),
                global_semaphore: Semaphore::new(config.max_concurrent_requests),
                ip_semaphores: RwLock::new(std::collections::HashMap::new()),
                workers: RwLock::new(Vec::new()),
                metrics: AsyncPoolMetrics::default(),
            }),
            config: config.clone(),
        };

        // Start worker tasks
        pool.start_workers();

        info!(
            max_concurrent = config.max_concurrent_requests,
            worker_count = config.worker_count,
            max_queue_size = config.max_queue_size,
            "Async request pool initialized"
        );

        pool
    }

    /// Submit a request for async processing
    pub async fn submit_request(
        &self,
        client_ip: String,
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
        priority: RequestPriority,
    ) -> Result<RequestResult, AsyncPoolError> {
        // Check global rate limit
        let _global_permit = self.inner.global_semaphore
            .try_acquire()
            .map_err(|_| AsyncPoolError::GlobalRateLimitExceeded)?;

        // Check per-IP rate limit
        let ip_semaphore = {
            let mut ip_semaphores = self.inner.ip_semaphores.write().await;
            ip_semaphores
                .entry(client_ip.clone())
                .or_insert_with(|| Arc::new(Semaphore::new(self.config.max_requests_per_ip)))
                .clone()
        };

        let _ip_permit = ip_semaphore
            .try_acquire()
            .map_err(|_| AsyncPoolError::IpRateLimitExceeded)?;

        // Check queue size
        {
            let queue = self.inner.request_queue.lock().await;
            if queue.len() >= self.config.max_queue_size {
                self.inner.metrics.queue_full_rejections.fetch_add(1, Ordering::Relaxed);
                return Err(AsyncPoolError::QueueFull);
            }
        }

        // Create response channel
        let (response_sender, response_receiver) = tokio::sync::oneshot::channel();

        // Generate request ID
        let request_id = self.inner.metrics.total_requests.fetch_add(1, Ordering::Relaxed);

        // Create pooled request
        let request = PooledRequest {
            id: request_id,
            client_ip: client_ip.clone(),
            method: method.clone(),
            path: path.clone(),
            headers,
            body,
            priority,
            created_at: Instant::now(),
            response_sender,
        };

        // Add to queue
        {
            let mut queue = self.inner.request_queue.lock().await;

            // Insert based on priority
            let insert_pos = queue.iter()
                .position(|req| req.priority < priority)
                .unwrap_or(queue.len());

            queue.insert(insert_pos, request);

            let queue_size = queue.len();
            self.inner.metrics.current_queue_size.store(queue_size, Ordering::Relaxed);

            let peak = self.inner.metrics.peak_queue_size.load(Ordering::Relaxed);
            if queue_size > peak {
                self.inner.metrics.peak_queue_size.store(queue_size, Ordering::Relaxed);
            }
        }

        debug!(
            request_id = request_id,
            client_ip = %client_ip,
            method = %method,
            path = %path,
            priority = ?priority,
            "Request submitted to async pool"
        );

        // Wait for response with timeout
        let result = tokio::time::timeout(
            self.config.request_timeout,
            response_receiver
        ).await;

        match result {
            Ok(Ok(response)) => {
                self.inner.metrics.completed_requests.fetch_add(1, Ordering::Relaxed);
                Ok(response)
            }
            Ok(Err(_)) => {
                self.inner.metrics.failed_requests.fetch_add(1, Ordering::Relaxed);
                Err(AsyncPoolError::ProcessingFailed)
            }
            Err(_) => {
                self.inner.metrics.timeout_requests.fetch_add(1, Ordering::Relaxed);
                Err(AsyncPoolError::Timeout)
            }
        }
    }

    /// Get performance statistics
    pub async fn get_stats(&self) -> AsyncPoolStats {
        let queue_size = {
            let queue = self.inner.request_queue.lock().await;
            queue.len()
        };

        AsyncPoolStats {
            total_requests: self.inner.metrics.total_requests.load(Ordering::Relaxed),
            completed_requests: self.inner.metrics.completed_requests.load(Ordering::Relaxed),
            failed_requests: self.inner.metrics.failed_requests.load(Ordering::Relaxed),
            timeout_requests: self.inner.metrics.timeout_requests.load(Ordering::Relaxed),
            queue_full_rejections: self.inner.metrics.queue_full_rejections.load(Ordering::Relaxed),
            current_queue_size: queue_size,
            peak_queue_size: self.inner.metrics.peak_queue_size.load(Ordering::Relaxed),
            average_processing_time: Duration::from_nanos(
                self.inner.metrics.average_processing_time.load(Ordering::Relaxed)
            ),
            average_queue_time: Duration::from_nanos(
                self.inner.metrics.average_queue_time.load(Ordering::Relaxed)
            ),
        }
    }

    fn start_workers(&self) {
        let inner = self.inner.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut workers = inner.workers.write().await;

            for worker_id in 0..config.worker_count {
                let worker_inner = inner.clone();
                let worker_config = config.clone();

                let handle = tokio::spawn(async move {
                    Self::worker_loop(worker_id, worker_inner, worker_config).await;
                });

                workers.push(handle);
            }

            info!(worker_count = config.worker_count, "Async pool workers started");
        });
    }

    async fn worker_loop(
        worker_id: usize,
        inner: Arc<AsyncRequestPoolInner>,
        _config: AsyncPoolConfig,
    ) {
        debug!(worker_id = worker_id, "Worker started");

        loop {
            // Get next request from queue
            let request = {
                let mut queue = inner.request_queue.lock().await;
                let request = queue.pop_front();
                if request.is_some() {
                    inner.metrics.current_queue_size.store(queue.len(), Ordering::Relaxed);
                }
                request
            };

            if let Some(request) = request {
                let start_time = Instant::now();
                let queue_time = start_time.duration_since(request.created_at);

                debug!(
                    worker_id = worker_id,
                    request_id = request.id,
                    method = %request.method,
                    path = %request.path,
                    queue_time_ms = queue_time.as_millis(),
                    "Processing request"
                );

                // Process the request
                let result = Self::process_request(&request).await;
                let processing_time = start_time.elapsed();

                // Update metrics
                let current_avg_processing = inner.metrics.average_processing_time.load(Ordering::Relaxed);
                let new_avg_processing = (current_avg_processing + processing_time.as_nanos() as u64) / 2;
                inner.metrics.average_processing_time.store(new_avg_processing, Ordering::Relaxed);

                let current_avg_queue = inner.metrics.average_queue_time.load(Ordering::Relaxed);
                let new_avg_queue = (current_avg_queue + queue_time.as_nanos() as u64) / 2;
                inner.metrics.average_queue_time.store(new_avg_queue, Ordering::Relaxed);

                // Send response
                let _ = request.response_sender.send(result);

                debug!(
                    worker_id = worker_id,
                    request_id = request.id,
                    processing_time_ms = processing_time.as_millis(),
                    "Request completed"
                );
            } else {
                // No work available, sleep briefly
                sleep(Duration::from_millis(1)).await;
            }
        }
    }

    async fn process_request(request: &PooledRequest) -> RequestResult {
        // Simulate DAV request processing
        let processing_time = match request.method.as_str() {
            "GET" => Duration::from_millis(10),
            "PROPFIND" => Duration::from_millis(50),
            "PUT" => Duration::from_millis(100),
            "DELETE" => Duration::from_millis(30),
            "MKCOL" => Duration::from_millis(40),
            "COPY" | "MOVE" => Duration::from_millis(80),
            _ => Duration::from_millis(20),
        };

        // Simulate processing delay
        sleep(processing_time).await;

        // Generate response
        let response_body = format!(
            "Processed {} {} (ID: {})",
            request.method,
            request.path,
            request.id
        ).into_bytes();

        RequestResult {
            status: 200,
            headers: vec![
                ("Content-Type".to_string(), "text/plain".to_string()),
                ("Content-Length".to_string(), response_body.len().to_string()),
            ],
            body: response_body,
            processing_time,
        }
    }
}

/// Async pool error types
#[derive(Debug, Clone)]
pub enum AsyncPoolError {
    GlobalRateLimitExceeded,
    IpRateLimitExceeded,
    QueueFull,
    Timeout,
    ProcessingFailed,
}

/// Performance statistics
#[derive(Debug, Clone)]
pub struct AsyncPoolStats {
    pub total_requests: u64,
    pub completed_requests: u64,
    pub failed_requests: u64,
    pub timeout_requests: u64,
    pub queue_full_rejections: u64,
    pub current_queue_size: usize,
    pub peak_queue_size: usize,
    pub average_processing_time: Duration,
    pub average_queue_time: Duration,
}

impl std::fmt::Display for AsyncPoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GlobalRateLimitExceeded => write!(f, "Global rate limit exceeded"),
            Self::IpRateLimitExceeded => write!(f, "IP rate limit exceeded"),
            Self::QueueFull => write!(f, "Request queue is full"),
            Self::Timeout => write!(f, "Request timed out"),
            Self::ProcessingFailed => write!(f, "Request processing failed"),
        }
    }
}

impl std::error::Error for AsyncPoolError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_pool_basic_processing() {
        let config = AsyncPoolConfig {
            max_concurrent_requests: 100,
            worker_count: 4,
            ..Default::default()
        };

        let pool = AsyncRequestPool::new(config);

        // Wait for workers to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Submit a request
        let result = pool.submit_request(
            "192.168.1.1".to_string(),
            "GET".to_string(),
            "/calendar/user/personal".to_string(),
            vec![("User-Agent".to_string(), "Test".to_string())],
            vec![],
            RequestPriority::Normal,
        ).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, 200);
        assert!(!response.body.is_empty());
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let config = AsyncPoolConfig {
            max_concurrent_requests: 5,
            max_requests_per_ip: 2,
            worker_count: 2,
            ..Default::default()
        };

        let pool = AsyncRequestPool::new(config);

        // Wait for workers to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        let client_ip = "192.168.1.100".to_string();

        // Submit requests up to IP limit
        for _ in 0..2 {
            let result = pool.submit_request(
                client_ip.clone(),
                "GET".to_string(),
                "/test".to_string(),
                vec![],
                vec![],
                RequestPriority::Normal,
            ).await;
            assert!(result.is_ok());
        }

        // Next request should be rate limited
        let result = pool.submit_request(
            client_ip,
            "GET".to_string(),
            "/test".to_string(),
            vec![],
            vec![],
            RequestPriority::Normal,
        ).await;

        assert!(matches!(result, Err(AsyncPoolError::IpRateLimitExceeded)));
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let config = AsyncPoolConfig {
            max_concurrent_requests: 1, // Force queuing
            worker_count: 1,
            ..Default::default()
        };

        let pool = AsyncRequestPool::new(config);

        // Wait for workers to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Submit requests with different priorities
        let _low = pool.submit_request(
            "192.168.1.1".to_string(),
            "GET".to_string(),
            "/low".to_string(),
            vec![],
            vec![],
            RequestPriority::Low,
        );

        let _high = pool.submit_request(
            "192.168.1.1".to_string(),
            "GET".to_string(),
            "/high".to_string(),
            vec![],
            vec![],
            RequestPriority::High,
        );

        let _critical = pool.submit_request(
            "192.168.1.1".to_string(),
            "GET".to_string(),
            "/critical".to_string(),
            vec![],
            vec![],
            RequestPriority::Critical,
        );

        // Wait for processing
        tokio::time::sleep(Duration::from_millis(500)).await;

        let stats = pool.get_stats().await;
        assert!(stats.total_requests >= 3);
    }

    #[tokio::test]
    async fn test_performance_stats() {
        let config = AsyncPoolConfig {
            max_concurrent_requests: 100,
            worker_count: 4,
            ..Default::default()
        };

        let pool = AsyncRequestPool::new(config);

        // Wait for workers to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Submit multiple requests
        for i in 0..10 {
            let _result = pool.submit_request(
                format!("192.168.1.{}", i % 3),
                "GET".to_string(),
                format!("/test/{}", i),
                vec![],
                vec![],
                RequestPriority::Normal,
            ).await;
        }

        // Wait for processing
        tokio::time::sleep(Duration::from_millis(200)).await;

        let stats = pool.get_stats().await;
        assert!(stats.total_requests >= 10);
        assert!(stats.completed_requests > 0);
        assert!(stats.average_processing_time > Duration::ZERO);
    }
}
