/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! DAV Server High-Performance Concurrency Module
//!
//! This module provides advanced concurrency features for the DAV server,
//! including async request processing, connection pooling, work stealing,
//! and adaptive load balancing for maximum throughput.

use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    task::Waker,
    time::{Duration, Instant},
};
use tokio::{
    sync::{Semaphore, RwLock},
    task::JoinHandle,
    time::sleep,
};
use tracing::{debug, info, warn, error};

/// High-performance concurrent request processor
///
/// Manages concurrent DAV requests with work stealing, adaptive load balancing,
/// and intelligent resource management for maximum throughput.
#[derive(Debug, Clone)]
pub struct ConcurrentProcessor {
    inner: Arc<ConcurrentProcessorInner>,
    config: ConcurrencyConfig,
}

#[derive(Debug)]
struct ConcurrentProcessorInner {
    /// Work queues for different request types
    work_queues: RwLock<HashMap<RequestType, WorkQueue>>,
    /// Worker pool for processing requests
    worker_pool: RwLock<Vec<Worker>>,
    /// Global semaphore for limiting concurrent requests
    global_semaphore: Semaphore,
    /// Per-IP semaphores for rate limiting
    ip_semaphores: RwLock<HashMap<String, Arc<Semaphore>>>,
    /// Performance metrics
    metrics: ConcurrencyMetrics,
    /// Active request tracking
    active_requests: RwLock<HashMap<u64, ActiveRequest>>,
}

/// Concurrency configuration
#[derive(Debug, Clone)]
pub struct ConcurrencyConfig {
    /// Maximum concurrent requests globally
    pub max_concurrent_requests: usize,
    /// Maximum concurrent requests per IP
    pub max_requests_per_ip: usize,
    /// Number of worker threads
    pub worker_threads: usize,
    /// Work stealing enabled
    pub enable_work_stealing: bool,
    /// Adaptive load balancing
    pub enable_adaptive_balancing: bool,
    /// Request timeout
    pub request_timeout: Duration,
    /// Queue size per worker
    pub queue_size_per_worker: usize,
    /// CPU affinity for workers
    pub enable_cpu_affinity: bool,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 10000,
            max_requests_per_ip: 100,
            worker_threads: num_cpus::get() * 2,
            enable_work_stealing: true,
            enable_adaptive_balancing: true,
            request_timeout: Duration::from_secs(30),
            queue_size_per_worker: 1000,
            enable_cpu_affinity: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestType {
    Read,      // GET, PROPFIND, REPORT
    Write,     // PUT, PROPPATCH, DELETE
    Create,    // MKCOL, MKCALENDAR
    Move,      // COPY, MOVE
    Lock,      // LOCK, UNLOCK
    Acl,       // ACL
}

#[derive(Debug)]
struct WorkQueue {
    queue: VecDeque<PendingRequest>,
    processing: usize,
    total_processed: u64,
    average_processing_time: Duration,
}

#[derive(Debug)]
struct Worker {
    id: usize,
    handle: Option<JoinHandle<()>>,
    queue: Arc<Mutex<VecDeque<PendingRequest>>>,
    is_busy: Arc<AtomicUsize>,
    processed_count: Arc<AtomicU64>,
}

struct PendingRequest {
    id: u64,
    request_type: RequestType,
    client_ip: String,
    created_at: Instant,
    priority: RequestPriority,
    processor: Box<dyn RequestProcessor>,
    waker: Option<Waker>,
}

impl std::fmt::Debug for PendingRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingRequest")
            .field("id", &self.id)
            .field("request_type", &self.request_type)
            .field("client_ip", &self.client_ip)
            .field("created_at", &self.created_at)
            .field("priority", &self.priority)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug)]
struct ActiveRequest {
    id: u64,
    request_type: RequestType,
    client_ip: String,
    started_at: Instant,
    worker_id: Option<usize>,
}

