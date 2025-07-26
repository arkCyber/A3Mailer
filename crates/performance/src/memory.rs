//! Memory Management for A3Mailer
//!
//! This module provides intelligent memory management with garbage collection,
//! memory monitoring, and optimization capabilities.

use crate::{MemoryConfig, Result, PerformanceError};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};

/// Memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub usage_percent: f64,
    pub heap_size_bytes: u64,
    pub gc_count: u64,
    pub last_gc_duration_ms: u64,
    pub fragmentation_percent: f64,
}

/// Memory pool for efficient allocation
#[derive(Debug)]
pub struct MemoryPool {
    pool_size: usize,
    block_size: usize,
    allocated_blocks: Vec<bool>,
    free_blocks: Vec<usize>,
    total_allocations: u64,
    total_deallocations: u64,
}

impl MemoryPool {
    /// Create a new memory pool
    pub fn new(pool_size: usize, block_size: usize) -> Self {
        let num_blocks = pool_size / block_size;
        let mut free_blocks = Vec::with_capacity(num_blocks);
        for i in 0..num_blocks {
            free_blocks.push(i);
        }

        Self {
            pool_size,
            block_size,
            allocated_blocks: vec![false; num_blocks],
            free_blocks,
            total_allocations: 0,
            total_deallocations: 0,
        }
    }

    /// Allocate a block from the pool
    pub fn allocate(&mut self) -> Option<usize> {
        if let Some(block_index) = self.free_blocks.pop() {
            self.allocated_blocks[block_index] = true;
            self.total_allocations += 1;
            Some(block_index)
        } else {
            None
        }
    }

    /// Deallocate a block back to the pool
    pub fn deallocate(&mut self, block_index: usize) -> Result<()> {
        if block_index >= self.allocated_blocks.len() {
            return Err(PerformanceError::MemoryError(
                "Invalid block index for deallocation".to_string()
            ));
        }

        if !self.allocated_blocks[block_index] {
            return Err(PerformanceError::MemoryError(
                "Block is not allocated".to_string()
            ));
        }

        self.allocated_blocks[block_index] = false;
        self.free_blocks.push(block_index);
        self.total_deallocations += 1;
        Ok(())
    }

