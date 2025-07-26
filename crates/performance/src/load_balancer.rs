//! Load Balancer for A3Mailer
//!
//! This module provides intelligent load balancing with multiple algorithms,
//! health checking, and automatic failover capabilities.

use crate::{LoadBalancerConfig, Result, PerformanceError};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};

/// Backend server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backend {
    pub id: String,
    pub address: String,
    pub port: u16,
    pub weight: u32,
    pub is_healthy: bool,
    pub response_time_ms: f64,
    pub active_connections: u32,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub last_health_check: Option<Instant>,
}

impl Backend {
    /// Create a new backend
    pub fn new(id: String, address: String, port: u16, weight: u32) -> Self {
        Self {
            id,
            address,
            port,
            weight,
            is_healthy: true,
            response_time_ms: 0.0,
            active_connections: 0,
            total_requests: 0,
            failed_requests: 0,
            last_health_check: None,
        }
    }

    /// Get backend URL
    pub fn get_url(&self) -> String {
        format!("http://{}:{}", self.address, self.port)
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            1.0
        } else {
            1.0 - (self.failed_requests as f64 / self.total_requests as f64)
        }
    }

    /// Update backend statistics
    pub fn update_stats(&mut self, success: bool, response_time: Duration) {
        self.total_requests += 1;
        if !success {
            self.failed_requests += 1;
        }
        
        // Update response time with exponential moving average
        let response_time_ms = response_time.as_millis() as f64;
        if self.response_time_ms == 0.0 {
            self.response_time_ms = response_time_ms;
        } else {
            self.response_time_ms = (self.response_time_ms * 0.8) + (response_time_ms * 0.2);
        }
    }
}

/// Load balancing algorithms
#[derive(Debug, Clone)]
pub enum LoadBalancingAlgorithm {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    LeastResponseTime,
    IpHash,
    Random,
}

/// Round robin load balancer
#[derive(Debug)]
pub struct RoundRobinBalancer {
    current: AtomicUsize,
}

impl RoundRobinBalancer {
    pub fn new() -> Self {
        Self {
            current: AtomicUsize::new(0),
        }
    }

    pub fn select_backend(&self, backends: &[Arc<Backend>]) -> Option<Arc<Backend>> {
        if backends.is_empty() {
            return None;
        }

        let index = self.current.fetch_add(1, Ordering::Relaxed) % backends.len();
        Some(backends[index].clone())
    }
}

/// Least connections load balancer
#[derive(Debug)]
pub struct LeastConnectionsBalancer;

impl LeastConnectionsBalancer {
    pub fn new() -> Self {
        Self
    }

    pub fn select_backend(&self, backends: &[Arc<Backend>]) -> Option<Arc<Backend>> {
        backends.iter()
            .filter(|backend| backend.is_healthy)
            .min_by_key(|backend| backend.active_connections)
            .cloned()
    }
}

/// Weighted round robin load balancer
#[derive(Debug)]
pub struct WeightedRoundRobinBalancer {
    current_weights: RwLock<HashMap<String, i32>>,
}

impl WeightedRoundRobinBalancer {
    pub fn new() -> Self {
        Self {
            current_weights: RwLock::new(HashMap::new()),
        }
    }

    pub async fn select_backend(&self, backends: &[Arc<Backend>]) -> Option<Arc<Backend>> {
        if backends.is_empty() {
            return None;
        }

        let mut current_weights = self.current_weights.write().await;
        let mut selected_backend = None;
        let mut max_weight = i32::MIN;

        // Calculate total weight
        let total_weight: i32 = backends.iter()
            .filter(|backend| backend.is_healthy)
            .map(|backend| backend.weight as i32)
            .sum();

        if total_weight == 0 {
            return None;
        }

        // Update current weights and find the backend with maximum current weight
        for backend in backends.iter().filter(|b| b.is_healthy) {
            let current_weight = current_weights.entry(backend.id.clone()).or_insert(0);
            *current_weight += backend.weight as i32;

            if *current_weight > max_weight {
                max_weight = *current_weight;
                selected_backend = Some(backend.clone());
            }
        }

        // Decrease the selected backend's current weight
        if let Some(ref backend) = selected_backend {
            if let Some(weight) = current_weights.get_mut(&backend.id) {
                *weight -= total_weight;
            }
        }

        selected_backend
    }
}

/// Least response time load balancer
#[derive(Debug)]
pub struct LeastResponseTimeBalancer;

impl LeastResponseTimeBalancer {
    pub fn new() -> Self {
        Self
    }