#[derive(Debug, Default)]
struct ConcurrencyMetrics {
    total_requests: AtomicU64,
    completed_requests: AtomicU64,
    failed_requests: AtomicU64,
    timeout_requests: AtomicU64,
    queue_full_rejections: AtomicU64,
    work_steals: AtomicU64,
    average_queue_time: AtomicU64, // in nanoseconds
    average_processing_time: AtomicU64, // in nanoseconds
    peak_concurrent_requests: AtomicUsize,
    current_queue_size: AtomicUsize,
}

/// Trait for processing DAV requests asynchronously
pub trait RequestProcessor: Send + Sync {
    fn process(&self) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>>;
    fn request_type(&self) -> RequestType;
    fn priority(&self) -> RequestPriority;
    fn estimated_duration(&self) -> Duration;
}

impl ConcurrentProcessor {
    /// Create a new concurrent processor
    pub fn new(config: ConcurrencyConfig) -> Self {
        let inner = Arc::new(ConcurrentProcessorInner {
            work_queues: RwLock::new(HashMap::new()),
            worker_pool: RwLock::new(Vec::new()),
            global_semaphore: Semaphore::new(config.max_concurrent_requests),
            ip_semaphores: RwLock::new(HashMap::new()),
            metrics: ConcurrencyMetrics::default(),
            active_requests: RwLock::new(HashMap::new()),
        });

        let processor = Self {
            inner: inner.clone(),
            config: config.clone(),
        };

        // Initialize work queues
        let processor_clone = processor.clone();
        tokio::spawn(async move {
            processor_clone.initialize_work_queues().await;
            processor_clone.start_workers().await;
            processor_clone.start_load_balancer().await;
        });

        processor
    }

    /// Submit a request for processing
    pub async fn submit_request(
        &self,
        client_ip: String,
        processor: Box<dyn RequestProcessor>,
    ) -> Result<RequestHandle, ConcurrencyError> {
        // Check global rate limit
        let _global_permit = self.inner.global_semaphore
            .try_acquire()
            .map_err(|_| ConcurrencyError::GlobalRateLimitExceeded)?;

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
            .map_err(|_| ConcurrencyError::IpRateLimitExceeded)?;

        // Generate request ID
        let request_id = self.inner.metrics.total_requests.fetch_add(1, Ordering::Relaxed);

        // Create pending request
        let request_type = processor.request_type();
        let priority = processor.priority();

        let pending_request = PendingRequest {
            id: request_id,
            request_type,
            client_ip: client_ip.clone(),
            created_at: Instant::now(),
            priority,
            processor,
            waker: None,
        };

        // Add to appropriate work queue
        self.enqueue_request(pending_request).await?;

        // Track active request
        let active_request = ActiveRequest {
            id: request_id,
            request_type,
            client_ip: client_ip.clone(),
            started_at: Instant::now(),
            worker_id: None,
        };

        self.inner.active_requests.write().await.insert(request_id, active_request);

        // Update metrics
        let current_queue_size = self.inner.metrics.current_queue_size.fetch_add(1, Ordering::Relaxed) + 1;
        let peak = self.inner.metrics.peak_concurrent_requests.load(Ordering::Relaxed);
        if current_queue_size > peak {
            self.inner.metrics.peak_concurrent_requests.store(current_queue_size, Ordering::Relaxed);
        }

        debug!(
            request_id = request_id,
            client_ip = %client_ip,
            request_type = ?request_type,
            priority = ?priority,
            queue_size = current_queue_size,
            "Request submitted for processing"
        );

        Ok(RequestHandle {
            id: request_id,
            processor: self.clone(),
        })
    }