    /// Get pool utilization
    pub fn utilization(&self) -> f64 {
        let allocated_count = self.allocated_blocks.iter().filter(|&&allocated| allocated).count();
        allocated_count as f64 / self.allocated_blocks.len() as f64
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> (u64, u64, f64) {
        (self.total_allocations, self.total_deallocations, self.utilization())
    }
}

/// Garbage collector
#[derive(Debug)]
pub struct GarbageCollector {
    gc_threshold_bytes: u64,
    last_gc_time: Instant,
    gc_count: u64,
    total_gc_time: Duration,
    force_gc_interval: Duration,
}

impl GarbageCollector {
    /// Create a new garbage collector
    pub fn new(gc_threshold_bytes: u64) -> Self {
        Self {
            gc_threshold_bytes,
            last_gc_time: Instant::now(),
            gc_count: 0,
            total_gc_time: Duration::new(0, 0),
            force_gc_interval: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Check if garbage collection should be triggered
    pub fn should_gc(&self, current_memory_usage: u64) -> bool {
        current_memory_usage > self.gc_threshold_bytes ||
        self.last_gc_time.elapsed() > self.force_gc_interval
    }

    /// Perform garbage collection
    pub async fn collect(&mut self) -> Result<Duration> {
        debug!("Starting garbage collection");
        let start_time = Instant::now();

        // Simulate garbage collection work
        // In a real implementation, this would:
        // 1. Mark all reachable objects
        // 2. Sweep unreachable objects
        // 3. Compact memory if needed
        tokio::time::sleep(Duration::from_millis(10)).await;

        let gc_duration = start_time.elapsed();
        self.last_gc_time = Instant::now();
        self.gc_count += 1;
        self.total_gc_time += gc_duration;

        info!("Garbage collection completed in {}ms", gc_duration.as_millis());
        Ok(gc_duration)
    }

    /// Get garbage collection statistics
    pub fn get_stats(&self) -> (u64, Duration, Duration) {
        let avg_gc_time = if self.gc_count > 0 {
            self.total_gc_time / self.gc_count as u32
        } else {
            Duration::new(0, 0)
        };
        
        (self.gc_count, avg_gc_time, self.total_gc_time)
    }
}

/// Memory monitor for tracking usage
#[derive(Debug)]
pub struct MemoryMonitor {
    config: MemoryConfig,
    peak_usage: u64,
    samples: Vec<(Instant, u64)>,
    max_samples: usize,
    oom_protection_enabled: bool,
}

impl MemoryMonitor {
    /// Create a new memory monitor
    pub fn new(config: &MemoryConfig) -> Self {
        Self {
            config: config.clone(),
            peak_usage: 0,
            samples: Vec::new(),
            max_samples: 1000,
            oom_protection_enabled: config.oom_protection_enabled,
        }
    }

    /// Record memory usage sample
    pub fn record_usage(&mut self, usage_bytes: u64) {
        let now = Instant::now();
        
        // Update peak usage
        if usage_bytes > self.peak_usage {
            self.peak_usage = usage_bytes;
        }

        // Add sample
        self.samples.push((now, usage_bytes));

        // Keep only recent samples
        if self.samples.len() > self.max_samples {
            self.samples.remove(0);
        }

        // Check for OOM protection
        if self.oom_protection_enabled {
            let max_heap_bytes = self.config.max_heap_size_mb * 1024 * 1024;
            if usage_bytes > (max_heap_bytes * 95 / 100) {
                warn!("Memory usage approaching limit: {} / {} bytes", usage_bytes, max_heap_bytes);
            }
        }
    }

    /// Get memory usage trend
    pub fn get_usage_trend(&self, duration: Duration) -> Option<f64> {
        let cutoff_time = Instant::now() - duration;
        let recent_samples: Vec<_> = self.samples.iter()
            .filter(|(time, _)| *time > cutoff_time)
            .collect();

        if recent_samples.len() < 2 {
            return None;
        }

        let first_usage = recent_samples.first().unwrap().1 as f64;
        let last_usage = recent_samples.last().unwrap().1 as f64;
        
        Some((last_usage - first_usage) / first_usage)
    }

    /// Get average memory usage over duration
    pub fn get_average_usage(&self, duration: Duration) -> Option<f64> {
        let cutoff_time = Instant::now() - duration;
        let recent_samples: Vec<_> = self.samples.iter()
            .filter(|(time, _)| *time > cutoff_time)
            .map(|(_, usage)| *usage as f64)
            .collect();

        if recent_samples.is_empty() {
            return None;
        }

        Some(recent_samples.iter().sum::<f64>() / recent_samples.len() as f64)
    }

    /// Get peak usage
    pub fn get_peak_usage(&self) -> u64 {
        self.peak_usage
    }
}

/// Main memory manager
pub struct MemoryManager {
    config: MemoryConfig,
    memory_pools: Arc<RwLock<Vec<MemoryPool>>>,
    garbage_collector: Arc<RwLock<GarbageCollector>>,
    memory_monitor: Arc<RwLock<MemoryMonitor>>,
    stats: Arc<RwLock<MemoryStats>>,
}

impl MemoryManager {
    /// Create a new memory manager
    pub async fn new(config: &MemoryConfig) -> Result<Self> {
        info!("Initializing memory manager");

        // Initialize memory pools
        let mut pools = Vec::new();
        
        // Create pools for different block sizes
        pools.push(MemoryPool::new(1024 * 1024, 64));    // 1MB pool with 64-byte blocks
        pools.push(MemoryPool::new(1024 * 1024, 256));   // 1MB pool with 256-byte blocks
        pools.push(MemoryPool::new(1024 * 1024, 1024));  // 1MB pool with 1KB blocks
        pools.push(MemoryPool::new(1024 * 1024, 4096));  // 1MB pool with 4KB blocks

        let memory_pools = Arc::new(RwLock::new(pools));

        // Initialize garbage collector
        let gc_threshold = config.gc_threshold_mb * 1024 * 1024;
        let garbage_collector = Arc::new(RwLock::new(GarbageCollector::new(gc_threshold)));

        // Initialize memory monitor
        let memory_monitor = Arc::new(RwLock::new(MemoryMonitor::new(config)));

        // Initialize stats
        let stats = Arc::new(RwLock::new(MemoryStats {
            total_bytes: config.max_heap_size_mb * 1024 * 1024,
            used_bytes: 0,
            free_bytes: config.max_heap_size_mb * 1024 * 1024,
            usage_percent: 0.0,
            heap_size_bytes: config.max_heap_size_mb * 1024 * 1024,
            gc_count: 0,
            last_gc_duration_ms: 0,
            fragmentation_percent: 0.0,
        }));

        info!("Memory manager initialized successfully");
        Ok(Self {
            config: config.clone(),
            memory_pools,
            garbage_collector,
            memory_monitor,
            stats,
        })
    }

    /// Allocate memory from appropriate pool
    pub async fn allocate(&self, size: usize) -> Result<Option<usize>> {
        let mut pools = self.memory_pools.write().await;
        
        // Find appropriate pool based on size
        for pool in pools.iter_mut() {
            if size <= pool.block_size {
                if let Some(block_index) = pool.allocate() {
                    self.update_stats().await;
                    return Ok(Some(block_index));
                }
            }
        }

        // No suitable pool found or all pools full
        warn!("Failed to allocate {} bytes from memory pools", size);
        Ok(None)
    }

    /// Deallocate memory back to pool
    pub async fn deallocate(&self, pool_index: usize, block_index: usize) -> Result<()> {
        let mut pools = self.memory_pools.write().await;
        
        if pool_index >= pools.len() {
            return Err(PerformanceError::MemoryError(
                "Invalid pool index".to_string()
            ));
        }

        pools[pool_index].deallocate(block_index)?;
        self.update_stats().await;
        Ok(())
    }

    /// Force garbage collection
    pub async fn force_gc(&self) -> Result<()> {
        let mut gc = self.garbage_collector.write().await;
        let gc_duration = gc.collect().await?;
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.gc_count += 1;
        stats.last_gc_duration_ms = gc_duration.as_millis() as u64;
        
        Ok(())
    }

    /// Monitor memory usage
    pub async fn monitor_memory(&self) -> Result<()> {
        let current_usage = self.get_current_memory_usage().await?;
        
        // Record usage in monitor
        {
            let mut monitor = self.memory_monitor.write().await;
            monitor.record_usage(current_usage);
        }

        // Check if GC should be triggered
        {
            let gc = self.garbage_collector.read().await;
            if gc.should_gc(current_usage) {
                drop(gc);
                self.force_gc().await?;
            }
        }

        self.update_stats().await;
        Ok(())
    }

    /// Get current memory usage
    async fn get_current_memory_usage(&self) -> Result<u64> {
        // In a real implementation, this would query the system for actual memory usage
        // For now, we'll simulate based on pool utilization
        let pools = self.memory_pools.read().await;
        let mut total_used = 0u64;
        
        for pool in pools.iter() {
            let utilization = pool.utilization();
            total_used += (pool.pool_size as f64 * utilization) as u64;
        }
        
        Ok(total_used)
    }

    /// Update memory statistics
    async fn update_stats(&self) -> Result<()> {
        let current_usage = self.get_current_memory_usage().await?;
        let mut stats = self.stats.write().await;
        
        stats.used_bytes = current_usage;
        stats.free_bytes = stats.total_bytes.saturating_sub(current_usage);
        stats.usage_percent = (current_usage as f64 / stats.total_bytes as f64) * 100.0;
        
        // Calculate fragmentation
        let pools = self.memory_pools.read().await;
        let mut total_fragmentation = 0.0;
        for pool in pools.iter() {
            let utilization = pool.utilization();
            if utilization > 0.0 && utilization < 1.0 {
                total_fragmentation += 1.0 - utilization;
            }
        }
        stats.fragmentation_percent = (total_fragmentation / pools.len() as f64) * 100.0;
        
        Ok(())
    }

    /// Get memory statistics
    pub async fn get_stats(&self) -> Result<MemoryStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    /// Optimize memory usage
    pub async fn optimize(&self) -> Result<()> {
        debug!("Optimizing memory usage");
        
        let stats = self.get_stats().await?;
        
        // Trigger GC if usage is high
        if stats.usage_percent > 80.0 {
            warn!("High memory usage: {:.1}%. Triggering garbage collection.", stats.usage_percent);
            self.force_gc().await?;
        }
        
        // Check fragmentation
        if stats.fragmentation_percent > 50.0 {
            warn!("High memory fragmentation: {:.1}%", stats.fragmentation_percent);
            // In a real implementation, we might trigger memory compaction here
        }
        
        Ok(())
    }

    /// Get memory usage trend
    pub async fn get_usage_trend(&self, duration: Duration) -> Option<f64> {
        let monitor = self.memory_monitor.read().await;
        monitor.get_usage_trend(duration)
    }

    /// Shutdown memory manager
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down memory manager");
        
        // Force final garbage collection
        self.force_gc().await?;
        
        // Clear all pools
        {
            let mut pools = self.memory_pools.write().await;
            pools.clear();
        }
        
        info!("Memory manager shutdown complete");
        Ok(())
    }
}
