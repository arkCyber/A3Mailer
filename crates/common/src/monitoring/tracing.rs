/*
 * SPDX-FileCopyrightText: 2020 A3Mailer Team Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! # Distributed Tracing
//!
//! This module provides distributed tracing capabilities using OpenTelemetry
//! for comprehensive observability across the entire system.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use serde::{Serialize, Deserialize};
use tracing::{debug, info, warn, Span};

/// Tracing configuration
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Enable distributed tracing
    pub enabled: bool,
    /// Service name for tracing
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// Sampling rate (0.0 to 1.0)
    pub sampling_rate: f64,
    /// OTLP endpoint for trace export
    pub otlp_endpoint: Option<String>,
    /// Additional resource attributes
    pub resource_attributes: HashMap<String, String>,
    /// Enable console exporter for development
    pub enable_console_exporter: bool,
    /// Batch export timeout
    pub batch_timeout: Duration,
    /// Maximum batch size
    pub max_batch_size: usize,
}

impl Default for TracingConfig {
    fn default() -> Self {
        let mut resource_attributes = HashMap::new();
        resource_attributes.insert("service.name".to_string(), "stalwart-mail".to_string());
        resource_attributes.insert("service.version".to_string(), env!("CARGO_PKG_VERSION").to_string());

        Self {
            enabled: true,
            service_name: "stalwart-mail".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            sampling_rate: 0.1, // 10% sampling by default
            otlp_endpoint: None,
            resource_attributes,
            enable_console_exporter: false,
            batch_timeout: Duration::from_secs(5),
            max_batch_size: 512,
        }
    }
}

/// Span context for distributed tracing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanContext {
    /// Trace ID
    pub trace_id: String,
    /// Span ID
    pub span_id: String,
    /// Parent span ID
    pub parent_span_id: Option<String>,
    /// Trace flags
    pub trace_flags: u8,
    /// Baggage items
    pub baggage: HashMap<String, String>,
}

impl SpanContext {
    /// Create a new span context
    pub fn new(trace_id: String, span_id: String) -> Self {
        Self {
            trace_id,
            span_id,
            parent_span_id: None,
            trace_flags: 1, // Sampled
            baggage: HashMap::new(),
        }
    }

    /// Create a child span context
    pub fn create_child(&self, child_span_id: String) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: child_span_id,
            parent_span_id: Some(self.span_id.clone()),
            trace_flags: self.trace_flags,
            baggage: self.baggage.clone(),
        }
    }

    /// Add baggage item
    pub fn add_baggage(&mut self, key: String, value: String) {
        self.baggage.insert(key, value);
    }

    /// Get baggage item
    pub fn get_baggage(&self, key: &str) -> Option<&String> {
        self.baggage.get(key)
    }
}

/// Trace span information
#[derive(Debug, Clone)]
pub struct TraceSpan {
    /// Span context
    pub context: SpanContext,
    /// Operation name
    pub operation_name: String,
    /// Start time
    pub start_time: Instant,
    /// End time
    pub end_time: Option<Instant>,
    /// Span attributes
    pub attributes: HashMap<String, String>,
    /// Span events
    pub events: Vec<SpanEvent>,
    /// Span status
    pub status: SpanStatus,
}

/// Span event
#[derive(Debug, Clone)]
pub struct SpanEvent {
    /// Event name
    pub name: String,
    /// Event timestamp
    pub timestamp: Instant,
    /// Event attributes
    pub attributes: HashMap<String, String>,
}

/// Span status
#[derive(Debug, Clone, PartialEq)]
pub enum SpanStatus {
    /// Span completed successfully
    Ok,
    /// Span completed with error
    Error,
    /// Span is still active
    Active,
}

/// Tracing manager
pub struct TracingManager {
    config: TracingConfig,
    active_spans: Arc<RwLock<HashMap<String, TraceSpan>>>,
    completed_spans: Arc<RwLock<Vec<TraceSpan>>>,
}

impl TracingManager {
    /// Create a new tracing manager
    pub fn new(config: TracingConfig) -> Self {
        info!("Initializing tracing manager with config: {:?}", config);
        Self {
            config,
            active_spans: Arc::new(RwLock::new(HashMap::new())),
            completed_spans: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Start a new trace span
    pub fn start_span(&self, operation_name: String, parent_context: Option<&SpanContext>) -> SpanContext {
        if !self.config.enabled {
            return SpanContext::new("disabled".to_string(), "disabled".to_string());
        }

        let trace_id = if let Some(parent) = parent_context {
            parent.trace_id.clone()
        } else {
            self.generate_trace_id()
        };

        let span_id = self.generate_span_id();
        let context = if let Some(parent) = parent_context {
            parent.create_child(span_id)
        } else {
            SpanContext::new(trace_id, span_id)
        };

        let span = TraceSpan {
            context: context.clone(),
            operation_name: operation_name.clone(),
            start_time: Instant::now(),
            end_time: None,
            attributes: HashMap::new(),
            events: Vec::new(),
            status: SpanStatus::Active,
        };

        debug!("Starting span: {} (trace_id: {}, span_id: {})",
               operation_name, context.trace_id, context.span_id);

        let mut active_spans = self.active_spans.write().unwrap();
        active_spans.insert(context.span_id.clone(), span);

        context
    }

    /// End a trace span
    pub fn end_span(&self, span_id: &str, status: SpanStatus) {
        if !self.config.enabled {
            return;
        }

        let mut active_spans = self.active_spans.write().unwrap();
        if let Some(mut span) = active_spans.remove(span_id) {
            span.end_time = Some(Instant::now());
            span.status = status;

            debug!("Ending span: {} (duration: {:?})",
                   span.operation_name, span.end_time.unwrap() - span.start_time);

            let mut completed_spans = self.completed_spans.write().unwrap();
            completed_spans.push(span);

            // Keep only recent completed spans
            if completed_spans.len() > 10000 {
                completed_spans.drain(0..1000);
            }
        }
    }

    /// Add attribute to a span
    pub fn add_span_attribute(&self, span_id: &str, key: String, value: String) {
        if !self.config.enabled {
            return;
        }

        let mut active_spans = self.active_spans.write().unwrap();
        if let Some(span) = active_spans.get_mut(span_id) {
            span.attributes.insert(key, value);
        }
    }

    /// Add event to a span
    pub fn add_span_event(&self, span_id: &str, event_name: String, attributes: HashMap<String, String>) {
        if !self.config.enabled {
            return;
        }

        let mut active_spans = self.active_spans.write().unwrap();
        if let Some(span) = active_spans.get_mut(span_id) {
            let event = SpanEvent {
                name: event_name,
                timestamp: Instant::now(),
                attributes,
            };
            span.events.push(event);
        }
    }

    /// Get active span count
    pub fn get_active_span_count(&self) -> usize {
        self.active_spans.read().unwrap().len()
    }

    /// Get completed span count
    pub fn get_completed_span_count(&self) -> usize {
        self.completed_spans.read().unwrap().len()
    }

    /// Get recent completed spans
    pub fn get_recent_spans(&self, count: usize) -> Vec<TraceSpan> {
        let completed_spans = self.completed_spans.read().unwrap();
        completed_spans.iter().rev().take(count).cloned().collect()
    }

    /// Generate a new trace ID
    fn generate_trace_id(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        Instant::now().hash(&mut hasher);
        std::thread::current().id().hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Generate a new span ID
    fn generate_span_id(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        Instant::now().hash(&mut hasher);
        std::process::id().hash(&mut hasher);
        format!("{:08x}", hasher.finish() as u32)
    }

    /// Get configuration
    pub fn get_config(&self) -> &TracingConfig {
        &self.config
    }

    /// Cleanup old spans
    pub fn cleanup(&self) {
        debug!("Cleaning up old tracing data");

        // Clean up very old active spans (potential leaks)
        let cutoff_time = Instant::now() - Duration::from_secs(3600); // 1 hour
        let mut active_spans = self.active_spans.write().unwrap();
        let initial_count = active_spans.len();
        active_spans.retain(|_, span| span.start_time > cutoff_time);
        let removed_count = initial_count - active_spans.len();

        if removed_count > 0 {
            warn!("Cleaned up {} potentially leaked active spans", removed_count);
        }

        // Clean up old completed spans
        let mut completed_spans = self.completed_spans.write().unwrap();
        if completed_spans.len() > 5000 {
            let remove_count = completed_spans.len() - 5000;
            completed_spans.drain(0..remove_count);
            info!("Cleaned up {} old completed spans", remove_count);
        }
    }
}

impl Default for TracingManager {
    fn default() -> Self {
        Self::new(TracingConfig::default())
    }
}

/// Convenience macro for creating traced functions
#[macro_export]
macro_rules! traced_function {
    ($tracing_manager:expr, $operation_name:expr, $parent_context:expr, $body:block) => {{
        let context = $tracing_manager.start_span($operation_name.to_string(), $parent_context);
        let result = $body;
        $tracing_manager.end_span(&context.span_id, SpanStatus::Ok);
        result
    }};
}

/// Convenience macro for creating traced async functions
#[macro_export]
macro_rules! traced_async_function {
    ($tracing_manager:expr, $operation_name:expr, $parent_context:expr, $body:block) => {{
        let context = $tracing_manager.start_span($operation_name.to_string(), $parent_context);
        let result = $body.await;
        $tracing_manager.end_span(&context.span_id, SpanStatus::Ok);
        result
    }};
}