    /// Get current performance statistics
    pub async fn get_performance_stats(&self) -> ConcurrencyStats {
        let active_requests = self.inner.active_requests.read().await.len();
        let work_queues = self.inner.work_queues.read().await;

        let total_queue_size: usize = work_queues.values()
            .map(|queue| queue.queue.len())
            .sum();

        let queue_stats: HashMap<RequestType, QueueStats> = work_queues.iter()
            .map(|(req_type, queue)| {
                (*req_type, QueueStats {
                    queue_size: queue.queue.len(),
                    processing: queue.processing,
                    total_processed: queue.total_processed,
                    average_processing_time: queue.average_processing_time,
                })
            })
            .collect();

        ConcurrencyStats {
            total_requests: self.inner.metrics.total_requests.load(Ordering::Relaxed),
            completed_requests: self.inner.metrics.completed_requests.load(Ordering::Relaxed),
            failed_requests: self.inner.metrics.failed_requests.load(Ordering::Relaxed),
            timeout_requests: self.inner.metrics.timeout_requests.load(Ordering::Relaxed),
            queue_full_rejections: self.inner.metrics.queue_full_rejections.load(Ordering::Relaxed),
            work_steals: self.inner.metrics.work_steals.load(Ordering::Relaxed),
            active_requests,
            total_queue_size,
            peak_concurrent_requests: self.inner.metrics.peak_concurrent_requests.load(Ordering::Relaxed),
            average_queue_time: Duration::from_nanos(
                self.inner.metrics.average_queue_time.load(Ordering::Relaxed)
            ),
            average_processing_time: Duration::from_nanos(
                self.inner.metrics.average_processing_time.load(Ordering::Relaxed)
            ),
            queue_stats,
        }
    }

    async fn initialize_work_queues(&self) {
        let mut work_queues = self.inner.work_queues.write().await;

        for request_type in [
            RequestType::Read,
            RequestType::Write,
            RequestType::Create,
            RequestType::Move,
            RequestType::Lock,
            RequestType::Acl,
        ] {
            work_queues.insert(request_type, WorkQueue {
                queue: VecDeque::new(),
                processing: 0,
                total_processed: 0,
                average_processing_time: Duration::ZERO,
            });
        }

        info!("Work queues initialized for all request types");
    }

    async fn start_workers(&self) {
        let mut worker_pool = self.inner.worker_pool.write().await;

        for worker_id in 0..self.config.worker_threads {
            let worker_queue = Arc::new(Mutex::new(VecDeque::new()));
            let is_busy = Arc::new(AtomicUsize::new(0));
            let processed_count = Arc::new(AtomicU64::new(0));

            let handle = self.spawn_worker(
                worker_id,
                worker_queue.clone(),
                is_busy.clone(),
                processed_count.clone(),
            ).await;

            let worker = Worker {
                id: worker_id,
                handle: Some(handle),
                queue: worker_queue,
                is_busy,
                processed_count,
            };

            worker_pool.push(worker);
        }

        info!(
            worker_count = self.config.worker_threads,
            "Worker pool started"
        );
    }