    pub fn select_backend(&self, backends: &[Arc<Backend>]) -> Option<Arc<Backend>> {
        backends.iter()
            .filter(|backend| backend.is_healthy)
            .min_by(|a, b| {
                let a_score = a.response_time_ms * (a.active_connections as f64 + 1.0);
                let b_score = b.response_time_ms * (b.active_connections as f64 + 1.0);
                a_score.partial_cmp(&b_score).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }
}

/// Request trait for load balancing
pub trait BalancedRequest: Send + Sync {
    type Output: Send + Sync;
    
    async fn execute(&self, backend: &Backend) -> Result<Self::Output>;
    fn get_client_ip(&self) -> Option<String> { None }
}

/// Circuit breaker for backend health
#[derive(Debug)]
pub struct CircuitBreaker {
    failure_threshold: u32,
    recovery_timeout: Duration,
    failure_count: AtomicUsize,
    last_failure: RwLock<Option<Instant>>,
    state: RwLock<CircuitBreakerState>,
}

#[derive(Debug, Clone, PartialEq)]
enum CircuitBreakerState {
    Closed,  // Normal operation
    Open,    // Failing, reject requests
    HalfOpen, // Testing if backend recovered
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            recovery_timeout,
            failure_count: AtomicUsize::new(0),
            last_failure: RwLock::new(None),
            state: RwLock::new(CircuitBreakerState::Closed),
        }
    }

    pub async fn can_execute(&self) -> bool {
        let state = self.state.read().await;
        match *state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                // Check if we should transition to half-open
                if let Some(last_failure) = *self.last_failure.read().await {
                    if last_failure.elapsed() > self.recovery_timeout {
                        drop(state);
                        let mut state = self.state.write().await;
                        *state = CircuitBreakerState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen => true,
        }
    }

    pub async fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        let mut state = self.state.write().await;
        *state = CircuitBreakerState::Closed;
    }

    pub async fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        
        {
            let mut last_failure = self.last_failure.write().await;
            *last_failure = Some(Instant::now());
        }

        if failures >= self.failure_threshold as usize {
            let mut state = self.state.write().await;
            *state = CircuitBreakerState::Open;
        }
    }
}

/// Main load balancer
pub struct LoadBalancer {
    config: LoadBalancerConfig,
    backends: Arc<RwLock<Vec<Arc<Backend>>>>,
    algorithm: LoadBalancingAlgorithm,
    round_robin: RoundRobinBalancer,
    least_connections: LeastConnectionsBalancer,
    weighted_round_robin: WeightedRoundRobinBalancer,
    least_response_time: LeastResponseTimeBalancer,
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
}

impl LoadBalancer {
    /// Create a new load balancer
    pub async fn new(config: &LoadBalancerConfig) -> Result<Self> {
        info!("Initializing load balancer");

        let algorithm = match config.strategy.as_str() {
            "round_robin" => LoadBalancingAlgorithm::RoundRobin,
            "least_connections" => LoadBalancingAlgorithm::LeastConnections,
            "weighted_round_robin" => LoadBalancingAlgorithm::WeightedRoundRobin,
            "least_response_time" => LoadBalancingAlgorithm::LeastResponseTime,
            "ip_hash" => LoadBalancingAlgorithm::IpHash,
            "random" => LoadBalancingAlgorithm::Random,
            _ => LoadBalancingAlgorithm::RoundRobin,
        };

        let load_balancer = Self {
            config: config.clone(),
            backends: Arc::new(RwLock::new(Vec::new())),
            algorithm,
            round_robin: RoundRobinBalancer::new(),
            least_connections: LeastConnectionsBalancer::new(),
            weighted_round_robin: WeightedRoundRobinBalancer::new(),
            least_response_time: LeastResponseTimeBalancer::new(),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
        };

        // Initialize with default backends
        load_balancer.initialize_backends().await?;

        info!("Load balancer initialized with algorithm: {:?}", algorithm);
        Ok(load_balancer)
    }

    /// Initialize default backends
    async fn initialize_backends(&self) -> Result<()> {
        let mut backends = self.backends.write().await;
        
        // Add some default backends for demonstration
        backends.push(Arc::new(Backend::new(
            "backend1".to_string(),
            "127.0.0.1".to_string(),
            8001,
            100,
        )));
        
        backends.push(Arc::new(Backend::new(
            "backend2".to_string(),
            "127.0.0.1".to_string(),
            8002,
            100,
        )));

        Ok(())
    }

    /// Add a backend
    pub async fn add_backend(&self, backend: Backend) -> Result<()> {
        let mut backends = self.backends.write().await;
        backends.push(Arc::new(backend));
        
        info!("Added backend: {}", backends.last().unwrap().id);
        Ok(())
    }

    /// Remove a backend
    pub async fn remove_backend(&self, backend_id: &str) -> Result<bool> {
        let mut backends = self.backends.write().await;
        let initial_len = backends.len();
        
        backends.retain(|backend| backend.id != backend_id);
        
        let removed = backends.len() < initial_len;
        if removed {
            info!("Removed backend: {}", backend_id);
        }
        
        Ok(removed)
    }

