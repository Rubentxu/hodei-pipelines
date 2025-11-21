//! Metrics Collection Module (US-023)
//!
//! This module provides Prometheus and OpenTelemetry metrics collection
//! for the Hodei Jobs distributed CI/CD system.
//!
//! Features:
//! - Prometheus metrics server with HTTP /metrics endpoint
//! - Custom business metrics (pipeline success rate, worker utilization)
//! - Technical resource metrics (CPU, memory, disk, network)
//! - Performance tracking metrics (latency, throughput, error rates)

pub mod data_models;
pub mod prometheus;

// Re-export commonly used types
pub use prometheus::{MetricsCollector, MetricsServer, PerformanceMetrics, ResourceMetrics};