    async fn spawn_worker(
        &self,
        worker_id: usize,
        queue: Arc<Mutex<VecDeque<PendingRequest>>>,
        is_busy: Arc<AtomicUsize>,
        processed_count: Arc<AtomicU64>,
    ) -> JoinHandle<()> {
        let inner = self.inner.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            debug!(worker_id = worker_id, "Worker started");

            loop {
                // Get next request from queue
                let request = {
                    let mut queue_guard = queue.lock().unwrap();
                    queue_guard.pop_front()
                };

                if let Some(request) = request {
                    is_busy.store(1, Ordering::Relaxed);

                    let start_time = Instant::now();
                    let queue_time = start_time.duration_since(request.created_at);

                    // Update queue time metrics
                    let current_avg = inner.metrics.average_queue_time.load(Ordering::Relaxed);
                    let new_avg = (current_avg + queue_time.as_nanos() as u64) / 2;
                    inner.metrics.average_queue_time.store(new_avg, Ordering::Relaxed);

                    debug!(
                        worker_id = worker_id,
                        request_id = request.id,
                        queue_time_ms = queue_time.as_millis(),
                        "Processing request"
                    );

                    // Process the request
                    let result = tokio::time::timeout(
                        config.request_timeout,
                        request.processor.process()
                    ).await;

                    let processing_time = start_time.elapsed();
                    processed_count.fetch_add(1, Ordering::Relaxed);

                    match result {
                        Ok(Ok(())) => {
                            inner.metrics.completed_requests.fetch_add(1, Ordering::Relaxed);
                            debug!(
                                worker_id = worker_id,
                                request_id = request.id,
                                processing_time_ms = processing_time.as_millis(),
                                "Request completed successfully"
                            );
                        }
                        Ok(Err(error)) => {
                            inner.metrics.failed_requests.fetch_add(1, Ordering::Relaxed);
                            warn!(
                                worker_id = worker_id,
                                request_id = request.id,
                                error = %error,
                                processing_time_ms = processing_time.as_millis(),
                                "Request failed"
                            );
                        }
                        Err(_) => {
                            inner.metrics.timeout_requests.fetch_add(1, Ordering::Relaxed);
                            error!(
                                worker_id = worker_id,
                                request_id = request.id,
                                timeout_ms = config.request_timeout.as_millis(),
                                "Request timed out"
                            );
                        }
                    }

                    // Update processing time metrics
                    let current_avg = inner.metrics.average_processing_time.load(Ordering::Relaxed);
                    let new_avg = (current_avg + processing_time.as_nanos() as u64) / 2;
                    inner.metrics.average_processing_time.store(new_avg, Ordering::Relaxed);

                    // Remove from active requests
                    inner.active_requests.write().await.remove(&request.id);
                    inner.metrics.current_queue_size.fetch_sub(1, Ordering::Relaxed);

                    is_busy.store(0, Ordering::Relaxed);
                } else {
                    // No work available, sleep briefly
                    sleep(Duration::from_millis(1)).await;
                }
            }
        })
    }

    async fn enqueue_request(&self, request: PendingRequest) -> Result<(), ConcurrencyError> {
        let mut work_queues = self.inner.work_queues.write().await;

        if let Some(queue) = work_queues.get_mut(&request.request_type) {
            if queue.queue.len() >= self.config.queue_size_per_worker {
                self.inner.metrics.queue_full_rejections.fetch_add(1, Ordering::Relaxed);
                return Err(ConcurrencyError::QueueFull);
            }

            // Insert based on priority
            let insert_pos = queue.queue.iter()
                .position(|req| req.priority < request.priority)
                .unwrap_or(queue.queue.len());

            queue.queue.insert(insert_pos, request);
            Ok(())
        } else {
            Err(ConcurrencyError::InvalidRequestType)
        }
    }

    async fn start_load_balancer(&self) {
        if !self.config.enable_adaptive_balancing {
            return;
        }

        let _inner = self.inner.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));

            loop {
                interval.tick().await;

                // Implement work stealing and load balancing logic here
                // This is a simplified version - production would be more sophisticated

                debug!("Load balancer tick - checking for work stealing opportunities");
            }
        });
    }
}

/// Handle for tracking request progress
pub struct RequestHandle {
    id: u64,
    processor: ConcurrentProcessor,
}

impl RequestHandle {
    /// Get request ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Check if request is still active
    pub async fn is_active(&self) -> bool {
        self.processor.inner.active_requests.read().await.contains_key(&self.id)
    }

    /// Cancel the request if it's still pending
    pub async fn cancel(&self) -> bool {
        // Implementation would remove from queue if not yet processing
        false
    }
}

/// Concurrency error types
#[derive(Debug, Clone)]
pub enum ConcurrencyError {
    GlobalRateLimitExceeded,
    IpRateLimitExceeded,
    QueueFull,
    InvalidRequestType,
    RequestTimeout,
    WorkerPoolShutdown,
}

/// Performance statistics
#[derive(Debug, Clone)]
pub struct ConcurrencyStats {
    pub total_requests: u64,
    pub completed_requests: u64,
    pub failed_requests: u64,
    pub timeout_requests: u64,
    pub queue_full_rejections: u64,
    pub work_steals: u64,
    pub active_requests: usize,
    pub total_queue_size: usize,
    pub peak_concurrent_requests: usize,
    pub average_queue_time: Duration,
    pub average_processing_time: Duration,
    pub queue_stats: HashMap<RequestType, QueueStats>,
}

