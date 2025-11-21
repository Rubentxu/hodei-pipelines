//! Prometheus Metrics Collection Module (US-023)
//!
//! This module provides Prometheus metrics collection and server implementation.
//!
//! Features:
//! - HTTP server exposing /metrics endpoint
//! - Counter, Gauge, Histogram, Summary metric types
//! - Business and technical metrics registration
//! - Automatic resource metrics collection

mod custom_metrics;
mod metrics_server;
mod performance_metrics;
mod resource_metrics;

pub use custom_metrics::MetricsCollector;
pub use metrics_server::MetricsServer;
pub use performance_metrics::PerformanceMetrics;
pub use resource_metrics::ResourceMetrics;

// Re-export metric types for convenience
pub use prometheus::{Counter, Gauge, Histogram, IntCounter, IntGauge};