    /// Execute a balanced request
    pub async fn execute_request<T>(&self, request: T) -> Result<T::Output>
    where
        T: BalancedRequest,
    {
        let backend = self.select_backend(&request).await?;
        
        // Check circuit breaker
        let circuit_breaker = self.get_or_create_circuit_breaker(&backend.id).await;
        if !circuit_breaker.can_execute().await {
            return Err(PerformanceError::LoadBalancerError(
                format!("Circuit breaker open for backend: {}", backend.id)
            ));
        }

        let start_time = Instant::now();
        
        match request.execute(&backend).await {
            Ok(result) => {
                let response_time = start_time.elapsed();
                self.update_backend_stats(&backend.id, true, response_time).await;
                circuit_breaker.record_success().await;
                Ok(result)
            }
            Err(e) => {
                let response_time = start_time.elapsed();
                self.update_backend_stats(&backend.id, false, response_time).await;
                circuit_breaker.record_failure().await;
                Err(e)
            }
        }
    }

    /// Select a backend using the configured algorithm
    async fn select_backend<T>(&self, request: &T) -> Result<Arc<Backend>>
    where
        T: BalancedRequest,
    {
        let backends = self.backends.read().await;
        let healthy_backends: Vec<Arc<Backend>> = backends.iter()
            .filter(|backend| backend.is_healthy)
            .cloned()
            .collect();

        if healthy_backends.is_empty() {
            return Err(PerformanceError::LoadBalancerError(
                "No healthy backends available".to_string()
            ));
        }

        let selected = match self.algorithm {
            LoadBalancingAlgorithm::RoundRobin => {
                self.round_robin.select_backend(&healthy_backends)
            }
            LoadBalancingAlgorithm::LeastConnections => {
                self.least_connections.select_backend(&healthy_backends)
            }
            LoadBalancingAlgorithm::WeightedRoundRobin => {
                self.weighted_round_robin.select_backend(&healthy_backends).await
            }
            LoadBalancingAlgorithm::LeastResponseTime => {
                self.least_response_time.select_backend(&healthy_backends)
            }
            LoadBalancingAlgorithm::IpHash => {
                // Simple IP hash implementation
                if let Some(client_ip) = request.get_client_ip() {
                    let hash = self.hash_string(&client_ip);
                    let index = hash % healthy_backends.len();
                    Some(healthy_backends[index].clone())
                } else {
                    self.round_robin.select_backend(&healthy_backends)
                }
            }
            LoadBalancingAlgorithm::Random => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let index = rng.gen_range(0..healthy_backends.len());
                Some(healthy_backends[index].clone())
            }
        };

        selected.ok_or_else(|| PerformanceError::LoadBalancerError(
            "Failed to select backend".to_string()
        ))
    }

    /// Update backend statistics
    async fn update_backend_stats(&self, backend_id: &str, success: bool, response_time: Duration) {
        let mut backends = self.backends.write().await;
        
        if let Some(backend) = backends.iter_mut().find(|b| b.id == backend_id) {
            let backend_mut = Arc::make_mut(backend);
            backend_mut.update_stats(success, response_time);
        }
    }

    /// Get or create circuit breaker for backend
    async fn get_or_create_circuit_breaker(&self, backend_id: &str) -> Arc<CircuitBreaker> {
        let mut circuit_breakers = self.circuit_breakers.write().await;
        
        circuit_breakers.entry(backend_id.to_string())
            .or_insert_with(|| CircuitBreaker::new(
                self.config.failover_threshold,
                Duration::from_secs(30)
            ))
            .clone()
    }

    /// Simple string hash function
    fn hash_string(&self, s: &str) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish() as usize
    }

    /// Get load balancer statistics
    pub async fn get_stats(&self) -> Result<HashMap<String, String>> {
        let backends = self.backends.read().await;
        let mut stats = HashMap::new();
        
        stats.insert("total_backends".to_string(), backends.len().to_string());
        
        let healthy_count = backends.iter().filter(|b| b.is_healthy).count();
        stats.insert("healthy_backends".to_string(), healthy_count.to_string());
        
        let total_requests: u64 = backends.iter().map(|b| b.total_requests).sum();
        stats.insert("total_requests".to_string(), total_requests.to_string());
        
        let total_failures: u64 = backends.iter().map(|b| b.failed_requests).sum();
        stats.insert("total_failures".to_string(), total_failures.to_string());
        
        let success_rate = if total_requests > 0 {
            ((total_requests - total_failures) as f64 / total_requests as f64) * 100.0
        } else {
            100.0
        };
        stats.insert("success_rate".to_string(), format!("{:.2}%", success_rate));
        
        Ok(stats)
    }

    /// Shutdown load balancer
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down load balancer");
        
        // Cleanup would go here
        
        info!("Load balancer shutdown complete");
        Ok(())
    }
}