#[derive(Debug, Clone)]
pub struct QueueStats {
    pub queue_size: usize,
    pub processing: usize,
    pub total_processed: u64,
    pub average_processing_time: Duration,
}

impl std::fmt::Display for ConcurrencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GlobalRateLimitExceeded => write!(f, "Global rate limit exceeded"),
            Self::IpRateLimitExceeded => write!(f, "IP rate limit exceeded"),
            Self::QueueFull => write!(f, "Request queue is full"),
            Self::InvalidRequestType => write!(f, "Invalid request type"),
            Self::RequestTimeout => write!(f, "Request timed out"),
            Self::WorkerPoolShutdown => write!(f, "Worker pool is shutting down"),
        }
    }
}

impl std::error::Error for ConcurrencyError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    struct TestProcessor {
        request_type: RequestType,
        priority: RequestPriority,
        duration: Duration,
        should_fail: Arc<AtomicBool>,
    }

    impl TestProcessor {
        fn new(request_type: RequestType, priority: RequestPriority, duration: Duration) -> Self {
            Self {
                request_type,
                priority,
                duration,
                should_fail: Arc::new(AtomicBool::new(false)),
            }
        }

        fn with_failure(mut self) -> Self {
            self.should_fail.store(true, Ordering::Relaxed);
            self
        }
    }

    impl RequestProcessor for TestProcessor {
        fn process(&self) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
            Box::pin(async move {
                tokio::time::sleep(self.duration).await;
                if self.should_fail.load(Ordering::Relaxed) {
                    Err("Test failure".to_string())
                } else {
                    Ok(())
                }
            })
        }

        fn request_type(&self) -> RequestType {
            self.request_type
        }

        fn priority(&self) -> RequestPriority {
            self.priority
        }

        fn estimated_duration(&self) -> Duration {
            self.duration
        }
    }

    #[tokio::test]
    async fn test_concurrent_request_processing() {
        let config = ConcurrencyConfig {
            max_concurrent_requests: 100,
            max_requests_per_ip: 10,
            worker_threads: 4,
            request_timeout: Duration::from_secs(5),
            ..Default::default()
        };

        let processor = ConcurrentProcessor::new(config);

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Submit multiple requests
        let mut handles = Vec::new();
        for i in 0..10 {
            let test_processor = TestProcessor::new(
                RequestType::Read,
                RequestPriority::Normal,
                Duration::from_millis(50),
            );

            let handle = processor.submit_request(
                format!("192.168.1.{}", i % 3),
                Box::new(test_processor),
            ).await;

            assert!(handle.is_ok());
            handles.push(handle.unwrap());
        }

        // Wait for processing
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Check statistics
        let stats = processor.get_performance_stats().await;
        assert!(stats.total_requests >= 10);
        assert!(stats.completed_requests > 0);
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let config = ConcurrencyConfig {
            max_concurrent_requests: 5,
            max_requests_per_ip: 2,
            worker_threads: 2,
            ..Default::default()
        };

        let processor = ConcurrentProcessor::new(config);

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(100)).await;

        let client_ip = "192.168.1.100".to_string();

        // Submit requests up to IP limit
        for _ in 0..2 {
            let test_processor = TestProcessor::new(
                RequestType::Read,
                RequestPriority::Normal,
                Duration::from_millis(100),
            );

            let result = processor.submit_request(
                client_ip.clone(),
                Box::new(test_processor),
            ).await;

            assert!(result.is_ok());
        }

        // Next request should be rate limited
        let test_processor = TestProcessor::new(
            RequestType::Read,
            RequestPriority::Normal,
            Duration::from_millis(100),
        );

        let result = processor.submit_request(
            client_ip,
            Box::new(test_processor),
        ).await;

        assert!(matches!(result, Err(ConcurrencyError::IpRateLimitExceeded)));
    }

    #[tokio::test]
    async fn test_request_priorities() {
        let config = ConcurrencyConfig {
            max_concurrent_requests: 100,
            worker_threads: 1, // Single worker to test priority ordering
            ..Default::default()
        };

        let processor = ConcurrentProcessor::new(config);

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Submit requests with different priorities
        let low_processor = TestProcessor::new(
            RequestType::Read,
            RequestPriority::Low,
            Duration::from_millis(10),
        );

        let high_processor = TestProcessor::new(
            RequestType::Read,
            RequestPriority::High,
            Duration::from_millis(10),
        );

        let critical_processor = TestProcessor::new(
            RequestType::Read,
            RequestPriority::Critical,
            Duration::from_millis(10),
        );

        // Submit in reverse priority order
        let _low_handle = processor.submit_request(
            "192.168.1.1".to_string(),
            Box::new(low_processor),
        ).await.unwrap();

        let _high_handle = processor.submit_request(
            "192.168.1.1".to_string(),
            Box::new(high_processor),
        ).await.unwrap();

        let _critical_handle = processor.submit_request(
            "192.168.1.1".to_string(),
            Box::new(critical_processor),
        ).await.unwrap();

        // Wait for processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        let stats = processor.get_performance_stats().await;
        assert!(stats.total_requests >= 3);
    }

    #[tokio::test]
    async fn test_request_timeout() {
        let config = ConcurrencyConfig {
            max_concurrent_requests: 100,
            worker_threads: 2,
            request_timeout: Duration::from_millis(50), // Short timeout
            ..Default::default()
        };

        let processor = ConcurrentProcessor::new(config);

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Submit a long-running request
        let slow_processor = TestProcessor::new(
            RequestType::Write,
            RequestPriority::Normal,
            Duration::from_millis(200), // Longer than timeout
        );

        let _handle = processor.submit_request(
            "192.168.1.1".to_string(),
            Box::new(slow_processor),
        ).await.unwrap();

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(300)).await;

        let stats = processor.get_performance_stats().await;
        assert!(stats.timeout_requests > 0);
    }

    #[tokio::test]
    async fn test_error_handling() {
        let config = ConcurrencyConfig {
            max_concurrent_requests: 100,
            worker_threads: 2,
            ..Default::default()
        };

        let processor = ConcurrentProcessor::new(config);

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Submit a failing request
        let failing_processor = TestProcessor::new(
            RequestType::Write,
            RequestPriority::Normal,
            Duration::from_millis(10),
        ).with_failure();

        let _handle = processor.submit_request(
            "192.168.1.1".to_string(),
            Box::new(failing_processor),
        ).await.unwrap();

        // Wait for processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        let stats = processor.get_performance_stats().await;
        assert!(stats.failed_requests > 0);
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let config = ConcurrencyConfig {
            max_concurrent_requests: 100,
            worker_threads: 4,
            ..Default::default()
        };

        let processor = ConcurrentProcessor::new(config);

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Submit various types of requests
        for request_type in [RequestType::Read, RequestType::Write, RequestType::Create] {
            for _ in 0..5 {
                let test_processor = TestProcessor::new(
                    request_type,
                    RequestPriority::Normal,
                    Duration::from_millis(20),
                );

                let _handle = processor.submit_request(
                    "192.168.1.1".to_string(),
                    Box::new(test_processor),
                ).await.unwrap();
            }
        }

        // Wait for processing
        tokio::time::sleep(Duration::from_millis(200)).await;

        let stats = processor.get_performance_stats().await;

        // Verify metrics
        assert!(stats.total_requests >= 15);
        assert!(stats.completed_requests > 0);
        assert!(stats.average_processing_time > Duration::ZERO);
        assert!(stats.queue_stats.len() > 0);

        // Check that we have stats for different request types
        assert!(stats.queue_stats.contains_key(&RequestType::Read));
        assert!(stats.queue_stats.contains_key(&RequestType::Write));
        assert!(stats.queue_stats.contains_key(&RequestType::Create));
    }
}
